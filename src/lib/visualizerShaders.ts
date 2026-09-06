/** Flowing plasma and kaleidoscope — motion in the pattern, not brightness flashes. */
export const plasmaFragmentShader = `#version 300 es
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
  vec2 p = uv + 0.32 * vec2(sin(t * 0.23), cos(t * 0.19));
  float v = 0.0;
  v += sin(p.x * 6.2 + t * 0.85);
  v += sin(p.y * 7.4 - t * 0.62);
  v += sin((p.x + p.y) * 4.6 + t * 0.41);
  v += sin(length(p + vec2(sin(t * 0.31), cos(t * 0.27))) * 9.5 - t * 1.05);
  v = v * 0.25 + 0.5;
  vec3 col = mix(u_colorBack * 0.42, u_colorA, smoothstep(0.18, 0.52, v));
  col = mix(col, u_colorB, smoothstep(0.42, 0.74, v));
  col = mix(col, u_colorC, smoothstep(0.68, 0.96, v));
  fragColor = vec4(col, 1.0);
}
`;

export const kaleidoFragmentShader = `#version 300 es
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

const float PI = 3.14159265;

void main() {
  vec2 uv = (gl_FragCoord.xy / (u_resolution * u_pixelRatio)) * 2.0 - 1.0;
  uv.x *= u_resolution.x / max(u_resolution.y, 1.0);
  float t = u_time * u_speed;
  float r = length(uv);
  float a = atan(uv.y, uv.x) + t * 0.38;
  float segments = 8.0;
  a = mod(a, PI * 2.0 / segments);
  a = abs(a - PI / segments);
  vec2 p = vec2(cos(a), sin(a)) * r;
  p += 0.24 * vec2(sin(t * 0.52), cos(t * 0.41));
  float v = sin(p.x * 7.0 + t * 0.95) + sin(p.y * 8.5 - t * 1.08) + sin((p.x + p.y) * 5.0 + t * 0.62);
  v = v * 0.25 + 0.5;
  vec3 col = mix(u_colorBack * 0.28, u_colorA, smoothstep(0.12, 0.45, v));
  col = mix(col, u_colorB, smoothstep(0.4, 0.72, v));
  col = mix(col, u_colorC, smoothstep(0.7, 1.0, v) * 0.55);
  float vignette = smoothstep(1.35, 0.32, r);
  col *= vignette;
  fragColor = vec4(col, 1.0);
}
`;
