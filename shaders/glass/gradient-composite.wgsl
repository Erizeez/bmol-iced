// Mixes the locally blurred image with the original image. The fallback keeps
// transparent or unavailable samples from turning the transition black.

@group(0) @binding(0) var u_blurred: texture_2d<f32>;
@group(0) @binding(1) var u_original: texture_2d<f32>;
@group(0) @binding(2) var u_sampler: sampler;

struct Uniforms {
  fallback: vec4f,
  gradient: vec4f,
};

@group(0) @binding(3) var<uniform> u: Uniforms;

@fragment
fn fs_main(@location(0) v_uv: vec2f) -> @location(0) vec4f {
  let blurred = textureSampleLevel(u_blurred, u_sampler, v_uv, 0.0);
  let original = textureSampleLevel(u_original, u_sampler, v_uv, 0.0);
  // The sidebar medium is intentionally opaque. Invalid transparent samples
  // fall back to the known sidebar medium instead of importing black RGB from
  // the transparent-window compositor.
  let base = select(u.fallback.rgb, original.rgb, original.a > 0.001);
  let sampledBlur = select(base, blurred.rgb, blurred.a > 0.001);
  let gradientHeight = max(u.gradient.y - u.gradient.x, 0.000001);
  let amount = select(
    1.0,
    clamp((u.gradient.y - v_uv.y) / gradientHeight, 0.0, 1.0),
    v_uv.y > u.gradient.x,
  );
  // Replace the sharp source with the blur according to the vertical amount.
  // This prevents a translucent overlay from leaving a sharp copy of the
  // scrolling text visible underneath the blurred result.
  return vec4f(mix(base, sampledBlur, amount), 1.0);
}
