/** Winamp-style plasma and kaleidoscope visualizers for ShaderMount. */
export const plasmaFragmentShader = `#version 300 es
precision mediump float;

uniform float u_time;
uniform vec2 u_resolution;
uniform float u_pixelRatio;
uniform vec3 u_colorA;
uniform vec3 u_colorB;
uniform vec3 u_colorC;
uniform vec3 u_colorBack;
uniform float u_beat;
uniform float u_speed;

out vec4 fragColor;

void main() {
  vec2 uv = (gl_FragCoord.xy / (u_resolution * u_pixelRatio)) * 2.0 - 1.0;
  uv.x *= u_resolution.x / max(u_resolution.y, 1.0);
  float t = u_time * u_speed;
  float v = 0.0;
  v += sin(uv.x * 8.0 + t);
  v += sin(uv.y * 9.2 + t * 1.21);
  v += sin((uv.x + uv.y) * 6.4 + t * 0.72);
  v += sin(length(uv) * 12.0 - t * 1.6);
  v = v * 0.25 + 0.5;
  vec3 col = mix(u_colorA, u_colorB, smoothstep(0.15, 0.7, v));
  col = mix(col, u_colorC, smoothstep(0.55, 1.0, v));
  float glow = 0.42 + u_beat * 0.7;
  col = mix(u_colorBack * 0.25, col, glow);
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
uniform float u_beat;
uniform float u_speed;

out vec4 fragColor;

const float PI = 3.14159265;

void main() {
  vec2 uv = (gl_FragCoord.xy / (u_resolution * u_pixelRatio)) * 2.0 - 1.0;
  uv.x *= u_resolution.x / max(u_resolution.y, 1.0);
  float t = u_time * u_speed;
  float r = length(uv);
  float a = atan(uv.y, uv.x) + t * 0.18;
  float segments = 8.0;
  a = mod(a, PI * 2.0 / segments);
  a = abs(a - PI / segments);
  vec2 p = vec2(cos(a), sin(a)) * r;
  p += 0.18 * vec2(sin(t * 0.7), cos(t * 0.55));
  float v = sin(p.x * 7.0 + t) + sin(p.y * 8.5 - t * 1.1) + sin((p.x + p.y) * 5.0 + t * 0.8);
  v = v * 0.25 + 0.5;
  vec3 col = mix(u_colorBack * 0.2, u_colorA, smoothstep(0.12, 0.45, v));
  col = mix(col, u_colorB, smoothstep(0.4, 0.72, v));
  col = mix(col, u_colorC, smoothstep(0.7, 1.0, v) * (0.4 + u_beat * 0.6));
  float vignette = smoothstep(1.35, 0.35, r);
  col *= vignette * (0.55 + u_beat * 0.55);
  fragColor = vec4(col, 1.0);
}
`;
