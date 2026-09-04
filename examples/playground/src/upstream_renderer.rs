//! Exact rendering path for the checked-in liquid-glass-studio WebGPU shaders.
//!
//! The shader text is included directly from the source repository. This
//! module only expands its `#include` directives and provides the same four
//! render passes and uniform layout as the browser implementation.

use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

const UPSTREAM_VERTEX: &str =
    include_str!("../../../liquid-glass-studio/src/shaders-wgsl/vertex.wgsl");
const UPSTREAM_BACKGROUND: &str =
    include_str!("../../../liquid-glass-studio/src/shaders-wgsl/fragment-bg.wgsl");
const UPSTREAM_BLUR_HORIZONTAL: &str =
    include_str!("../../../liquid-glass-studio/src/shaders-wgsl/fragment-bg-hblur.wgsl");
const UPSTREAM_BLUR_VERTICAL: &str =
    include_str!("../../../liquid-glass-studio/src/shaders-wgsl/fragment-bg-vblur.wgsl");
const UPSTREAM_MAIN: &str =
    include_str!("../../../liquid-glass-studio/src/shaders-wgsl/fragment-main.wgsl");
const UPSTREAM_SDF: &str =
    include_str!("../../../liquid-glass-studio/src/shaders-wgsl/lib/sdf.wgsl");
const UPSTREAM_MATH: &str =
    include_str!("../../../liquid-glass-studio/src/shaders-wgsl/lib/math.wgsl");
const UPSTREAM_COLOR: &str =
    include_str!("../../../liquid-glass-studio/src/shaders-wgsl/lib/color.wgsl");

const FULLSCREEN_VERTICES: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];
const UPSTREAM_BLUR_RADIUS: i32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct UpstreamUniform {
    resolution_dpr_pad: [f32; 4],
    mouse_and_spring: [f32; 4],
    shape: [f32; 4],
    merge_glare_shadow: [f32; 4],
    shadow_position_bg_ratio: [f32; 3],
    bg_type: i32,
    flags: [i32; 4],
    tint: [f32; 4],
    refraction_and_fresnel: [f32; 6],
    glare: [f32; 5],
    pad: f32,
}

