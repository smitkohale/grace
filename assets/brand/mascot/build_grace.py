# SPDX-License-Identifier: GPL-3.0-or-later
"""Builds Grace, the mascot, as a low-poly 3D model.

The only reference is the 2D master render (assets/brand/grace-mascot.png), so
Grace is assembled here from simple shapes, with flat colours from the palette
in docs/DESIGN.md. Run it headless; nobody needs to open Blender:

    blender -b --factory-startup -P assets/brand/mascot/build_grace.py -- \
        --out assets/brand/mascot/grace.glb [--render DIR] [--blend FILE]

Blender axes: Z is up and Grace faces -Y. She is about 2 units tall.
Node names are a contract with the app (ui/src/mascot/scene.ts):
Grace, Body, ArmL, ArmR, LegL, LegR, Scarf, Tail, TailSeg0..5, Cubes, Cube0..3,
LookPivot, Head, EyeL, EyeR, PupilL, PupilR, CheekL, CheekR, Ahoge, Bow, Clip.
Every parent node is unrotated, so a child's position is simply its world
position minus its parent's.

This script is GPL-3.0-or-later because it runs inside Blender. The model and
images it produces are Grace's own artwork (Apache-2.0, like the rest of the repo).
"""

import argparse
import math
import os
import sys

import bmesh
import bpy
from mathutils import Euler, Matrix, Vector

PALETTE = {
    "white": "#F3EEE9",
    "teal": "#55C9CD",
    "black": "#2E2C2F",
    "amber": "#FCC067",
    "silver": "#A6AAB0",
}

BODY_AT = Vector((0, 0, 0.325))
NECK_AT = Vector((0, 0, 0.66))
TAIL_AT = Vector((0.24, 0.24, 0.2))


# ----- materials ---------------------------------------------------------------


def srgb_to_linear(channel):
    return channel / 12.92 if channel <= 0.04045 else ((channel + 0.055) / 1.055) ** 2.4


def linear_rgba(hex_color):
    hex_color = hex_color.lstrip("#")
    rgb = [srgb_to_linear(int(hex_color[i : i + 2], 16) / 255) for i in (0, 2, 4)]
    return (*rgb, 1.0)


def make_material(name, hex_color, roughness=0.6, metallic=0.0, emission=0.0):
    mat = bpy.data.materials.new(name)
    if hasattr(mat, "use_nodes"):
        try:
            mat.use_nodes = True
        except (AttributeError, TypeError):
            pass
    color = linear_rgba(hex_color)
    mat.diffuse_color = color
    nodes = mat.node_tree.nodes
    bsdf = next((n for n in nodes if n.type == "BSDF_PRINCIPLED"), None)
    if bsdf is None:
        bsdf = nodes.new("ShaderNodeBsdfPrincipled")
        output = next(n for n in nodes if n.type == "OUTPUT_MATERIAL")
        mat.node_tree.links.new(bsdf.outputs[0], output.inputs[0])
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    if emission > 0:
        bsdf.inputs["Emission Color"].default_value = color
        bsdf.inputs["Emission Strength"].default_value = emission
    return mat


# ----- geometry helpers --------------------------------------------------------


def finish_mesh(name, bm, materials):
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    for mat in materials:
        mesh.materials.append(mat)
    return mesh


def box_bm(size, bevel=0.0):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.scale(bm, vec=Vector(size), verts=bm.verts)
    if bevel > 0:
        bmesh.ops.bevel(
            bm, geom=list(bm.edges), offset=bevel, segments=1, profile=0.5, affect="EDGES"
        )
    return bm


def box(name, size, material, bevel=0.0):
    return finish_mesh(name, box_bm(size, bevel), [material])


def prism_bm(outline, depth, taper=1.0):
    """Extrudes a polygon drawn in the XZ plane (front view) along Y, centred on y=0.
    `taper` scales the back face around the polygon's centre (1 = straight)."""
    bm = bmesh.new()
    cx = sum(p[0] for p in outline) / len(outline)
    cz = sum(p[1] for p in outline) / len(outline)
    front = [bm.verts.new((x, -depth / 2, z)) for x, z in outline]
    back = [
        bm.verts.new((cx + (x - cx) * taper, depth / 2, cz + (z - cz) * taper)) for x, z in outline
    ]
    bm.faces.new(front)
    bm.faces.new(list(reversed(back)))
    count = len(outline)
    for i in range(count):
        j = (i + 1) % count
        bm.faces.new((front[i], back[i], back[j], front[j]))
    return bm


