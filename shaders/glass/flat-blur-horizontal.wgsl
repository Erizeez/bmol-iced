// Horizontal Gaussian blur constrained to one rectangular medium.

const MAX_BLUR_RADIUS: i32 = 200;

struct Uniforms {
  u_resolution: vec2f,
  u_blurRadius: i32,
  u_weightOffset: i32,
  u_region: vec4f,
  u_gradient: vec4f,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var u_source: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;
@group(0) @binding(3) var<storage, read> u_blurWeights: array<f32>;

fn boundedUv(uv: vec2f) -> vec2f {
  let minUv = u.u_region.xy / u.u_resolution;
  let maxUv = (u.u_region.xy + u.u_region.zw) / u.u_resolution;
  return clamp(uv, minUv, maxUv);
}

fn localBlurRadius(pixelY: f32) -> i32 {
  let gradientHeight = u.u_gradient.y - u.u_gradient.x;
  if (gradientHeight <= 0.0) {
    return u.u_blurRadius;
  }
  let amount = clamp((u.u_gradient.y - pixelY) / gradientHeight, 0.0, 1.0);
  return i32(round(mix(u.u_gradient.z, u.u_gradient.w, amount)));
}

@fragment
fn fs_main(@location(0) v_uv: vec2f) -> @location(0) vec4f {
  let texelSize = 1.0 / u.u_resolution;
  let blurRadius = localBlurRadius(v_uv.y * u.u_resolution.y);
  var premultiplied = vec3f(0.0);
  var alpha = 0.0;
  let center = textureSampleLevel(u_source, u_sampler, boundedUv(v_uv), 0.0);
  let centerWeight = u_blurWeights[u.u_weightOffset];
  premultiplied += center.rgb * center.a * centerWeight;
  alpha += center.a * centerWeight;
  for (var i: i32 = 1; i <= blurRadius; i = i + 1) {
    if (i > MAX_BLUR_RADIUS) { break; }
    let weight = u_blurWeights[u.u_weightOffset + i];
    let offset = f32(i) * texelSize.x;
    let positive = textureSampleLevel(u_source, u_sampler, boundedUv(v_uv + vec2f(offset, 0.0)), 0.0);
    let negative = textureSampleLevel(u_source, u_sampler, boundedUv(v_uv - vec2f(offset, 0.0)), 0.0);
    premultiplied += positive.rgb * positive.a * weight;
    premultiplied += negative.rgb * negative.a * weight;
    alpha += (positive.a + negative.a) * weight;
  }
  return vec4f(premultiplied / max(alpha, 0.000001), clamp(alpha, 0.0, 1.0));
}
