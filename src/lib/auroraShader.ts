/** Full-view aurora + starfield, adapted from a Three.js ShaderMaterial demo. */
export const auroraFragmentShader = `#version 300 es
precision mediump float;

uniform float u_time;
uniform vec2 u_resolution;
uniform float u_pixelRatio;

uniform float u_dithering;
uniform float u_speed;
uniform float u_seed;
uniform vec3 u_colorBase;
uniform vec3 u_colorHigh;
uniform vec3 u_skyDark;
uniform vec3 u_skyDeep;
uniform float u_starDensity;
uniform float u_starSize;
uniform float u_starBlinkRate;
uniform float u_starIntensity;
uniform vec3 u_starColor;

out vec4 fragColor;

const int RAY_ITERATIONS = 56;
const float LUMINANCE_FACTOR = 0.1;
const float Y_OFFSET_BOTTOM = 50.0;
const float VOLUME_DEPTH = 75.0;
const vec3 BOUND_LOW = vec3(-250.0, Y_OFFSET_BOTTOM, -500.0);
const vec3 BOUND_HIGH = vec3(250.0, Y_OFFSET_BOTTOM + VOLUME_DEPTH, 500.0);

float generateRandomFloat(vec2 seedVal) {
  vec3 p3 = fract(vec3(seedVal.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

float computeHash1(float v) {
  vec3 hVec = fract(vec3(v) * 0.1031);
  hVec += dot(hVec, hVec.yzx + 19.19);
  return fract((hVec.x + hVec.y) * hVec.z);
}

float computeHash3(vec3 v3) {
  v3 = fract(v3 * vec3(0.1031, 0.1030, 0.0973));
  v3 += dot(v3, v3.yxz + 33.33);
  return fract((v3.xxy + v3.yxx) * v3.zyx).x;
}

float evaluateVolumeNoise(vec3 coord) {
  vec3 gridP = floor(coord);
  vec3 fractP = fract(coord);
  fractP = fractP * fractP * (3.0 - 2.0 * fractP);

  return mix(
    mix(
      mix(computeHash3(gridP + vec3(0.0, 0.0, 0.0)), computeHash3(gridP + vec3(1.0, 0.0, 0.0)), fractP.x),
      mix(computeHash3(gridP + vec3(0.0, 1.0, 0.0)), computeHash3(gridP + vec3(1.0, 1.0, 0.0)), fractP.x),
      fractP.y
    ),
    mix(
      mix(computeHash3(gridP + vec3(0.0, 0.0, 1.0)), computeHash3(gridP + vec3(1.0, 0.0, 1.0)), fractP.x),
      mix(computeHash3(gridP + vec3(0.0, 1.0, 1.0)), computeHash3(gridP + vec3(1.0, 1.0, 1.0)), fractP.x),
      fractP.y
    ),
    fractP.z
  );
}

float smoothCurve(float valT) {
  return valT * valT * valT * (valT * (6.0 * valT - 15.0) + 10.0);
}

float pickGradient(float hVal, float posVal) {
  return mod(floor(hVal * 10000.0), 2.0) < 0.5 ? posVal : -posVal;
}

float calculateLineNoise(float pt) {
  float ptInt = floor(pt);
  float ptFract = pt - ptInt;
  float wght = smoothCurve(ptFract);
  return mix(
    pickGradient(computeHash1(ptInt), ptFract),
    pickGradient(computeHash1(ptInt + 1.0), ptFract - 1.0),
    wght
  ) * 2.0;
}

float fractalVolumePattern(vec3 spacePt) {
  float accum = 0.0;
  float wghtSum = 0.0;
  float curWght = 1.0;
  float curFreq = 1.0;

  for (int stepId = 0; stepId < 3; stepId++) {
    float nVal = evaluateVolumeNoise(spacePt * curFreq);
    accum += (1.0 - nVal) * curWght;
    wghtSum += curWght;
    curWght *= 0.5;
    curFreq *= 2.0;
  }

  return clamp(accum / wghtSum, 0.0, 1.0);
}

float calculateRadiance(float curDist, float glowRadius, float pwr) {
  curDist = max(curDist, 1e-7);
  return pow(glowRadius / curDist, pwr);
}

vec2 computeBoxIntersection(vec3 rOrigin, vec3 rVector, vec3 bLow, vec3 bHigh) {
  vec3 tLower = (bLow - rOrigin) / rVector;
  vec3 tUpper = (bHigh - rOrigin) / rVector;
  vec3 tm1 = min(tLower, tUpper);
  vec3 tm2 = max(tLower, tUpper);
  float tNearBound = max(max(tm1.x, tm1.y), tm1.z);
  float tFarBound = min(min(tm2.x, tm2.y), tm2.z);
  return vec2(tNearBound, tFarBound);
}

bool isWithinBounds(vec3 curPos) {
  float epsilon = 1e-4;
  return (curPos.x > BOUND_LOW.x - epsilon) && (curPos.y > BOUND_LOW.y - epsilon) && (curPos.z > BOUND_LOW.z - epsilon) &&
    (curPos.x < BOUND_HIGH.x + epsilon) && (curPos.y < BOUND_HIGH.y + epsilon) && (curPos.z < BOUND_HIGH.z + epsilon);
}

bool checkVolumeHit(vec3 rOrg, vec3 rDir, out float enterDist, out float travelDist) {
  vec2 hitPoints = computeBoxIntersection(rOrg, rDir, BOUND_LOW, BOUND_HIGH);
  if (isWithinBounds(rOrg)) {
    hitPoints.x = 1e-4;
  }
  enterDist = hitPoints.x;
  travelDist = hitPoints.y - hitPoints.x;
  return hitPoints.x > 0.0 && (hitPoints.x < hitPoints.y);
}

vec3 blendAtmosphereTints(float heightRatio) {
  return mix(u_colorBase, u_colorHigh, heightRatio);
}

vec3 warpSpatialCoords(vec3 rawPos, float timeFlow) {
  float normHeight = (rawPos.y - Y_OFFSET_BOTTOM) / VOLUME_DEPTH;
  vec3 warpedP = 0.04 * vec3(rawPos.x, 2.0 * timeFlow, 0.225 * rawPos.z + timeFlow * 0.5);
  warpedP.xz += vec2(u_seed * 17.3, u_seed * 29.1);
  warpedP.x += 0.3 * normHeight + 5.5 * cos(0.005 * rawPos.z);
  warpedP.x += 0.02 * calculateLineNoise(0.1 * rawPos.z + timeFlow * 2.0);
  return warpedP;
}

float sampleCloudThickness(vec3 localPt) {
  float timeFlow = u_time * u_speed;
  vec3 shiftedPt = warpSpatialCoords(localPt, timeFlow);
  float basePattern = fractalVolumePattern(shiftedPt);
  vec3 shapePt = vec3(basePattern, localPt.y - BOUND_LOW.y, basePattern);
  vec3 squishedPt = shapePt * vec3(1.0, 0.006, 1.0);
  squishedPt.y += 0.48;
  squishedPt.y += 0.015 * calculateLineNoise(1.0 * timeFlow + shiftedPt.z);
  squishedPt.y += 0.015 * calculateLineNoise(-2.0 * timeFlow + shiftedPt.z);
  float thickness = calculateRadiance(length(squishedPt), 0.55, 12.0);
  thickness *= cos(0.13 * shiftedPt.x);
  return max(0.0, thickness);
}

vec3 renderAtmosphericLights(vec3 camOrg, vec3 camDir, float noiseShift) {
  vec3 accumColor = vec3(0.0);
  float startTrace = 0.0;
  float traceLen = 0.0;
  if (!checkVolumeHit(camOrg, camDir, startTrace, traceLen)) {
    return accumColor;
  }

  float marchStep = traceLen / float(RAY_ITERATIONS);
  startTrace += marchStep * (noiseShift * 0.25);
  vec3 currentPos = camOrg + startTrace * camDir;
  float currentDist = startTrace;
  vec3 lightAccum = vec3(0.0);

  for (int stepIdx = 0; stepIdx < RAY_ITERATIONS; stepIdx++) {
    float localDens = sampleCloudThickness(currentPos);
    lightAccum += localDens * blendAtmosphereTints((currentPos.y - BOUND_LOW.y) / (BOUND_HIGH.y - BOUND_LOW.y));
    currentDist += marchStep;
    currentPos = camOrg + camDir * currentDist;
  }

  return LUMINANCE_FACTOR * lightAccum * marchStep;
}

vec3 renderStarfield(vec3 viewDir, float timeFlow) {
  float gridScale = 400.0;
  vec3 spaceGrid = floor(viewDir * gridScale);
  vec3 spaceLocal = fract(viewDir * gridScale) - 0.5;
  float cellHash = computeHash3(spaceGrid);
  float threshold = 1.0 - (u_starDensity * 0.15);
  float starExistence = step(threshold, cellHash);
  float pixelSize = (gridScale * 1.5) / max(u_resolution.y, 1.0);
  float radius = max(0.08, pixelSize) * u_starSize;
  radius = min(radius, 0.5);
  float starGlow = smoothstep(radius, 0.0, length(spaceLocal));
  starGlow *= (0.08 / radius);
  float blinkChance = fract(cellHash * 31.415);
  float isTwinkling = step(0.85, blinkChance);
  float blinkAnim = 0.3 + 0.7 * sin(timeFlow * u_starBlinkRate * (1.5 + blinkChance * 2.0) + cellHash * 100.0);
  float twinkleFlow = mix(1.0, blinkAnim, isTwinkling);
  vec3 starTint = mix(vec3(0.7, 0.85, 1.0), u_starColor, fract(cellHash * 13.0));
  return starTint * starExistence * starGlow * twinkleFlow;
}

vec3 paintBackdrop(vec3 viewVec) {
  return mix(u_skyDark, u_skyDeep, clamp(viewVec.y * 1.5, 0.0, 1.0));
}

vec3 applyToneMapping(vec3 rawCol) {
  return clamp((rawCol * (2.51 * rawCol + 0.03)) / (rawCol * (2.43 * rawCol + 0.59) + 0.14), 0.0, 1.0);
}

mat3 buildCameraFrame(vec3 focusPt, vec3 upVec) {
  vec3 zAxis = normalize(focusPt);
  vec3 xAxis = normalize(cross(zAxis, upVec));
  vec3 yAxis = cross(xAxis, zAxis);
  return mat3(xAxis, yAxis, -zAxis);
}

void main() {
  vec2 res = max(u_resolution, vec2(1.0));
  vec2 fragCoord = gl_FragCoord.xy;
  vec2 screenPos = fragCoord - res.xy / 2.0;
  float focalLen = (0.5 * res.y) / tan(radians(60.0) / 2.0);
  vec3 sightVec = normalize(vec3(screenPos, -focalLen));

  vec3 viewerLoc = vec3(0.0, 10.0, 0.0);
  vec3 gazePoint = vec3(-0.4, 0.45, -1.0);
  gazePoint.x += sin(u_time * 0.1) * 0.1;
  gazePoint.y += cos(u_time * 0.05) * 0.05;

  mat3 viewTransform = buildCameraFrame(gazePoint, vec3(0.0, 1.0, 0.0));
  sightVec = normalize(viewTransform * sightVec);

  float ditherShift = generateRandomFloat(fragCoord + vec2(u_time * 13.0, u_time * 27.0));
  vec3 finalOutput = paintBackdrop(sightVec);

  float screenY = fragCoord.y / max(res.y, 1.0);
  float fadeMask = smoothstep(0.05, 0.4, screenY);

  vec3 stars = renderStarfield(sightVec, u_time) * u_starIntensity;
  finalOutput += stars * fadeMask;

  vec3 auroraLights = renderAtmosphericLights(viewerLoc + sightVec * 10.0, sightVec, ditherShift);
  finalOutput += auroraLights * fadeMask;

  finalOutput = applyToneMapping(finalOutput);
  finalOutput = pow(finalOutput, vec3(0.4545));
  float cleanNoise = generateRandomFloat(fragCoord + vec2(u_time * 17.0, -u_time * 11.0));
  finalOutput += (cleanNoise - 0.5) * u_dithering;

  fragColor = vec4(finalOutput, 1.0);
}
`;
