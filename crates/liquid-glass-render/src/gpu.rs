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
    output: wgpu::Texture,
    output_view: wgpu::TextureView,
}

impl FrameTargets {
    fn new(device: &wgpu::Device, size: GpuSize, format: wgpu::TextureFormat) -> Self {
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let scene = device.create_texture(&descriptor("liquid-glass scene texture"));
        let output = device.create_texture(&descriptor("liquid-glass output texture"));
        let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());

        Self { _scene: scene, scene_view, output, output_view }
    }
}

/// A real `wgpu` offscreen compositor for one SDF Glass panel.
///
/// This backend intentionally has no window or Iced dependency. It renders a
/// gradient scene into an offscreen texture, then samples that texture in a
/// second pass while evaluating a rounded SDF and glass material.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: GpuSize,
    output_format: wgpu::TextureFormat,
    targets: FrameTargets,
    sampler: wgpu::Sampler,
    glass_bind_group_layout: wgpu::BindGroupLayout,
    glass_bind_group: wgpu::BindGroup,
    glass_uniform: wgpu::Buffer,
    background_pipeline: wgpu::RenderPipeline,
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                std::mem::size_of::<GlassUniform>() as u64,
                            ),
                        },
                        count: None,
                    },
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
            &sampler,
            &glass_uniform,
        );

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("liquid-glass panel shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../../../shaders/glass/panel.wgsl"
            ))),
        });
        let background_pipeline = create_pipeline(
            &device,
            "liquid-glass background pipeline",
            &shader,
            None,
            "background_fragment",
            output_format,
        );
        let glass_pipeline = create_pipeline(
            &device,
            "liquid-glass SDF pipeline",
            &shader,
            Some(&glass_bind_group_layout),
            "glass_fragment",
            output_format,
        );

        Self {
            device,
            queue,
            size,
            output_format,
            targets,
            sampler,
            glass_bind_group_layout,
            glass_bind_group,
            glass_uniform,
            background_pipeline,
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
            &self.sampler,
            &self.glass_uniform,
        );
        Ok(())
    }

    /// Encodes and submits one background + glass frame.
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
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("liquid-glass scene pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.scene_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.04,
                            g: 0.06,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.background_pipeline);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("liquid-glass pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.glass_pipeline);
            pass.set_bind_group(0, &self.glass_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
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

fn create_glass_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    scene_view: &wgpu::TextureView,
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
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 2, resource: uniform.as_entire_binding() },
        ],
    })
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
        let renderer = GpuRenderer::from_device(device, queue, GpuSize::new(128, 128));
        let node = GlassNode::new(GlassId(1), Rect::new(16.0, 16.0, 96.0, 64.0))
            .material(GlassMaterial::regular());

        renderer.render_panel(&node, 0.0);
        assert_eq!(renderer.size(), GpuSize::new(128, 128));
    }
}
