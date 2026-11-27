# SPDX-License-Identifier: GPL-3.0-or-later
"""Builds Grace, the mascot, as a low-poly 3D model that matches the master render.

The only reference is the 2D master render (assets/brand/grace-mascot.png,
1448 x 1086). Every part here is placed from measurements of that image:
X(px) and Z(py) turn image pixels into model units, so a front view of the
model lines up with the picture. Run it headless; nobody needs to open Blender:

    blender -b --factory-startup -P assets/brand/mascot/build_grace.py -- \
        --out assets/brand/mascot/grace.glb [--render DIR] [--compare FILE]

`--compare FILE` renders the front view at the master's size and framing with a
transparent background, for overlaying on the master.

Blender axes: Z is up and Grace faces -Y. She is about 2 units tall.
Node names are a contract with the app (ui/src/mascot/scene.ts):
Grace, Body, ArmL, ArmR, LegL, LegR, Scarf, Tail, Cubes, Cube0, Cube1,
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

# ----- measurements -------------------------------------------------------------

S = 0.0024  # model units per pixel of the master render
CAMERA_PITCH = math.radians(6)  # the master looks down slightly
MASTER_W, MASTER_H = 1448, 1086


def X(px):
    return (px - 690) * S


def Z(py):
    return (950 - py) * S


def D(depth_px):
    """Depth: 0 is the front of the body, negative is toward the viewer."""
    return depth_px * S


def P(px, py, depth_px=0.0):
    return Vector((X(px), D(depth_px), Z(py)))


# Sampled from the master render (lit whites #F5E1D7 to #FEF6EF, teal #03A4A5 to
# #4EB9B7, blacks #262423 to #433A35, pupils #FEC66D, glowing cubes #8EF6F5).
PALETTE = {
    "white": "#F7ECE4",
    "teal": "#26B2B4",
    "black": "#2C2826",
    "amber": "#FEC66D",
    "silver": "#BDB3AE",
    "glow": "#5FE3E3",
}

BODY_AT = P(692.5, 787, 130)
NECK_AT = P(690, 715, 145)
TAIL_AT = P(870, 835, 180)


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


def bevel(bm, offset):
    if offset > 0:
        bmesh.ops.bevel(
            bm,
            geom=list(bm.edges),
            offset=offset,
            segments=1,
            profile=0.5,
            affect="EDGES",
            clamp_overlap=True,
        )
    return bm


def box_bm(size, bevel_by=0.0):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.scale(bm, vec=Vector(size), verts=bm.verts)
    return bevel(bm, bevel_by)


def px_box(x0, x1, z_top, z_bottom, y_front, y_back, bevel_px=0.0):
    """A box from image-pixel bounds; returns (bmesh, centre)."""
    size = (abs(x1 - x0) * S, abs(y_back - y_front) * S, abs(z_bottom - z_top) * S)
    centre = P((x0 + x1) / 2, (z_top + z_bottom) / 2, (y_front + y_back) / 2)
    return box_bm(size, bevel_px * S), centre


def plate(outline_px, y_front, thickness, bevel_px=4.0, tilt=0.0, taper=1.0, yaw=0.0):
    """A flat plate whose front outline is drawn in image pixels. Returns
    (bmesh, centre); tilt leans the bottom toward the viewer and yaw turns the
    right end toward the viewer (radians)."""
    pts = [(X(px), Z(py)) for px, py in outline_px]
    cx = sum(p[0] for p in pts) / len(pts)
    cz = sum(p[1] for p in pts) / len(pts)
    depth = thickness * S
    bm = bmesh.new()
    front = [bm.verts.new((x - cx, -depth / 2, z - cz)) for x, z in pts]
    back = [bm.verts.new(((x - cx) * taper, depth / 2, (z - cz) * taper)) for x, z in pts]
    bm.faces.new(front)
    bm.faces.new(list(reversed(back)))
    for i in range(len(pts)):
        j = (i + 1) % len(pts)
        bm.faces.new((front[i], back[i], back[j], front[j]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bevel(bm, bevel_px * S)
    if tilt:
        bm.transform(Euler((-tilt, 0, 0)).to_matrix().to_4x4())
    if yaw:
        bm.transform(Euler((0, 0, -yaw)).to_matrix().to_4x4())
    return bm, Vector((cx, D(y_front + thickness / 2), cz))


def tube_bm(points, heights, depths, sides=7, twist=0.35):
    """A faceted tube through `points` with elliptical sections (low-poly look)."""
    bm = bmesh.new()
    rings = []
    for i, p in enumerate(points):
        before = points[max(i - 1, 0)]
        after = points[min(i + 1, len(points) - 1)]
        tangent = (after - before).normalized()
        side = tangent.cross(Vector((0, 1, 0)))
        if side.length < 1e-6:
            side = Vector((0, 0, 1))
        side.normalize()
        normal = tangent.cross(side).normalized()
        ring = []
        for k in range(sides):
            a = 2 * math.pi * k / sides + twist * i
            offset = side * math.cos(a) * heights[i] / 2 + normal * math.sin(a) * depths[i] / 2
            ring.append(bm.verts.new(p + offset))
        rings.append(ring)
    for a, b in zip(rings, rings[1:]):
        for k in range(sides):
            m = (k + 1) % sides
            bm.faces.new((a[k], a[m], b[m], b[k]))
    bm.faces.new(rings[0])
    bm.faces.new(list(reversed(rings[-1])))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def loft_bm(points, widths, thicknesses):
    """A flat strand: rectangles of width x thickness swept through `points`,
    with its wide side facing the viewer."""
    bm = bmesh.new()
    rings = []
    toward_viewer = Vector((0, -1, 0))
    for i, p in enumerate(points):
        before = points[max(i - 1, 0)]
        after = points[min(i + 1, len(points) - 1)]
        tangent = (after - before).normalized()
        side = tangent.cross(toward_viewer).normalized()
        normal = side.cross(tangent).normalized()
        w, t = widths[i] / 2, thicknesses[i] / 2
        corners = (p + side * w + normal * t, p - side * w + normal * t,
                   p - side * w - normal * t, p + side * w - normal * t)
        rings.append([bm.verts.new(c) for c in corners])
    for a, b in zip(rings, rings[1:]):
        for k in range(4):
            m = (k + 1) % 4
            bm.faces.new((a[k], a[m], b[m], b[k]))
    bm.faces.new(rings[0])
    bm.faces.new(list(reversed(rings[-1])))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    return bm


def paint(bm, index, where=None):
    """Sets the material of every face (or of faces where `where(face)` is true)."""
    bm.normal_update()
    for face in bm.faces:
        if where is None or where(face):
            face.material_index = index
    return bm


def merge(name, pieces, materials, origin):
    """Combines (bmesh, world centre) pieces into one mesh whose origin is `origin`."""
    out = bmesh.new()
    for bm, centre in pieces:
        bm.transform(Matrix.Translation(Vector(centre) - Vector(origin)))
        temp = bpy.data.meshes.new("temp")
        bm.to_mesh(temp)
        bm.free()
        out.from_mesh(temp)
        bpy.data.meshes.remove(temp)
    return finish_mesh(name, out, materials)


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


# ----- Grace -------------------------------------------------------------------

WHITE, TEAL, BLACK, AMBER, SILVER, GLOW = range(6)


def build(mats):
    palette = [mats[k] for k in ("white", "teal", "black", "amber", "silver", "glow")]
    root = node("Grace", None, (0, 0, 0))

    # Body: a wide white block; the legs are tall boxes flush with its sides.
    bm, centre = px_box(541, 844, 700, 874, 0, 260, bevel_px=10)
    body = node("Body", merge("Body", [(paint(bm, WHITE), centre)], palette, BODY_AT), BODY_AT, root)

    for name, (x0, x1) in (("LegL", (542, 659)), ("LegR", (724, 841))):
        origin = P((x0 + x1) / 2, 874, 130)
        pieces = []
        bm, c = px_box(x0, x1, 852, 894, 70, 190, bevel_px=6)
        pieces.append((paint(bm, WHITE), c))
        bm, c = px_box(x0 - 3, x1 + 3, 892, 909, 66, 194, bevel_px=3)
        pieces.append((paint(bm, TEAL), c))
        bm, c = px_box(x0 - 2, x1 + 2, 908, 950, 64, 196, bevel_px=7)
        pieces.append((paint(bm, BLACK), c))
        node(name, merge(name, pieces, palette, origin), origin, body, BODY_AT)

    # Small arms hidden inside the body's sides at rest; they swing out to wave.
    for name, x in (("ArmL", 556), ("ArmR", 829)):
        shoulder = P(x, 722, 150)
        bm, c = px_box(x - 17, x + 17, 722, 852, 105, 195, bevel_px=8)
        node(name, merge(name, [(paint(bm, WHITE), c)], palette, shoulder), shoulder, body, BODY_AT)

    # Scarf: a black collar plate with a triangular flap, teal diamond, silver
    # buckle, and two loose ends behind her on the left.
    scarf_at = P(690, 760, 0)
    pieces = []
    bm, c = px_box(527, 854, 715, 781, -12, 262, bevel_px=6)
    pieces.append((paint(bm, BLACK), c))
    bm, c = plate([(531, 717), (850, 717), (850, 780), (690, 845), (531, 780)], -30, 18, bevel_px=5)
    pieces.append((paint(bm, BLACK), c))
    ring = bmesh.new()
    o = [ring.verts.new((X(x), 0, Z(z))) for x, z in ((690, 750), (719, 781), (690, 812), (661, 781))]
    i_ = [ring.verts.new((X(x), 0, Z(z))) for x, z in ((690, 762), (707, 781), (690, 800), (673, 781))]
    for k in range(4):
        m = (k + 1) % 4
        ring.faces.new((o[k], o[m], i_[m], i_[k]))
    extruded = bmesh.ops.extrude_face_region(ring, geom=list(ring.faces))
    moved = [g for g in extruded["geom"] if isinstance(g, bmesh.types.BMVert)]
    bmesh.ops.translate(ring, vec=(0, -7 * S, 0), verts=moved)
    pieces.append((paint(ring, TEAL), Vector((0, D(-30), 0))))
    for bar in ((816, 825, 730, 796), (850, 859, 730, 796), (816, 859, 730, 739), (816, 859, 787, 796), (835, 840, 745, 781)):
        bm, c = px_box(bar[0], bar[1], bar[2], bar[3], -38, -28, bevel_px=2)
        pieces.append((paint(bm, SILVER), c))
    for outline, y in (
        ([(546, 733), (504, 742), (480, 802), (521, 834), (550, 792)], 40),
        ([(560, 762), (522, 788), (531, 848), (566, 822)], 95),
    ):
        bm, c = plate(outline, y, 26, bevel_px=4)
        pieces.append((paint(bm, BLACK), c))
    node("Scarf", merge("Scarf", pieces, palette, scarf_at), scarf_at, body, BODY_AT)

    # Tail: one chunky faceted curl. Black near her body with a teal band, a teal
    # fin on top, and a teal face at the tip.
    tail = node("Tail", None, TAIL_AT, body, BODY_AT)
    path = [P(860, 830, 185), P(960, 845, 190), P(1060, 845, 190), P(1140, 815, 180), P(1180, 760, 170), P(1186, 688, 160)]
    heights = [x * S for x in (150, 160, 160, 140, 118, 92)]
    depths = [x * S for x in (140, 148, 146, 128, 108, 82)]
    # One continuous faceted tube (no joints), teal on the tip's outer faces.
    bm = tube_bm(path, heights, depths, sides=7, twist=0.25)
    paint(bm, WHITE)
    paint(bm, TEAL, lambda f: f.calc_center_median().x > X(1158) and f.normal.x > 0.45)
    pieces = [(bm, Vector((0, 0, 0)))]
    bm, c = plate([(1030, 755), (1098, 690), (1114, 700), (1068, 762)], 140, 26, bevel_px=3)
    pieces.append((paint(bm, TEAL), c))
    # The scarf's long end hanging down behind her right hip: black, teal edge.
    bm, c = plate([(856, 745), (962, 758), (1000, 792), (996, 896), (936, 868), (846, 802)], 60, 40, bevel_px=5)
    pieces.append((paint(bm, BLACK), c))
    bm, c = plate([(968, 788), (1010, 792), (1006, 900), (982, 900)], 56, 44, bevel_px=3)
    pieces.append((paint(bm, TEAL), c))
    node("TailMesh", merge("TailMesh", pieces, palette, TAIL_AT), TAIL_AT, tail, TAIL_AT)

    cubes = node("Cubes", None, (0, 0, 0), root)
    for k, (cx, cz, size) in enumerate(((1216, 613, 50), (1241, 690, 36))):
        bm = box_bm((size * S,) * 3, size * S * 0.07)
        node(f"Cube{k}", merge(f"Cube{k}", [(paint(bm, GLOW), Vector((0, 0, 0)))], palette, (0, 0, 0)),
             P(cx, cz, 40), cubes, (0, 0, 0), (0.12, 0.3 + 0.2 * k, 0.1))

    # Head --------------------------------------------------------------------------
    look = node("LookPivot", None, NECK_AT, root)
    pieces = []
    bm, c = px_box(482, 902, 335, 715, -70, 360, bevel_px=16)  # the face block
    pieces.append((paint(bm, WHITE), c))
    # Hair cap: its top face shows because the camera looks down ~6 degrees, so the
    # back edge appears ~40 px higher than the front (master: 290 back, 340 front).
    bm, c = px_box(470, 910, 326, 352, -84, 372, bevel_px=10)
    pieces.append((paint(bm, WHITE), c))
    bm, c = px_box(470, 910, 330, 722, 250, 382, bevel_px=12)  # back hair
    pieces.append((paint(bm, WHITE), c))
    # Bangs: layered flat strands with straight cuts, tips just above and between the eyes.
    bm, c = plate([(468, 330), (912, 330), (912, 372), (468, 372)], -92, 16, bevel_px=4)
    pieces.append((paint(bm, WHITE), c))
    for outline, y, tilt in (
        ([(478, 335), (650, 335), (564, 457), (473, 589)], -96, 0.05),
        ([(640, 335), (760, 335), (765, 611), (645, 557)], -94, 0.04),
        ([(745, 335), (892, 335), (895, 576), (745, 495)], -98, 0.06),
        ([(598, 335), (700, 335), (654, 598), (566, 455)], -104, 0.07),
    ):
        bm, c = plate(outline, y, 22, bevel_px=4, tilt=tilt)
        pieces.append((paint(bm, WHITE), c))
    # Her right side (viewer's left): big white flares with a teal strand underneath.
    for outline, y, thick, material, tilt in (
        ([(476, 452), (362, 486), (366, 566), (428, 656), (476, 566)], -40, 90, TEAL, 0.0),
        ([(486, 333), (442, 330), (318, 452), (372, 522), (480, 470)], -82, 70, WHITE, 0.12),
        ([(470, 396), (390, 430), (312, 560), (402, 548), (476, 470)], -70, 60, WHITE, -0.1),
        ([(476, 540), (400, 560), (322, 612), (398, 640), (470, 600)], -60, 50, WHITE, 0.06),
        ([(484, 470), (408, 556), (444, 700), (470, 772), (486, 690)], -86, 70, WHITE, 0.08),
        ([(482, 560), (430, 600), (452, 740), (480, 700)], -96, 40, WHITE, 0.15),
        ([(470, 500), (380, 562), (398, 700), (448, 762)], 20, 150, WHITE, 0.0),
    ):
        bm, c = plate(outline, y, thick, bevel_px=5, taper=0.92, tilt=tilt)
        pieces.append((paint(bm, material), c))
    # Her left side (viewer's right): white locks under a broad diagonal teal band.
    for outline, y, thick, material in (
        ([(900, 340), (980, 420), (1046, 560), (1040, 772), (992, 722), (930, 560)], -60, 180, WHITE),
        ([(905, 520), (1000, 600), (1030, 782), (962, 742)], 40, 160, WHITE),
        ([(944, 600), (1004, 600), (998, 742), (970, 742)], -70, 50, TEAL),
        ([(778, 333), (862, 333), (1020, 560), (1016, 618), (940, 606)], -106, 24, TEAL),
    ):
        bm, c = plate(outline, y, thick, bevel_px=5)
        pieces.append((paint(bm, material), c))
    head = node("Head", merge("Head", pieces, palette, NECK_AT), NECK_AT, look, NECK_AT)

    # Face details, kept as separate nodes so the app can animate them.
    for side, eye, pupil, cheek in (("L", "EyeL", "PupilL", "CheekL"), ("R", "EyeR", "PupilR", "CheekR")):
        ex = 572.5 if side == "L" else 810
        eye_at = P(ex, 620.8, -74)
        bm = box_bm((70 * S, 8 * S, 110.4 * S), 8 * S)
        eye_obj = node(eye, merge(eye, [(paint(bm, BLACK), Vector((0, 0, 0)))], palette, (0, 0, 0)), eye_at, head, NECK_AT)
        pupil_at = P(ex + 0.5, 632.75, -80)
        bm = box_bm((22 * S, 5 * S, 59.5 * S), 3 * S)
        node(pupil, merge(pupil, [(paint(bm, AMBER), Vector((0, 0, 0)))], palette, (0, 0, 0)), pupil_at, eye_obj, eye_at)
        cx = 498 if side == "L" else 879
        cheek_at = P(cx, 666, -73)
        pieces = []
        for size in ((41, 5, 12), (12, 5, 41)):
            bm = box_bm(tuple(v * S for v in size), 2 * S)
            pieces.append((paint(bm, TEAL), Vector((0, 0, 0))))
        node(cheek, merge(cheek, pieces, palette, (0, 0, 0)), cheek_at, head, NECK_AT)

    # The top strand (ahoge): a flat white blade rising and curving to her left,
    # with a teal upper face near the tip.
    base = P(676, 342, 60)
    points = [P(680, 342, 60), P(692, 286, 60), P(708, 236, 60), P(735, 196, 60), P(770, 160, 60), P(806, 128, 60)]
    bm = loft_bm(points, [x * S for x in (70, 66, 62, 56, 46, 30)], [20 * S] * 6)
    paint(bm, WHITE)
    paint(bm, TEAL, lambda f: f.calc_center_median().x > X(758))
    node("Ahoge", merge("Ahoge", [(bm, Vector((0, 0, 0)))], palette, base), base, head, NECK_AT)

    # The bow: three blocky black lobes with teal outer faces.
    bow_at = P(952, 390, 150)
    pieces = []
    bm, c = px_box(918, 986, 350, 430, 100, 200, bevel_px=8)
    pieces.append((paint(bm, BLACK), c))
    for outline in (
        [(915, 405), (930, 268), (1012, 248), (1010, 330), (985, 408)],
        [(955, 340), (1100, 312), (1126, 412), (960, 428)],
        [(950, 402), (1082, 455), (1130, 552), (1086, 572), (940, 452)],
    ):
        bm, c = plate(outline, 90, 120, bevel_px=7, taper=0.94, yaw=0.45)
        paint(bm, BLACK)
        paint(bm, TEAL, lambda f: f.normal.x > 0.45)
        pieces.append((bm, c))
    node("Bow", merge("Bow", pieces, palette, bow_at), bow_at, head, NECK_AT)

    # The white cross clip on the teal band.
    clip_at = P(945, 487, -112)
    pieces = []
    for angle in (math.pi / 4, -math.pi / 4):
        bm = box_bm((74 * S, 14 * S, 22 * S), 4 * S)
        bm.transform(Euler((0, angle, 0)).to_matrix().to_4x4())
        pieces.append((paint(bm, WHITE), Vector((0, 0, 0))))
    node("Clip", merge("Clip", pieces, palette, (0, 0, 0)), clip_at, head, NECK_AT)
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


def setup_render(width, height, transparent):
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_WORKBENCH"
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "MATERIAL"
    shading.show_shadows = False
    shading.show_cavity = True
    scene.render.resolution_x, scene.render.resolution_y = width, height
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = transparent
    if scene.world is None:
        scene.world = bpy.data.worlds.new("bg")
    scene.world.color = linear_rgba("#E8DCD2")[:3]
    shading.background_type = "WORLD"
    return scene


def camera(name, ortho_scale=None, lens=85):
    cam = bpy.data.objects.new(name, bpy.data.cameras.new(name))
    if ortho_scale:
        cam.data.type = "ORTHO"
        cam.data.ortho_scale = ortho_scale
    else:
        cam.data.lens = lens
    bpy.context.scene.collection.objects.link(cam)
    bpy.context.scene.camera = cam
    return cam


def render_compare(path):
    """Front view framed like the master: 1 px of the render = 1 px of the master."""
    scene = setup_render(MASTER_W, MASTER_H, transparent=True)
    cam = camera("compare", ortho_scale=MASTER_W * S)
    target = P(MASTER_W / 2, MASTER_H / 2, 0)
    cam.location = target + Vector((0, -math.cos(CAMERA_PITCH), math.sin(CAMERA_PITCH))) * 20
    look_at(cam, target)
    scene.render.filepath = path
    bpy.ops.render.render(write_still=True)


def render_previews(out_dir):
    os.makedirs(out_dir, exist_ok=True)
    scene = setup_render(720, 540, transparent=False)
    cam = camera("preview", lens=85)
    for name, location in (("three_quarter", (3.0, -7.0, 2.0)), ("side", (7.8, 0.4, 1.4)), ("back", (-1.5, 7.6, 1.7))):
        cam.location = location
        look_at(cam, (0.25, 0.3, 1.0))
        scene.render.filepath = os.path.join(out_dir, f"grace-{name}.png")
        bpy.ops.render.render(write_still=True)


def main():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    parser.add_argument("--render")
    parser.add_argument("--compare")
    parser.add_argument("--blend")
    args = parser.parse_args(argv)

    clear_scene()
    materials = {
        "white": make_material("white", PALETTE["white"], roughness=0.55),
        "teal": make_material("teal", PALETTE["teal"], roughness=0.45),
        "black": make_material("black", PALETTE["black"], roughness=0.6),
        "amber": make_material("amber", PALETTE["amber"], roughness=0.4, emission=0.6),
        "silver": make_material("silver", PALETTE["silver"], roughness=0.35, metallic=0.6),
        "glow": make_material("glow", PALETTE["glow"], roughness=0.4, emission=0.25),
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
    if args.compare:
        render_compare(os.path.abspath(args.compare))
    if args.render:
        render_previews(os.path.abspath(args.render))
    print("Grace built:", os.path.abspath(args.out))


if __name__ == "__main__":
    main()