impl UpstreamUniform {
    fn source_defaults(size: liquid_glass::GpuSize, pointer: [f32; 2]) -> Self {
        Self {
            resolution_dpr_pad: [size.width as f32, size.height as f32, 1.0, 0.0],
            mouse_and_spring: [pointer[0], pointer[1], pointer[0], pointer[1]],
            // liquid-glass-studio Controls.tsx defaults: 200 x 200, 80%
            // corner radius and superellipse exponent 5.
            shape: [200.0, 200.0, 80.0, 5.0],
            // merge=0.05, glare angle=-45 degrees, shadow expand=25,
            // shadow factor=15%.
            merge_glare_shadow: [0.05, -std::f32::consts::FRAC_PI_4, 25.0, 0.15],
            // App.tsx negates the control vector (0, -10) before upload.
            shadow_position_bg_ratio: [0.0, 10.0, 1.0],
            // Procedural checkerboard, matching the source's default bgType=0.
            bg_type: 0,
            // textureReady=false, showShape1=true, blurRadius=1,
            // blurEdge=true.
            flags: [0, 1, UPSTREAM_BLUR_RADIUS, 1],
            tint: [1.0, 1.0, 1.0, 0.0],
            // thickness=20, IOR=1.4, dispersion=7, Fresnel range=30,
            // hardness=20%, strength=20%.
            refraction_and_fresnel: [20.0, 1.4, 7.0, 30.0, 0.20, 0.20],
            // glare range=30, hardness=20%, convergence=50%, opposite=80%,
            // strength=90%.
            glare: [30.0, 0.20, 0.50, 0.80, 0.90],
            pad: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct UpstreamBlurUniform {
    resolution: [f32; 2],
    radius: i32,
    pad: i32,
}

struct UpstreamTargets {
    _background: wgpu::Texture,
    background_view: wgpu::TextureView,
    _blur_horizontal: wgpu::Texture,
    blur_horizontal_view: wgpu::TextureView,
    _blur_vertical: wgpu::Texture,
    blur_vertical_view: wgpu::TextureView,
    output: wgpu::Texture,
    output_view: wgpu::TextureView,
}

impl UpstreamTargets {
    fn new(
        device: &wgpu::Device,
        size: liquid_glass::GpuSize,
        format: wgpu::TextureFormat,
    ) -> Self {
        let descriptor = |label| wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let background = device.create_texture(&descriptor("upstream background"));
        let blur_horizontal = device.create_texture(&descriptor("upstream horizontal blur"));
        let blur_vertical = device.create_texture(&descriptor("upstream vertical blur"));
        let output = device.create_texture(&descriptor("upstream glass output"));
        let background_view = background.create_view(&wgpu::TextureViewDescriptor::default());
        let blur_horizontal_view =
            blur_horizontal.create_view(&wgpu::TextureViewDescriptor::default());
        let blur_vertical_view = blur_vertical.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _background: background,
            background_view,
            _blur_horizontal: blur_horizontal,
            blur_horizontal_view,
            _blur_vertical: blur_vertical,
            blur_vertical_view,
            output,
            output_view,
        }
    }
}

/// Minimal four-pass compositor matching liquid-glass-studio commit d13c3e5.
pub struct UpstreamRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: liquid_glass::GpuSize,
    format: wgpu::TextureFormat,
    targets: UpstreamTargets,
    sampler: wgpu::Sampler,
    vertex_buffer: wgpu::Buffer,
    main_uniform: wgpu::Buffer,
    blur_uniform: wgpu::Buffer,
    blur_weights: wgpu::Buffer,
    placeholder: wgpu::Texture,
    background_pipeline: wgpu::RenderPipeline,
    blur_horizontal_pipeline: wgpu::RenderPipeline,
    blur_vertical_pipeline: wgpu::RenderPipeline,
    main_pipeline: wgpu::RenderPipeline,
    background_bind_group: wgpu::BindGroup,
    blur_horizontal_bind_group: wgpu::BindGroup,
    blur_vertical_bind_group: wgpu::BindGroup,
    main_bind_group: wgpu::BindGroup,
}

impl UpstreamRenderer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        size: liquid_glass::GpuSize,
        format: wgpu::TextureFormat,
    ) -> Self {
        let targets = UpstreamTargets::new(&device, size, format);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("upstream linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..wgpu::SamplerDescriptor::default()
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("upstream fullscreen vertices"),
            contents: bytemuck::cast_slice(&FULLSCREEN_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let main_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("upstream main uniform"),
            size: std::mem::size_of::<UpstreamUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blur_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("upstream blur uniform"),
            size: std::mem::size_of::<UpstreamBlurUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blur_weights = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("upstream Gaussian weights"),
            contents: bytemuck::cast_slice(&upstream_gaussian_weights(UPSTREAM_BLUR_RADIUS)),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let placeholder = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("upstream placeholder texture"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let background_pipeline =
            upstream_pipeline(&device, "upstream background pipeline", UPSTREAM_BACKGROUND, format);
        let blur_horizontal_pipeline = upstream_pipeline(
            &device,
            "upstream horizontal blur pipeline",
            UPSTREAM_BLUR_HORIZONTAL,
            format,
        );
        let blur_vertical_pipeline = upstream_pipeline(
            &device,
            "upstream vertical blur pipeline",
            UPSTREAM_BLUR_VERTICAL,
            format,
        );
        let main_pipeline =
            upstream_pipeline(&device, "upstream main pipeline", UPSTREAM_MAIN, format);
        let placeholder_view = placeholder.create_view(&wgpu::TextureViewDescriptor::default());
        let background_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upstream background bind group"),
            layout: &background_pipeline.get_bind_group_layout(0),
            entries: &[
                buffer_entry(0, &main_uniform),
                texture_entry(1, &placeholder_view),
                sampler_entry(2, &sampler),
            ],
        });
        let blur_horizontal_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upstream horizontal blur bind group"),
            layout: &blur_horizontal_pipeline.get_bind_group_layout(0),
            entries: &[
                buffer_entry(0, &blur_uniform),
                texture_entry(1, &targets.background_view),
                sampler_entry(2, &sampler),
                buffer_entry(3, &blur_weights),
            ],
        });
        let blur_vertical_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upstream vertical blur bind group"),
            layout: &blur_vertical_pipeline.get_bind_group_layout(0),
            entries: &[
                buffer_entry(0, &blur_uniform),
                texture_entry(1, &targets.blur_horizontal_view),
                sampler_entry(2, &sampler),
                buffer_entry(3, &blur_weights),
            ],
        });
        let main_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upstream main bind group"),
            layout: &main_pipeline.get_bind_group_layout(0),
            entries: &[
                buffer_entry(0, &main_uniform),
                texture_entry(1, &targets.blur_vertical_view),
                texture_entry(2, &targets.background_view),
                sampler_entry(3, &sampler),
            ],
        });

        Self {
            device,
            queue,
            size,
            format,
            targets,
            sampler,
            vertex_buffer,
            main_uniform,
            blur_uniform,
            blur_weights,
            placeholder,
            background_pipeline,
            blur_horizontal_pipeline,
            blur_vertical_pipeline,
            main_pipeline,
            background_bind_group,
            blur_horizontal_bind_group,
            blur_vertical_bind_group,
            main_bind_group,
        }
    }

    pub fn resize(&mut self, size: liquid_glass::GpuSize) {
        self.size = size;
        self.targets = UpstreamTargets::new(&self.device, size, self.format);
        self.rebuild_bind_groups();
    }

    #[allow(dead_code)]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn output_texture(&self) -> &wgpu::Texture {
        &self.targets.output
    }

    pub fn render(&self, pointer: [f32; 2]) {
        self.render_to_view(pointer, &self.targets.output_view);
    }

    /// Runs the original four passes with the final pass targeting `output`.
    /// This lets the reference demo present upstream pixels without an extra
    /// sampling or composition pass.
    pub fn render_to_view(&self, pointer: [f32; 2], output: &wgpu::TextureView) {
        let main_uniform = UpstreamUniform::source_defaults(self.size, pointer);
        let blur_uniform = UpstreamBlurUniform {
            resolution: [self.size.width as f32, self.size.height as f32],
            radius: UPSTREAM_BLUR_RADIUS,
            pad: 0,
        };
        self.queue.write_buffer(&self.main_uniform, 0, bytemuck::bytes_of(&main_uniform));
        self.queue.write_buffer(&self.blur_uniform, 0, bytemuck::bytes_of(&blur_uniform));

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("upstream liquid glass encoder"),
        });
        encode_pass(
            &mut encoder,
            "upstream background pass",
            &self.targets.background_view,
            &self.background_pipeline,
            &self.background_bind_group,
            &self.vertex_buffer,
        );
        encode_pass(
            &mut encoder,
            "upstream horizontal blur pass",
            &self.targets.blur_horizontal_view,
            &self.blur_horizontal_pipeline,
            &self.blur_horizontal_bind_group,
            &self.vertex_buffer,
        );
        encode_pass(
            &mut encoder,
            "upstream vertical blur pass",
            &self.targets.blur_vertical_view,
            &self.blur_vertical_pipeline,
            &self.blur_vertical_bind_group,
            &self.vertex_buffer,
        );
        encode_pass(
            &mut encoder,
            "upstream main pass",
            output,
            &self.main_pipeline,
            &self.main_bind_group,
            &self.vertex_buffer,
        );
        self.queue.submit([encoder.finish()]);
    }

    fn rebuild_bind_groups(&mut self) {
        let placeholder_view =
            self.placeholder.create_view(&wgpu::TextureViewDescriptor::default());
        self.background_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upstream background bind group"),
            layout: &self.background_pipeline.get_bind_group_layout(0),
            entries: &[
                buffer_entry(0, &self.main_uniform),
                texture_entry(1, &placeholder_view),
                sampler_entry(2, &self.sampler),
            ],
        });
        self.blur_horizontal_bind_group =
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("upstream horizontal blur bind group"),
                layout: &self.blur_horizontal_pipeline.get_bind_group_layout(0),
                entries: &[
                    buffer_entry(0, &self.blur_uniform),
                    texture_entry(1, &self.targets.background_view),
                    sampler_entry(2, &self.sampler),
                    buffer_entry(3, &self.blur_weights),
                ],
            });
        self.blur_vertical_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upstream vertical blur bind group"),
            layout: &self.blur_vertical_pipeline.get_bind_group_layout(0),
            entries: &[
                buffer_entry(0, &self.blur_uniform),
                texture_entry(1, &self.targets.blur_horizontal_view),
                sampler_entry(2, &self.sampler),
                buffer_entry(3, &self.blur_weights),
            ],
        });
        self.main_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("upstream main bind group"),
            layout: &self.main_pipeline.get_bind_group_layout(0),
            entries: &[
                buffer_entry(0, &self.main_uniform),
                texture_entry(1, &self.targets.blur_vertical_view),
                texture_entry(2, &self.targets.background_view),
                sampler_entry(3, &self.sampler),
            ],
        });
    }
}

