use std::{borrow::Cow, fmt};

use bytemuck::{Pod, Zeroable};
use liquid_glass_scene::{GlassNode, GlassShape};

const DEFAULT_OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Pixel dimensions for an offscreen render target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GpuSize {
    pub width: u32,
    pub height: u32,
}

impl GpuSize {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Errors returned while creating the headless GPU backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuError {
    InvalidSize,
    AdapterUnavailable(String),
    DeviceUnavailable(String),
}

impl fmt::Display for GpuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize => write!(formatter, "offscreen target dimensions must be non-zero"),
            Self::AdapterUnavailable(error) => {
                write!(formatter, "no suitable GPU adapter: {error}")
            }
            Self::DeviceUnavailable(error) => {
                write!(formatter, "failed to create GPU device: {error}")
            }
        }
    }
}

impl std::error::Error for GpuError {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GlassUniform {
    viewport_and_origin: [f32; 4],
    size_radius_blur: [f32; 4],
    tint_opacity_refraction_time: [f32; 4],
}

struct FrameTargets {
    _scene: wgpu::Texture,
    scene_view: wgpu::TextureView,
    _downsampled: wgpu::Texture,
    downsampled_view: wgpu::TextureView,
    _blur_horizontal: wgpu::Texture,
    blur_horizontal_view: wgpu::TextureView,
    _blur_vertical: wgpu::Texture,
    blur_vertical_view: wgpu::TextureView,
    output: wgpu::Texture,
    output_view: wgpu::TextureView,
}

impl FrameTargets {
    fn new(device: &wgpu::Device, size: GpuSize, format: wgpu::TextureFormat) -> Self {
        let descriptor = |label, target_size: GpuSize| wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: target_size.width,
                height: target_size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let blur_size = GpuSize::new(size.width.div_ceil(2), size.height.div_ceil(2));
        let scene = device.create_texture(&descriptor("liquid-glass scene texture", size));
        let downsampled =
            device.create_texture(&descriptor("liquid-glass downsampled texture", blur_size));
        let blur_horizontal =
            device.create_texture(&descriptor("liquid-glass horizontal blur texture", blur_size));
        let blur_vertical =
            device.create_texture(&descriptor("liquid-glass vertical blur texture", blur_size));
        let output = device.create_texture(&descriptor("liquid-glass output texture", size));
        let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());
        let downsampled_view = downsampled.create_view(&wgpu::TextureViewDescriptor::default());
        let blur_horizontal_view =
            blur_horizontal.create_view(&wgpu::TextureViewDescriptor::default());
        let blur_vertical_view = blur_vertical.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            _scene: scene,
            scene_view,
            _downsampled: downsampled,
            downsampled_view,
            _blur_horizontal: blur_horizontal,
            blur_horizontal_view,
            _blur_vertical: blur_vertical,
            blur_vertical_view,
            output,
            output_view,
        }
    }
}

/// A real `wgpu` offscreen compositor for one SDF Glass panel.
///
/// This backend intentionally has no window or Iced dependency. It renders a
/// gradient scene into an offscreen texture, downsamples it, applies
/// horizontal and vertical blur passes, and samples the result while
/// evaluating a rounded SDF and glass material.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: GpuSize,
    output_format: wgpu::TextureFormat,
    targets: FrameTargets,
    sampler: wgpu::Sampler,
    downsample_bind_group: wgpu::BindGroup,
    blur_horizontal_bind_group: wgpu::BindGroup,
    blur_vertical_bind_group: wgpu::BindGroup,
    glass_bind_group_layout: wgpu::BindGroupLayout,
    glass_bind_group: wgpu::BindGroup,
    glass_uniform: wgpu::Buffer,
    background_pipeline: wgpu::RenderPipeline,
    downsample_pipeline: wgpu::RenderPipeline,
    blur_horizontal_pipeline: wgpu::RenderPipeline,
    blur_vertical_pipeline: wgpu::RenderPipeline,
    glass_pipeline: wgpu::RenderPipeline,
}

