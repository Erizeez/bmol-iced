struct GlassUniform {
    viewport_and_origin: vec4<f32>,
    size_radius_blur: vec4<f32>,
    tint_opacity_refraction_time: vec4<f32>,
};

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> glass: GlassUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn fullscreen_vertex(@builtin(vertex_index) index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    let position = positions[index];
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.uv = position * 0.5 + vec2<f32>(0.5);
    output.uv.y = 1.0 - output.uv.y;
    return output;
}

fn saturate(value: f32) -> f32 {
    return clamp(value, 0.0, 1.0);
}

fn rounded_box(point: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(point) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn background_fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = input.uv;
    let top = vec3<f32>(0.10, 0.18, 0.42);
    let bottom = vec3<f32>(0.02, 0.05, 0.12);
    let glow = vec3<f32>(0.26, 0.08, 0.34) * (1.0 - distance(uv, vec2<f32>(0.72, 0.24)));
    return vec4<f32>(mix(top, bottom, uv.y) + glow, 1.0);
}

@fragment
fn glass_fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let viewport = glass.viewport_and_origin.xy;
    let origin = glass.viewport_and_origin.zw;
    let size = glass.size_radius_blur.xy;
    let radius = glass.size_radius_blur.z;
    let blur_radius = glass.size_radius_blur.w;
    let pixel = input.uv * viewport;
    let local = pixel - origin;
    let distance_to_edge = rounded_box(local - size * 0.5, size * 0.5, radius);
    let mask = 1.0 - smoothstep(0.0, 1.5, distance_to_edge);

    let refraction = normalize(local - size * 0.5) * 0.0025 * glass.tint_opacity_refraction_time.w;
    let blur_step = blur_radius / max(viewport.x, viewport.y) * 0.25;
    let sample_uv = input.uv + refraction;
    let background = textureSample(scene_texture, scene_sampler, sample_uv);
    let blur_a = textureSample(scene_texture, scene_sampler, sample_uv + vec2<f32>( blur_step, 0.0));
    let blur_b = textureSample(scene_texture, scene_sampler, sample_uv + vec2<f32>(-blur_step, 0.0));
    let blurred = (background + blur_a + blur_b) / 3.0;
    let edge = pow(saturate(1.0 - abs(distance_to_edge) / max(radius, 1.0)), 2.0);
    let tint = glass.tint_opacity_refraction_time.rgb;
    let fresnel = vec3<f32>(0.55, 0.72, 1.0) * edge * 0.20;
    let glass_color = blurred.rgb * 0.76 + tint * 0.24 + fresnel;
    return vec4<f32>(mix(background.rgb, glass_color, mask), 1.0);
}