def mark_side_faces(bm, material_index, threshold=0.6):
    """Gives faces that don't point front or back (along Y) another material."""
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.normal_update()
    for face in bm.faces:
        if abs(face.normal.y) < threshold:
            face.material_index = material_index


def loft_bm(points, widths, thicknesses):
    """A strand: rectangles of the given width/thickness swept through `points`."""
    bm = bmesh.new()
    rings = []
    up = Vector((0, -1, 0))
    for i, point in enumerate(points):
        p = Vector(point)
        before = Vector(points[max(i - 1, 0)])
        after = Vector(points[min(i + 1, len(points) - 1)])
        tangent = (after - before).normalized()
        side = tangent.cross(up)
        if side.length < 1e-6:
            side = Vector((1, 0, 0))
        side.normalize()
        normal = side.cross(tangent).normalized()
        w, t = widths[i] / 2, thicknesses[i] / 2
        corners = (
            p + side * w + normal * t,
            p - side * w + normal * t,
            p - side * w - normal * t,
            p + side * w - normal * t,
        )
        rings.append([bm.verts.new(c) for c in corners])
    for a, b in zip(rings, rings[1:]):
        for k in range(4):
            m = (k + 1) % 4
            bm.faces.new((a[k], a[m], b[m], b[k]))
    bm.faces.new(rings[0])
    bm.faces.new(list(reversed(rings[-1])))
    return bm


def merge_bms(name, pieces, materials):
    """Combines (bmesh, location, rotation, material_index_offset) pieces into one mesh."""
    out = bmesh.new()
    for bm, location, rotation in pieces:
        matrix = Matrix.Translation(Vector(location)) @ Euler(rotation).to_matrix().to_4x4()
        bm.transform(matrix)
        temp = bpy.data.meshes.new("temp")
        bm.to_mesh(temp)
        bm.free()
        out.from_mesh(temp)
        bpy.data.meshes.remove(temp)
    return finish_mesh(name, out, materials)


def with_material(bm, index):
    for face in bm.faces:
        face.material_index = index
    return bm


def node(name, data, world, parent=None, parent_world=Vector((0, 0, 0)), rotation=(0, 0, 0)):
    """Links an object at `world` under an unrotated `parent` located at `parent_world`."""
    obj = bpy.data.objects.new(name, data)
    if data is None:
        obj.empty_display_size = 0.1
    bpy.context.scene.collection.objects.link(obj)
    obj.parent = parent
    obj.location = Vector(world) - Vector(parent_world)
    obj.rotation_euler = Euler(rotation)
    return obj


def centred(mesh, origin):
    """Moves a mesh built in world coordinates so `origin` becomes its origin."""
    mesh.transform(Matrix.Translation(-Vector(origin)))
    return mesh


# ----- Grace -------------------------------------------------------------------


