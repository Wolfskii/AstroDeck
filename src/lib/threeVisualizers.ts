import * as THREE from "three";
import { sampleBand } from "./osAudioViz";

export type VisualizerTick = (
  dt: number,
  elapsed: number,
  colors: THREE.Color[],
  beat: number,
  playing: boolean,
  bands: number[] | null
) => void;

export type VisualizerCtx = {
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  glow: THREE.CanvasTexture;
  displayed: THREE.Color[];
  disposables: { dispose: () => void }[];
};

export function setupSpectrum(ctx: VisualizerCtx): VisualizerTick {
  const { scene, camera, displayed, disposables } = ctx;
  camera.fov = 55;
  camera.position.set(0, 7.2, 12.5);
  camera.lookAt(0, 1.4, 0);
  scene.fog = new THREE.FogExp2(0x05070c, 0.04);

  const count = 56;
  const geometry = new THREE.BoxGeometry(0.18, 1, 0.18);
  const material = new THREE.MeshBasicMaterial({
    transparent: true,
    opacity: 0.95,
    toneMapped: false,
  });
  const mesh = new THREE.InstancedMesh(geometry, material, count);
  scene.add(mesh);
  disposables.push(geometry, material);

  const dummy = new THREE.Object3D();
  const tint = new THREE.Color();
  const radius = 5.4;

  const core = new THREE.Mesh(
    new THREE.SphereGeometry(0.55, 24, 16),
    new THREE.MeshBasicMaterial({ color: displayed[0], transparent: true, opacity: 0.9, toneMapped: false })
  );
  scene.add(core);
  disposables.push(core.geometry, core.material);

  return (_dt, elapsed, colors, beat, playing, bands) => {
    const spin = playing ? elapsed * 0.22 : elapsed * 0.04;
    for (let i = 0; i < count; i += 1) {
      const angle = (i / count) * Math.PI * 2 + spin * 0.15;
      const wave =
        0.35 * Math.abs(Math.sin(elapsed * 2.1 + i * 0.41)) +
        0.25 * Math.abs(Math.sin(elapsed * 3.4 + i * 0.17)) +
        0.2 * Math.abs(Math.sin(elapsed * 5.2 + i * 0.09));
      const live = sampleBand(bands, i, count);
      const level = live == null ? wave : live * 0.88 + wave * 0.12;
      const height = 0.45 + (0.55 + beat * 2.4) * level;
      dummy.position.set(Math.cos(angle) * radius, height * 0.5, Math.sin(angle) * radius);
      dummy.scale.set(1, height, 1);
      dummy.rotation.set(0, -angle, 0);
      dummy.updateMatrix();
      mesh.setMatrixAt(i, dummy.matrix);
      tint.copy(colors[i % 3]).multiplyScalar(0.45 + beat * 0.85 + level * 0.35);
      mesh.setColorAt(i, tint);
    }
    mesh.instanceMatrix.needsUpdate = true;
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    (core.material as THREE.MeshBasicMaterial).color.copy(colors[0]);
    core.scale.setScalar(0.85 + beat * 0.7);
    camera.position.x = Math.sin(elapsed * 0.12) * 1.2;
    camera.lookAt(0, 1.4, 0);
  };
}

export function setupBokeh(ctx: VisualizerCtx): VisualizerTick {
  const { scene, camera, glow, displayed, disposables } = ctx;
  camera.position.set(0, 0, 14);
  camera.lookAt(0, 0, 0);
  scene.fog = new THREE.FogExp2(0x05070c, 0.028);

  const count = 28;
  const orbs: Array<{
    sprite: THREE.Sprite;
    origin: THREE.Vector3;
    speed: number;
    phase: number;
    base: number;
    colorIndex: number;
  }> = [];

  for (let i = 0; i < count; i += 1) {
    const material = new THREE.SpriteMaterial({
      map: glow,
      color: displayed[i % 3],
      blending: THREE.AdditiveBlending,
      transparent: true,
      depthWrite: false,
      opacity: 0.72,
    });
    const sprite = new THREE.Sprite(material);
    const origin = new THREE.Vector3(
      (Math.random() - 0.5) * 16,
      (Math.random() - 0.5) * 10,
      (Math.random() - 0.5) * 8
    );
    sprite.position.copy(origin);
    const base = 1.6 + Math.random() * 3.8;
    sprite.scale.setScalar(base);
    scene.add(sprite);
    disposables.push(material);
    orbs.push({
      sprite,
      origin,
      speed: 0.12 + Math.random() * 0.28,
      phase: Math.random() * Math.PI * 2,
      base,
      colorIndex: i % 3,
    });
  }

  return (_dt, elapsed, colors, beat, playing) => {
    const motion = playing ? 1 : 0.2;
    for (const orb of orbs) {
      const t = elapsed * orb.speed * motion;
      orb.sprite.position.set(
        orb.origin.x + Math.sin(t + orb.phase) * 1.4,
        orb.origin.y + Math.cos(t * 0.8 + orb.phase) * 1.1,
        orb.origin.z + Math.sin(t * 0.6) * 0.8
      );
      const size = orb.base * (0.85 + beat * 0.9);
      orb.sprite.scale.setScalar(size);
      orb.sprite.material.color.copy(colors[orb.colorIndex]);
      orb.sprite.material.opacity = 0.35 + beat * 0.5;
    }
  };
}

