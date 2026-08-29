// Main glass composition shader ported from liquid-glass-studio.

const PI: f32 = 3.14159265359;
const N_R: f32 = 0.98;
const N_G: f32 = 1.0;
const N_B: f32 = 1.02;

struct Uniforms {
  u_resolution: vec2f,
  u_dpr: f32,
  _pad0: f32,
  u_mouse: vec2f,
  u_mouseSpring: vec2f,
  u_shapeWidth: f32,
  u_shapeHeight: f32,
  u_shapeRadius: f32,
  u_shapeRoundness: f32,
  u_capsuleBezierX: vec4f,
  u_capsuleBezierY: vec4f,
  u_mergeRate: f32,
  u_glareAngle: f32,
  u_shadowExpand: f32,
  u_shadowFactor: f32,
  u_shadowPosition: vec2f,
  u_bgTextureRatio: f32,
  u_bgType: i32,
  u_bgTextureReady: i32,
  u_showShape1: i32,
  u_blurRadius: i32,
  u_blurEdge: i32,
  u_tint: vec4f,
  u_refThickness: f32,
  u_refFactor: f32,
  u_refDispersion: f32,
  u_refFresnelRange: f32,
  u_refFresnelHardness: f32,
  u_refFresnelFactor: f32,
  u_glareRange: f32,
  u_glareHardness: f32,
  u_glareConvergence: f32,
  u_glareOppositeFactor: f32,
  u_glareFactor: f32,
  _pad1: f32,
  _pad2: vec4f,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var u_blurredBg: texture_2d<f32>;
@group(0) @binding(2) var u_bg: texture_2d<f32>;
@group(0) @binding(3) var u_sampler: sampler;

fn sdCircle(p: vec2f, r: f32) -> f32 {
  return length(p) - r;
}

fn continuousCornerSDF(p_in: vec2f, r: f32, n: f32) -> f32 {
  let p = abs(p_in);
  let exponent = max(n, 2.0);
  let v = pow(pow(p.x, exponent) + pow(p.y, exponent), 1.0 / exponent);
  return v - r;
}

fn roundedRectSDF(p_in: vec2f, center: vec2f, width: f32, height: f32, cornerRadius: f32, n: f32) -> f32 {
  let p = p_in - center;
  let cr = cornerRadius * u.u_dpr;
  let d = abs(p) - vec2f(width * u.u_dpr, height * u.u_dpr) * 0.5;
  var dist: f32;
  if (d.x > -cr && d.y > -cr) {
    let cornerCenter = sign(p) * (vec2f(width * u.u_dpr, height * u.u_dpr) * 0.5 - vec2f(cr));
    let cornerP = p - cornerCenter;
    dist = continuousCornerSDF(cornerP, cr, n);
  } else {
    dist = min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));
  }
  return dist;
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
  let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
  return mix(b, a, h) - k * h * (1.0 - h);
}

fn cubicCoordinate(points: vec4f, t: f32) -> f32 {
  let oneMinusT = 1.0 - t;
  return points.x * oneMinusT * oneMinusT * oneMinusT
    + 3.0 * points.y * oneMinusT * oneMinusT * t
    + 3.0 * points.z * oneMinusT * t * t
    + points.w * t * t * t;
}

fn cubicDerivative(points: vec4f, t: f32) -> f32 {
  let oneMinusT = 1.0 - t;
  return 3.0 * (points.y - points.x) * oneMinusT * oneMinusT
    + 6.0 * (points.z - points.y) * oneMinusT * t
    + 3.0 * (points.w - points.z) * t * t;
}

fn capsuleBezierParameter(longitudinal: f32) -> f32 {
  let points = u.u_capsuleBezierX;
  var t = clamp((longitudinal - points.x) / max(points.w - points.x, 0.000001), 0.0, 1.0);
  for (var iteration = 0; iteration < 5; iteration += 1) {
    let derivative = cubicDerivative(points, t);
    if (abs(derivative) > 0.000001) {
      t = clamp(t - (cubicCoordinate(points, t) - longitudinal) / derivative, 0.0, 1.0);
    }
  }
  return t;
}