def build(mat):
    W, T, K, A, S = mat["white"], mat["teal"], mat["black"], mat["amber"], mat["silver"]
    root = node("Grace", None, (0, 0, 0))

    # Body, legs and arms -------------------------------------------------------
    body = node("Body", box("Body", (0.52, 0.4, 0.27), W, bevel=0.035), BODY_AT, root)

    for side, name in ((-1, "LegL"), (1, "LegR")):
        x = 0.19 * side
        leg_at = Vector((x, 0, 0.21))
        mesh = merge_bms(
            name,
            [
                (with_material(box_bm((0.225, 0.27, 0.17), 0.03), 0), (x, -0.01, 0.085), (0, 0, 0)),
                (with_material(box_bm((0.215, 0.255, 0.04), 0.006), 1), (x, -0.005, 0.19), (0, 0, 0)),
                (with_material(box_bm((0.195, 0.22, 0.05), 0.004), 2), (x, 0, 0.23), (0, 0, 0)),
            ],
            [K, T, W],
        )
        node(name, centred(mesh, leg_at), leg_at, body, BODY_AT)

    # Small arms tucked flush against her sides: hidden from the front at rest,
    # they swing out to wave or hold the clipboard.
    for side, name in ((-1, "ArmL"), (1, "ArmR")):
        shoulder = Vector((0.272 * side, 0.05, 0.43))
        arm = box(name, (0.06, 0.14, 0.19), W, bevel=0.015)
        arm.transform(Matrix.Translation((0, 0, -0.095)))  # origin at the shoulder
        node(name, arm, shoulder, body, BODY_AT)

    # Scarf: band, front flap with the teal diamond, buckle, trailing ends -------
    band = bmesh.new()
    bmesh.ops.create_cone(band, cap_ends=True, segments=8, radius1=0.33, radius2=0.31, depth=0.11)
    diamond = bmesh.new()
    outer, inner = 0.058, 0.034
    o = [diamond.verts.new(v) for v in ((0, 0, outer), (outer, 0, 0), (0, 0, -outer), (-outer, 0, 0))]
    i_ = [diamond.verts.new(v) for v in ((0, 0, inner), (inner, 0, 0), (0, 0, -inner), (-inner, 0, 0))]
    for k in range(4):
        m = (k + 1) % 4
        diamond.faces.new((o[k], o[m], i_[m], i_[k]))
    extruded = bmesh.ops.extrude_face_region(diamond, geom=list(diamond.faces))
    moved = [g for g in extruded["geom"] if isinstance(g, bmesh.types.BMVert)]
    bmesh.ops.translate(diamond, vec=(0, -0.016, 0), verts=moved)
    scarf_pieces = [
        (with_material(band, 0), (0, 0.02, 0.615), (0, 0, math.pi / 8)),
        (with_material(prism_bm([(-0.33, 0.665), (0.33, 0.665), (0.0, 0.405)], 0.07), 0), (0, -0.315, 0), (0, 0, 0)),
        (with_material(diamond, 1), (0, -0.352, 0.555), (0, 0, 0)),
        (with_material(box_bm((0.075, 0.035, 0.1), 0.008), 2), (0.315, -0.285, 0.515), (0, 0.25, 0)),
    ]
    for lx, lz, rot in ((-0.42, 0.52, 0.6), (-0.47, 0.44, 0.95)):
        end = prism_bm([(-0.06, 0.08), (0.08, 0.06), (0.03, -0.12), (-0.07, -0.08)], 0.04)
        scarf_pieces.append((with_material(end, 0), (lx, 0.12, lz), (0.2, rot, 0.5)))
    scarf_at = Vector((0, 0, 0.6))
    node("Scarf", centred(merge_bms("Scarf", scarf_pieces, [K, T, S]), scarf_at), scarf_at, body, BODY_AT)

    # Tail: a chunky curl of faceted segments, plus floating cubes ----------------
    tail = node("Tail", None, TAIL_AT, body, BODY_AT)
    path = [
        (0.30, 0.26, 0.21), (0.52, 0.32, 0.19), (0.75, 0.34, 0.23),
        (0.95, 0.31, 0.34), (1.06, 0.26, 0.5), (1.1, 0.21, 0.67),
    ]
    sizes = [0.28, 0.27, 0.25, 0.22, 0.19, 0.15]
    for k, (point, size) in enumerate(zip(path, sizes)):
        before = Vector(path[max(k - 1, 0)])
        after = Vector(path[min(k + 1, len(path) - 1)])
        direction = (after - before).normalized()
        bm = box_bm((size * 1.25, size * 0.95, size), bevel=size * 0.12)
        bm.normal_update()
        for face in bm.faces:
            if k in (1, 4) and face.normal.z > 0.5:
                face.material_index = 1  # teal accent on top
            if k == 0 and face.normal.y < -0.5:
                face.material_index = 2  # black accent near the body
        angle = math.atan2(direction.z, direction.x)
        node(f"TailSeg{k}", finish_mesh(f"TailSeg{k}", bm, [W, T, K]), point, tail, TAIL_AT, (0, -angle, 0.1 * k))

    cubes = node("Cubes", None, (0, 0, 0), root)
    for k, (loc, size) in enumerate(
        (((1.16, 0.14, 0.98), 0.075), ((1.27, 0.14, 0.84), 0.06), ((1.05, 0.1, 1.1), 0.045), ((1.32, 0.2, 1.02), 0.035))
    ):
        node(f"Cube{k}", box(f"Cube{k}", (size,) * 3, T, bevel=size * 0.08), loc, cubes, (0, 0, 0), (0.3 * k, 0.5, 0.2 * k))

    # Head -------------------------------------------------------------------------
    look = node("LookPivot", None, NECK_AT, root)
    # A low, wide hair mass on a rounded block, not a hat.
    cap = box_bm((1.2, 1.06, 0.34), bevel=0.08)
    for v in cap.verts:
        if v.co.z > 0.1:
            v.co.x *= 0.95
            v.co.y *= 0.95
    head_pieces = [
        (with_material(box_bm((1.0, 0.86, 0.9), 0.06), 0), (0, 0.02, 1.08), (0, 0, 0)),
        (with_material(cap, 0), (0, 0.03, 1.45), (0, 0, 0)),
        (with_material(box_bm((1.18, 0.5, 0.82), 0.08), 0), (0, 0.27, 1.12), (0, 0, 0)),
    ]
    # Bangs: layered shards hanging from the hairline, tips tilted forward so their
    # facets catch the light like the render's hair. Outlines hang below z=0.
    hairline = 1.62
    head_pieces.append((with_material(prism_bm([(-0.6, 0), (0.56, 0), (0.56, -0.2), (-0.6, -0.2)], 0.09, taper=0.9), 0), (0, -0.53, hairline), (0, 0, 0)))
    for k, (outline, tilt, turn) in enumerate((
        ([(-0.62, 0.0), (-0.22, 0.0), (-0.52, -0.64)], 0.16, 0.05),
        ([(-0.42, 0.0), (0.0, 0.0), (-0.17, -0.55)], 0.2, -0.04),
        ([(-0.1, 0.0), (0.32, 0.0), (0.06, -0.5)], 0.18, 0.06),
        ([(0.12, 0.0), (0.55, 0.0), (0.38, -0.6)], 0.14, -0.05),
        ([(-0.24, 0.0), (0.16, 0.0), (-0.04, -0.4)], 0.24, 0.0),
    )):
        bm = with_material(prism_bm(outline, 0.1, taper=0.72), 0)
        head_pieces.append((bm, (0, -0.56 - 0.02 * (k % 3), hairline), (-tilt, 0, turn)))
    # Side locks: white on her right (viewer's left) with a teal underside; the long
    # teal streak on her left (viewer's right), with white locks behind it.
    for outline, depth, y, front, sides in (
        ([(-0.66, 1.46), (-0.44, 1.46), (-0.5, 0.55), (-0.7, 0.78)], 0.3, -0.24, 0, 1),
        ([(-0.72, 1.36), (-0.56, 1.36), (-0.74, 0.62)], 0.34, -0.06, 1, 0),
        ([(0.3, 1.5), (0.56, 1.46), (0.5, 0.56), (0.36, 0.86)], 0.12, -0.42, 1, 1),
        ([(0.5, 1.44), (0.7, 1.42), (0.72, 0.6), (0.52, 0.78)], 0.3, -0.22, 0, 1),
    ):
        bm = with_material(prism_bm(outline, depth, taper=0.9), front)
        mark_side_faces(bm, sides)
        head_pieces.append((bm, (0, y, 0), (0, 0, 0)))
    head = node("Head", centred(merge_bms("Head", head_pieces, [W, T]), NECK_AT), NECK_AT, look, NECK_AT)

    # Face details: separate nodes so the app can animate them.
    for side, eye, pupil, cheek in ((-1, "EyeL", "PupilL", "CheekL"), (1, "EyeR", "PupilR", "CheekR")):
        eye_at = Vector((0.27 * side, -0.44, 0.94))
        eye_obj = node(eye, box(eye, (0.165, 0.03, 0.25), K, bevel=0.025), eye_at, head, NECK_AT)
        node(pupil, box(pupil, (0.034, 0.02, 0.115), A), eye_at + Vector((0, -0.012, 0)), eye_obj, eye_at)
        plus = merge_bms(
            cheek,
            [
                (box_bm((0.08, 0.016, 0.022)), (0, 0, 0), (0, 0, 0)),
                (box_bm((0.022, 0.016, 0.08)), (0, 0, 0), (0, 0, 0)),
            ],
            [T],
        )
        node(cheek, plus, (0.465 * side, -0.445, 0.79), head, NECK_AT)

    # The top strand (ahoge): a flat white blade curving to her left, teal tip.
    base = Vector((0, 0.02, 1.6))
    points = [(0.0, 0.02, 1.6), (0.01, 0.01, 1.71), (0.04, -0.01, 1.81), (0.1, -0.03, 1.88), (0.19, -0.05, 1.91), (0.28, -0.06, 1.88)]
    bm = loft_bm(points, [0.17, 0.16, 0.15, 0.14, 0.12, 0.08], [0.045, 0.04, 0.035, 0.03, 0.026, 0.02])
    for face in bm.faces:
        if face.calc_center_median().x > 0.14:
            face.material_index = 1
    node("Ahoge", centred(finish_mesh("Ahoge", bm, [W, T]), base), base, head, NECK_AT)

    # The bow on her left side: black loops with teal edges.
    bow_at = Vector((0.55, 0.06, 1.42))
    bow_pieces = []
    for outline in (
        [(0.0, 0.02), (0.3, 0.22), (0.37, 0.05), (0.05, -0.04)],
        [(0.0, -0.03), (0.07, -0.07), (0.38, -0.32), (0.3, -0.39)],
    ):
        bm = with_material(prism_bm(outline, 0.18, taper=0.9), 0)
        mark_side_faces(bm, 1)
        bow_pieces.append((bm, bow_at, (0, 0.1, -0.3)))
    bow_pieces.append((with_material(box_bm((0.12, 0.16, 0.14), 0.02), 0), bow_at, (0, 0, 0)))
    node("Bow", centred(merge_bms("Bow", bow_pieces, [K, T]), bow_at), bow_at, head, NECK_AT)

    # The white cross clip on the teal streak.
    clip_at = Vector((0.43, -0.49, 1.2))
    clip = merge_bms(
        "Clip",
        [
            (box_bm((0.15, 0.03, 0.045), 0.008), clip_at, (0, 0.785, 0)),
            (box_bm((0.15, 0.03, 0.045), 0.008), clip_at, (0, -0.785, 0)),
        ],
        [W],
    )
    node("Clip", centred(clip, clip_at), clip_at, head, NECK_AT)
    return root


