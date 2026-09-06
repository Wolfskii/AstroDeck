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
    opacity: 0.92,
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
    new THREE.MeshBasicMaterial({
      color: displayed[0],
      transparent: true,
      opacity: 0.72,
      toneMapped: false,
    })
  );
  scene.add(core);
  disposables.push(core.geometry, core.material);

  return (_dt, elapsed, colors, _beat, playing, bands) => {
    const spin = playing ? elapsed * 0.28 : elapsed * 0.06;
    for (let i = 0; i < count; i += 1) {
      const angle = (i / count) * Math.PI * 2 + spin * 0.18;
      const wave =
        0.35 * Math.abs(Math.sin(elapsed * 1.7 + i * 0.41)) +
        0.28 * Math.abs(Math.sin(elapsed * 2.6 + i * 0.17)) +
        0.18 * Math.abs(Math.sin(elapsed * 4.1 + i * 0.09));
      const live = sampleBand(bands, i, count);
      const level = live == null ? wave : live * 0.82 + wave * 0.18;
      const height = 0.55 + 2.15 * level;
      dummy.position.set(Math.cos(angle) * radius, height * 0.5, Math.sin(angle) * radius);
      dummy.scale.set(1, height, 1);
      dummy.rotation.set(0, -angle, 0);
      dummy.updateMatrix();
      mesh.setMatrixAt(i, dummy.matrix);
      tint.copy(colors[i % 3]).multiplyScalar(0.62 + level * 0.28);
      mesh.setColorAt(i, tint);
    }
    mesh.instanceMatrix.needsUpdate = true;
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    (core.material as THREE.MeshBasicMaterial).color.copy(colors[0]);
    camera.position.x = Math.sin(elapsed * 0.14) * 1.6;
    camera.position.z = 12.5 + Math.cos(elapsed * 0.11) * 0.8;
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
      opacity: 0.48,
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
      speed: 0.1 + Math.random() * 0.22,
      phase: Math.random() * Math.PI * 2,
      base,
      colorIndex: i % 3,
    });
  }

  return (_dt, elapsed, colors, _beat, playing) => {
    const motion = playing ? 1 : 0.22;
    for (const orb of orbs) {
      const t = elapsed * orb.speed * motion;
      orb.sprite.position.set(
        orb.origin.x + Math.sin(t + orb.phase) * 3.4,
        orb.origin.y + Math.cos(t * 0.72 + orb.phase) * 2.4,
        orb.origin.z + Math.sin(t * 0.48 + orb.phase * 0.5) * 1.8
      );
      orb.sprite.scale.setScalar(orb.base);
      orb.sprite.material.color.copy(colors[orb.colorIndex]);
      orb.sprite.material.opacity = 0.46;
    }
    camera.position.x = Math.sin(elapsed * 0.07) * 0.9;
    camera.lookAt(0, 0, 0);
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
      opacity: 0.28,
    })
  );
  core.scale.set(3.2, 3.2, 1);
  scene.add(core);
  disposables.push(core.material);

  return (_dt, elapsed, colors, _beat, playing) => {
    const rate = playing ? 0.38 : 0.12;
    for (const ring of rings) {
      const age = (elapsed * rate + ring.phase) % 1;
      const scale = 0.4 + age * 15;
      ring.mesh.scale.set(scale, scale, 1);
      ring.mesh.rotation.z = elapsed * 0.08 + ring.phase;
      const material = ring.mesh.material as THREE.MeshBasicMaterial;
      material.opacity = (1 - age) * 0.38;
      material.color.copy(colors[Math.floor(ring.phase * 3) % 3]);
    }
    core.material.color.copy(colors[0]);
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
        gl_FragColor = vec4(vColor * 1.15, a);
      }
    `,
  });
  const points = new THREE.Points(geometry, material);
  scene.add(points);
  disposables.push(geometry, material);

  return (_dt, elapsed, palette, _beat, playing) => {
    points.rotation.y = elapsed * (playing ? 0.55 : 0.12);
    points.position.y = Math.sin(elapsed * 0.35) * 0.6;
    const attr = geometry.getAttribute("color") as THREE.BufferAttribute;
    for (let i = 0; i < count; i += 1) {
      const tint = i % 2 === 0 ? palette[0] : palette[1];
      attr.setXYZ(i, tint.r, tint.g, tint.b);
    }
    attr.needsUpdate = true;
  };
}

export function setupRain(ctx: VisualizerCtx): VisualizerTick {
  const { scene, camera, disposables } = ctx;
  camera.position.set(0, 0, 16);
  camera.lookAt(0, 0, 0);
  scene.fog = new THREE.FogExp2(0x05070c, 0.032);

  const count = 180;
  const positions = new Float32Array(count * 6);
  const colors = new Float32Array(count * 6);
  const drops: Array<{ x: number; y: number; z: number; speed: number; len: number; tint: number }> =
    [];

  for (let i = 0; i < count; i += 1) {
    drops.push({
      x: (Math.random() - 0.5) * 22,
      y: (Math.random() - 0.5) * 24,
      z: (Math.random() - 0.5) * 14,
      speed: 4.2 + Math.random() * 6.5,
      len: 0.55 + Math.random() * 1.1,
      tint: i % 3,
    });
  }

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
  const material = new THREE.LineBasicMaterial({
    vertexColors: true,
    transparent: true,
    opacity: 0.72,
    depthWrite: false,
  });
  scene.add(new THREE.LineSegments(geometry, material));
  disposables.push(geometry, material);

  return (dt, elapsed, palette, _beat, playing) => {
    const motion = playing ? 1 : 0.18;
    const pos = geometry.getAttribute("position") as THREE.BufferAttribute;
    const col = geometry.getAttribute("color") as THREE.BufferAttribute;
    const wind = Math.sin(elapsed * 0.22) * 0.35;
    for (let i = 0; i < count; i += 1) {
      const drop = drops[i];
      drop.y -= drop.speed * dt * motion;
      if (drop.y < -13) {
        drop.y = 13;
        drop.x = (Math.random() - 0.5) * 22;
        drop.z = (Math.random() - 0.5) * 14;
      }
      const x = drop.x + wind * (1.2 + drop.z * 0.04);
      pos.setXYZ(i * 2, x, drop.y, drop.z);
      pos.setXYZ(i * 2 + 1, x + wind * 0.25, drop.y - drop.len, drop.z);
      const tint = palette[drop.tint];
      col.setXYZ(i * 2, tint.r, tint.g, tint.b);
      col.setXYZ(i * 2 + 1, tint.r * 0.2, tint.g * 0.2, tint.b * 0.2);
    }
    pos.needsUpdate = true;
    col.needsUpdate = true;
    camera.position.x = Math.sin(elapsed * 0.06) * 0.7;
    camera.lookAt(0, 0, 0);
  };
}

export function setupEmbers(ctx: VisualizerCtx): VisualizerTick {
  const { scene, camera, glow, displayed, disposables } = ctx;
  camera.position.set(0, 1.2, 15);
  camera.lookAt(0, 1.2, 0);
  scene.fog = new THREE.FogExp2(0x05070c, 0.03);

  const count = 90;
  const motes: Array<{
    sprite: THREE.Sprite;
    x: number;
    y: number;
    z: number;
    speed: number;
    drift: number;
    phase: number;
    size: number;
    colorIndex: number;
  }> = [];

  for (let i = 0; i < count; i += 1) {
    const material = new THREE.SpriteMaterial({
      map: glow,
      color: displayed[i % 3],
      blending: THREE.AdditiveBlending,
      transparent: true,
      depthWrite: false,
      opacity: 0.42,
    });
    const sprite = new THREE.Sprite(material);
    const mote = {
      sprite,
      x: (Math.random() - 0.5) * 16,
      y: (Math.random() - 0.5) * 14,
      z: (Math.random() - 0.5) * 10,
      speed: 0.55 + Math.random() * 1.1,
      drift: 0.4 + Math.random() * 0.9,
      phase: Math.random() * Math.PI * 2,
      size: 0.45 + Math.random() * 1.15,
      colorIndex: i % 3,
    };
    sprite.scale.setScalar(mote.size);
    scene.add(sprite);
    disposables.push(material);
    motes.push(mote);
  }

  return (dt, elapsed, colors, _beat, playing) => {
    const motion = playing ? 1 : 0.2;
    for (const mote of motes) {
      mote.y += mote.speed * dt * motion;
      if (mote.y > 8.5) {
        mote.y = -7.5;
        mote.x = (Math.random() - 0.5) * 16;
        mote.z = (Math.random() - 0.5) * 10;
      }
      mote.sprite.position.set(
        mote.x + Math.sin(elapsed * mote.drift + mote.phase) * 1.6,
        mote.y,
        mote.z + Math.cos(elapsed * mote.drift * 0.7 + mote.phase) * 0.8
      );
      mote.sprite.scale.setScalar(mote.size);
      mote.sprite.material.color.copy(colors[mote.colorIndex]);
      mote.sprite.material.opacity = 0.4;
    }
    camera.position.x = Math.sin(elapsed * 0.08) * 1.1;
    camera.lookAt(0, 1.2, 0);
  };
}

export function setupLattice(ctx: VisualizerCtx): VisualizerTick {
  const { scene, camera, displayed, disposables } = ctx;
  camera.position.set(0, 1.6, 14);
  camera.lookAt(0, 0, 0);
  scene.fog = new THREE.FogExp2(0x06070d, 0.04);

  const group = new THREE.Group();
  scene.add(group);
  const meshes: THREE.LineSegments[] = [];

  function addWire(geometry: THREE.BufferGeometry, scale: number, y: number) {
    const edges = new THREE.EdgesGeometry(geometry);
    const material = new THREE.LineBasicMaterial({
      color: displayed[meshes.length % 3],
      transparent: true,
      opacity: 0.55,
    });
    const line = new THREE.LineSegments(edges, material);
    line.scale.setScalar(scale);
    line.position.y = y;
    group.add(line);
    meshes.push(line);
    disposables.push(geometry, edges, material);
  }

  addWire(new THREE.IcosahedronGeometry(2.4, 0), 1, 0);
  addWire(new THREE.OctahedronGeometry(1.35, 0), 1, 0);
  addWire(new THREE.TetrahedronGeometry(0.7, 0), 1, 0);
  addWire(new THREE.IcosahedronGeometry(3.8, 0), 1, 0);

  for (let i = 0; i < 8; i += 1) {
    const angle = (i / 8) * Math.PI * 2;
    const geometry = new THREE.OctahedronGeometry(0.38, 0);
    const edges = new THREE.EdgesGeometry(geometry);
    const material = new THREE.LineBasicMaterial({
      color: displayed[i % 3],
      transparent: true,
      opacity: 0.5,
    });
    const line = new THREE.LineSegments(edges, material);
    line.position.set(Math.cos(angle) * 5.2, Math.sin(angle * 2) * 0.8, Math.sin(angle) * 5.2);
    group.add(line);
    meshes.push(line);
    disposables.push(geometry, edges, material);
  }

  return (dt, elapsed, palette, _beat, playing) => {
    const spin = playing ? 1 : 0.22;
    group.rotation.y += dt * 0.18 * spin;
    group.rotation.x = Math.sin(elapsed * 0.16) * 0.18;
    meshes.forEach((mesh, index) => {
      mesh.rotation.y += dt * (0.12 + index * 0.015) * spin;
      (mesh.material as THREE.LineBasicMaterial).color.copy(palette[index % 3]);
    });
    camera.position.x = Math.sin(elapsed * 0.12) * 2.2;
    camera.position.y = 1.6 + Math.cos(elapsed * 0.09) * 0.6;
    camera.lookAt(0, 0, 0);
  };
}