export function setupRipple(ctx: VisualizerCtx): VisualizerTick {
  const { scene, camera, glow, displayed, disposables } = ctx;
  camera.position.set(0, 0, 16);
  camera.lookAt(0, 0, 0);

  const ringCount = 8;
  const rings: Array<{ mesh: THREE.Mesh; phase: number }> = [];
  for (let i = 0; i < ringCount; i += 1) {
    const geometry = new THREE.RingGeometry(0.96, 1.04, 72);
    const material = new THREE.MeshBasicMaterial({
      color: displayed[i % 3],
      transparent: true,
      opacity: 0,
      side: THREE.DoubleSide,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
      toneMapped: false,
    });
    const mesh = new THREE.Mesh(geometry, material);
    scene.add(mesh);
    rings.push({ mesh, phase: i / ringCount });
    disposables.push(geometry, material);
  }

  const core = new THREE.Sprite(
    new THREE.SpriteMaterial({
      map: glow,
      color: displayed[0],
      blending: THREE.AdditiveBlending,
      transparent: true,
      depthWrite: false,
      opacity: 0.9,
    })
  );
  core.scale.set(5, 5, 1);
  scene.add(core);
  disposables.push(core.material);

  return (_dt, elapsed, colors, beat, playing) => {
    const rate = playing ? 0.42 : 0.12;
    for (const ring of rings) {
      const age = (elapsed * rate + ring.phase) % 1;
      const scale = 0.35 + age * 16;
      ring.mesh.scale.set(scale, scale, 1);
      const material = ring.mesh.material as THREE.MeshBasicMaterial;
      material.opacity = (1 - age) * (0.12 + beat * 0.7);
      material.color.copy(colors[Math.floor(ring.phase * 3) % 3]);
    }
    core.material.color.copy(colors[0]);
    core.scale.setScalar(4.2 + beat * 5.5);
    core.material.opacity = 0.45 + beat * 0.5;
  };
}

export function setupHelix(ctx: VisualizerCtx): VisualizerTick {
  const { scene, camera, glow, displayed, disposables } = ctx;
  camera.position.set(0, 0, 18);
  camera.lookAt(0, 0, 0);
  scene.fog = new THREE.FogExp2(0x05070c, 0.03);

  const count = 420;
  const positions = new Float32Array(count * 3);
  const colors = new Float32Array(count * 3);
  const sizes = new Float32Array(count);

  for (let i = 0; i < count; i += 1) {
    const strand = i % 2;
    const t = (i / count) * 22;
    const angle = t * 1.15 + strand * Math.PI;
    positions[i * 3] = Math.cos(angle) * 2.15;
    positions[i * 3 + 1] = t - 11;
    positions[i * 3 + 2] = Math.sin(angle) * 2.15;
    const tint = strand === 0 ? displayed[0] : displayed[1];
    colors[i * 3] = tint.r;
    colors[i * 3 + 1] = tint.g;
    colors[i * 3 + 2] = tint.b;
    sizes[i] = 7 + (i % 5);
  }

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
  geometry.setAttribute("aSize", new THREE.BufferAttribute(sizes, 1));
  const material = new THREE.ShaderMaterial({
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
    uniforms: { uMap: { value: glow } },
    vertexShader: `
      attribute vec3 color;
      attribute float aSize;
      varying vec3 vColor;
      void main() {
        vColor = color;
        vec4 mv = modelViewMatrix * vec4(position, 1.0);
        gl_PointSize = aSize * (240.0 / max(1.0, -mv.z));
        gl_Position = projectionMatrix * mv;
      }
    `,
    fragmentShader: `
      uniform sampler2D uMap;
      varying vec3 vColor;
      void main() {
        float a = texture2D(uMap, gl_PointCoord).a;
        if (a < 0.02) discard;
        gl_FragColor = vec4(vColor * 1.4, a);
      }
    `,
  });
  const points = new THREE.Points(geometry, material);
  scene.add(points);
  disposables.push(geometry, material);

  return (_dt, elapsed, palette, beat, playing) => {
    points.rotation.y = elapsed * (playing ? 0.55 : 0.12);
    points.position.y = Math.sin(elapsed * 0.35) * 0.6;
    const scale = 1 + beat * 0.12;
    points.scale.set(scale, 1, scale);
    const attr = geometry.getAttribute("color") as THREE.BufferAttribute;
    for (let i = 0; i < count; i += 1) {
      const tint = i % 2 === 0 ? palette[0] : palette[1];
      attr.setXYZ(i, tint.r, tint.g, tint.b);
    }
    attr.needsUpdate = true;
  };
}