# ----- scene, export, previews ---------------------------------------------------


def clear_scene():
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)
    for block in (bpy.data.meshes, bpy.data.materials, bpy.data.cameras, bpy.data.lights):
        for item in list(block):
            block.remove(item)


def look_at(obj, target):
    direction = Vector(target) - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def render_previews(out_dir):
    scene = bpy.context.scene
    os.makedirs(out_dir, exist_ok=True)
    scene.render.engine = "BLENDER_WORKBENCH"
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "MATERIAL"
    shading.show_shadows = True
    shading.show_cavity = True
    scene.render.resolution_x, scene.render.resolution_y = 720, 540
    scene.render.film_transparent = False
    if scene.world is None:
        scene.world = bpy.data.worlds.new("bg")
    scene.world.color = linear_rgba("#E8DCD2")[:3]
    shading.background_type = "WORLD"
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    cam.data.lens = 85
    scene.collection.objects.link(cam)
    scene.camera = cam
    for name, location in (
        ("front", (0.0, -7.6, 1.6)),
        ("three_quarter", (2.8, -7.0, 1.9)),
        ("side", (7.6, 0.0, 1.4)),
        ("back", (-1.5, 7.4, 1.6)),
    ):
        cam.location = location
        look_at(cam, (0.25, 0, 1.0))
        scene.render.filepath = os.path.join(out_dir, f"grace-{name}.png")
        bpy.ops.render.render(write_still=True)


def main():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--render")
    parser.add_argument("--blend")
    args = parser.parse_args(argv)

    clear_scene()
    materials = {
        "white": make_material("white", PALETTE["white"], roughness=0.65),
        "teal": make_material("teal", PALETTE["teal"], roughness=0.5),
        "black": make_material("black", PALETTE["black"], roughness=0.55),
        "amber": make_material("amber", PALETTE["amber"], roughness=0.4, emission=0.7),
        "silver": make_material("silver", PALETTE["silver"], roughness=0.35, metallic=0.7),
    }
    build(materials)
    bpy.context.view_layer.update()

    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=os.path.abspath(args.out),
        export_format="GLB",
        export_apply=True,
        export_yup=True,
        export_animations=False,
        export_cameras=False,
        export_lights=False,
    )
    if args.blend:
        bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(args.blend))
    if args.render:
        render_previews(os.path.abspath(args.render))
    print("Grace built:", os.path.abspath(args.out))


if __name__ == "__main__":
    main()
