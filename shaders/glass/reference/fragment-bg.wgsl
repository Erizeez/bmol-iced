// Background pass ported from liquid-glass-studio/src/shaders-wgsl/fragment-bg.wgsl.

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
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var u_bgTexture: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;

fn chessboard(uv: vec2f, size: f32, mode: i32) -> f32 {
  let yBars = step(size * 2.0, (uv.y * 2.0) % (size * 4.0));
  let xBars = step(size * 2.0, (uv.x * 2.0) % (size * 4.0));
  if (mode == 0) {
    return yBars;
  } else if (mode == 1) {
    return xBars;
  } else {
    return abs(yBars - xBars);
  }
}

fn halfColor(uv: vec2f) -> f32 {
  if (uv.y > 0.5) {
    return 1.0;
  }
  return 0.0;
}

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

fn continuousCapsuleProfile(
  longitudinal: f32,
  halfLong: f32,
  radius: f32,
  bulge: f32,
) -> vec2f {
  let trimSin = 0.309016994;
  let trimCos = 0.951056516;
  let trimTan = 0.324919696;
  let capCenter = max(halfLong - radius, 0.0);
  let trimX = capCenter + radius * trimSin;
  let shoulderLength = min(radius * 0.75, capCenter);
  let innerX = max(capCenter - shoulderLength, 0.0);

  if (longitudinal <= innerX) {
    if (innerX <= 0.000001) {
      return vec2f(radius + bulge, 0.0);
    }
    let q = longitudinal / innerX;
    let arch = max(1.0 - q * q, 0.0);
    let crown = bulge * 0.25;
    let y = radius + bulge + crown * arch * arch * arch;
    let slope = -6.0 * crown * q * arch * arch / innerX;
    return vec2f(y, slope);
  }

  let span = max(trimX - innerX, 0.000001);
  let t = clamp((longitudinal - innerX) / span, 0.0, 1.0);
  let y0 = radius + bulge;
  let y1 = radius * trimCos;
  let first1 = -trimTan * span;
  let second1 = -span * span / (radius * trimCos * trimCos * trimCos);
  let delta = y1 - y0;
  let a3 = 10.0 * delta - 4.0 * first1 + 0.5 * second1;
  let a4 = -15.0 * delta + 7.0 * first1 - second1;
  let a5 = 6.0 * delta - 3.0 * first1 + 0.5 * second1;
  let t2 = t * t;
  let t3 = t2 * t;
  let t4 = t3 * t;
  let t5 = t4 * t;
  let y = y0 + a3 * t3 + a4 * t4 + a5 * t5;
  let slope = (3.0 * a3 * t2 + 4.0 * a4 * t3 + 5.0 * a5 * t4) / span;
  return vec2f(y, slope);
}

fn continuousCapsuleSDF(
  p_in: vec2f,
  center: vec2f,
  width: f32,
  height: f32,
  bulgePixels: f32,
) -> f32 {
  let p = abs(p_in - center);
  let halfSize = vec2f(width, height) * u.u_dpr * 0.5;
  let radius = min(halfSize.x, halfSize.y);
  let bulge = bulgePixels * u.u_dpr / u.u_resolution.y;
  let horizontal = halfSize.x >= halfSize.y;
  let halfLong = select(halfSize.y, halfSize.x, horizontal);
  let longitudinal = select(p.y, p.x, horizontal);
  let transverse = select(p.x, p.y, horizontal);
  let capCenter = max(halfLong - radius, 0.0);

  if (capCenter <= 0.000001) {
    return sdCircle(p, radius);
  }

  let trimX = capCenter + radius * 0.309016994;
  if (longitudinal >= trimX) {
    return length(vec2f(longitudinal - capCenter, transverse)) - radius;
  }

  let profile = continuousCapsuleProfile(longitudinal, halfLong, radius, bulge);
  return (transverse - profile.x) / sqrt(1.0 + profile.y * profile.y);
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
      -u.u_shapeRoundness,
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

fn getCoverUV(uv_in: vec2f, canvasAspect: f32, textureAspect: f32) -> vec2f {
  var uv = uv_in;
  if (canvasAspect > textureAspect) {
    let scale = textureAspect / canvasAspect;
    uv.y = uv.y * scale + 0.5 - 0.5 * scale;
  } else {
    let scale = canvasAspect / textureAspect;
    uv.x = uv.x * scale + 0.5 - 0.5 * scale;
  }
  return uv;
}

@fragment
fn fs_main(@builtin(position) frag_coord: vec4f, @location(0) v_uv: vec2f) -> @location(0) vec4f {
  let u_resolution1x = u.u_resolution / u.u_dpr;
  var bgColor = vec3f(1.0);
  let pixel = vec2f(frag_coord.x, u.u_resolution.y - frag_coord.y);
  let gl_uv = vec2f(v_uv.x, 1.0 - v_uv.y);

  if (u.u_bgType <= 0) {
    bgColor = vec3f(1.0 - chessboard(pixel / u.u_dpr, 20.0, 2) / 4.0);
  } else if (u.u_bgType <= 1) {
    if (gl_uv.x < 0.5 && gl_uv.y > 0.5) {
      bgColor = vec3f(chessboard(pixel / u.u_dpr, 10.0, 0));
    } else if (gl_uv.x > 0.5 && gl_uv.y < 0.5) {
      bgColor = vec3f(chessboard(pixel / u.u_dpr, 10.0, 1));
    } else if (gl_uv.x < 0.5 && gl_uv.y < 0.5) {
      bgColor = vec3f(0.0);
    }
  } else if (u.u_bgType <= 2) {
    bgColor = vec3f(halfColor(pixel / u.u_resolution) * 0.6 + 0.3);
  } else if (u.u_bgType <= 11) {
    if (u.u_bgTextureReady != 1) {
      bgColor = vec3f(1.0 - chessboard(pixel / u.u_dpr, 20.0, 2) / 4.0);
    } else {
      let uv = getCoverUV(v_uv, u.u_resolution.x / u.u_resolution.y, u.u_bgTextureRatio);
      bgColor = textureSampleLevel(u_bgTexture, u_sampler, uv, 0.0).rgb;
    }
  }

  let p1 = (vec2f(0.0) - u.u_resolution * 0.5 + vec2f(u.u_shadowPosition.x * u.u_dpr, u.u_shadowPosition.y * u.u_dpr)) / u.u_resolution.y;
  let p2 = (vec2f(0.0) - u.u_mouseSpring + vec2f(u.u_shadowPosition.x * u.u_dpr, u.u_shadowPosition.y * u.u_dpr)) / u.u_resolution.y;
  let merged = mainSDF(p1, p2, pixel);
  let shadow = exp(-1.0 / u.u_shadowExpand * abs(merged) * u_resolution1x.y) * 0.6 * u.u_shadowFactor;

  return vec4f(bgColor - vec3f(shadow), 1.0);
}