fn continuousCapsuleSDF(
  p_in: vec2f,
  center: vec2f,
  width: f32,
  height: f32,
) -> f32 {
  let p = abs(p_in - center);
  let halfSize = vec2f(width, height) * u.u_dpr * 0.5;
  let radius = min(halfSize.x, halfSize.y);
  let horizontal = halfSize.x >= halfSize.y;
  let halfLong = select(halfSize.y, halfSize.x, horizontal);
  let longitudinal = select(p.y, p.x, horizontal);
  let transverse = select(p.x, p.y, horizontal);
  let capCenter = max(halfLong - radius, 0.0);

  if (capCenter <= 0.000001) {
    return sdCircle(p, radius);
  }

  let normalizedLongitudinal = (longitudinal - capCenter) / radius;
  if (normalizedLongitudinal <= u.u_capsuleBezierX.x) {
    return transverse - radius;
  }
  if (normalizedLongitudinal >= u.u_capsuleBezierX.w) {
    return length(vec2f(longitudinal - capCenter, transverse)) - radius;
  }

  let t = capsuleBezierParameter(normalizedLongitudinal);
  let curveY = cubicCoordinate(u.u_capsuleBezierY, t);
  let derivativeX = cubicDerivative(u.u_capsuleBezierX, t);
  let derivativeY = cubicDerivative(u.u_capsuleBezierY, t);
  let profile = radius * (1.0 - curveY);
  let slope = -derivativeY / max(derivativeX, 0.000001);
  return (transverse - profile) / sqrt(1.0 + slope * slope);
}

fn mainSDF(p1: vec2f, p2: vec2f, p: vec2f) -> f32 {
  let p1n = p1 + p / u.u_resolution.y;
  let p2n = p2 + p / u.u_resolution.y;
  var d1: f32;
  if (u.u_showShape1 == 1) {
    d1 = sdCircle(p1n, 100.0 * u.u_dpr / u.u_resolution.y);
  } else {
    d1 = 1.0;
  }
  var d2: f32;
  if (u.u_shapeRoundness < 0.0) {
    d2 = continuousCapsuleSDF(
      p2n,
      vec2f(0.0),
      u.u_shapeWidth / u.u_resolution.y,
      u.u_shapeHeight / u.u_resolution.y,
    );
  } else {
    d2 = roundedRectSDF(
      p2n,
      vec2f(0.0),
      u.u_shapeWidth / u.u_resolution.y,
      u.u_shapeHeight / u.u_resolution.y,
      u.u_shapeRadius / u.u_resolution.y,
      u.u_shapeRoundness,
    );
  }
  return smin(d1, d2, u.u_mergeRate);
}

fn safeAsin(x: f32) -> f32 {
  return asin(clamp(x, -1.0, 1.0));
}

fn getNormal(p1: vec2f, p2: vec2f, p: vec2f) -> vec2f {
  let h = vec2f(1.0, 1.0);
  let grad = vec2f(
    mainSDF(p1, p2, p + vec2f(h.x, 0.0)) - mainSDF(p1, p2, p - vec2f(h.x, 0.0)),
    mainSDF(p1, p2, p + vec2f(0.0, h.y)) - mainSDF(p1, p2, p - vec2f(0.0, h.y)),
  ) / (2.0 * h);
  return grad * 1.414213562 * 1000.0;
}

fn safeNormalize(v: vec2f) -> vec2f {
  let len = length(v);
  if (len < 1e-8) {
    return vec2f(0.0);
  }
  return v / len;
}

const D65_WHITE: vec3f = vec3f(0.95045592705, 1.0, 1.08905775076);
const RGB_TO_XYZ_M_COL0: vec3f = vec3f(0.4124, 0.3576, 0.1805);
const RGB_TO_XYZ_M_COL1: vec3f = vec3f(0.2126, 0.7152, 0.0722);
const RGB_TO_XYZ_M_COL2: vec3f = vec3f(0.0193, 0.1192, 0.9505);
const XYZ_TO_RGB_M_COL0: vec3f = vec3f(3.2406255, -1.537208, -0.4986286);
const XYZ_TO_RGB_M_COL1: vec3f = vec3f(-0.9689307, 1.8757561, 0.0415175);
const XYZ_TO_RGB_M_COL2: vec3f = vec3f(0.0557101, -0.2040211, 1.0569959);

