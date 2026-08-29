use std::{borrow::Cow, fmt};

use bytemuck::{Pod, Zeroable};
use liquid_glass_scene::{GlassNode, GlassScene, GlassShape};

const DEFAULT_OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[allow(clippy::cast_possible_truncation)]
const GLASS_UNIFORM_SIZE: u32 = std::mem::size_of::<GlassUniform>() as u32;
const MAX_BLUR_RADIUS: usize = 200;

const FULLSCREEN_VERTEX_ATTRIBUTES: &[wgpu::VertexAttribute] = &[wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x2,
    offset: 0,
    shader_location: 0,
}];

/// Maximum number of glass nodes that can be drawn in one frame.
pub const MAX_GLASS_NODES: usize = 64;

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
    InvalidBackgroundFrame { width: u32, height: u32, stride: u32, byte_len: usize },
    AdapterUnavailable(String),
    DeviceUnavailable(String),
    SceneNodeLimitExceeded { limit: usize },
}

impl fmt::Display for GpuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize => write!(formatter, "offscreen target dimensions must be non-zero"),
            Self::InvalidBackgroundFrame { width, height, stride, byte_len } => write!(
                formatter,
                "invalid RGBA8 backdrop frame ({width}x{height}, stride {stride}, {byte_len} bytes)",
            ),
            Self::AdapterUnavailable(error) => {
                write!(formatter, "no suitable GPU adapter: {error}")
            }
            Self::DeviceUnavailable(error) => {
                write!(formatter, "failed to create GPU device: {error}")
            }
            Self::SceneNodeLimitExceeded { limit } => {
                write!(formatter, "scene contains more than {limit} glass nodes")
            }
        }
    }
}

impl std::error::Error for GpuError {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GlassUniform {
    resolution_dpr_pad: [f32; 4],
    mouse_and_spring: [f32; 4],
    shape: [f32; 4],
    capsule_bezier_x: [f32; 4],
    capsule_bezier_y: [f32; 4],
    merge_glare_shadow: [f32; 4],
    shadow_position_bg_ratio: [f32; 3],
    bg_type: i32,
    flags: [i32; 4],
    tint: [f32; 4],
    refraction_and_fresnel: [f32; 6],
    glare: [f32; 5],
    _pad: [f32; 5],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct BlurUniform {
    resolution: [f32; 2],
    radius: i32,
    weight_offset: i32,
}

struct FrameTargets {
    scene: wgpu::Texture,
    scene_view: wgpu::TextureView,
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
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };
        let scene = device.create_texture(&descriptor("liquid-glass scene texture", size));
        let blur_horizontal =
            device.create_texture(&descriptor("liquid-glass horizontal blur texture", size));
        let blur_vertical =
            device.create_texture(&descriptor("liquid-glass vertical blur texture", size));
        let output = device.create_texture(&descriptor("liquid-glass output texture", size));
        let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());
        let blur_horizontal_view =
            blur_horizontal.create_view(&wgpu::TextureViewDescriptor::default());
        let blur_vertical_view = blur_vertical.create_view(&wgpu::TextureViewDescriptor::default());
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            scene,
            scene_view,
            _blur_horizontal: blur_horizontal,
            blur_horizontal_view,
            _blur_vertical: blur_vertical,
            blur_vertical_view,
            output,
            output_view,
        }
    }
}

