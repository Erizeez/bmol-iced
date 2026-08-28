// Signed distance functions ported from liquid-glass-studio.

fn sdCircle(p: vec2f, r: f32) -> f32 {
  return length(p) - r;
}

fn sdBox(p: vec2f, halfSize: vec2f) -> f32 {
  let q = abs(p) - halfSize;
  return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0);
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

fn smoothCapsuleSDF(
  p_in: vec2f,
  center: vec2f,
  width: f32,
  height: f32,
  blendPixels: f32,
) -> f32 {
  let p = p_in - center;
  let halfSize = vec2f(width, height) * u.u_dpr * 0.5;
  let radius = min(halfSize.x, halfSize.y);
  let blend = max(blendPixels * u.u_dpr / u.u_resolution.y, 0.000001);

  if (halfSize.x >= halfSize.y) {
    let offset = max(halfSize.x - radius, 0.0);
    if (offset <= 0.000001) {
      return sdCircle(p, radius);
    }
    let body = sdBox(p, vec2f(offset, radius));
    let caps = min(
      sdCircle(p + vec2f(offset, 0.0), radius),
      sdCircle(p - vec2f(offset, 0.0), radius),
    );
    return smin(body, caps, blend);
  }

  let offset = max(halfSize.y - radius, 0.0);
  if (offset <= 0.000001) {
    return sdCircle(p, radius);
  }
  let body = sdBox(p, vec2f(radius, offset));
  let caps = min(
    sdCircle(p + vec2f(0.0, offset), radius),
    sdCircle(p - vec2f(0.0, offset), radius),
  );
  return smin(body, caps, blend);
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
    d2 = smoothCapsuleSDF(
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