fn UNCOMPAND_SRGB(a: f32) -> f32 {
  if (a > 0.04045) { return pow((a + 0.055) / 1.055, 2.4); }
  return a / 12.92;
}

fn COMPAND_RGB(a: f32) -> f32 {
  if (a <= 0.0031308) { return 12.92 * a; }
  return 1.055 * pow(a, 0.41666666666) - 0.055;
}

fn SRGB_TO_RGB(srgb: vec3f) -> vec3f {
  return vec3f(UNCOMPAND_SRGB(srgb.x), UNCOMPAND_SRGB(srgb.y), UNCOMPAND_SRGB(srgb.z));
}

fn RGB_TO_XYZ(rgb: vec3f) -> vec3f {
  return vec3f(dot(rgb, RGB_TO_XYZ_M_COL0), dot(rgb, RGB_TO_XYZ_M_COL1), dot(rgb, RGB_TO_XYZ_M_COL2));
}

fn XYZ_TO_RGB(xyz: vec3f) -> vec3f {
  return vec3f(dot(xyz, XYZ_TO_RGB_M_COL0), dot(xyz, XYZ_TO_RGB_M_COL1), dot(xyz, XYZ_TO_RGB_M_COL2));
}

fn SRGB_TO_XYZ(srgb: vec3f) -> vec3f { return RGB_TO_XYZ(SRGB_TO_RGB(srgb)); }
fn XYZ_TO_SRGB(xyz: vec3f) -> vec3f { return vec3f(COMPAND_RGB(XYZ_TO_RGB(xyz).x), COMPAND_RGB(XYZ_TO_RGB(xyz).y), COMPAND_RGB(XYZ_TO_RGB(xyz).z)); }

fn XYZ_TO_LAB_F(x: f32) -> f32 {
  if (x > 0.00885645167) { return pow(x, 0.333333333); }
  return 7.78703703704 * x + 0.13793103448;
}

fn XYZ_TO_LAB(xyz: vec3f) -> vec3f {
  let xyz_scaled = vec3f(XYZ_TO_LAB_F(xyz.x / D65_WHITE.x), XYZ_TO_LAB_F(xyz.y / D65_WHITE.y), XYZ_TO_LAB_F(xyz.z / D65_WHITE.z));
  return vec3f(116.0 * xyz_scaled.y - 16.0, 500.0 * (xyz_scaled.x - xyz_scaled.y), 200.0 * (xyz_scaled.y - xyz_scaled.z));
}

fn SRGB_TO_LAB(srgb: vec3f) -> vec3f { return XYZ_TO_LAB(SRGB_TO_XYZ(srgb)); }
fn LAB_TO_LCH(Lab: vec3f) -> vec3f { return vec3f(Lab.x, sqrt(dot(Lab.yz, Lab.yz)), atan2(Lab.z, Lab.y) * 57.2957795131); }
fn SRGB_TO_LCH(srgb: vec3f) -> vec3f { return LAB_TO_LCH(SRGB_TO_LAB(srgb)); }

fn LAB_TO_XYZ_F(x: f32) -> f32 {
  if (x > 0.206897) { return x * x * x; }
  return 0.12841854934 * (x - 0.137931034);
}

fn LAB_TO_XYZ(Lab: vec3f) -> vec3f {
  let w = (Lab.x + 16.0) / 116.0;
  return D65_WHITE * vec3f(LAB_TO_XYZ_F(w + Lab.y / 500.0), LAB_TO_XYZ_F(w), LAB_TO_XYZ_F(w - Lab.z / 200.0));
}

fn LAB_TO_SRGB(lab: vec3f) -> vec3f {
  let rgb = XYZ_TO_RGB(LAB_TO_XYZ(lab));
  return vec3f(COMPAND_RGB(rgb.x), COMPAND_RGB(rgb.y), COMPAND_RGB(rgb.z));
}

fn LCH_TO_LAB(LCh: vec3f) -> vec3f {
  return vec3f(LCh.x, LCh.y * cos(LCh.z * 0.01745329251), LCh.y * sin(LCh.z * 0.01745329251));
}