/// Presents an upstream and enhanced texture in equal-width panes.
pub struct ComparisonPresenter {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    vertex_buffer: wgpu::Buffer,
}

impl ComparisonPresenter {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        upstream: &wgpu::Texture,
        enhanced: &wgpu::Texture,
    ) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("comparison sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..wgpu::SamplerDescriptor::default()
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("comparison fullscreen vertices"),
            contents: bytemuck::cast_slice(&FULLSCREEN_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("liquid glass comparison shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(COMPARISON_SHADER)),
        });
        let pipeline = pipeline_from_module(&device, "comparison pipeline", &shader, format);
        let bind_group = comparison_bind_group(&device, &pipeline, &sampler, upstream, enhanced);
        Self { device, queue, pipeline, bind_group, sampler, vertex_buffer }
    }

    pub fn update_sources(&mut self, upstream: &wgpu::Texture, enhanced: &wgpu::Texture) {
        self.bind_group =
            comparison_bind_group(&self.device, &self.pipeline, &self.sampler, upstream, enhanced);
    }

    pub fn present(&self, view: &wgpu::TextureView) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("comparison present encoder"),
        });
        encode_pass(
            &mut encoder,
            "comparison present pass",
            view,
            &self.pipeline,
            &self.bind_group,
            &self.vertex_buffer,
        );
        self.queue.submit([encoder.finish()]);
    }
}

