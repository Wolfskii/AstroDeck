/** Extra appearance shaders — pattern motion only, no global brightness pulse. */
export const silkFragmentShader = `#version 300 es
precision mediump float;

uniform float u_time;
uniform vec2 u_resolution;
uniform float u_pixelRatio;
uniform vec3 u_colorA;
uniform vec3 u_colorB;
uniform vec3 u_colorC;
uniform vec3 u_colorBack;
uniform float u_speed;

out vec4 fragColor;

void main() {
  vec2 uv = (gl_FragCoord.xy / (u_resolution * u_pixelRatio)) * 2.0 - 1.0;
  uv.x *= u_resolution.x / max(u_resolution.y, 1.0);
  float t = u_time * u_speed;
  vec2 p = uv;
  p.y += 0.18 * sin(p.x * 2.4 + t * 0.62);
  p.x += 0.14 * sin(p.y * 3.05 - t * 0.48);
  p.y += 0.08 * sin(p.x * 5.1 - t * 0.33);
  float w1 = sin(p.x * 3.6 + p.y * 1.15 + t * 0.72);
  float w2 = sin(p.x * 1.2 - p.y * 3.8 - t * 0.51);
  float w3 = sin((p.x + p.y) * 2.05 + t * 0.38);
  float v = w1 * 0.42 + w2 * 0.34 + w3 * 0.24;
  v = v * 0.5 + 0.5;
  vec3 col = mix(u_colorBack * 0.4, u_colorA, smoothstep(0.12, 0.48, v));
  col = mix(col, u_colorB, smoothstep(0.38, 0.72, v));
  col = mix(col, u_colorC, smoothstep(0.66, 0.94, v) * 0.55);
  fragColor = vec4(col, 1.0);
}
`;

export const vortexFragmentShader = `#version 300 es
precision mediump float;

uniform float u_time;
uniform vec2 u_resolution;
uniform float u_pixelRatio;
uniform vec3 u_colorA;
uniform vec3 u_colorB;
uniform vec3 u_colorC;
uniform vec3 u_colorBack;
uniform float u_speed;

out vec4 fragColor;

void main() {
  vec2 uv = (gl_FragCoord.xy / (u_resolution * u_pixelRatio)) * 2.0 - 1.0;
  uv.x *= u_resolution.x / max(u_resolution.y, 1.0);
  float t = u_time * u_speed;
  float ang = atan(uv.y, uv.x);
  float rad = length(uv);
  // Integer windings so sin() stays continuous across atan's -PI/PI seam.
  float bands = 0.5 + 0.5 * sin(ang * 8.0 + rad * 9.5 - t * 2.2);
  float bands2 = 0.5 + 0.5 * sin(ang * 5.0 - rad * 12.0 - t * 1.4);
  vec3 col = mix(u_colorBack * 0.35, u_colorA, bands * 0.7);
  col = mix(col, u_colorB, bands2 * 0.45);
  col = mix(col, u_colorC, smoothstep(0.72, 0.98, bands) * 0.28);
  float hole = smoothstep(0.0, 0.42, rad);
  float edge = 1.0 - smoothstep(1.15, 1.7, rad);
  col = mix(u_colorBack * 0.5, col, hole * edge);
  fragColor = vec4(col, 1.0);
}
`;

export const mosaicFragmentShader = `#version 300 es
precision mediump float;

uniform float u_time;
uniform vec2 u_resolution;
uniform float u_pixelRatio;
uniform vec3 u_colorA;
uniform vec3 u_colorB;
uniform vec3 u_colorC;
uniform vec3 u_colorBack;
uniform float u_speed;

out vec4 fragColor;

vec2 hash2(vec2 p) {
  p = vec2(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)));
  return fract(sin(p) * 43758.5453);
}

void main() {
  vec2 uv = (gl_FragCoord.xy / (u_resolution * u_pixelRatio));
  uv.x *= u_resolution.x / max(u_resolution.y, 1.0);
  float t = u_time * u_speed;
  float scale = 6.2;
  vec2 p = uv * scale;
  vec2 n = floor(p);
  vec2 f = fract(p);
  float minD = 8.0;
  float minD2 = 8.0;
  vec2 winner = vec2(0.0);
  for (int j = -1; j <= 1; j++) {
    for (int i = -1; i <= 1; i++) {
      vec2 g = vec2(float(i), float(j));
      vec2 o = hash2(n + g);
      o = 0.5 + 0.5 * sin(t * 0.55 + 6.28318 * o);
      vec2 r = g + o - f;
      float d = dot(r, r);
      if (d < minD) {
        minD2 = minD;
        minD = d;
        winner = o;
      } else if (d < minD2) {
        minD2 = d;
      }
    }
  }
  float grout = smoothstep(0.0, 0.07, sqrt(minD2) - sqrt(minD));
  vec3 cell = mix(u_colorA, u_colorB, winner.x);
  cell = mix(cell, u_colorC, winner.y * 0.65);
  vec3 col = mix(u_colorBack * 0.28, cell, grout);
  fragColor = vec4(col, 1.0);
}
`;