fn LCH_TO_SRGB(lch: vec3f) -> vec3f { return LAB_TO_SRGB(LCH_TO_LAB(lch)); }

fn vec2ToAngle(v: vec2f) -> f32 {
  var angle = atan2(v.y, v.x);
  if (angle < 0.0) { angle += 2.0 * PI; }
  return angle;
}

fn sampleBlurred(v_uv: vec2f, offset: vec2f) -> vec4f {
  if (u.u_bgType == 12 && u.u_bgTextureReady != 1) {
    // In a real transparent window the OS owns the pixels behind us. Keep
    // the custom surface neutral so the actual desktop remains visible.
    return vec4f(1.0);
  }
  let sharp = textureSampleLevel(u_bg, u_sampler, v_uv + offset, 0.0);
  let blurred = textureSampleLevel(u_blurredBg, u_sampler, v_uv + offset, 0.0);
  return blurred;
}

fn getTextureDispersion(v_uv: vec2f, mixRate: f32, offset: vec2f, factor: f32) -> vec4f {
  var pixel = vec4f(1.0);
  if (u.u_bgType == 12 && u.u_bgTextureReady != 1) {
    return pixel;
  }
  let bgR = textureSampleLevel(u_bg, u_sampler, v_uv + offset * (1.0 - (N_R - 1.0) * factor), 0.0).r;
  let bgG = textureSampleLevel(u_bg, u_sampler, v_uv + offset * (1.0 - (N_G - 1.0) * factor), 0.0).g;
  let bgB = textureSampleLevel(u_bg, u_sampler, v_uv + offset * (1.0 - (N_B - 1.0) * factor), 0.0).b;
  let blurR = textureSampleLevel(u_blurredBg, u_sampler, v_uv + offset * (1.0 - (N_R - 1.0) * factor), 0.0).r;
  let blurG = textureSampleLevel(u_blurredBg, u_sampler, v_uv + offset * (1.0 - (N_G - 1.0) * factor), 0.0).g;
  let blurB = textureSampleLevel(u_blurredBg, u_sampler, v_uv + offset * (1.0 - (N_B - 1.0) * factor), 0.0).b;
  pixel.r = mix(bgR, blurR, mixRate);
  pixel.g = mix(bgG, blurG, mixRate);
  pixel.b = mix(bgB, blurB, mixRate);
  return pixel;
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4f, @location(0) v_uv: vec2f) -> @location(0) vec4f {
  let u_resolution1x = u.u_resolution / u.u_dpr;
  let pixel = vec2f(frag_coord.x, u.u_resolution.y - frag_coord.y);
  let p1 = (vec2f(0.0) - u.u_resolution * 0.5) / u.u_resolution.y;
  let p2 = (vec2f(0.0) - u.u_mouseSpring) / u.u_resolution.y;
  let merged = mainSDF(p1, p2, pixel);
  let shadowP1 = (vec2f(0.0) - u.u_resolution * 0.5 + vec2f(u.u_shadowPosition.x * u.u_dpr, u.u_shadowPosition.y * u.u_dpr)) / u.u_resolution.y;
  let shadowP2 = (vec2f(0.0) - u.u_mouseSpring + vec2f(u.u_shadowPosition.x * u.u_dpr, u.u_shadowPosition.y * u.u_dpr)) / u.u_resolution.y;
  let shadowMerged = mainSDF(shadowP1, shadowP2, pixel);
  var outColor: vec4f;

  if (merged < 0.005) {
    let nmerged = -1.0 * (merged * u_resolution1x.y);
    let x_R_ratio = 1.0 - nmerged / u.u_refThickness;
    let thetaI = safeAsin(pow(x_R_ratio, 2.0));
    let thetaT = safeAsin(1.0 / u.u_refFactor * sin(thetaI));
    var edgeFactor = -1.0 * tan(thetaT - thetaI);
    if (nmerged >= u.u_refThickness) { edgeFactor = 0.0; }

    if (edgeFactor <= 0.0) {
      outColor = sampleBlurred(v_uv, vec2f(0.0));
      outColor = mix(outColor, vec4f(u.u_tint.r, u.u_tint.g, u.u_tint.b, 1.0), u.u_tint.a * 0.8);
    } else {
      let edgeH = nmerged / u.u_refThickness;
      let normal = getNormal(p1, p2, pixel);
      var blurMixRate: f32;
      if (u.u_blurEdge > 0) { blurMixRate = 1.0; } else { blurMixRate = edgeH; }
      let refOffset = -normal * edgeFactor * 0.05 * u.u_dpr * vec2f(u.u_resolution.y / (u_resolution1x.x * u.u_dpr), 1.0);
      let blurredPixel = getTextureDispersion(v_uv, blurMixRate, vec2f(refOffset.x, -refOffset.y), u.u_refDispersion);
      outColor = mix(blurredPixel, vec4f(u.u_tint.r, u.u_tint.g, u.u_tint.b, 1.0), u.u_tint.a * 0.8);

      let fresnelFactor = clamp(pow(1.0 + merged * u_resolution1x.y / 1500.0 * pow(500.0 / u.u_refFresnelRange, 2.0) + u.u_refFresnelHardness, 5.0), 0.0, 1.0);
      var fresnelTintLCH = SRGB_TO_LCH(mix(vec3f(1.0), vec3f(u.u_tint.r, u.u_tint.g, u.u_tint.b), u.u_tint.a * 0.5));
      fresnelTintLCH.x += 20.0 * fresnelFactor * u.u_refFresnelFactor;
      fresnelTintLCH.x = clamp(fresnelTintLCH.x, 0.0, 100.0);
      outColor = mix(outColor, vec4f(LCH_TO_SRGB(fresnelTintLCH), 1.0), fresnelFactor * u.u_refFresnelFactor * 0.7 * length(normal));

      let glareGeoFactor = clamp(pow(1.0 + merged * u_resolution1x.y / 1500.0 * pow(500.0 / u.u_glareRange, 2.0) + u.u_glareHardness, 5.0), 0.0, 1.0);
      let glareAngle = (vec2ToAngle(safeNormalize(normal)) - PI / 4.0 + u.u_glareAngle) * 2.0;
      var glareFarside: i32 = 0;
      if ((glareAngle > PI * (2.0 - 0.5) && glareAngle < PI * (4.0 - 0.5)) || glareAngle < PI * (0.0 - 0.5)) { glareFarside = 1; }
      var glareSideFactor: f32;
      if (glareFarside == 1) { glareSideFactor = 1.2 * u.u_glareOppositeFactor; } else { glareSideFactor = 1.2; }
      var glareAngleFactor = (0.5 + sin(glareAngle) * 0.5) * glareSideFactor * u.u_glareFactor;
      glareAngleFactor = clamp(pow(glareAngleFactor, 0.1 + u.u_glareConvergence * 2.0), 0.0, 1.0);
      var glareTintLCH = SRGB_TO_LCH(mix(blurredPixel.rgb, vec3f(u.u_tint.r, u.u_tint.g, u.u_tint.b), u.u_tint.a * 0.5));
      glareTintLCH.x += 150.0 * glareAngleFactor * glareGeoFactor;
      glareTintLCH.y += 30.0 * glareAngleFactor * glareGeoFactor;
      glareTintLCH.x = clamp(glareTintLCH.x, 0.0, 120.0);
      outColor = mix(outColor, vec4f(LCH_TO_SRGB(glareTintLCH), 1.0), glareAngleFactor * glareGeoFactor * length(normal));
    }
  } else {
    outColor = textureSampleLevel(u_bg, u_sampler, v_uv, 0.0);
  }

  let shapeAlpha = 1.0 - smoothstep(-0.001, 0.001, merged);
  outColor = mix(outColor, textureSampleLevel(u_bg, u_sampler, v_uv, 0.0), 1.0 - shapeAlpha);
  let shadowVisible = (1.0 - shapeAlpha) * step(0.0, shadowMerged);
  let shadowAlpha = exp(-1.0 / u.u_shadowExpand * max(shadowMerged, 0.0) * u_resolution1x.y)
    * 0.6 * u.u_shadowFactor * shadowVisible;
  outColor = mix(outColor, vec4f(0.0, 0.0, 0.0, 1.0), shadowAlpha);
  return vec4f(outColor.rgb, max(shapeAlpha * clamp(u._pad1, 0.0, 1.0), shadowAlpha));
}