/// A real `wgpu` offscreen compositor for a scene of SDF Glass nodes.
///
/// This backend intentionally has no window or Iced dependency. It renders a
/// reference background into an offscreen texture, applies full-resolution
/// horizontal and vertical Gaussian blur passes, and samples the result while
/// evaluating the reference SDF, refraction, dispersion, Fresnel, glare, and
/// tint composition. Multiple nodes are composed in z-order through ping-pong
/// targets, so each later node samples the already-composited lower layers.
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: GpuSize,
    output_format: wgpu::TextureFormat,
    targets: FrameTargets,
    sampler: wgpu::Sampler,
    fullscreen_vertex_buffer: wgpu::Buffer,
    background_bind_group: wgpu::BindGroup,
    blur_horizontal_from_scene_bind_group: wgpu::BindGroup,
    blur_horizontal_from_output_bind_group: wgpu::BindGroup,
    blur_vertical_bind_group: wgpu::BindGroup,
    glass_bind_group_layout: wgpu::BindGroupLayout,
    glass_from_scene_bind_group: wgpu::BindGroup,
    glass_from_output_bind_group: wgpu::BindGroup,
    glass_uniform: wgpu::Buffer,
    blur_uniform: wgpu::Buffer,
    blur_weights: wgpu::Buffer,
    glass_uniform_stride: u32,
    placeholder_texture: wgpu::Texture,
    background_texture: Option<wgpu::Texture>,
    background_texture_ratio: f32,
    background_pipeline: wgpu::RenderPipeline,
    blur_horizontal_pipeline: wgpu::RenderPipeline,
    blur_vertical_pipeline: wgpu::RenderPipeline,
    glass_pipeline: wgpu::RenderPipeline,
    copy_bind_group_layout: wgpu::BindGroupLayout,
    copy_bind_group: wgpu::BindGroup,
    copy_pipeline: wgpu::RenderPipeline,
    transparent_background: bool,
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

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
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
    #[allow(clippy::too_many_lines)]
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
            mipmap_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let fullscreen_vertex_buffer = create_fullscreen_vertex_buffer(&device, &queue);
        let glass_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("liquid-glass backdrop layout"),
                entries: &[
                    texture_binding(0),
                    texture_binding(1),
                    sampler_binding(2),
                    uniform_binding(3, std::mem::size_of::<GlassUniform>()),
                    storage_binding(4),
                ],
            });
        let glass_uniform_stride = uniform_stride(&device);
        let glass_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("liquid-glass material uniform"),
            size: u64::from(glass_uniform_stride) * MAX_GLASS_NODES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blur_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("liquid-glass blur uniform"),
            size: u64::from(glass_uniform_stride) * MAX_GLASS_NODES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blur_weights = create_blur_weights_buffer(&device, MAX_GLASS_NODES);
        let placeholder_texture = create_placeholder_texture(&device);
        let glass_from_scene_bind_group = create_glass_bind_group(
            &device,
            &glass_bind_group_layout,
            &targets.scene_view,
            &targets.blur_vertical_view,
            &sampler,
            &glass_uniform,
            &blur_weights,
        );
        let glass_from_output_bind_group = create_glass_bind_group(
            &device,
            &glass_bind_group_layout,
            &targets.output_view,
            &targets.blur_vertical_view,
            &sampler,
            &glass_uniform,
            &blur_weights,
        );
        let background_bind_group = create_background_bind_group(
            &device,
            &glass_bind_group_layout,
            &placeholder_texture,
            &sampler,
            &glass_uniform,
            &blur_weights,
        );
        let blur_horizontal_from_scene_bind_group = create_blur_bind_group(
            &device,
            &glass_bind_group_layout,
            &targets.scene_view,
            &sampler,
            &blur_uniform,
            &blur_weights,
        );
        let blur_horizontal_from_output_bind_group = create_blur_bind_group(
            &device,
            &glass_bind_group_layout,
            &targets.output_view,
            &sampler,
            &blur_uniform,
            &blur_weights,
        );
        let blur_vertical_bind_group = create_blur_bind_group(
            &device,
            &glass_bind_group_layout,
            &targets.blur_horizontal_view,
            &sampler,
            &blur_uniform,
            &blur_weights,
        );

        let (background_pipeline, blur_horizontal_pipeline, blur_vertical_pipeline, glass_pipeline) =
            create_pipelines(&device, &glass_bind_group_layout, output_format);
        let copy_bind_group_layout = create_copy_bind_group_layout(&device);
        let copy_bind_group = create_copy_bind_group(
            &device,
            &copy_bind_group_layout,
            &targets.output_view,
            &sampler,
        );
        let copy_pipeline = create_copy_pipeline(&device, &copy_bind_group_layout, output_format);

        Self {
            device,
            queue,
            size,
            output_format,
            targets,
            sampler,
            fullscreen_vertex_buffer,
            background_bind_group,
            blur_horizontal_from_scene_bind_group,
            blur_horizontal_from_output_bind_group,
            blur_vertical_bind_group,
            glass_bind_group_layout,
            glass_from_scene_bind_group,
            glass_from_output_bind_group,
            glass_uniform,
            blur_uniform,
            blur_weights,
            glass_uniform_stride,
            placeholder_texture,
            background_texture: None,
            background_texture_ratio: 1.0,
            background_pipeline,
            blur_horizontal_pipeline,
            blur_vertical_pipeline,
            glass_pipeline,
            copy_bind_group_layout,
            copy_bind_group,
            copy_pipeline,
            transparent_background: false,
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
        self.glass_from_scene_bind_group = create_glass_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &self.targets.scene_view,
            &self.targets.blur_vertical_view,
            &self.sampler,
            &self.glass_uniform,
            &self.blur_weights,
        );
        self.glass_from_output_bind_group = create_glass_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &self.targets.output_view,
            &self.targets.blur_vertical_view,
            &self.sampler,
            &self.glass_uniform,
            &self.blur_weights,
        );
        self.background_bind_group = create_background_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            self.background_texture.as_ref().unwrap_or(&self.placeholder_texture),
            &self.sampler,
            &self.glass_uniform,
            &self.blur_weights,
        );
        self.blur_horizontal_from_scene_bind_group = create_blur_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &self.targets.scene_view,
            &self.sampler,
            &self.blur_uniform,
            &self.blur_weights,
        );
        self.blur_horizontal_from_output_bind_group = create_blur_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &self.targets.output_view,
            &self.sampler,
            &self.blur_uniform,
            &self.blur_weights,
        );
        self.copy_bind_group = create_copy_bind_group(
            &self.device,
            &self.copy_bind_group_layout,
            &self.targets.output_view,
            &self.sampler,
        );
        self.blur_vertical_bind_group = create_blur_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &self.targets.blur_horizontal_view,
            &self.sampler,
            &self.blur_uniform,
            &self.blur_weights,
        );
        Ok(())
    }

    /// Uses an externally decoded image as the reference project's backdrop.
    ///
    /// The texture must be created with [`wgpu::TextureUsages::TEXTURE_BINDING`]
    /// and use a filterable RGBA format.
    pub fn set_background_texture(&mut self, texture: wgpu::Texture, aspect_ratio: f32) {
        self.background_texture_ratio = aspect_ratio.max(f32::EPSILON);
        let bind_group = create_background_bind_group(
            &self.device,
            &self.glass_bind_group_layout,
            &texture,
            &self.sampler,
            &self.glass_uniform,
            &self.blur_weights,
        );
        self.background_texture = Some(texture);
        self.background_bind_group = bind_group;
    }

    /// Uploads a platform-captured RGBA8 backdrop frame.
    ///
    /// The frame can contain padded rows (`stride > width * 4`). Padding is
    /// removed while preparing the GPU upload. The frame is sampled by the
    /// full reference refraction, dispersion, Fresnel, and blur pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::InvalidBackgroundFrame`] when the dimensions,
    /// stride, or byte length do not describe a complete RGBA8 frame.
    ///
    /// This method does not panic for malformed frame metadata.
    #[allow(clippy::cast_precision_loss)]
    pub fn set_background_rgba8(
        &mut self,
        width: u32,
        height: u32,
        stride: u32,
        rgba8: &[u8],
    ) -> Result<(), GpuError> {
        let Some(tight_stride) = width.checked_mul(4) else {
            return Err(GpuError::InvalidBackgroundFrame {
                width,
                height,
                stride,
                byte_len: rgba8.len(),
            });
        };
        let Some(expected) = usize::try_from(stride).ok().and_then(|stride| {
            usize::try_from(height).ok().and_then(|height| stride.checked_mul(height))
        }) else {
            return Err(GpuError::InvalidBackgroundFrame {
                width,
                height,
                stride,
                byte_len: rgba8.len(),
            });
        };
        if width == 0 || height == 0 || stride < tight_stride || rgba8.len() != expected {
            return Err(GpuError::InvalidBackgroundFrame {
                width,
                height,
                stride,
                byte_len: rgba8.len(),
            });
        }

        let frame_height = height;
        let packed = if stride == tight_stride {
            None
        } else {
            let tight_stride = usize::try_from(tight_stride).map_err(|_| {
                GpuError::InvalidBackgroundFrame { width, height, stride, byte_len: rgba8.len() }
            })?;
            let height = usize::try_from(height).map_err(|_| GpuError::InvalidBackgroundFrame {
                width,
                height,
                stride,
                byte_len: rgba8.len(),
            })?;
            let tight_len =
                tight_stride.checked_mul(height).ok_or(GpuError::InvalidBackgroundFrame {
                    width,
                    height: u32::MAX,
                    stride,
                    byte_len: rgba8.len(),
                })?;
            let mut packed = Vec::with_capacity(tight_len);
            let stride = usize::try_from(stride).map_err(|_| GpuError::InvalidBackgroundFrame {
                width,
                height: frame_height,
                stride,
                byte_len: rgba8.len(),
            })?;
            for row in rgba8.chunks_exact(stride).take(height) {
                packed.extend_from_slice(&row[..tight_stride]);
            }
            Some(packed)
        };
        let pixels = packed.as_deref().unwrap_or(rgba8);
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("liquid-glass platform backdrop texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(tight_stride),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.set_background_texture(texture, width as f32 / height as f32);
        Ok(())
    }

    /// Controls whether the base pass writes an opaque background or leaves
    /// it transparent for the operating-system window compositor.
    ///
    /// A transparent frame still needs a backdrop source for local glass
    /// optics. Applications that want refraction of the real desktop should
    /// provide a platform-captured texture with [`Self::set_background_texture`]
    /// or [`Self::set_background_rgba8`]; window alpha alone can reveal the
    /// desktop, but it cannot make desktop pixels available to a custom shader.
    pub fn set_transparent_background(&mut self, transparent: bool) {
        self.transparent_background = transparent;
    }

    /// Encodes and submits one background, blur, and glass frame.
    ///
    /// # Panics
    ///
    /// This panics only if the renderer's compile-time scene node capacity is
    /// smaller than one node.
    pub fn render_panel(&self, node: &GlassNode, time_seconds: f32) {
        self.render_nodes_to_view(&self.targets.output_view, &[node], time_seconds, false)
            .expect("a single glass node must fit in the scene uniform buffer");
    }

    /// Encodes a scene containing multiple glass nodes into the offscreen target.
    ///
    /// Nodes are drawn in ascending `z_index` order.
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::SceneNodeLimitExceeded`] when the scene is larger
    /// than the renderer's dynamic uniform capacity.
    pub fn render_scene(&self, scene: &GlassScene, time_seconds: f32) -> Result<(), GpuError> {
        let nodes = scene.nodes_in_render_order();
        self.render_nodes_to_view(&self.targets.output_view, &nodes, time_seconds, false)
    }

    /// Encodes a scene into an externally owned texture view.
    ///
    /// The external view must use the format passed to
    /// [`Self::from_device_with_format`].
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::SceneNodeLimitExceeded`] when the scene is larger
    /// than the renderer's dynamic uniform capacity.
    pub fn render_scene_to_view(
        &self,
        output_view: &wgpu::TextureView,
        scene: &GlassScene,
        time_seconds: f32,
    ) -> Result<(), GpuError> {
        let nodes = scene.nodes_in_render_order();
        self.render_nodes_to_view(output_view, &nodes, time_seconds, true)
    }

    /// Encodes and submits one frame into an externally owned texture view.
    ///
    /// # Panics
    ///
    /// This panics only if the renderer's compile-time scene node capacity is
    /// smaller than one node.
    pub fn render_panel_to_view(
        &self,
        output_view: &wgpu::TextureView,
        node: &GlassNode,
        time_seconds: f32,
    ) {
        self.render_nodes_to_view(output_view, &[node], time_seconds, true)
            .expect("a single glass node must fit in the scene uniform buffer");
    }

    /// Renders a scene directly to a configured `wgpu` `SurfaceTexture`.
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::SceneNodeLimitExceeded`] when the scene is larger
    /// than the renderer's dynamic uniform capacity.
    pub fn render_scene_to_surface_texture(
        &self,
        surface_texture: wgpu::SurfaceTexture,
        scene: &GlassScene,
        time_seconds: f32,
    ) -> Result<(), GpuError> {
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let result = self.render_scene_to_view(&view, scene, time_seconds);
        if result.is_ok() {
            surface_texture.present();
        }
        result
    }

    #[allow(clippy::too_many_lines)]
    fn render_nodes_to_view(
        &self,
        output_view: &wgpu::TextureView,
        nodes: &[&GlassNode],
        time_seconds: f32,
        copy_to_external_view: bool,
    ) -> Result<(), GpuError> {
        if nodes.len() > MAX_GLASS_NODES {
            return Err(GpuError::SceneNodeLimitExceeded { limit: MAX_GLASS_NODES });
        }

        for (index, node) in nodes.iter().enumerate() {
            let offset = u64::from(self.glass_uniform_stride)
                * u64::try_from(index).expect("scene node index fits in u64");
            let uniform = uniform_for_node(
                self.size,
                node,
                time_seconds,
                self.background_texture.is_some(),
                self.background_texture_ratio,
                self.transparent_background,
            );
            self.queue.write_buffer(&self.glass_uniform, offset, bytemuck::bytes_of(&uniform));
        }
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("liquid-glass frame encoder"),
        });
        encode_fullscreen_pass(
            &mut encoder,
            "liquid-glass scene pass",
            &self.targets.scene_view,
            &self.background_pipeline,
            &self.fullscreen_vertex_buffer,
            Some(&self.background_bind_group),
            Some(0),
            wgpu::Color { r: 0.04, g: 0.06, b: 0.12, a: 1.0 },
        );

        let mut source = PingPongSource::Scene;
        for (index, _) in nodes.iter().enumerate() {
            let blur_radius = blur_radius_for_node(nodes[index]);
            let uniform_offset =
                self.glass_uniform_stride * u32::try_from(index).expect("node index fits in u32");
            let weight_offset = (MAX_BLUR_RADIUS + 1) * index * std::mem::size_of::<f32>();
            let blur_uniform = BlurUniform {
                resolution: gpu_size_as_f32(self.size),
                radius: blur_radius,
                weight_offset: i32::try_from(weight_offset / std::mem::size_of::<f32>())
                    .expect("blur weight offset fits in i32"),
            };
            self.queue.write_buffer(
                &self.blur_uniform,
                u64::from(uniform_offset),
                bytemuck::bytes_of(&blur_uniform),
            );
            let blur_weights = gaussian_weights(blur_radius);
            self.queue.write_buffer(
                &self.blur_weights,
                u64::try_from(weight_offset).expect("blur weight offset fits in u64"),
                bytemuck::cast_slice(&blur_weights),
            );

            let (
                source_texture,
                destination_texture,
                destination_view,
                blur_bind_group,
                glass_bind_group,
            ) = match source {
                PingPongSource::Scene => (
                    &self.targets.scene,
                    &self.targets.output,
                    &self.targets.output_view,
                    &self.blur_horizontal_from_scene_bind_group,
                    &self.glass_from_scene_bind_group,
                ),
                PingPongSource::Output => (
                    &self.targets.output,
                    &self.targets.scene,
                    &self.targets.scene_view,
                    &self.blur_horizontal_from_output_bind_group,
                    &self.glass_from_output_bind_group,
                ),
            };
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: source_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: destination_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                },
            );
            encode_fullscreen_pass(
                &mut encoder,
                "liquid-glass hierarchical horizontal blur pass",
                &self.targets.blur_horizontal_view,
                &self.blur_horizontal_pipeline,
                &self.fullscreen_vertex_buffer,
                Some(blur_bind_group),
                Some(uniform_offset),
                wgpu::Color::BLACK,
            );
            encode_fullscreen_pass(
                &mut encoder,
                "liquid-glass hierarchical vertical blur pass",
                &self.targets.blur_vertical_view,
                &self.blur_vertical_pipeline,
                &self.fullscreen_vertex_buffer,
                Some(&self.blur_vertical_bind_group),
                Some(uniform_offset),
                wgpu::Color::BLACK,
            );
            encode_glass_node_pass(
                &mut encoder,
                destination_view,
                &self.glass_pipeline,
                glass_bind_group,
                &self.fullscreen_vertex_buffer,
                uniform_offset,
            );
            source = source.other();
        }

        if source == PingPongSource::Scene {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.targets.scene,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.targets.output,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        if copy_to_external_view {
            encode_fullscreen_pass(
                &mut encoder,
                "liquid-glass present copy pass",
                output_view,
                &self.copy_pipeline,
                &self.fullscreen_vertex_buffer,
                Some(&self.copy_bind_group),
                None,
                wgpu::Color::BLACK,
            );
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    /// Renders one frame directly to a configured `wgpu` `SurfaceTexture` and presents it.
    pub fn render_panel_to_surface_texture(
        &self,
        surface_texture: wgpu::SurfaceTexture,
        node: &GlassNode,
        time_seconds: f32,
    ) {
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.render_panel_to_view(&view, node, time_seconds);
        surface_texture.present();
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

#[allow(clippy::too_many_arguments)]
fn encode_fullscreen_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    bind_group: Option<&wgpu::BindGroup>,
    dynamic_offset: Option<u32>,
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
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    if let Some(bind_group) = bind_group {
        if let Some(dynamic_offset) = dynamic_offset {
            pass.set_bind_group(0, bind_group, &[dynamic_offset]);
        } else {
            pass.set_bind_group(0, bind_group, &[]);
        }
    }
    pass.draw(0..4, 0..1);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PingPongSource {
    Scene,
    Output,
}

impl PingPongSource {
    const fn other(self) -> Self {
        match self {
            Self::Scene => Self::Output,
            Self::Output => Self::Scene,
        }
    }
}

fn encode_glass_node_pass(
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    vertex_buffer: &wgpu::Buffer,
    uniform_offset: u32,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("liquid-glass pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
        })],
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    pass.set_bind_group(0, bind_group, &[uniform_offset]);
    pass.draw(0..4, 0..1);
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
fn uniform_for_node(
    size: GpuSize,
    node: &GlassNode,
    time_seconds: f32,
    background_texture_ready: bool,
    background_texture_ratio: f32,
    transparent_background: bool,
) -> GlassUniform {
    let center_x = node.bounds.x + node.bounds.width * 0.5;
    let center_y = size.height as f32 - node.bounds.y - node.bounds.height * 0.5;
    let material = node.material;
    let tint = tint_with_whiteness(material);
    let shape_radius = shape_radius(node);
    let shape_roundness = shape_roundness(node);
    let (capsule_bezier_x, capsule_bezier_y) = capsule_bezier_uniforms(node);
    GlassUniform {
        resolution_dpr_pad: [size.width as f32, size.height as f32, 1.0, 0.0],
        mouse_and_spring: [center_x, center_y, center_x, center_y],
        shape: [node.bounds.width, node.bounds.height, shape_radius, shape_roundness],
        capsule_bezier_x,
        capsule_bezier_y,
        merge_glare_shadow: [
            0.05,
            time_seconds * 0.20,
            material.shadow.expand.max(1.0),
            material.shadow.factor.clamp(0.0, 0.6),
        ],
        shadow_position_bg_ratio: [
            material.shadow.offset[0],
            material.shadow.offset[1],
            background_texture_ratio,
        ],
        bg_type: if transparent_background {
            12
        } else if background_texture_ready {
            11
        } else {
            0
        },
        flags: [
            i32::from(background_texture_ready),
            0,
            material.blur.radius.round() as i32,
            i32::from(material.blur.edge_blur),
        ],
        tint,
        refraction_and_fresnel: [
            (material.refraction.thickness * 100.0).max(1.0),
            material.refraction.index,
            (material.dispersion.strength * 100.0).max(0.0),
            (material.fresnel.range * 40.0).max(1.0),
            material.fresnel.hardness,
            material.fresnel.strength,
        ],
        glare: [30.0, 0.2, 0.5, 0.8, 0.9],
        _pad: [material.opacity, 0.0, 0.0, 0.0, 0.0],
    }
}

fn tint_with_whiteness(material: liquid_glass_scene::GlassMaterial) -> [f32; 4] {
    let tint_alpha = material.tint.a.clamp(0.0, 1.0);
    let white_alpha = material.whiteness.clamp(0.0, 1.0);
    let combined_alpha = 1.0 - (1.0 - tint_alpha) * (1.0 - white_alpha);
    if combined_alpha <= f32::EPSILON {
        return [material.tint.r, material.tint.g, material.tint.b, 0.0];
    }

    let tint_weight = tint_alpha * (1.0 - white_alpha);
    [
        (material.tint.r * tint_weight + white_alpha) / combined_alpha,
        (material.tint.g * tint_weight + white_alpha) / combined_alpha,
        (material.tint.b * tint_weight + white_alpha) / combined_alpha,
        combined_alpha,
    ]
}

#[allow(clippy::cast_precision_loss)]
fn gpu_size_as_f32(size: GpuSize) -> [f32; 2] {
    [size.width as f32, size.height as f32]
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
fn blur_radius_for_node(node: &GlassNode) -> i32 {
    node.material.blur.radius.round().clamp(0.0, MAX_BLUR_RADIUS as f32) as i32
}

#[allow(clippy::cast_precision_loss, clippy::needless_range_loop)]
fn gaussian_weights(radius: i32) -> [f32; MAX_BLUR_RADIUS + 1] {
    let mut weights = [0.0; MAX_BLUR_RADIUS + 1];
    let radius = usize::try_from(radius).expect("blur radius is non-negative");
    let sigma = (radius as f32 / 3.0).max(f32::EPSILON);
    let mut sum = 0.0;
    for index in 0..=radius {
        let index_f32 = index as f32;
        let weight = (-0.5 * index_f32 * index_f32 / (sigma * sigma)).exp();
        weights[index] = weight;
        sum += if index == 0 { weight } else { weight * 2.0 };
    }
    if sum > 0.0 {
        for weight in &mut weights[..=radius] {
            *weight /= sum;
        }
    }
    weights
}

fn create_blur_weights_buffer(device: &wgpu::Device, node_capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("liquid-glass gaussian weights"),
        size: u64::try_from(node_capacity * (MAX_BLUR_RADIUS + 1) * std::mem::size_of::<f32>())
            .expect("blur weights size fits in u64"),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_placeholder_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("liquid-glass placeholder texture"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn create_fullscreen_vertex_buffer(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Buffer {
    let vertices = [-1.0_f32, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("liquid-glass fullscreen vertices"),
        size: u64::try_from(std::mem::size_of_val(&vertices)).expect("vertex buffer size fits"),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&vertices));
    buffer
}

fn create_glass_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    scene_view: &wgpu::TextureView,
    blur_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
    blur_weights: &wgpu::Buffer,
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
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniform,
                    offset: 0,
                    size: Some(
                        wgpu::BufferSize::new(u64::from(GLASS_UNIFORM_SIZE))
                            .expect("glass uniform size is non-zero"),
                    ),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: blur_weights,
                    offset: 0,
                    size: None,
                }),
            },
        ],
    })
}

fn create_copy_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("liquid-glass copy layout"),
        entries: &[texture_binding(0), sampler_binding(1)],
    })
}

fn create_copy_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("liquid-glass copy bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}

fn create_background_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    placeholder_texture: &wgpu::Texture,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
    blur_weights: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let placeholder_view = placeholder_texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("liquid-glass background bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&placeholder_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&placeholder_view),
            },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniform,
                    offset: 0,
                    size: Some(
                        wgpu::BufferSize::new(u64::from(GLASS_UNIFORM_SIZE))
                            .expect("glass uniform size is non-zero"),
                    ),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: blur_weights,
                    offset: 0,
                    size: None,
                }),
            },
        ],
    })
}

fn create_blur_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
    blur_weights: &wgpu::Buffer,
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
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniform,
                    offset: 0,
                    size: Some(
                        wgpu::BufferSize::new(u64::from(GLASS_UNIFORM_SIZE))
                            .expect("glass uniform size is non-zero"),
                    ),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: blur_weights,
                    offset: 0,
                    size: None,
                }),
            },
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

fn storage_binding(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_binding(binding: u32, size: usize) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: wgpu::BufferSize::new(size as u64),
        },
        count: None,
    }
}

fn uniform_stride(device: &wgpu::Device) -> u32 {
    let alignment = device.limits().min_uniform_buffer_offset_alignment.max(1);
    GLASS_UNIFORM_SIZE.div_ceil(alignment) * alignment
}

fn create_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader: &wgpu::ShaderModule,
    bind_group_layout: Option<&wgpu::BindGroupLayout>,
    fragment_entry: &str,
    blend: Option<wgpu::BlendState>,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[bind_group_layout.expect("pipeline requires a bind group layout")],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: FULLSCREEN_VERTEX_ATTRIBUTES,
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format: output_format,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview: None,
        cache: None,
    })
}

fn create_copy_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("liquid-glass hierarchical copy shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(format!(
            "{}\n{}",
            include_str!("../../../shaders/glass/reference/vertex.wgsl"),
            include_str!("../../../shaders/glass/copy.wgsl"),
        ))),
    });
    create_pipeline(
        device,
        "liquid-glass hierarchical copy pipeline",
        &shader,
        Some(bind_group_layout),
        "fs_main",
        None,
        output_format,
    )
}

fn create_pipelines(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::RenderPipeline, wgpu::RenderPipeline, wgpu::RenderPipeline) {
    let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("liquid-glass reference background shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(reference_shader(
            include_str!("../../../shaders/glass/reference/fragment-bg.wgsl"),
            ReferencePass::Background,
        ))),
    });
    let horizontal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("liquid-glass reference horizontal blur shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(reference_shader(
            include_str!("../../../shaders/glass/reference/fragment-bg-hblur.wgsl"),
            ReferencePass::Blur,
        ))),
    });
    let vertical_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("liquid-glass reference vertical blur shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(reference_shader(
            include_str!("../../../shaders/glass/reference/fragment-bg-vblur.wgsl"),
            ReferencePass::Blur,
        ))),
    });
    let glass_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("liquid-glass reference main shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(reference_shader(
            include_str!("../../../shaders/glass/reference/fragment-main.wgsl"),
            ReferencePass::Main,
        ))),
    });
    let background = create_pipeline(
        device,
        "liquid-glass background pipeline",
        &background_shader,
        Some(bind_group_layout),
        "fs_main",
        None,
        output_format,
    );
    let horizontal = create_pipeline(
        device,
        "liquid-glass horizontal blur pipeline",
        &horizontal_shader,
        Some(bind_group_layout),
        "fs_main",
        None,
        output_format,
    );
    let vertical = create_pipeline(
        device,
        "liquid-glass vertical blur pipeline",
        &vertical_shader,
        Some(bind_group_layout),
        "fs_main",
        None,
        output_format,
    );
    let glass = create_pipeline(
        device,
        "liquid-glass SDF pipeline",
        &glass_shader,
        Some(bind_group_layout),
        "fs_main",
        Some(wgpu::BlendState::ALPHA_BLENDING),
        output_format,
    );

    (background, horizontal, vertical, glass)
}

#[derive(Clone, Copy)]
enum ReferencePass {
    Background,
    Blur,
    Main,
}

fn reference_shader(fragment: &str, pass: ReferencePass) -> String {
    let vertex = include_str!("../../../shaders/glass/reference/vertex.wgsl");
    let mut fragment = fragment.to_owned();
    match pass {
        ReferencePass::Background => {
            fragment = fragment
                .replace(
                    "@binding(0) var<uniform> u: Uniforms",
                    "@binding(3) var<uniform> u: Uniforms",
                )
                .replace("@binding(1) var u_bgTexture", "@binding(0) var u_bgTexture");
        }
        ReferencePass::Blur => {
            fragment = fragment
                .replace(
                    "@binding(0) var<uniform> u: BlurUniforms",
                    "@binding(3) var<uniform> u: BlurUniforms",
                )
                .replace("@binding(1) var u_prevPassTexture", "@binding(0) var u_prevPassTexture")
                .replace(
                    "@binding(3) var<storage, read> u_blurWeights",
                    "@binding(4) var<storage, read> u_blurWeights",
                );
        }
        ReferencePass::Main => {
            fragment = fragment
                .replace(
                    "@binding(0) var<uniform> u: Uniforms",
                    "@binding(3) var<uniform> u: Uniforms",
                )
                .replace("@binding(2) var u_bg", "@binding(0) var u_bg")
                .replace("@binding(3) var u_sampler", "@binding(2) var u_sampler");
        }
    }
    format!("{vertex}\n{fragment}")
}

fn shape_radius(node: &GlassNode) -> f32 {
    match node.shape {
        GlassShape::RoundedRect { radius } => radius,
        GlassShape::Superellipse { .. } => node.bounds.width.min(node.bounds.height) * 0.4,
        GlassShape::Capsule => node.bounds.height * 0.5,
        GlassShape::Circle => node.bounds.width.min(node.bounds.height) * 0.5,
        GlassShape::Ellipse => node.bounds.width.min(node.bounds.height) * 0.25,
    }
}

fn shape_roundness(node: &GlassNode) -> f32 {
    match node.shape {
        GlassShape::Superellipse { exponent } => exponent,
        GlassShape::RoundedRect { .. } => node.corner_curve.exponent(),
        GlassShape::Capsule => -1.0,
        GlassShape::Circle | GlassShape::Ellipse => 2.0,
    }
}

#[allow(clippy::cast_possible_truncation)]
fn capsule_bezier_uniforms(node: &GlassNode) -> ([f32; 4], [f32; 4]) {
    if !matches!(node.shape, GlassShape::Capsule) {
        return ([0.0; 4], [0.0; 4]);
    }
    let capsule =
        node.g2_continuity.capsule(f64::from(node.bounds.width), f64::from(node.bounds.height));
    let Some(bezier) = capsule.shoulder else {
        return ([0.0; 4], [0.0; 4]);
    };
    (
        [bezier.p0.x as f32, bezier.p1.x as f32, bezier.p2.x as f32, bezier.p3.x as f32],
        [bezier.p0.y as f32, bezier.p1.y as f32, bezier.p2.y as f32, bezier.p3.y as f32],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use liquid_glass_scene::{CornerCurve, G2Continuity, G2Profile, GlassId, GlassMaterial, Rect};

    #[test]
    fn noop_device_can_build_and_submit_glass_frame() {
        let (device, queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
        let mut renderer = GpuRenderer::from_device(device, queue, GpuSize::new(128, 128));
        let padded_backdrop = vec![0_u8; 12 * 2];
        renderer
            .set_background_rgba8(2, 2, 12, &padded_backdrop)
            .expect("padded RGBA8 backdrop upload");
        let node = GlassNode::new(GlassId(1), Rect::new(16.0, 16.0, 96.0, 64.0))
            .material(GlassMaterial::regular());

        renderer.render_panel(&node, 0.0);
        assert_eq!(renderer.size(), GpuSize::new(128, 128));

        let mut scene = GlassScene::default();
        scene.push(node.clone());
        let mut front = GlassNode::new(GlassId(2), Rect::new(48.0, 40.0, 64.0, 56.0))
            .shape(GlassShape::Capsule)
            .material(GlassMaterial::interactive());
        front.z_index = 1;
        scene.push(front);
        renderer.render_scene(&scene, 0.0).expect("render multiple glass nodes");

        renderer.resize(GpuSize::new(65, 33)).expect("resize blur targets");
        renderer.render_scene(&scene, 0.0).expect("render multiple glass nodes after resize");
        assert_eq!(renderer.size(), GpuSize::new(65, 33));
    }

    #[test]
    fn rounded_rects_and_capsules_use_continuous_curves() {
        let continuous = GlassNode::new(GlassId(3), Rect::new(0.0, 0.0, 72.0, 36.0))
            .shape(GlassShape::RoundedRect { radius: 12.0 });
        let circular = continuous.clone().corner_curve(CornerCurve::Circular);
        let capsule = continuous.clone().shape(GlassShape::Capsule);
        let circle = continuous.clone().shape(GlassShape::Circle);

        assert!((shape_roundness(&continuous) - 5.0).abs() < f32::EPSILON);
        assert!((shape_roundness(&circular) - 2.0).abs() < f32::EPSILON);
        assert!((shape_roundness(&capsule) + 1.0).abs() < f32::EPSILON);
        let (bezier_x, bezier_y) = capsule_bezier_uniforms(&capsule);
        assert!(bezier_x[0] < 0.0);
        assert!(bezier_x[3] > 0.0);
        assert!(bezier_y[3] > 0.0);
        let custom_capsule = capsule.clone().g2_continuity(G2Continuity::new(
            G2Profile::ROUNDED_RECTANGLE,
            G2Profile::new(0.2, 0.0, 1.0, 1.0),
        ));
        let (custom_bezier_x, _) = capsule_bezier_uniforms(&custom_capsule);
        assert!((bezier_x[0] - custom_bezier_x[0]).abs() > f32::EPSILON);
        assert!((shape_roundness(&circle) - 2.0).abs() < f32::EPSILON);
        assert_eq!(std::mem::size_of::<GlassUniform>(), 208);
    }

    #[test]
    fn whiteness_adds_a_neutral_layer_above_tint() {
        let mut material = GlassMaterial::clear();
        material.tint = liquid_glass_scene::Color::rgba(0.2, 0.4, 0.8, 0.10);
        material.whiteness = 0.20;

        let effective = tint_with_whiteness(material);

        assert!((effective[3] - 0.28).abs() < f32::EPSILON);
        assert!(effective[0] > material.tint.r);
        assert!(effective[1] > material.tint.g);
        assert!(effective[2] > material.tint.b);
    }
}