fn upstream_shader(fragment: &str) -> String {
    let expanded = fragment
        .replace("#include './lib/sdf.wgsl'", UPSTREAM_SDF)
        .replace("#include './lib/math.wgsl'", UPSTREAM_MATH)
        .replace("#include './lib/color.wgsl'", UPSTREAM_COLOR);
    format!("{UPSTREAM_VERTEX}\n{expanded}")
}

fn upstream_pipeline(
    device: &wgpu::Device,
    label: &str,
    fragment: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(upstream_shader(fragment))),
    });
    pipeline_from_module(device, label, &shader, format)
}

fn pipeline_from_module(
    device: &wgpu::Device,
    label: &str,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: None,
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..wgpu::PrimitiveState::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview: None,
        cache: None,
    })
}

fn encode_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    vertex_buffer: &wgpu::Buffer,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..4, 0..1);
}

fn upstream_gaussian_weights(radius: i32) -> [f32; 4] {
    let sigma = radius as f32 / 3.0;
    let center = 1.0;
    let side = (-0.5 / (sigma * sigma)).exp();
    let sum = center + side * 2.0;
    [center / sum, side / sum, 0.0, 0.0]
}

fn buffer_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource: buffer.as_entire_binding() }
}

fn texture_entry(binding: u32, view: &wgpu::TextureView) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) }
}

fn sampler_entry(binding: u32, sampler: &wgpu::Sampler) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::Sampler(sampler) }
}

fn comparison_bind_group(
    device: &wgpu::Device,
    pipeline: &wgpu::RenderPipeline,
    sampler: &wgpu::Sampler,
    upstream: &wgpu::Texture,
    enhanced: &wgpu::Texture,
) -> wgpu::BindGroup {
    let upstream_view = upstream.create_view(&wgpu::TextureViewDescriptor::default());
    let enhanced_view = enhanced.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("comparison bind group"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            texture_entry(0, &upstream_view),
            texture_entry(1, &enhanced_view),
            sampler_entry(2, sampler),
        ],
    })
}

const COMPARISON_SHADER: &str = r#"
struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
};

@vertex
fn vs_main(@location(0) position: vec2f) -> VertexOutput {
  var output: VertexOutput;
  output.position = vec4f(position, 0.0, 1.0);
  output.uv = vec2f(position.x * 0.5 + 0.5, 0.5 - position.y * 0.5);
  return output;
}

@group(0) @binding(0) var upstream_texture: texture_2d<f32>;
@group(0) @binding(1) var enhanced_texture: texture_2d<f32>;
@group(0) @binding(2) var linear_sampler: sampler;

@fragment
fn fs_main(@location(0) uv: vec2f) -> @location(0) vec4f {
  if (uv.x < 0.5) {
    return textureSampleLevel(upstream_texture, linear_sampler, vec2f(uv.x * 2.0, uv.y), 0.0);
  }
  return textureSampleLevel(enhanced_texture, linear_sampler, vec2f((uv.x - 0.5) * 2.0, uv.y), 0.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_uniform_layout_matches_upstream_gpu_utils() {
        assert_eq!(std::mem::size_of::<UpstreamUniform>(), 160);
        assert_eq!(std::mem::size_of::<UpstreamBlurUniform>(), 16);
    }

    #[test]
    fn source_shader_is_loaded_directly_and_includes_expand() {
        assert!(UPSTREAM_MAIN.contains("let thetaT = safeAsin"));
        let expanded = upstream_shader(UPSTREAM_MAIN);
        assert!(!expanded.lines().any(|line| line.trim_start().starts_with("#include ")));
        assert!(expanded.contains("fn mainSDF"));
        assert!(expanded.contains("fn SRGB_TO_LCH"));
    }
}
