// Signed distance functions ported from liquid-glass-studio.

fn sdCircle(p: vec2f, r: f32) -> f32 {
  return length(p) - r;
}

fn continuousCornerSDF(p_in: vec2f, r: f32, n: f32) -> f32 {
  let p = abs(p_in);
  let exponent = max(n, 2.0);
  let v = pow(pow(p.x, exponent) + pow(p.y, exponent), 1.0 / exponent);
  return v - r;
}

fn roundedRectSDF(
  p_in: vec2f,
  center: vec2f,
  width: f32,
  height: f32,
  cornerRadius: f32,
  n: f32,
) -> f32 {
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