impl fmt::Debug for GpuRenderer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("GpuRenderer").field("size", &self.size).finish_non_exhaustive()
    }
}

impl GpuRenderer {
    /// Creates a headless renderer using the best available local adapter.
    ///
    /// # Errors
    ///
    /// Returns an error when no compatible adapter or logical device can be
    /// created for the requested target.
    pub async fn new_headless(size: GpuSize) -> Result<Self, GpuError> {
        if !size.is_valid() {
            return Err(GpuError::InvalidSize);
        }

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| GpuError::AdapterUnavailable(error.to_string()))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("liquid-glass device"),
                ..Default::default()
            })
            .await
            .map_err(|error| GpuError::DeviceUnavailable(error.to_string()))?;

        Ok(Self::from_device(device, queue, size))
    }

    /// Builds a renderer from an existing device. Useful for tests and for
    /// applications that own the window/surface lifecycle.
    ///
    /// # Panics
    ///
    /// Panics when either target dimension is zero.
    #[must_use]
    pub fn from_device(device: wgpu::Device, queue: wgpu::Queue, size: GpuSize) -> Self {
        Self::from_device_with_format(device, queue, size, DEFAULT_OUTPUT_FORMAT)
    }

    /// Builds a renderer whose output pipelines target the supplied texture format.
    ///
    /// This is used by window integrations because a platform Surface may
    /// prefer BGRA sRGB instead of RGBA sRGB.
    ///
    /// # Panics
    ///
    /// Panics when either target dimension is zero.
    #[must_use]
    pub fn from_device_with_format(
        device: wgpu::Device,
        queue: wgpu::Queue,
        size: GpuSize,
        output_format: wgpu::TextureFormat,
    ) -> Self {
        assert!(size.is_valid(), "GPU renderer size must be non-zero");
        let targets = FrameTargets::new(&device, size, output_format);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("liquid-glass linear sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let glass_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("liquid-glass backdrop layout"),
                entries: &[
                    texture_binding(0),
                    texture_binding(1),
                    sampler_binding(2),
                    uniform_binding(3, std::mem::size_of::<GlassUniform>()),
                ],
            });
        let glass_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("liquid-glass material uniform"),
            size: std::mem::size_of::<GlassUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let glass_bind_group = create_glass_bind_group(
            &device,
            &glass_bind_group_layout,
            &targets.scene_view,
            &targets.blur_vertical_view,
            &sampler,
            &glass_uniform,
        );
        let downsample_bind_group = create_blur_bind_group(
            &device,
            &glass_bind_group_layout,
            &targets.scene_view,
            &sampler,
            &glass_uniform,
        );
        let blur_horizontal_bind_group = create_blur_bind_group(
            &device,
            &glass_bind_group_layout,
            &targets.downsampled_view,
            &sampler,
            &glass_uniform,
        );
        let blur_vertical_bind_group = create_blur_bind_group(
            &device,
            &glass_bind_group_layout,
            &targets.blur_horizontal_view,
            &sampler,
            &glass_uniform,
        );

        let (
            background_pipeline,
            downsample_pipeline,
            blur_horizontal_pipeline,
            blur_vertical_pipeline,
            glass_pipeline,
        ) = create_pipelines(&device, &glass_bind_group_layout, output_format);

        Self {
            device,
            queue,
            size,
            output_format,
            targets,
            sampler,
            downsample_bind_group,
            blur_horizontal_bind_group,
            blur_vertical_bind_group,
            glass_bind_group_layout,
            glass_bind_group,
            glass_uniform,
            background_pipeline,
            downsample_pipeline,
            blur_horizontal_pipeline,
            blur_vertical_pipeline,
            glass_pipeline,
        }
    }

    /// Recreates offscreen targets after a logical size change.
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::InvalidSize`] when either target dimension is zero.
    pub fn resize(&mut self, size: GpuSize) -> Result<(), GpuError> {
        if !size.is_valid() {
            return Err(GpuError::InvalidSize);
        }
        self.size = size;
        self.targets = FrameTargets::new(&self.device, size, self.output_format);
        self.glass_bind_group = create_glass_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &self.targets.scene_view,
            &self.targets.blur_vertical_view,
            &self.sampler,
            &self.glass_uniform,
        );
        self.downsample_bind_group = create_blur_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &self.targets.scene_view,
            &self.sampler,
            &self.glass_uniform,
        );
        self.blur_horizontal_bind_group = create_blur_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &self.targets.downsampled_view,
            &self.sampler,
            &self.glass_uniform,
        );
        self.blur_vertical_bind_group = create_blur_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &self.targets.blur_horizontal_view,
            &self.sampler,
            &self.glass_uniform,
        );
        Ok(())
    }

    /// Encodes and submits one background, blur, and glass frame.
    #[allow(clippy::cast_precision_loss)]
    pub fn render_panel(&self, node: &GlassNode, time_seconds: f32) {
        self.render_panel_to_view(&self.targets.output_view, node, time_seconds);
    }

    /// Encodes and submits one frame into an externally owned texture view.
    ///
    /// The external view must use the format passed to
    /// [`Self::from_device_with_format`]. This is the integration point for a
    /// window Surface or another compositor-owned target.
    #[allow(clippy::cast_precision_loss)]
    pub fn render_panel_to_view(
        &self,
        output_view: &wgpu::TextureView,
        node: &GlassNode,
        time_seconds: f32,
    ) {
        let uniform = GlassUniform {
            viewport_and_origin: [
                self.size.width as f32,
                self.size.height as f32,
                node.bounds.x,
                node.bounds.y,
            ],
            size_radius_blur: [
                node.bounds.width,
                node.bounds.height,
                shape_radius(node),
                node.material.blur.radius,
            ],
            tint_opacity_refraction_time: [
                node.material.tint.r,
                node.material.tint.g,
                node.material.tint.b,
                node.material.opacity
                    + node.material.refraction.strength * 0.05
                    + time_seconds * 0.0,
            ],
        };
        self.queue.write_buffer(&self.glass_uniform, 0, bytemuck::bytes_of(&uniform));

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("liquid-glass frame encoder"),
        });
        encode_fullscreen_pass(
            &mut encoder,
            "liquid-glass scene pass",
            &self.targets.scene_view,
            &self.background_pipeline,
            None,
            wgpu::Color { r: 0.04, g: 0.06, b: 0.12, a: 1.0 },
        );
        encode_fullscreen_pass(
            &mut encoder,
            "liquid-glass downsample pass",
            &self.targets.downsampled_view,
            &self.downsample_pipeline,
            Some(&self.downsample_bind_group),
            wgpu::Color::BLACK,
        );
        encode_fullscreen_pass(
            &mut encoder,
            "liquid-glass horizontal blur pass",
            &self.targets.blur_horizontal_view,
            &self.blur_horizontal_pipeline,
            Some(&self.blur_horizontal_bind_group),
            wgpu::Color::BLACK,
        );
        encode_fullscreen_pass(
            &mut encoder,
            "liquid-glass vertical blur pass",
            &self.targets.blur_vertical_view,
            &self.blur_vertical_pipeline,
            Some(&self.blur_vertical_bind_group),
            wgpu::Color::BLACK,
        );
        encode_fullscreen_pass(
            &mut encoder,
            "liquid-glass pass",
            output_view,
            &self.glass_pipeline,
            Some(&self.glass_bind_group),
            wgpu::Color::BLACK,
        );
        self.queue.submit([encoder.finish()]);
    }

    /// Renders one frame directly to a configured `wgpu` `SurfaceTexture` and presents it.
    #[allow(clippy::cast_precision_loss)]
    pub fn render_panel_to_surface_texture(
        &self,
        surface_texture: wgpu::SurfaceTexture,
        node: &GlassNode,
        time_seconds: f32,
    ) {
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render_panel_to_view(&view, node, time_seconds);
        self.queue.present(surface_texture);
    }

    /// Returns the final texture for a future surface/present pass.
    #[must_use]
    pub const fn output_texture(&self) -> &wgpu::Texture {
        &self.targets.output
    }

    /// Returns the dimensions of the current offscreen target.
    #[must_use]
    pub const fn size(&self) -> GpuSize {
        self.size
    }

    /// Returns the device used to create the renderer's resources.
    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns the texture format targeted by the render pipelines.
    #[must_use]
    pub const fn output_format(&self) -> wgpu::TextureFormat {
        self.output_format
    }
}

fn encode_fullscreen_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: Option<&wgpu::BindGroup>,
    clear: wgpu::Color,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
        })],
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    if let Some(bind_group) = bind_group {
        pass.set_bind_group(0, bind_group, &[]);
    }
    pass.draw(0..3, 0..1);
}

fn create_glass_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    scene_view: &wgpu::TextureView,
    blur_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("liquid-glass backdrop bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(scene_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(blur_view),
            },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 3, resource: uniform.as_entire_binding() },
        ],
    })
}

fn create_blur_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("liquid-glass blur bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 3, resource: uniform.as_entire_binding() },
        ],
    })
}

fn texture_binding(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_binding(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn uniform_binding(binding: u32, size: usize) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(size as u64),
        },
        count: None,
    }
}

fn create_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader: &wgpu::ShaderModule,
    bind_group_layout: Option<&wgpu::BindGroupLayout>,
    fragment_entry: &str,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[bind_group_layout],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("fullscreen_vertex"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_pipelines(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
) -> (
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("liquid-glass panel shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
            "../../../shaders/glass/panel.wgsl"
        ))),
    });
    let background = create_pipeline(
        device,
        "liquid-glass background pipeline",
        &shader,
        None,
        "background_fragment",
        output_format,
    );
    let downsample = create_pipeline(
        device,
        "liquid-glass downsample pipeline",
        &shader,
        Some(bind_group_layout),
        "downsample_fragment",
        output_format,
    );
    let horizontal = create_pipeline(
        device,
        "liquid-glass horizontal blur pipeline",
        &shader,
        Some(bind_group_layout),
        "blur_horizontal_fragment",
        output_format,
    );
    let vertical = create_pipeline(
        device,
        "liquid-glass vertical blur pipeline",
        &shader,
        Some(bind_group_layout),
        "blur_vertical_fragment",
        output_format,
    );
    let glass = create_pipeline(
        device,
        "liquid-glass SDF pipeline",
        &shader,
        Some(bind_group_layout),
        "glass_fragment",
        output_format,
    );

    (background, downsample, horizontal, vertical, glass)
}

fn shape_radius(node: &GlassNode) -> f32 {
    match node.shape {
        GlassShape::RoundedRect { radius } => radius,
        GlassShape::Superellipse { .. } => node.bounds.width.min(node.bounds.height) * 0.2,
        GlassShape::Capsule => node.bounds.height * 0.5,
        GlassShape::Circle => node.bounds.width.min(node.bounds.height) * 0.5,
        GlassShape::Ellipse => node.bounds.width.min(node.bounds.height) * 0.25,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use liquid_glass_scene::{GlassId, GlassMaterial, Rect};

    #[test]
    fn noop_device_can_build_and_submit_glass_frame() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let mut renderer = GpuRenderer::from_device(device, queue, GpuSize::new(128, 128));
        let node = GlassNode::new(GlassId(1), Rect::new(16.0, 16.0, 96.0, 64.0))
            .material(GlassMaterial::regular());

        renderer.render_panel(&node, 0.0);
        assert_eq!(renderer.size(), GpuSize::new(128, 128));

        renderer.resize(GpuSize::new(65, 33)).expect("resize blur targets");
        renderer.render_panel(&node, 0.0);
        assert_eq!(renderer.size(), GpuSize::new(65, 33));
    }
}
