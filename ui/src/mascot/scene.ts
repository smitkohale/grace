// Grace, live: loads the glTF model and animates its named parts.
// Loaded lazily, so three.js never slows down the first screen.

import {
  CanvasTexture,
  DirectionalLight,
  HemisphereLight,
  type Material,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  NeutralToneMapping,
  type Object3D,
  PCFShadowMap,
  PerspectiveCamera,
  PlaneGeometry,
  PMREMGenerator,
  Scene,
  SRGBColorSpace,
  Vector3,
  WebGLRenderer,
} from "three";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import {
  approach,
  burst,
  clampLook,
  hopHeight,
  type MascotState,
  nextBlinkDelay,
  poseFor,
  waveAngle,
} from "./motion";

export interface GraceScene {
  setState(state: MascotState): void;
  setPaused(paused: boolean): void;
  /** Click, tap or Enter: she waves (or hops) and her cubes burst out. */
  press(): void;
  dispose(): void;
}

export interface SceneOptions {
  modelUrl: string;
  state: MascotState;
  reducedMotion: boolean;
}

const IDLE_FPS = 30;
const WAVE_SECONDS = 1.6;
const HOP_SECONDS = 0.4;
const BURST_SECONDS = 0.9;

export async function createGraceScene(
  canvas: HTMLCanvasElement,
  options: SceneOptions,
): Promise<GraceScene> {
  const renderer = new WebGLRenderer({
    canvas,
    antialias: true,
    alpha: true,
    powerPreference: "low-power",
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.outputColorSpace = SRGBColorSpace;
  // Soft, true-to-colour studio look like the master render.
  renderer.toneMapping = NeutralToneMapping;
  renderer.toneMappingExposure = 0.88;

  const scene = new Scene();
  // Same view as the master render: straight on, looking down about 6 degrees,
  // with a long lens so the boxes stay boxy.
  const camera = new PerspectiveCamera(16, 1, 0.1, 50);
  const target = new Vector3(0.21, 1.0, -0.36);
  const pitch = (6.3 * Math.PI) / 180;
  const distance = 9.3;
  camera.position.set(
    target.x,
    target.y + Math.sin(pitch) * distance,
    target.z + Math.cos(pitch) * distance,
  );
  camera.lookAt(target);

  // A soft contact shadow so she stands on the card instead of floating.
  const shadowCanvas = document.createElement("canvas");
  shadowCanvas.width = 128;
  shadowCanvas.height = 128;
  const paint = shadowCanvas.getContext("2d");
  if (paint) {
    const gradient = paint.createRadialGradient(64, 64, 0, 64, 64, 64);
    gradient.addColorStop(0, "rgba(53, 49, 46, 0.32)");
    gradient.addColorStop(1, "rgba(53, 49, 46, 0)");
    paint.fillStyle = gradient;
    paint.fillRect(0, 0, 128, 128);
  }
  const shadow = new Mesh(
    new PlaneGeometry(2.7, 1.2),
    new MeshBasicMaterial({
      map: new CanvasTexture(shadowCanvas),
      transparent: true,
      depthWrite: false,
    }),
  );
  shadow.rotation.x = -Math.PI / 2;
  shadow.position.set(0.3, 0.002, -0.36);
  scene.add(shadow);

  // Lighting like the master render: a soft studio room for reflections, warm
  // light from above with peach-toned shadows, and a strong key from the upper
  // left that casts soft shadows (bangs on the face, head on the scarf).
  const pmrem = new PMREMGenerator(renderer);
  const room = new RoomEnvironment();
  const environment = pmrem.fromScene(room, 0.04);
  // Only the finished lighting is kept; the generator's working memory goes now.
  pmrem.dispose();
  room.dispose();
  scene.environment = environment.texture;
  scene.environmentIntensity = 0.25;
  // Hands the GPU memory back at once instead of whenever the page collects garbage.
  const release = () => {
    environment.dispose();
    renderer.dispose();
    renderer.forceContextLoss();
  };
  scene.add(new HemisphereLight(0xfff4ec, 0xa88a7c, 0.5));
  // From above and to the left: tops brightest, fronts a little darker, her
  // left-hand sides in warm shadow, as in the master render.
  const key = new DirectionalLight(0xfff1e6, 3.0);
  key.position.set(-3, 7, 4);
  key.castShadow = true;
  key.shadow.mapSize.set(1024, 1024);
  key.shadow.bias = -0.0004;
  key.shadow.normalBias = 0.02;
  key.shadow.radius = 4;
  const bounds = key.shadow.camera;
  bounds.left = -1.6;
  bounds.right = 1.9;
  bounds.top = 2.2;
  bounds.bottom = -0.6;
  bounds.near = 0.5;
  bounds.far = 20;
  scene.add(key);
  const fill = new DirectionalLight(0xffe9dd, 0.5);
  fill.position.set(5, 1.5, 4);
  scene.add(fill);
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = PCFShadowMap;

  let gltf: Awaited<ReturnType<GLTFLoader["loadAsync"]>>;
  try {
    gltf = await new GLTFLoader().loadAsync(options.modelUrl);
  } catch (error) {
    release();
    throw error;
  }
  const model = gltf.scene;
  scene.add(model);
  model.traverse((object) => {
    const mesh = object as Mesh;
    if (!mesh.isMesh) return;
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    for (const material of materials) {
      if (material instanceof MeshStandardMaterial) {
        material.flatShading = true;
        material.needsUpdate = true;
      }
    }
  });

  const find = (name: string): Object3D => {
    const found = model.getObjectByName(name);
    if (!found) throw new Error(`Grace's model is missing the "${name}" part`);
    return found;
  };
  const root = find("Grace");
  const look = find("LookPivot");
  const eyes = [find("EyeL"), find("EyeR")];
  const pupils = [find("PupilL"), find("PupilR")];
  const arms = [find("ArmL"), find("ArmR")];
  const ahoge = find("Ahoge");
  const tail = find("Tail");
  // However many floating cubes the model has (the master render shows two).
  const cubes: Object3D[] = [];
  for (let k = 0; model.getObjectByName(`Cube${k}`); k++) cubes.push(find(`Cube${k}`));
  const cubeHome = cubes.map((cube) => cube.position.clone());
  const pupilHome = pupils.map((pupil) => pupil.position.clone());
  const ahogeHome = ahoge.rotation.z;
  const pupilMaterials = pupils.flatMap((pupil) => materialsOf(pupil));

  let state = options.state;
  let pose = poseFor(state);
  let paused = false;
  let hover = false;
  let frame = 0;
  let last = performance.now();
  let sinceRender = 1;
  let time = 0;
  let nextBlink = nextBlinkDelay() / 1000;
  let blinkLeft = 0;
  let waveLeft = state === "hello" ? WAVE_SECONDS : 0;
  let hopLeft = 0;
  let burstLeft = 0;
  let dragging = false;
  let dragStartX = 0;
  let spin = 0;
  let spinTarget = 0;
  const lookNow = { yaw: 0, pitch: 0 };
  const lookTarget = { yaw: 0, pitch: 0 };
  const current = {
    eyeOpen: pose.eyeOpen,
    pitch: pose.headPitch,
    roll: pose.headRoll,
    ahoge: pose.ahoge,
  };

  const resize = () => {
    const parent = canvas.parentElement;
    const width = parent?.clientWidth || canvas.clientWidth || 300;
    const height = parent?.clientHeight || canvas.clientHeight || 300;
    renderer.setSize(width, height, false);
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    sinceRender = 1;
    if (options.reducedMotion || paused) render();
  };
  const resizeObserver = new ResizeObserver(resize);
  if (canvas.parentElement) resizeObserver.observe(canvas.parentElement);

  const onPointerMove = (event: PointerEvent) => {
    if (dragging) {
      spinTarget = Math.max(-0.9, Math.min(0.9, (event.clientX - dragStartX) * 0.01));
      return;
    }
    const rect = canvas.getBoundingClientRect();
    const dx = (event.clientX - (rect.left + rect.width / 2)) / (window.innerWidth / 2);
    const dy = (event.clientY - (rect.top + rect.height / 2)) / (window.innerHeight / 2);
    const turn = clampLook(dx, dy);
    lookTarget.yaw = turn.yaw;
    lookTarget.pitch = turn.pitch;
  };
  const onPointerDown = (event: PointerEvent) => {
    dragging = true;
    dragStartX = event.clientX - spin / 0.01;
    canvas.setPointerCapture?.(event.pointerId);
  };
  const onPointerUp = () => {
    dragging = false;
    spinTarget = 0;
  };
  const onEnter = () => {
    hover = true;
  };
  const onLeave = () => {
    hover = false;
  };
  if (!options.reducedMotion) {
    window.addEventListener("pointermove", onPointerMove);
    canvas.addEventListener("pointerdown", onPointerDown);
    canvas.addEventListener("pointerenter", onEnter);
    canvas.addEventListener("pointerleave", onLeave);
    window.addEventListener("pointerup", onPointerUp);
  }

  function animate(dt: number) {
    time += dt;
    // Blinks.
    nextBlink -= dt;
    if (nextBlink <= 0) {
      blinkLeft = 0.13;
      nextBlink = nextBlinkDelay() / 1000;
    }
    blinkLeft = Math.max(0, blinkLeft - dt);
    const blink = blinkLeft > 0 ? 0.1 : 1;
    current.eyeOpen = approach(current.eyeOpen, Math.min(pose.eyeOpen, blink), 30, dt);
    for (const eye of eyes) eye.scale.y = current.eyeOpen;

    // Head: pose plus a gentle turn toward the pointer.
    lookNow.yaw = approach(lookNow.yaw, state === "sleeping" ? 0 : lookTarget.yaw, 6, dt);
    lookNow.pitch = approach(lookNow.pitch, state === "sleeping" ? 0 : lookTarget.pitch, 6, dt);
    current.pitch = approach(current.pitch, pose.headPitch, 4, dt);
    current.roll = approach(current.roll, pose.headRoll, 4, dt);
    look.rotation.set(current.pitch + lookNow.pitch, lookNow.yaw, current.roll);
    pupils.forEach((pupil, i) => {
      const home = pupilHome[i] as Vector3;
      pupil.position.x = home.x + lookNow.yaw * 0.08;
      pupil.position.y = home.y - lookNow.pitch * 0.12;
    });

    // Top strand perks up on hover; eyes glow a little brighter.
    current.ahoge = approach(current.ahoge, pose.ahoge + (hover ? 0.28 : 0), 8, dt);
    ahoge.rotation.z = ahogeHome + current.ahoge;
    for (const material of pupilMaterials) {
      if (material instanceof MeshStandardMaterial) {
        material.emissiveIntensity = approach(material.emissiveIntensity, hover ? 1.4 : 0.7, 8, dt);
      }
    }

    // Body: bob, hop and drag-to-turn with a spring back.
    hopLeft = Math.max(0, hopLeft - dt);
    const hop = hopHeight(1 - hopLeft / HOP_SECONDS);
    const lift = hopLeft > 0 ? hop : 0;
    root.position.y = pose.bob * Math.sin((time * 2 * Math.PI) / pose.bobPeriod) + lift;
    shadow.scale.setScalar(1 - lift * 2.5);
    spin = approach(spin, dragging ? spinTarget : 0, dragging ? 18 : 5, dt);
    root.rotation.y = spin;
    tail.rotation.y = 0.07 * Math.sin(time * 0.9);

    // Arms: wave on hello and when pressed.
    waveLeft = Math.max(0, waveLeft - dt);
    const wave = waveLeft > 0 ? waveAngle(1 - waveLeft / WAVE_SECONDS) : 0;
    (arms[1] as Object3D).rotation.z = wave;

    // Cubes: float, orbit while thinking, rest while sleeping, burst when pressed.
    burstLeft = Math.max(0, burstLeft - dt);
    const out = burstLeft > 0 ? burst(1 - burstLeft / BURST_SECONDS) : 0;
    cubes.forEach((cube, k) => {
      const home = cubeHome[k] as Vector3;
      if (pose.cubes === "orbit") {
        // Circling her head while she thinks.
        const angle = time * 1.4 + (k * 2 * Math.PI) / cubes.length;
        cube.position.set(
          Math.cos(angle) * 0.95,
          1.15 + 0.1 * Math.sin(angle * 2),
          -0.35 + Math.sin(angle) * 0.85,
        );
      } else if (pose.cubes === "rest") {
        cube.position.set(home.x - 0.05 * k, 0.05, home.z + 0.1 * k);
      } else {
        cube.position.set(home.x, home.y + 0.03 * Math.sin(time * 1.3 + k * 1.7), home.z);
      }
      if (out > 0) {
        const away = new Vector3(
          cube.position.x - 0.4,
          cube.position.y - 0.9,
          cube.position.z,
        ).normalize();
        cube.position.addScaledVector(away, out * 0.35);
      }
      cube.rotation.x += dt * 0.4;
      cube.rotation.y += dt * 0.6;
    });
  }

  function render() {
    renderer.render(scene, camera);
  }

  function loop(now: number) {
    frame = requestAnimationFrame(loop);
    const dt = Math.min(0.1, (now - last) / 1000);
    last = now;
    const busy = waveLeft > 0 || hopLeft > 0 || burstLeft > 0 || dragging || Math.abs(spin) > 0.002;
    sinceRender += dt;
    // At most 30 fps while idle; full rate while she's moving for you.
    if (!busy && sinceRender < 1 / IDLE_FPS) return;
    animate(sinceRender);
    sinceRender = 0;
    render();
  }

  function start() {
    if (frame || paused || options.reducedMotion) return;
    last = performance.now();
    frame = requestAnimationFrame(loop);
  }

  function stop() {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
  }

  /** With reduced motion: hold the state's pose, no movement. */
  function still() {
    current.eyeOpen = pose.eyeOpen;
    current.pitch = pose.headPitch;
    current.roll = pose.headRoll;
    current.ahoge = pose.ahoge;
    waveLeft = 0;
    hopLeft = 0;
    burstLeft = 0;
    animate(0);
    render();
  }

  resize();
  if (options.reducedMotion) still();
  else start();

  return {
    setState(next) {
      if (next === state) return;
      state = next;
      pose = poseFor(next);
      if (next === "hello") waveLeft = WAVE_SECONDS;
      if (options.reducedMotion) still();
    },
    setPaused(next) {
      paused = next;
      if (paused) stop();
      else start();
    },
    press() {
      if (options.reducedMotion) return;
      if (state !== "sleeping") waveLeft = WAVE_SECONDS;
      hopLeft = HOP_SECONDS;
      burstLeft = BURST_SECONDS;
    },
    dispose() {
      stop();
      resizeObserver.disconnect();
      window.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerdown", onPointerDown);
      canvas.removeEventListener("pointerenter", onEnter);
      canvas.removeEventListener("pointerleave", onLeave);
      window.removeEventListener("pointerup", onPointerUp);
      model.traverse((object) => {
        const mesh = object as Mesh;
        if (!mesh.isMesh) return;
        mesh.geometry.dispose();
        for (const material of materialsOf(mesh)) material.dispose();
      });
      shadow.geometry.dispose();
      shadow.material.map?.dispose();
      shadow.material.dispose();
      release();
    },
  };
}

function materialsOf(object: Object3D): Material[] {
  const mesh = object as Mesh;
  if (!mesh.isMesh) return [];
  return Array.isArray(mesh.material) ? mesh.material : [mesh.material];
}
