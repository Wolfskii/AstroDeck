import * as THREE from "three";
import type { SceneBackgroundId } from "./sceneBackgrounds";
import { resolveBeat, resolvePlaying } from "./visualizerBeat";
import { getOsAudioFrame } from "./osAudioViz";
import {
  setupBokeh,
  setupHelix,
  setupRipple,
  setupSpectrum,
} from "./threeVisualizers";

export type ThreeBackgroundHandle = {
  applyPalette: (palette: string[]) => void;
  setPlaying: (next: boolean) => void;
  dispose: () => void;
};

const FALLBACK = ["#7dd3fc", "#c084fc", "#fb7185", "#071018"];
const PALETTE_FADE_SEC = 0.7;

function paletteColors(hex: string[]): THREE.Color[] {
  const src = hex.length ? hex : FALLBACK;
  return [0, 1, 2, 3].map((index) => {
    const color = new THREE.Color(src[Math.min(index, src.length - 1)] ?? FALLBACK[index]);
    color.convertSRGBToLinear();
    return color;
  });
}

function mixColor(from: THREE.Color, to: THREE.Color, t: number, out: THREE.Color) {
  return out.copy(from).lerp(to, t);
}

function makeGlowTexture(size = 128): THREE.CanvasTexture {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) return new THREE.CanvasTexture(canvas);
  const gradient = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
  gradient.addColorStop(0, "rgba(255,255,255,1)");
  gradient.addColorStop(0.28, "rgba(255,255,255,0.45)");
  gradient.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, size, size);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

