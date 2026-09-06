/** Non-interactive liquid gradient, adapted from the Made By Beings Three.js demo. */
export const liquidGradientFragmentShader = `#version 300 es
precision mediump float;

uniform float u_time;
uniform vec2 u_resolution;
uniform float u_pixelRatio;

uniform vec3 u_color1;
uniform vec3 u_color2;
uniform vec3 u_color3;
uniform vec3 u_color4;
uniform vec3 u_color5;
uniform vec3 u_color6;
uniform vec3 u_colorBack;
uniform float u_speed;
uniform float u_intensity;
uniform float u_grainIntensity;
uniform float u_gradientSize;
uniform float u_gradientCount;
uniform float u_color1Weight;
uniform float u_color2Weight;

out vec4 fragColor;

float grain(vec2 uv, float time) {
  vec2 grainUv = uv * u_resolution * 0.5;
  float grainValue = fract(sin(dot(grainUv + time, vec2(12.9898, 78.233))) * 43758.5453);
  return grainValue * 2.0 - 1.0;
}

vec3 getGradientColor(vec2 uv, float time) {
  float t = time * u_speed;
  float radius = u_gradientSize;

  vec2 c1 = vec2(0.5 + sin(t * 0.40) * 0.40, 0.5 + cos(t * 0.50) * 0.40);
  vec2 c2 = vec2(0.5 + cos(t * 0.60) * 0.50, 0.5 + sin(t * 0.45) * 0.50);
  vec2 c3 = vec2(0.5 + sin(t * 0.35) * 0.45, 0.5 + cos(t * 0.55) * 0.45);
  vec2 c4 = vec2(0.5 + cos(t * 0.50) * 0.40, 0.5 + sin(t * 0.40) * 0.40);
  vec2 c5 = vec2(0.5 + sin(t * 0.70) * 0.35, 0.5 + cos(t * 0.60) * 0.35);
  vec2 c6 = vec2(0.5 + cos(t * 0.45) * 0.50, 0.5 + sin(t * 0.65) * 0.50);
  vec2 c7 = vec2(0.5 + sin(t * 0.55) * 0.38, 0.5 + cos(t * 0.48) * 0.42);
  vec2 c8 = vec2(0.5 + cos(t * 0.65) * 0.36, 0.5 + sin(t * 0.52) * 0.44);
  vec2 c9 = vec2(0.5 + sin(t * 0.42) * 0.41, 0.5 + cos(t * 0.58) * 0.39);
  vec2 c10 = vec2(0.5 + cos(t * 0.48) * 0.37, 0.5 + sin(t * 0.62) * 0.43);
  vec2 c11 = vec2(0.5 + sin(t * 0.68) * 0.33, 0.5 + cos(t * 0.44) * 0.46);
  vec2 c12 = vec2(0.5 + cos(t * 0.38) * 0.39, 0.5 + sin(t * 0.56) * 0.41);

  float i1 = 1.0 - smoothstep(0.0, radius, length(uv - c1));
  float i2 = 1.0 - smoothstep(0.0, radius, length(uv - c2));
  float i3 = 1.0 - smoothstep(0.0, radius, length(uv - c3));
  float i4 = 1.0 - smoothstep(0.0, radius, length(uv - c4));
  float i5 = 1.0 - smoothstep(0.0, radius, length(uv - c5));
  float i6 = 1.0 - smoothstep(0.0, radius, length(uv - c6));
  float i7 = 1.0 - smoothstep(0.0, radius, length(uv - c7));
  float i8 = 1.0 - smoothstep(0.0, radius, length(uv - c8));
  float i9 = 1.0 - smoothstep(0.0, radius, length(uv - c9));
  float i10 = 1.0 - smoothstep(0.0, radius, length(uv - c10));
  float i11 = 1.0 - smoothstep(0.0, radius, length(uv - c11));
  float i12 = 1.0 - smoothstep(0.0, radius, length(uv - c12));

  vec2 r1 = uv - 0.5;
  float a1 = t * 0.15;
  r1 = vec2(r1.x * cos(a1) - r1.y * sin(a1), r1.x * sin(a1) + r1.y * cos(a1));
  r1 += 0.5;
  vec2 r2 = uv - 0.5;
  float a2 = -t * 0.12;
  r2 = vec2(r2.x * cos(a2) - r2.y * sin(a2), r2.x * sin(a2) + r2.y * cos(a2));
  r2 += 0.5;
  float radial1 = 1.0 - smoothstep(0.0, 0.8, length(r1 - 0.5));
  float radial2 = 1.0 - smoothstep(0.0, 0.8, length(r2 - 0.5));

  vec3 color = vec3(0.0);
  color += u_color1 * i1 * u_color1Weight;
  color += u_color2 * i2 * u_color2Weight;
  color += u_color3 * i3 * u_color1Weight;
  color += u_color4 * i4 * u_color2Weight;
  color += u_color5 * i5 * u_color1Weight;
  color += u_color6 * i6 * u_color2Weight;

  if (u_gradientCount > 6.0) {
    color += u_color1 * i7 * u_color1Weight;
    color += u_color2 * i8 * u_color2Weight;
    color += u_color3 * i9 * u_color1Weight;
    color += u_color4 * i10 * u_color2Weight;
  }
  if (u_gradientCount > 10.0) {
    color += u_color5 * i11 * u_color1Weight;
    color += u_color6 * i12 * u_color2Weight;
  }

  color += mix(u_color1, u_color3, radial1) * 0.45 * u_color1Weight;
  color += mix(u_color2, u_color4, radial2) * 0.4 * u_color2Weight;
  color = clamp(color, vec3(0.0), vec3(1.0)) * u_intensity;

  float luminance = dot(color, vec3(0.299, 0.587, 0.114));
  color = mix(vec3(luminance), color, 1.35);
  color = pow(color, vec3(0.92));

  float brightness = length(color);
  color = mix(u_colorBack, color, max(brightness * 1.2, 0.15));
  brightness = length(color);
  if (brightness > 1.0) {
    color *= 1.0 / brightness;
  }
  return color;
}

void main() {
  vec2 uv = gl_FragCoord.xy / u_resolution;
  vec3 color = getGradientColor(uv, u_time);

  color += grain(uv, u_time) * u_grainIntensity;

  float brightness = length(color);
  color = mix(u_colorBack, color, max(brightness * 1.2, 0.15));
  color = clamp(color, vec3(0.0), vec3(1.0));
  brightness = length(color);
  if (brightness > 1.0) {
    color *= 1.0 / brightness;
  }

  fragColor = vec4(color, 1.0);
}
`;