export function createThreeBackground(
  host: HTMLElement,
  style: SceneBackgroundId,
  palette: string[]
): ThreeBackgroundHandle {
  const renderer = new THREE.WebGLRenderer({
    antialias: false,
    alpha: false,
    powerPreference: "low-power",
  });
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));
  renderer.setClearColor(0x05070c, 1);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.domElement.style.display = "block";
  renderer.domElement.style.width = "100%";
  renderer.domElement.style.height = "100%";
  host.replaceChildren(renderer.domElement);

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(60, 1, 0.1, 400);
  const clock = new THREE.Clock();
  const glow = makeGlowTexture();

  let displayed = paletteColors(palette);
  let from = displayed.map((color) => color.clone());
  let to = displayed.map((color) => color.clone());
  let fade = 1;
  let disposed = false;
  let raf = 0;
  let playing = true;
  const night = new THREE.Color(0x05070c);
  const fogScratch = new THREE.Color();

  const disposables: { dispose: () => void }[] = [glow];
  const tickers: Array<
    (
      dt: number,
      elapsed: number,
      colors: THREE.Color[],
      beat: number,
      playing: boolean,
      bands: number[] | null
    ) => void
  > = [];

  function resize() {
    const width = Math.max(1, host.clientWidth);
    const height = Math.max(1, host.clientHeight);
    camera.aspect = width / height;
    camera.updateProjectionMatrix();
    renderer.setSize(width, height, false);
  }

  const observer = new ResizeObserver(resize);
  observer.observe(host);
  resize();

  if (style === "cosmos") setupCosmos();
  else if (style === "warp") setupWarp();
  else if (style === "prism") setupPrism();
  else if (style === "horizon") setupHorizon();
  else if (style === "spectrum") tickers.push(setupSpectrum(ctx()));
  else if (style === "bokeh") tickers.push(setupBokeh(ctx()));
  else if (style === "ripple") tickers.push(setupRipple(ctx()));
  else if (style === "helix") tickers.push(setupHelix(ctx()));
  else setupHorizon();

  function ctx() {
    return { scene, camera, glow, displayed, disposables };
  }

  function setupCosmos() {
    camera.position.set(0, 9, 26);
    camera.lookAt(0, 0, 0);
    scene.fog = new THREE.FogExp2(0x05070c, 0.018);

    const count = 3200;
    const positions = new Float32Array(count * 3);
    const colors = new Float32Array(count * 3);
    const sizes = new Float32Array(count);
    const colorA = displayed[0];
    const colorB = displayed[1];
    const colorC = displayed[2];

    for (let i = 0; i < count; i += 1) {
      const arm = i % 3;
      const radius = Math.pow(Math.random(), 0.52) * 44;
      const spin = radius * 0.46 + (arm * Math.PI * 2) / 3;
      const spread = (Math.random() - 0.5) * (1.4 + radius * 0.07);
      positions[i * 3] = Math.cos(spin) * radius + Math.cos(spin + 1.2) * spread;
      positions[i * 3 + 1] = (Math.random() - 0.5) * (1.1 + (1 - radius / 44) * 5.5);
      positions[i * 3 + 2] = Math.sin(spin) * radius + Math.sin(spin + 1.2) * spread;
      const mix = Math.random();
      const tint = mix < 0.45 ? colorA : mix < 0.8 ? colorB : colorC;
      colors[i * 3] = tint.r;
      colors[i * 3 + 1] = tint.g;
      colors[i * 3 + 2] = tint.b;
      sizes[i] = 4 + Math.random() * 18;
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
    geometry.setAttribute("aSize", new THREE.BufferAttribute(sizes, 1));

    const material = new THREE.ShaderMaterial({
      transparent: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
      uniforms: {
        uMap: { value: glow },
      },
      vertexShader: `
        attribute vec3 color;
        attribute float aSize;
        varying vec3 vColor;
        void main() {
          vColor = color;
          vec4 mv = modelViewMatrix * vec4(position, 1.0);
          gl_PointSize = aSize * (280.0 / max(1.0, -mv.z));
          gl_Position = projectionMatrix * mv;
        }
      `,
      fragmentShader: `
        uniform sampler2D uMap;
        varying vec3 vColor;
        void main() {
          vec4 glow = texture2D(uMap, gl_PointCoord);
          float alpha = glow.a;
          if (alpha < 0.02) discard;
          gl_FragColor = vec4(vColor * 1.35, alpha);
        }
      `,
    });

    const points = new THREE.Points(geometry, material);
    scene.add(points);
    disposables.push(geometry, material);

    const core = new THREE.Sprite(
      new THREE.SpriteMaterial({
        map: glow,
        color: displayed[0],
        blending: THREE.AdditiveBlending,
        depthWrite: false,
        transparent: true,
        opacity: 0.85,
      })
    );
    core.scale.set(14, 14, 1);
    scene.add(core);
    disposables.push(core.material);

    tickers.push((dt, elapsed, paletteNow) => {
      points.rotation.y += dt * 0.045;
      core.material.color.copy(paletteNow[0]);
      core.scale.setScalar(12 + Math.sin(elapsed * 0.7) * 1.6);
      const attr = geometry.getAttribute("color") as THREE.BufferAttribute;
      if (fade < 1) {
        for (let i = 0; i < count; i += 1) {
          const mix = (i % 10) / 10;
          const tint = mix < 0.45 ? paletteNow[0] : mix < 0.8 ? paletteNow[1] : paletteNow[2];
          attr.setXYZ(i, tint.r, tint.g, tint.b);
        }
        attr.needsUpdate = true;
      }
    });
  }

  function setupWarp() {
    camera.fov = 70;
    camera.position.set(0, 0, 0);
    camera.lookAt(0, 0, -1);
    scene.fog = new THREE.Fog(0x05070c, 8, 95);
    renderer.setClearColor(0x03050a, 1);

    const count = 900;
    const positions = new Float32Array(count * 6);
    const colors = new Float32Array(count * 6);
    const speed = new Float32Array(count);
    const radius = new Float32Array(count);

    function place(index: number, z?: number) {
      const angle = Math.random() * Math.PI * 2;
      const r = 0.8 + Math.random() * 16;
      const depth = z ?? -6 - Math.random() * 90;
      radius[index] = r;
      speed[index] = 22 + Math.random() * 38;
      const x = Math.cos(angle) * r;
      const y = Math.sin(angle) * r;
      const streak = 0.35 + speed[index] * 0.045;
      positions[index * 6] = x;
      positions[index * 6 + 1] = y;
      positions[index * 6 + 2] = depth;
      positions[index * 6 + 3] = x;
      positions[index * 6 + 4] = y;
      positions[index * 6 + 5] = depth - streak;
    }

    for (let i = 0; i < count; i += 1) place(i);

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));
    const material = new THREE.LineBasicMaterial({
      vertexColors: true,
      blending: THREE.AdditiveBlending,
      transparent: true,
      opacity: 0.9,
      depthWrite: false,
    });
    const lines = new THREE.LineSegments(geometry, material);
    scene.add(lines);
    disposables.push(geometry, material);

    function paint(paletteNow: THREE.Color[]) {
      const attr = geometry.getAttribute("color") as THREE.BufferAttribute;
      for (let i = 0; i < count; i += 1) {
        const tint = radius[i] < 5 ? paletteNow[0] : radius[i] < 10 ? paletteNow[1] : paletteNow[2];
        attr.setXYZ(i * 2, tint.r, tint.g, tint.b);
        attr.setXYZ(i * 2 + 1, tint.r * 0.15, tint.g * 0.15, tint.b * 0.15);
      }
      attr.needsUpdate = true;
    }
    paint(displayed);

    tickers.push((dt, _elapsed, paletteNow) => {
      const pos = geometry.getAttribute("position") as THREE.BufferAttribute;
      for (let i = 0; i < count; i += 1) {
        let z = pos.getZ(i * 2) + speed[i] * dt;
        if (z > -2) {
          place(i);
          z = pos.getZ(i * 2);
        } else {
          const streak = 0.35 + speed[i] * 0.045;
          pos.setZ(i * 2, z);
          pos.setZ(i * 2 + 1, z - streak);
        }
      }
      pos.needsUpdate = true;
      if (fade < 1) paint(paletteNow);
    });
  }

  function setupPrism() {
    camera.position.set(0, 2.4, 14);
    camera.lookAt(0, 0, 0);
    scene.fog = new THREE.FogExp2(0x06070d, 0.035);
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.15;

    const ambient = new THREE.AmbientLight(0xffffff, 0.12);
    const key = new THREE.PointLight(displayed[0], 18, 40, 2);
    const fill = new THREE.PointLight(displayed[1], 12, 36, 2);
    const rim = new THREE.PointLight(displayed[2], 10, 32, 2);
    key.position.set(6, 8, 8);
    fill.position.set(-8, -2, 4);
    rim.position.set(0, 4, -10);
    scene.add(ambient, key, fill, rim);

    const group = new THREE.Group();
    scene.add(group);
    const crystals: THREE.Mesh[] = [];
    const wires: THREE.LineSegments[] = [];

    function addCrystal(scale: number, x: number, y: number, z: number) {
      const geometry = new THREE.IcosahedronGeometry(scale, 0);
      const material = new THREE.MeshPhongMaterial({
        color: displayed[crystals.length % 3],
        emissive: displayed[crystals.length % 3],
        emissiveIntensity: 0.18,
        shininess: 90,
        transparent: true,
        opacity: 0.82,
      });
      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(x, y, z);
      mesh.rotation.set(Math.random() * Math.PI, Math.random() * Math.PI, 0);
      group.add(mesh);
      crystals.push(mesh);
      const edges = new THREE.EdgesGeometry(geometry);
      const line = new THREE.LineSegments(
        edges,
        new THREE.LineBasicMaterial({
          color: displayed[(crystals.length + 1) % 3],
          transparent: true,
          opacity: 0.35,
        })
      );
      mesh.add(line);
      wires.push(line);
      disposables.push(geometry, material, edges, line.material);
    }

    addCrystal(2.4, 0, 0, 0);
    addCrystal(1.1, 4.2, 1.6, -1.4);
    addCrystal(0.85, -3.8, -1.2, 1.8);
    addCrystal(0.7, 2.4, -2.2, 3.1);
    addCrystal(0.95, -2.6, 2.4, -3.2);
    addCrystal(0.55, 5.1, -0.6, 2.2);
    addCrystal(0.6, -4.8, 0.8, -2.4);

    tickers.push((dt, elapsed, paletteNow) => {
      group.rotation.y += dt * 0.12;
      group.rotation.x = Math.sin(elapsed * 0.18) * 0.08;
      camera.position.x = Math.sin(elapsed * 0.12) * 1.6;
      camera.lookAt(0, 0, 0);
      key.color.copy(paletteNow[0]);
      fill.color.copy(paletteNow[1]);
      rim.color.copy(paletteNow[2]);
      crystals.forEach((mesh, index) => {
        mesh.rotation.y += dt * (0.15 + index * 0.03);
        const material = mesh.material as THREE.MeshPhongMaterial;
        material.color.copy(paletteNow[index % 3]);
        material.emissive.copy(paletteNow[index % 3]);
      });
      wires.forEach((line, index) => {
        (line.material as THREE.LineBasicMaterial).color.copy(paletteNow[(index + 1) % 3]);
      });
    });
  }

  function setupHorizon() {
    camera.fov = 58;
    camera.position.set(0, 2.6, 10);
    camera.lookAt(0, 3.4, -30);
    renderer.setClearColor(0x04060c, 1);

    const skyGeo = new THREE.SphereGeometry(120, 24, 16);
    const skyMat = new THREE.ShaderMaterial({
      side: THREE.BackSide,
      uniforms: {
        uTop: { value: displayed[3].clone() },
        uMid: { value: displayed[1].clone() },
        uSun: { value: displayed[0].clone() },
      },
      vertexShader: `
        varying vec3 vDir;
        void main() {
          vDir = position;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `,
      fragmentShader: `
        varying vec3 vDir;
        uniform vec3 uTop;
        uniform vec3 uMid;
        uniform vec3 uSun;
        void main() {
          vec3 n = normalize(vDir);
          float h = clamp(n.y * 0.5 + 0.5, 0.0, 1.0);
          vec3 col = mix(uMid, uTop, smoothstep(0.35, 1.0, h));
          float sun = smoothstep(0.18, 0.0, length(n.xy - vec2(0.0, 0.12)));
          col += uSun * sun * 0.35;
          gl_FragColor = vec4(col, 1.0);
        }
      `,
    });
    const sky = new THREE.Mesh(skyGeo, skyMat);
    scene.add(sky);
    disposables.push(skyGeo, skyMat);

    const groundGeo = new THREE.PlaneGeometry(220, 220, 1, 1);
    groundGeo.rotateX(-Math.PI / 2);
    const groundMat = new THREE.ShaderMaterial({
      uniforms: {
        uTime: { value: 0 },
        uLine: { value: displayed[0].clone() },
        uFill: { value: displayed[3].clone() },
        uFog: { value: displayed[1].clone() },
      },
      vertexShader: `
        varying vec3 vPos;
        void main() {
          vPos = position;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `,
      fragmentShader: `
        varying vec3 vPos;
        uniform float uTime;
        uniform vec3 uLine;
        uniform vec3 uFill;
        uniform vec3 uFog;
        void main() {
          vec2 uv = vec2(vPos.x, vPos.z + uTime * 9.0);
          float scale = 3.6;
          vec2 cell = abs(fract(uv / scale) - 0.5);
          float line = min(cell.x, cell.y);
          float grid = 1.0 - smoothstep(0.0, 0.045, line);
          float depth = length(vPos.xz);
          float fade = 1.0 - smoothstep(12.0, 78.0, depth);
          vec3 col = mix(uFill * 0.22, uLine, grid * fade);
          float fog = smoothstep(18.0, 88.0, depth);
          col = mix(col, uFog * 0.18, fog);
          gl_FragColor = vec4(col, 1.0);
        }
      `,
    });
    const ground = new THREE.Mesh(groundGeo, groundMat);
    ground.position.y = 0;
    scene.add(ground);
    disposables.push(groundGeo, groundMat);

    const sunGeo = new THREE.CircleGeometry(11, 64);
    const sunMat = new THREE.ShaderMaterial({
      transparent: true,
      depthWrite: false,
      uniforms: {
        uSun: { value: displayed[0].clone() },
        uBand: { value: displayed[2].clone() },
      },
      vertexShader: `
        varying vec2 vUv;
        void main() {
          vUv = uv;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `,
      fragmentShader: `
        varying vec2 vUv;
        uniform vec3 uSun;
        uniform vec3 uBand;
        void main() {
          vec2 p = vUv * 2.0 - 1.0;
          if (dot(p, p) > 1.0) discard;
          vec3 col = mix(uSun, uBand, smoothstep(0.15, 0.95, 1.0 - vUv.y));
          if (vUv.y < 0.52) {
            float stripes = step(0.45, fract(vUv.y * 18.0));
            col *= 0.35 + 0.65 * stripes;
          }
          gl_FragColor = vec4(col, 1.0);
        }
      `,
    });
    const sun = new THREE.Mesh(sunGeo, sunMat);
    sun.position.set(0, 7.2, -42);
    scene.add(sun);
    disposables.push(sunGeo, sunMat);

    const halo = new THREE.Sprite(
      new THREE.SpriteMaterial({
        map: glow,
        color: displayed[0],
        blending: THREE.AdditiveBlending,
        transparent: true,
        depthWrite: false,
        opacity: 0.7,
      })
    );
    halo.position.copy(sun.position);
    halo.scale.set(38, 28, 1);
    scene.add(halo);
    disposables.push(halo.material);

    tickers.push((dt, elapsed, paletteNow) => {
      groundMat.uniforms.uTime.value = elapsed;
      groundMat.uniforms.uLine.value.copy(paletteNow[0]);
      groundMat.uniforms.uFill.value.copy(paletteNow[3]);
      groundMat.uniforms.uFog.value.copy(paletteNow[1]);
      skyMat.uniforms.uTop.value.copy(paletteNow[3]);
      skyMat.uniforms.uMid.value.copy(paletteNow[1]);
      skyMat.uniforms.uSun.value.copy(paletteNow[0]);
      sunMat.uniforms.uSun.value.copy(paletteNow[0]);
      sunMat.uniforms.uBand.value.copy(paletteNow[2]);
      halo.material.color.copy(paletteNow[0]);
      camera.position.x = Math.sin(elapsed * 0.08) * 0.7;
      camera.lookAt(0, 3.4 + Math.sin(elapsed * 0.12) * 0.15, -30);
      void dt;
    });
  }

  function currentColors(): THREE.Color[] {
    if (fade >= 1) return to;
    const t = fade * fade * (3 - 2 * fade);
    return displayed.map((out, index) =>
      mixColor(from[index] ?? out, to[index] ?? out, t, out)
    );
  }

  function frame() {
    if (disposed) return;
    const dt = Math.min(0.05, clock.getDelta());
    const elapsed = clock.elapsedTime;
    if (fade < 1) fade = Math.min(1, fade + dt / PALETTE_FADE_SEC);
    const colors = currentColors();
    fogScratch.copy(night).lerp(colors[3], 0.28);
    renderer.setClearColor(fogScratch, 1);
    if (scene.fog) {
      if (scene.fog instanceof THREE.Fog || scene.fog instanceof THREE.FogExp2) {
        scene.fog.color.copy(fogScratch);
      }
    }
    const beat = resolveBeat(elapsed, playing);
    const motion = resolvePlaying(playing);
    const bands = getOsAudioFrame()?.bands ?? null;
    for (const tick of tickers) tick(dt, elapsed, colors, beat, motion, bands);
    renderer.render(scene, camera);
    raf = requestAnimationFrame(frame);
  }
  raf = requestAnimationFrame(frame);

  return {
    applyPalette(nextPalette: string[]) {
      from = currentColors().map((color) => color.clone());
      to = paletteColors(nextPalette);
      displayed = from.map((color) => color.clone());
      fade = 0;
    },
    setPlaying(next: boolean) {
      playing = next;
    },
    dispose() {
      disposed = true;
      cancelAnimationFrame(raf);
      observer.disconnect();
      for (const item of disposables) item.dispose();
      renderer.dispose();
      if (renderer.domElement.parentElement === host) {
        host.removeChild(renderer.domElement);
      }
    },
  };
}
