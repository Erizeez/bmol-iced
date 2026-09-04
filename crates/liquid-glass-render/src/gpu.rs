use std::{borrow::Cow, fmt};

use crate::ScrollEdgeStyle;
use bytemuck::{Pod, Zeroable};
use liquid_glass_scene::{
    GlassAccessibility, GlassEnvironment, GlassNode, GlassRenderOptions, GlassScene, GlassShape,
    GlassVariant, Rect,
};

const DEFAULT_OUTPUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[allow(clippy::cast_possible_truncation)]
const GLASS_UNIFORM_SIZE: u32 = std::mem::size_of::<GlassUniform>() as u32;
const MAX_BLUR_RADIUS: usize = 200;

const FEATURE_EDGE_BLUR: i32 = 1;
const FEATURE_REDUCED_TRANSPARENCY: i32 = 1 << 1;
const FEATURE_INCREASED_CONTRAST: i32 = 1 << 2;
const FEATURE_REDUCED_MOTION: i32 = 1 << 3;
const FEATURE_CLEAR_VARIANT: i32 = 1 << 4;

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
    // [refraction strength, opacity, interaction strength, environment
    // luminance, environment contrast].
    _pad: [f32; 5],
    // [tint response, ambient spill response, shadow response, size response].
    adaptive: [f32; 4],
    // [center x, center y, width, height] for up to four fused shapes in
    // bottom-origin logical pixels.
    fused_bounds: [[f32; 4]; 4],
    // [corner radius, corner exponent, enabled, unused] for each shape.
    fused_geometry: [[f32; 4]; 4],
    // [spring pointer x, spring pointer y, parallax, focus].
    interaction_state: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct BlurUniform {
    resolution: [f32; 2],
    radius: i32,
    weight_offset: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct FlatBlurUniform {
    tint: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct GradientCompositeUniform {
    fallback: [f32; 4],
    // [start_y, end_y, unused, unused] in normalized texture coordinates.
    gradient: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct RegionBlurUniform {
    resolution: [f32; 2],
    radius: i32,
    weight_offset: i32,
    region: [f32; 4],
    // [start_y, end_y, minimum_radius, maximum_radius]. A zero-height range
    // keeps the bounded blur uniform, while a valid range enables a vertical
    // top-to-bottom blur gradient.
    gradient: [f32; 4],
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
    shadow_pipeline: wgpu::RenderPipeline,
    glass_pipeline: wgpu::RenderPipeline,
    copy_bind_group_layout: wgpu::BindGroupLayout,
    copy_bind_group: wgpu::BindGroup,
    copy_pipeline: wgpu::RenderPipeline,
    foreground_copy_pipeline: wgpu::RenderPipeline,
    flat_blur_bind_group_layout: wgpu::BindGroupLayout,
    flat_blur_bind_group: wgpu::BindGroup,
    flat_blur_uniform: wgpu::Buffer,
    flat_blur_pipeline: wgpu::RenderPipeline,
    region_blur_bind_group_layout: wgpu::BindGroupLayout,
    region_blur_horizontal_bind_group: wgpu::BindGroup,
    region_blur_horizontal_from_output_bind_group: wgpu::BindGroup,
    region_blur_vertical_bind_group: wgpu::BindGroup,
    region_blur_uniform: wgpu::Buffer,
    region_blur_horizontal_pipeline: wgpu::RenderPipeline,
    region_blur_vertical_pipeline: wgpu::RenderPipeline,
    gradient_composite_bind_group_layout: wgpu::BindGroupLayout,
    gradient_composite_bind_group: wgpu::BindGroup,
    gradient_composite_uniform: wgpu::Buffer,
    gradient_composite_pipeline: wgpu::RenderPipeline,
    transparent_background: bool,
    options: GlassRenderOptions,
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
            size: u64::from(glass_uniform_stride) * (MAX_GLASS_NODES as u64 + 1),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let blur_weights = create_blur_weights_buffer(&device, MAX_GLASS_NODES + 2);
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

        let (
            background_pipeline,
            blur_horizontal_pipeline,
            blur_vertical_pipeline,
            shadow_pipeline,
            glass_pipeline,
        ) = create_pipelines(&device, &glass_bind_group_layout, output_format);
        let copy_bind_group_layout = create_copy_bind_group_layout(&device);
        let copy_bind_group = create_copy_bind_group(
            &device,
            &copy_bind_group_layout,
            &targets.output_view,
            &sampler,
        );
        let copy_pipeline =
            create_copy_pipeline(&device, &copy_bind_group_layout, output_format, None);
        let foreground_copy_pipeline = create_copy_pipeline(
            &device,
            &copy_bind_group_layout,
            output_format,
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let flat_blur_bind_group_layout = create_flat_blur_bind_group_layout(&device);
        let flat_blur_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("liquid-glass flat blur tint uniform"),
            size: u64::try_from(std::mem::size_of::<FlatBlurUniform>())
                .expect("flat blur uniform size fits in u64"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let flat_blur_bind_group = create_flat_blur_bind_group(
            &device,
            &flat_blur_bind_group_layout,
            &targets.blur_vertical_view,
            &sampler,
            &flat_blur_uniform,
        );
        let flat_blur_pipeline =
            create_flat_blur_pipeline(&device, &flat_blur_bind_group_layout, output_format);
        let region_blur_bind_group_layout = create_region_blur_bind_group_layout(&device);
        let region_blur_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("liquid-glass region blur uniform"),
            size: u64::try_from(std::mem::size_of::<RegionBlurUniform>())
                .expect("region blur uniform size fits in u64"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let region_blur_horizontal_bind_group = create_region_blur_bind_group(
            &device,
            &region_blur_bind_group_layout,
            &targets.scene_view,
            &sampler,
            &region_blur_uniform,
            &blur_weights,
        );
        let region_blur_horizontal_from_output_bind_group = create_region_blur_bind_group(
            &device,
            &region_blur_bind_group_layout,
            &targets.output_view,
            &sampler,
            &region_blur_uniform,
            &blur_weights,
        );
        let region_blur_vertical_bind_group = create_region_blur_bind_group(
            &device,
            &region_blur_bind_group_layout,
            &targets.blur_horizontal_view,
            &sampler,
            &region_blur_uniform,
            &blur_weights,
        );
        let region_blur_horizontal_pipeline = create_region_blur_pipeline(
            &device,
            &region_blur_bind_group_layout,
            output_format,
            true,
        );
        let region_blur_vertical_pipeline = create_region_blur_pipeline(
            &device,
            &region_blur_bind_group_layout,
            output_format,
            false,
        );
        let gradient_composite_bind_group_layout =
            create_gradient_composite_bind_group_layout(&device);
        let gradient_composite_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("liquid-glass gradient composite uniform"),
            size: u64::try_from(std::mem::size_of::<GradientCompositeUniform>())
                .expect("gradient composite uniform size fits in u64"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let gradient_composite_bind_group = create_gradient_composite_bind_group(
            &device,
            &gradient_composite_bind_group_layout,
            &targets.blur_vertical_view,
            &targets.scene_view,
            &sampler,
            &gradient_composite_uniform,
        );
        let gradient_composite_pipeline = create_gradient_composite_pipeline(
            &device,
            &gradient_composite_bind_group_layout,
            output_format,
        );

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
            shadow_pipeline,
            glass_pipeline,
            copy_bind_group_layout,
            copy_bind_group,
            copy_pipeline,
            foreground_copy_pipeline,
            flat_blur_bind_group_layout,
            flat_blur_bind_group,
            flat_blur_uniform,
            flat_blur_pipeline,
            region_blur_bind_group_layout,
            region_blur_horizontal_bind_group,
            region_blur_horizontal_from_output_bind_group,
            region_blur_vertical_bind_group,
            region_blur_uniform,
            region_blur_horizontal_pipeline,
            region_blur_vertical_pipeline,
            gradient_composite_bind_group_layout,
            gradient_composite_bind_group,
            gradient_composite_uniform,
            gradient_composite_pipeline,
            transparent_background: false,
            options: GlassRenderOptions::default(),
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
        self.flat_blur_bind_group = create_flat_blur_bind_group(
            &self.device,
            &self.flat_blur_bind_group_layout,
            &self.targets.blur_vertical_view,
            &self.sampler,
            &self.flat_blur_uniform,
        );
        self.region_blur_horizontal_bind_group = create_region_blur_bind_group(
            &self.device,
            &self.region_blur_bind_group_layout,
            &self.targets.scene_view,
            &self.sampler,
            &self.region_blur_uniform,
            &self.blur_weights,
        );
        self.region_blur_horizontal_from_output_bind_group = create_region_blur_bind_group(
            &self.device,
            &self.region_blur_bind_group_layout,
            &self.targets.output_view,
            &self.sampler,
            &self.region_blur_uniform,
            &self.blur_weights,
        );
        self.region_blur_vertical_bind_group = create_region_blur_bind_group(
            &self.device,
            &self.region_blur_bind_group_layout,
            &self.targets.blur_horizontal_view,
            &self.sampler,
            &self.region_blur_uniform,
            &self.blur_weights,
        );
        self.gradient_composite_bind_group = create_gradient_composite_bind_group(
            &self.device,
            &self.gradient_composite_bind_group_layout,
            &self.targets.blur_vertical_view,
            &self.targets.scene_view,
            &self.sampler,
            &self.gradient_composite_uniform,
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

    /// Replaces the renderer-wide accessibility policy.
    pub fn set_accessibility(&mut self, accessibility: GlassAccessibility) {
        self.options.accessibility = accessibility;
    }

    /// Replaces the renderer-wide backdrop environment estimate.
    pub fn set_environment(&mut self, environment: GlassEnvironment) {
        self.options.environment = environment;
    }

    /// Replaces all renderer-wide Liquid Glass options at once.
    pub fn set_render_options(&mut self, options: GlassRenderOptions) {
        self.options = options;
    }

    /// Returns the options used by subsequent composition calls.
    #[must_use]
    pub const fn render_options(&self) -> GlassRenderOptions {
        self.options
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
        self.options.environment = GlassEnvironment::from_rgba8(pixels);
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
        self.render_nodes_to_view(
            &self.targets.output_view,
            &[node],
            time_seconds,
            false,
            None,
            None,
            self.options,
        )
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
        self.render_nodes_to_view(
            &self.targets.output_view,
            &nodes,
            time_seconds,
            false,
            None,
            None,
            scene.render_options().unwrap_or(self.options),
        )
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
        self.render_nodes_to_view(
            output_view,
            &nodes,
            time_seconds,
            true,
            None,
            None,
            scene.render_options().unwrap_or(self.options),
        )
    }

    /// Encodes a scene into an externally owned view while using an external
    /// texture as the first compositing layer.
    ///
    /// The source texture must use the renderer's output format and include
    /// [`wgpu::TextureUsages::COPY_SRC`]. It is copied into the renderer's
    /// scene target before any glass node is evaluated, so every pixel drawn by
    /// the caller—including text and icons—can be refracted by the nodes.
    /// Nodes are still evaluated in ascending `z_index` order, and each later
    /// node samples the previous glass composite.
    ///
    /// # Errors
    ///
    /// Returns [`GpuError::SceneNodeLimitExceeded`] when the scene is larger
    /// than the renderer's dynamic uniform capacity.
    pub fn render_scene_to_view_with_source(
        &self,
        output_view: &wgpu::TextureView,
        source_texture: &wgpu::Texture,
        scene: &GlassScene,
        time_seconds: f32,
    ) -> Result<(), GpuError> {
        let nodes = scene.nodes_in_render_order();
        self.render_nodes_to_view(
            output_view,
            &nodes,
            time_seconds,
            true,
            Some(source_texture),
            None,
            scene.render_options().unwrap_or(self.options),
        )
    }

    /// Renders a scene over the renderer's current output instead of
    /// rebuilding it from an external source. This is used for glass controls
    /// that must sit above a post-composition layer such as a sidebar fade.
    pub fn render_scene_over_output(
        &self,
        output_view: &wgpu::TextureView,
        scene: &GlassScene,
        time_seconds: f32,
    ) -> Result<(), GpuError> {
        let nodes = scene.nodes_in_render_order();
        self.render_nodes_to_view(
            output_view,
            &nodes,
            time_seconds,
            true,
            Some(&self.targets.output),
            None,
            scene.render_options().unwrap_or(self.options),
        )
    }

    /// Encodes a scene using a plain rectangular Gaussian-blur surface below
    /// the liquid-glass nodes.
    ///
    /// The blur region is deliberately not a [`GlassNode`]: it has no SDF,
    /// refraction, dispersion, Fresnel, glare, or liquid edge treatment. The
    /// tint alpha controls the amount of the uniform medium mixed over the
    /// blurred source.
    pub fn render_scene_to_view_with_source_and_blur_region(
        &self,
        output_view: &wgpu::TextureView,
        source_texture: &wgpu::Texture,
        blur_region: (u32, u32, u32, u32),
        blur_radius: u32,
        tint: [f32; 4],
        scene: &GlassScene,
        time_seconds: f32,
    ) -> Result<(), GpuError> {
        let nodes = scene.nodes_in_render_order();
        self.render_nodes_to_view(
            output_view,
            &nodes,
            time_seconds,
            true,
            Some(source_texture),
            Some(SimpleBlurRegion { bounds: blur_region, radius: blur_radius, tint }),
            scene.render_options().unwrap_or(self.options),
        )
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
        self.render_nodes_to_view(
            output_view,
            &[node],
            time_seconds,
            true,
            None,
            None,
            self.options,
        )
        .expect("a single glass node must fit in the scene uniform buffer");
    }

    /// Renders the current backdrop texture into an external target view.
    ///
    /// This is used to seed an application-owned offscreen UI surface before
    /// the UI renderer draws its content over it. The resulting texture can
    /// then be passed to [`Self::render_scene_to_view_with_source`].
    pub fn render_background_to_view(&self, output_view: &wgpu::TextureView) {
        let node = GlassNode::new(
            liquid_glass_scene::GlassId(0),
            liquid_glass_scene::Rect::new(
                0.0,
                0.0,
                self.size.width as f32,
                self.size.height as f32,
            ),
        );
        let mut uniform = uniform_for_node(
            self.size,
            &node,
            0.0,
            self.background_texture.is_some(),
            self.background_texture_ratio,
            true,
            false,
            self.options,
        );
        uniform.bg_type = if self.background_texture.is_some() { 11 } else { 12 };
        self.queue.write_buffer(&self.glass_uniform, 0, bytemuck::bytes_of(&uniform));
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("liquid-glass external source background encoder"),
        });
        encode_fullscreen_pass(
            &mut encoder,
            "liquid-glass external source background pass",
            output_view,
            &self.background_pipeline,
            &self.fullscreen_vertex_buffer,
            Some(&self.background_bind_group),
            Some(0),
            wgpu::Color::BLACK,
        );
        self.queue.submit([encoder.finish()]);
    }

    /// Fills a physical-pixel region of an external view with a flat color.
    ///
    /// This is useful for transparent-window demos that have an opaque app
    /// surface beside a translucent native desktop-glass surface. The color's
    /// alpha is preserved so the platform compositor can remain visible. The
    /// fill happens before the caller draws its source UI, so that source
    /// pixels remain available to the glass compositor while the final
    /// foreground pass stays clear.
    pub fn render_solid_region_to_view(
        &self,
        output_view: &wgpu::TextureView,
        region: (u32, u32, u32, u32),
        color: [f32; 4],
    ) {
        let node = GlassNode::new(
            liquid_glass_scene::GlassId(0),
            liquid_glass_scene::Rect::new(
                0.0,
                0.0,
                self.size.width as f32,
                self.size.height as f32,
            ),
        );
        let mut uniform =
            uniform_for_node(self.size, &node, 0.0, false, 1.0, false, false, self.options);
        uniform.bg_type = 3;
        uniform.tint = srgb_to_linear_rgba(color);
        self.queue.write_buffer(&self.glass_uniform, 0, bytemuck::bytes_of(&uniform));
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("liquid-glass external solid region encoder"),
        });
        encode_scissored_fullscreen_pass(
            &mut encoder,
            "liquid-glass external solid region pass",
            output_view,
            &self.background_pipeline,
            &self.fullscreen_vertex_buffer,
            Some(&self.background_bind_group),
            Some(0),
            wgpu::Color::TRANSPARENT,
            region,
        );
        self.queue.submit([encoder.finish()]);
    }

    /// Alpha-composites an application-owned transparent layer over an
    /// already-composited glass view.
    ///
    /// The source texture must use the renderer's output format and have the
    /// same dimensions as the renderer. Pixels with zero alpha leave the
    /// existing glass result untouched.
    pub fn composite_texture_to_view(
        &self,
        output_view: &wgpu::TextureView,
        source_texture: &wgpu::Texture,
    ) {
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = create_copy_bind_group(
            &self.device,
            &self.copy_bind_group_layout,
            &source_view,
            &self.sampler,
        );
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("liquid-glass foreground composite encoder"),
        });
        encode_fullscreen_load_pass(
            &mut encoder,
            "liquid-glass foreground composite pass",
            output_view,
            &self.foreground_copy_pipeline,
            &self.fullscreen_vertex_buffer,
            &bind_group,
        );
        self.queue.submit([encoder.finish()]);
    }

    /// Alpha-composites an application-owned transparent layer into the
    /// renderer's ping-pong output before a later post-composition pass.
    pub fn composite_texture_to_output(&self, source_texture: &wgpu::Texture) {
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = create_copy_bind_group(
            &self.device,
            &self.copy_bind_group_layout,
            &source_view,
            &self.sampler,
        );
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("liquid-glass internal foreground composite encoder"),
        });
        encode_fullscreen_load_pass(
            &mut encoder,
            "liquid-glass internal foreground composite pass",
            &self.targets.output_view,
            &self.foreground_copy_pipeline,
            &self.fullscreen_vertex_buffer,
            &bind_group,
        );
        self.queue.submit([encoder.finish()]);
    }

    /// Copies the renderer's final internal output into an external target.
    pub fn copy_output_to_view(&self, output_view: &wgpu::TextureView) {
        let source_view = self.targets.output.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = create_copy_bind_group(
            &self.device,
            &self.copy_bind_group_layout,
            &source_view,
            &self.sampler,
        );
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("liquid-glass final output copy encoder"),
        });
        encode_fullscreen_load_pass(
            &mut encoder,
            "liquid-glass final output copy pass",
            output_view,
            &self.copy_pipeline,
            &self.fullscreen_vertex_buffer,
            &bind_group,
        );
        self.queue.submit([encoder.finish()]);
    }

    /// Applies a uniformly blurred overlay whose opacity fades from top to
    /// bottom. The blur uses `maximum_radius`; `minimum_radius` is retained
    /// for compatibility with the earlier radius-gradient implementation.
    pub fn render_vertical_gradient_blur_to_output(
        &self,
        region: (u32, u32, u32, u32),
        minimum_radius: u32,
        maximum_radius: u32,
        fallback_color: [f32; 4],
    ) {
        self.render_vertical_gradient_blur_to_output_with_source(
            &self.targets.output,
            region,
            region.1,
            minimum_radius,
            maximum_radius,
            fallback_color,
        );
    }

    /// Applies a uniformly blurred overlay with a flat opaque top, followed
    /// by a top-to-bottom alpha fade. This models a floating search field:
    /// content above its upper edge stays strongly blurred, while the area
    /// between its upper and lower edges fades to the clear list below.
    pub fn render_vertical_blur_with_flat_top_to_output(
        &self,
        region: (u32, u32, u32, u32),
        fade_start_y: u32,
        maximum_radius: u32,
        fallback_color: [f32; 4],
    ) {
        self.render_vertical_gradient_blur_to_output_with_source(
            &self.targets.output,
            region,
            fade_start_y,
            0,
            maximum_radius,
            fallback_color,
        );
    }

    /// Applies a reusable scroll-edge treatment to the current output.
    ///
    /// The top-edge form is intentionally expressed in physical pixels so a
    /// platform adapter can keep the fade aligned with a clipped scroll
    /// viewport. `Soft` fades the blur across the region; `Hard` keeps the
    /// entire region blurred and ends it at the region boundary.
    pub fn render_scroll_edge_to_output(
        &self,
        region: (u32, u32, u32, u32),
        fade_start_y: u32,
        maximum_radius: u32,
        fallback_color: [f32; 4],
        style: ScrollEdgeStyle,
    ) {
        let fade_start_y = match style {
            ScrollEdgeStyle::Soft => fade_start_y,
            ScrollEdgeStyle::Hard => region.1.saturating_add(region.3),
        };
        self.render_vertical_gradient_blur_to_output_with_source(
            &self.targets.output,
            region,
            fade_start_y,
            0,
            maximum_radius,
            fallback_color,
        );
    }

    fn render_vertical_gradient_blur_to_output_with_source(
        &self,
        source_texture: &wgpu::Texture,
        region: (u32, u32, u32, u32),
        fade_start_y: u32,
        _minimum_radius: u32,
        maximum_radius: u32,
        fallback_color: [f32; 4],
    ) {
        let x = region.0.min(self.size.width);
        let y = region.1.min(self.size.height);
        let width = region.2.min(self.size.width.saturating_sub(x));
        let height = region.3.min(self.size.height.saturating_sub(y));
        if width == 0 || height == 0 {
            return;
        }

        let maximum_radius = maximum_radius.min(MAX_BLUR_RADIUS as u32);
        let fade_start_y = fade_start_y.clamp(y, y + height);
        let gradient_weight_offset =
            (MAX_GLASS_NODES + 1) * (MAX_BLUR_RADIUS + 1) * std::mem::size_of::<f32>();
        let uniform = RegionBlurUniform {
            resolution: gpu_size_as_f32(self.size),
            radius: maximum_radius as i32,
            weight_offset: i32::try_from(gradient_weight_offset / std::mem::size_of::<f32>())
                .expect("gradient blur weight offset fits in i32"),
            region: [x as f32, y as f32, width as f32, height as f32],
            // Keep the blur itself uniform. The vertical gradient is carried
            // by the composite alpha below, so there is only one transition.
            gradient: [0.0; 4],
        };
        self.queue.write_buffer(&self.region_blur_uniform, 0, bytemuck::bytes_of(&uniform));
        let blur_weights = gaussian_weights(maximum_radius as i32);
        self.queue.write_buffer(
            &self.blur_weights,
            u64::try_from(gradient_weight_offset).expect("gradient weight offset fits in u64"),
            bytemuck::cast_slice(&blur_weights),
        );
        self.queue.write_buffer(
            &self.gradient_composite_uniform,
            0,
            bytemuck::bytes_of(&GradientCompositeUniform {
                fallback: srgb_to_linear_rgba(fallback_color),
                gradient: [
                    fade_start_y as f32 / self.size.height as f32,
                    (y + height) as f32 / self.size.height as f32,
                    0.0,
                    0.0,
                ],
            }),
        );
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let source_bind_group = create_region_blur_bind_group(
            &self.device,
            &self.region_blur_bind_group_layout,
            &source_view,
            &self.sampler,
            &self.region_blur_uniform,
            &self.blur_weights,
        );
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("liquid-glass vertical gradient blur encoder"),
        });
        let scissor = (x, y, width, height);
        // Preserve the unblurred output for the final mix. This prevents
        // transparent or not-yet-captured desktop pixels from turning into a
        // black rectangle when the gradient is applied.
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.targets.output,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.targets.scene,
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
        encode_scissored_fullscreen_pass(
            &mut encoder,
            "liquid-glass vertical gradient horizontal pass",
            &self.targets.blur_horizontal_view,
            &self.region_blur_horizontal_pipeline,
            &self.fullscreen_vertex_buffer,
            Some(&source_bind_group),
            None,
            wgpu::Color::BLACK,
            scissor,
        );
        encode_scissored_fullscreen_pass(
            &mut encoder,
            "liquid-glass vertical gradient vertical pass",
            &self.targets.blur_vertical_view,
            &self.region_blur_vertical_pipeline,
            &self.fullscreen_vertex_buffer,
            Some(&self.region_blur_vertical_bind_group),
            None,
            wgpu::Color::BLACK,
            scissor,
        );
        encode_scissored_fullscreen_pass(
            &mut encoder,
            "liquid-glass vertical gradient composite pass",
            &self.targets.output_view,
            &self.gradient_composite_pipeline,
            &self.fullscreen_vertex_buffer,
            Some(&self.gradient_composite_bind_group),
            None,
            wgpu::Color::TRANSPARENT,
            scissor,
        );
        self.queue.submit([encoder.finish()]);
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
        source_texture: Option<&wgpu::Texture>,
        simple_blur: Option<SimpleBlurRegion>,
        options: GlassRenderOptions,
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
                source_texture.is_some() || index > 0,
                options,
            );
            self.queue.write_buffer(&self.glass_uniform, offset, bytemuck::bytes_of(&uniform));
        }
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("liquid-glass frame encoder"),
        });
        if let Some(source_texture) = source_texture {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: source_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &self.targets.scene,
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
        } else {
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
        }

        let mut source = PingPongSource::Scene;
        if let Some(simple_blur) = simple_blur {
            let flat_blur_weight_offset =
                MAX_GLASS_NODES * (MAX_BLUR_RADIUS + 1) * std::mem::size_of::<f32>();
            let x = simple_blur.bounds.0.min(self.size.width);
            let y = simple_blur.bounds.1.min(self.size.height);
            let width = simple_blur.bounds.2.min(self.size.width.saturating_sub(x));
            let height = simple_blur.bounds.3.min(self.size.height.saturating_sub(y));
            if width > 0 && height > 0 {
                let blur_uniform = RegionBlurUniform {
                    resolution: gpu_size_as_f32(self.size),
                    radius: simple_blur.radius.min(MAX_BLUR_RADIUS as u32) as i32,
                    weight_offset: i32::try_from(
                        flat_blur_weight_offset / std::mem::size_of::<f32>(),
                    )
                    .expect("flat blur weight offset fits in i32"),
                    region: [x as f32, y as f32, width as f32, height as f32],
                    gradient: [0.0; 4],
                };
                self.queue.write_buffer(
                    &self.region_blur_uniform,
                    0,
                    bytemuck::bytes_of(&blur_uniform),
                );
                let blur_weights = gaussian_weights(blur_uniform.radius);
                self.queue.write_buffer(
                    &self.blur_weights,
                    u64::try_from(flat_blur_weight_offset)
                        .expect("flat blur weight offset fits in u64"),
                    bytemuck::cast_slice(&blur_weights),
                );
                self.queue.write_buffer(
                    &self.flat_blur_uniform,
                    0,
                    bytemuck::bytes_of(&FlatBlurUniform {
                        tint: srgb_to_linear_rgba(simple_blur.tint),
                    }),
                );
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
                encode_scissored_fullscreen_pass(
                    &mut encoder,
                    "liquid-glass bounded blur horizontal pass",
                    &self.targets.blur_horizontal_view,
                    &self.region_blur_horizontal_pipeline,
                    &self.fullscreen_vertex_buffer,
                    Some(&self.region_blur_horizontal_bind_group),
                    None,
                    wgpu::Color::BLACK,
                    (x, y, width, height),
                );
                encode_scissored_fullscreen_pass(
                    &mut encoder,
                    "liquid-glass bounded blur vertical pass",
                    &self.targets.blur_vertical_view,
                    &self.region_blur_vertical_pipeline,
                    &self.fullscreen_vertex_buffer,
                    Some(&self.region_blur_vertical_bind_group),
                    None,
                    wgpu::Color::BLACK,
                    (x, y, width, height),
                );
                encode_scissored_fullscreen_pass(
                    &mut encoder,
                    "liquid-glass flat blur tint pass",
                    &self.targets.output_view,
                    &self.flat_blur_pipeline,
                    &self.fullscreen_vertex_buffer,
                    Some(&self.flat_blur_bind_group),
                    None,
                    wgpu::Color::TRANSPARENT,
                    (x, y, width, height),
                );
                source = PingPongSource::Output;
            }
        }
        for (index, _) in nodes.iter().enumerate() {
            let blur_radius = blur_radius_for_node(nodes[index], options);
            // The shape remains bounded by the node's SDF, but the draw pass
            // must extend beyond it so cast shadows and edge light are not
            // clipped at the interactive control rectangle.
            let Some(effect_region) = node_effect_region(nodes[index], self.size) else {
                continue;
            };
            let Some(blur_region) = node_render_region(nodes[index], self.size, blur_radius) else {
                continue;
            };
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
                source_view,
                destination_texture,
                destination_view,
                shadow_bind_group,
                blur_bind_group,
                glass_bind_group,
            ) = match source {
                PingPongSource::Scene => (
                    &self.targets.scene,
                    &self.targets.scene_view,
                    &self.targets.output,
                    &self.targets.output_view,
                    &self.glass_from_scene_bind_group,
                    &self.blur_horizontal_from_output_bind_group,
                    &self.glass_from_output_bind_group,
                ),
                PingPongSource::Output => (
                    &self.targets.output,
                    &self.targets.output_view,
                    &self.targets.scene,
                    &self.targets.scene_view,
                    &self.glass_from_output_bind_group,
                    &self.blur_horizontal_from_scene_bind_group,
                    &self.glass_from_scene_bind_group,
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
            encode_scissored_fullscreen_pass(
                &mut encoder,
                "liquid-glass hierarchical horizontal blur pass",
                &self.targets.blur_horizontal_view,
                &self.blur_horizontal_pipeline,
                &self.fullscreen_vertex_buffer,
                Some(blur_bind_group),
                Some(uniform_offset),
                wgpu::Color::BLACK,
                blur_region,
            );
            encode_scissored_fullscreen_pass(
                &mut encoder,
                "liquid-glass hierarchical vertical blur pass",
                &self.targets.blur_vertical_view,
                &self.blur_vertical_pipeline,
                &self.fullscreen_vertex_buffer,
                Some(&self.blur_vertical_bind_group),
                Some(uniform_offset),
                wgpu::Color::BLACK,
                blur_region,
            );
            // Build the glass from the clean backdrop first. Applying the
            // shadow before this point would make the blur sample the shadow
            // and bleed it back into the material as an artificial inner
            // dark band.
            encode_glass_node_pass(
                &mut encoder,
                source_view,
                &self.glass_pipeline,
                glass_bind_group,
                &self.fullscreen_vertex_buffer,
                uniform_offset,
                effect_region,
            );
            // The remaining shadow is a faint SDF-derived elevation tail on
            // the finished composite. It is deliberately kept out of the
            // backdrop blur and refraction inputs, so it cannot look like a
            // second translucent surface inside the control.
            encode_glass_node_pass(
                &mut encoder,
                destination_view,
                &self.shadow_pipeline,
                shadow_bind_group,
                &self.fullscreen_vertex_buffer,
                uniform_offset,
                effect_region,
            );
            source = match source {
                PingPongSource::Scene => PingPongSource::Output,
                PingPongSource::Output => PingPongSource::Scene,
            };
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
    encode_fullscreen_pass_with_load(
        encoder,
        label,
        target,
        pipeline,
        vertex_buffer,
        bind_group,
        dynamic_offset,
        wgpu::LoadOp::Clear(clear),
    );
}

fn encode_fullscreen_load_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    bind_group: &wgpu::BindGroup,
) {
    encode_fullscreen_pass_with_load(
        encoder,
        label,
        target,
        pipeline,
        vertex_buffer,
        Some(bind_group),
        None,
        wgpu::LoadOp::Load,
    );
}

#[allow(clippy::too_many_arguments)]
fn encode_fullscreen_pass_with_load(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    bind_group: Option<&wgpu::BindGroup>,
    dynamic_offset: Option<u32>,
    load: wgpu::LoadOp<wgpu::Color>,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
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

#[allow(clippy::too_many_arguments)]
fn encode_scissored_fullscreen_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    bind_group: Option<&wgpu::BindGroup>,
    dynamic_offset: Option<u32>,
    clear_color: wgpu::Color,
    region: (u32, u32, u32, u32),
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
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
    pass.set_scissor_rect(region.0, region.1, region.2.max(1), region.3.max(1));
    if let Some(bind_group) = bind_group {
        if let Some(dynamic_offset) = dynamic_offset {
            pass.set_bind_group(0, bind_group, &[dynamic_offset]);
        } else {
            pass.set_bind_group(0, bind_group, &[]);
        }
    }
    let _ = clear_color;
    pass.draw(0..4, 0..1);
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SimpleBlurRegion {
    bounds: (u32, u32, u32, u32),
    radius: u32,
    tint: [f32; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PingPongSource {
    Scene,
    Output,
}

fn encode_glass_node_pass(
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    vertex_buffer: &wgpu::Buffer,
    uniform_offset: u32,
    region: (u32, u32, u32, u32),
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
    pass.set_scissor_rect(region.0, region.1, region.2.max(1), region.3.max(1));
    pass.set_bind_group(0, bind_group, &[uniform_offset]);
    pass.draw(0..4, 0..1);
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn node_effect_region(node: &GlassNode, size: GpuSize) -> Option<(u32, u32, u32, u32)> {
    let shadow = node.material.shadow;
    let padding = shadow.expand.max(0.0) + shadow.offset[0].abs().max(shadow.offset[1].abs()) + 4.0;
    let bounds = node_optical_bounds(node, size);
    let left = (bounds.x - padding).max(0.0).floor() as u32;
    let top = (bounds.y - padding).max(0.0).floor() as u32;
    let right = (bounds.x + bounds.width + padding).max(0.0).ceil() as u32;
    let bottom = (bounds.y + bounds.height + padding).max(0.0).ceil() as u32;
    let left = left.min(size.width);
    let top = top.min(size.height);
    let right = right.min(size.width);
    let bottom = bottom.min(size.height);
    let width = right.saturating_sub(left);
    let height = bottom.saturating_sub(top);
    (width > 0 && height > 0).then_some((left, top, width, height))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn node_render_region(
    node: &GlassNode,
    size: GpuSize,
    blur_radius: i32,
) -> Option<(u32, u32, u32, u32)> {
    let padding = blur_radius.max(0) as f32 + node.backdrop.padding.max(0.0) + 4.0;
    let bounds = node_optical_bounds(node, size);
    let left = (bounds.x - padding).max(0.0).floor() as u32;
    let top = (bounds.y - padding).max(0.0).floor() as u32;
    let right = (bounds.x + bounds.width + padding).max(0.0).ceil() as u32;
    let bottom = (bounds.y + bounds.height + padding).max(0.0).ceil() as u32;
    let left = left.min(size.width);
    let top = top.min(size.height);
    let right = right.min(size.width);
    let bottom = bottom.min(size.height);
    let width = right.saturating_sub(left);
    let height = bottom.saturating_sub(top);
    (width > 0 && height > 0).then_some((left, top, width, height))
}

/// Bounds every shape evaluated by `mainSDF`, including the fixed 200px
/// reference circle used by the source fusion demo. Scissoring only to the
/// primary node used to clip most of that circle and leave a dark sliver at
/// the merge neck, which made the enhanced path appear to have lost fusion.
#[allow(clippy::cast_precision_loss)]
fn node_optical_bounds(node: &GlassNode, size: GpuSize) -> Rect {
    let bounds = node.visual_bounds();
    if !node.material.show_shape1 {
        return bounds;
    }

    let reference_circle =
        Rect::new(size.width as f32 * 0.5 - 100.0, size.height as f32 * 0.5 - 100.0, 200.0, 200.0);
    let left = bounds.x.min(reference_circle.x);
    let top = bounds.y.min(reference_circle.y);
    let right = (bounds.x + bounds.width).max(reference_circle.x + reference_circle.width);
    let bottom = (bounds.y + bounds.height).max(reference_circle.y + reference_circle.height);
    Rect::new(left, top, right - left, bottom - top)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
fn uniform_for_node(
    size: GpuSize,
    node: &GlassNode,
    _time_seconds: f32,
    background_texture_ready: bool,
    background_texture_ratio: f32,
    transparent_background: bool,
    has_composited_source: bool,
    options: GlassRenderOptions,
) -> GlassUniform {
    let center_x = node.bounds.x + node.bounds.width * 0.5;
    let center_y = size.height as f32 - node.bounds.y - node.bounds.height * 0.5;
    let material = node.material;
    let accessibility = options.accessibility;
    let size_gain = size_response(node, material.adaptive.size);
    let mut feature_flags = 0;
    if material.blur.edge_blur {
        feature_flags |= FEATURE_EDGE_BLUR;
    }
    if accessibility.reduced_transparency {
        feature_flags |= FEATURE_REDUCED_TRANSPARENCY;
    }
    if accessibility.increased_contrast {
        feature_flags |= FEATURE_INCREASED_CONTRAST;
    }
    if accessibility.reduced_motion {
        feature_flags |= FEATURE_REDUCED_MOTION;
    }
    if material.variant == GlassVariant::Clear {
        feature_flags |= FEATURE_CLEAR_VARIANT;
    }
    let interaction = if accessibility.reduced_motion { 0.0 } else { node.interaction.strength() };
    let pointer_x = node.bounds.x + node.bounds.width * node.interaction.pointer[0];
    let pointer_y =
        size.height as f32 - node.bounds.y - node.bounds.height * node.interaction.pointer[1];
    let spring_pointer = if accessibility.reduced_motion {
        node.interaction.pointer
    } else {
        node.interaction.spring
    };
    let spring_x = node.bounds.x + node.bounds.width * spring_pointer[0];
    let spring_y = size.height as f32 - node.bounds.y - node.bounds.height * spring_pointer[1];
    let variant_factor = if material.variant == GlassVariant::Clear { 0.78 } else { 1.0 };
    let refraction_strength = if accessibility.reduced_transparency {
        0.0
    } else {
        material.refraction.strength * size_gain * variant_factor
    };
    let dispersion_strength = if accessibility.reduced_transparency {
        0.0
    } else {
        material.dispersion.strength * variant_factor
    };
    let fresnel_strength = if accessibility.reduced_transparency {
        material.fresnel.strength * 0.35
    } else {
        material.fresnel.strength * variant_factor
    };
    let blur_radius = blur_radius_for_node(node, options);
    let tint = tint_with_whiteness(material);
    let shape_radius = shape_radius(node);
    let shape_roundness = shape_roundness(node);
    let (capsule_bezier_x, capsule_bezier_y) = capsule_bezier_uniforms(node);
    let (fused_bounds, fused_geometry) = fused_shape_uniforms(size, node);
    GlassUniform {
        resolution_dpr_pad: [size.width as f32, size.height as f32, 1.0, 0.0],
        mouse_and_spring: [pointer_x, pointer_y, center_x, center_y],
        shape: [node.bounds.width, node.bounds.height, shape_radius, shape_roundness],
        capsule_bezier_x,
        capsule_bezier_y,
        merge_glare_shadow: [
            material.merge_rate.max(f32::EPSILON),
            // Apple-style system glass uses a stable vertical light field:
            // the upper and lower edges catch light while the sides remain
            // subdued. Do not rotate the glare with frame time.
            0.0,
            material.shadow.expand.max(1.0),
            (material.shadow.factor * material.adaptive.shadow * size_gain * variant_factor)
                .clamp(0.0, 0.6),
        ],
        shadow_position_bg_ratio: [
            material.shadow.offset[0],
            material.shadow.offset[1],
            background_texture_ratio,
        ],
        bg_type: if transparent_background {
            if has_composited_source { 13 } else { 12 }
        } else if background_texture_ready {
            11
        } else {
            0
        },
        flags: [
            i32::from(background_texture_ready),
            i32::from(material.show_shape1),
            blur_radius,
            feature_flags,
        ],
        tint,
        refraction_and_fresnel: [
            (material.refraction.thickness * 100.0).max(1.0),
            material.refraction.index,
            (dispersion_strength * 100.0).max(0.0),
            (material.fresnel.range * 40.0).max(1.0),
            material.fresnel.hardness,
            fresnel_strength,
        ],
        glare: [
            material.glare.range,
            material.glare.hardness,
            material.glare.convergence,
            material.glare.opposite_factor,
            material.glare.factor,
        ],
        // The final five floats are kept as a 16-byte-aligned tail in the
        // uniform. Use the first two instead of dropping refraction.strength:
        // the reference shader's offset is otherwise effectively hard-coded
        // and small controls look like plain translucent pills.
        _pad: [
            refraction_strength,
            if accessibility.reduced_transparency {
                material.opacity.max(0.92)
            } else {
                material.opacity
            },
            interaction,
            options.environment.luminance.clamp(0.0, 1.0),
            options.environment.contrast.clamp(0.0, 1.0),
        ],
        adaptive: [
            material.adaptive.tint.clamp(0.0, 2.0),
            material.adaptive.ambient.clamp(0.0, 2.0),
            material.adaptive.shadow.clamp(0.0, 2.0),
            material.adaptive.size.clamp(0.0, 2.0),
        ],
        fused_bounds,
        fused_geometry,
        interaction_state: [spring_x, spring_y, node.interaction.parallax, node.interaction.focus],
    }
}

#[allow(clippy::cast_precision_loss)]
fn fused_shape_uniforms(size: GpuSize, node: &GlassNode) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
    let mut bounds = [[0.0; 4]; 4];
    let mut geometry = [[0.0; 4]; 4];
    for (index, fused_shape) in node.fused_shapes.iter().take(4).enumerate() {
        let center_x = fused_shape.bounds.x + fused_shape.bounds.width * 0.5;
        let center_y = size.height as f32 - fused_shape.bounds.y - fused_shape.bounds.height * 0.5;
        bounds[index] = [center_x, center_y, fused_shape.bounds.width, fused_shape.bounds.height];
        geometry[index] =
            [shape_radius_for_layer(fused_shape), shape_roundness_for_layer(fused_shape), 1.0, 0.0];
    }
    (bounds, geometry)
}

fn shape_radius_for_layer(layer: &liquid_glass_scene::GlassShapeLayer) -> f32 {
    match layer.shape {
        GlassShape::RoundedRect { radius } => radius,
        GlassShape::Superellipse { .. } => layer.bounds.width.min(layer.bounds.height) * 0.4,
        GlassShape::Capsule => layer.bounds.height * 0.5,
        GlassShape::Circle => layer.bounds.width.min(layer.bounds.height) * 0.5,
        GlassShape::Ellipse => layer.bounds.width.min(layer.bounds.height) * 0.25,
    }
}

fn shape_roundness_for_layer(layer: &liquid_glass_scene::GlassShapeLayer) -> f32 {
    match layer.shape {
        GlassShape::Superellipse { exponent } => exponent,
        GlassShape::RoundedRect { .. } => layer.corner_curve.exponent(),
        // The fused path currently shares the rounded-rectangle evaluator;
        // use a continuous circular curve for capsule-like secondary shapes.
        GlassShape::Capsule | GlassShape::Circle | GlassShape::Ellipse => 2.0,
    }
}

fn tint_with_whiteness(material: liquid_glass_scene::GlassMaterial) -> [f32; 4] {
    let tint_alpha = material.tint.a.clamp(0.0, 1.0);
    let white_alpha = material.whiteness.clamp(0.0, 1.0);
    let combined_alpha = 1.0 - (1.0 - tint_alpha) * (1.0 - white_alpha);
    let tint =
        srgb_to_linear_rgba([material.tint.r, material.tint.g, material.tint.b, material.tint.a]);
    if combined_alpha <= f32::EPSILON {
        return [tint[0], tint[1], tint[2], 0.0];
    }

    let tint_weight = tint_alpha * (1.0 - white_alpha);
    [
        (tint[0] * tint_weight + white_alpha) / combined_alpha,
        (tint[1] * tint_weight + white_alpha) / combined_alpha,
        (tint[2] * tint_weight + white_alpha) / combined_alpha,
        combined_alpha,
    ]
}

fn srgb_to_linear_rgba(color: [f32; 4]) -> [f32; 4] {
    [
        srgb_channel_to_linear(color[0]),
        srgb_channel_to_linear(color[1]),
        srgb_channel_to_linear(color[2]),
        color[3].clamp(0.0, 1.0),
    ]
}

fn srgb_channel_to_linear(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
}

#[allow(clippy::cast_precision_loss)]
fn gpu_size_as_f32(size: GpuSize) -> [f32; 2] {
    [size.width as f32, size.height as f32]
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss)]
fn blur_radius_for_node(node: &GlassNode, options: GlassRenderOptions) -> i32 {
    let size_gain = size_response(node, node.material.adaptive.size);
    let accessibility_gain = if options.accessibility.reduced_transparency { 1.35 } else { 1.0 };
    let variant_gain = if node.material.variant == GlassVariant::Clear { 0.82 } else { 1.0 };
    (node.material.blur.radius * size_gain * accessibility_gain * variant_gain)
        .round()
        .clamp(0.0, MAX_BLUR_RADIUS as f32) as i32
}

#[allow(clippy::cast_precision_loss)]
fn size_response(node: &GlassNode, gain: f32) -> f32 {
    let minimum_dimension = node.bounds.width.min(node.bounds.height).max(1.0);
    let normalized = (minimum_dimension / 48.0).sqrt().clamp(0.65, 1.45);
    1.0 + (normalized - 1.0) * gain.clamp(0.0, 2.0)
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

fn create_flat_blur_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("liquid-glass flat blur layout"),
        entries: &[
            texture_binding(0),
            sampler_binding(1),
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        u64::try_from(std::mem::size_of::<FlatBlurUniform>())
                            .expect("flat blur uniform size fits in u64"),
                    ),
                },
                count: None,
            },
        ],
    })
}

fn create_flat_blur_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    blurred_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("liquid-glass flat blur bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(blurred_view),
            },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniform,
                    offset: 0,
                    size: Some(
                        wgpu::BufferSize::new(
                            u64::try_from(std::mem::size_of::<FlatBlurUniform>())
                                .expect("flat blur uniform size fits in u64"),
                        )
                        .expect("flat blur uniform size is non-zero"),
                    ),
                }),
            },
        ],
    })
}

fn create_gradient_composite_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("liquid-glass gradient composite layout"),
        entries: &[
            texture_binding(0),
            texture_binding(1),
            sampler_binding(2),
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        u64::try_from(std::mem::size_of::<GradientCompositeUniform>())
                            .expect("gradient composite uniform size fits in u64"),
                    ),
                },
                count: None,
            },
        ],
    })
}

fn create_gradient_composite_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    blurred_view: &wgpu::TextureView,
    original_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("liquid-glass gradient composite bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(blurred_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(original_view),
            },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniform,
                    offset: 0,
                    size: Some(
                        wgpu::BufferSize::new(
                            u64::try_from(std::mem::size_of::<GradientCompositeUniform>())
                                .expect("gradient composite uniform size fits in u64"),
                        )
                        .expect("gradient composite uniform size is non-zero"),
                    ),
                }),
            },
        ],
    })
}

fn create_region_blur_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("liquid-glass bounded blur layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        u64::try_from(std::mem::size_of::<RegionBlurUniform>())
                            .expect("region blur uniform size fits in u64"),
                    ),
                },
                count: None,
            },
            texture_binding(1),
            sampler_binding(2),
            storage_binding(3),
        ],
    })
}

fn create_region_blur_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source_view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
    blur_weights: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("liquid-glass bounded blur bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: uniform,
                    offset: 0,
                    size: Some(
                        wgpu::BufferSize::new(
                            u64::try_from(std::mem::size_of::<RegionBlurUniform>())
                                .expect("region blur uniform size fits in u64"),
                        )
                        .expect("region blur uniform size is non-zero"),
                    ),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(source_view),
            },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: blur_weights,
                    offset: 0,
                    size: None,
                }),
            },
        ],
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
    blend: Option<wgpu::BlendState>,
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
        blend,
        output_format,
    )
}

fn create_flat_blur_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("liquid-glass flat blur tint shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(format!(
            "{}\n{}",
            include_str!("../../../shaders/glass/reference/vertex.wgsl"),
            include_str!("../../../shaders/glass/flat-blur.wgsl"),
        ))),
    });
    create_pipeline(
        device,
        "liquid-glass flat blur tint pipeline",
        &shader,
        Some(bind_group_layout),
        "fs_main",
        None,
        output_format,
    )
}

fn create_gradient_composite_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("liquid-glass gradient composite shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(format!(
            "{}\n{}",
            include_str!("../../../shaders/glass/reference/vertex.wgsl"),
            include_str!("../../../shaders/glass/gradient-composite.wgsl"),
        ))),
    });
    create_pipeline(
        device,
        "liquid-glass gradient composite pipeline",
        &shader,
        Some(bind_group_layout),
        "fs_main",
        None,
        output_format,
    )
}

fn create_region_blur_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    output_format: wgpu::TextureFormat,
    horizontal: bool,
) -> wgpu::RenderPipeline {
    let shader_name =
        if horizontal { "flat-blur-horizontal.wgsl" } else { "flat-blur-vertical.wgsl" };
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("liquid-glass bounded blur shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(format!(
            "{}\n{}",
            include_str!("../../../shaders/glass/reference/vertex.wgsl"),
            if horizontal {
                include_str!("../../../shaders/glass/flat-blur-horizontal.wgsl")
            } else {
                include_str!("../../../shaders/glass/flat-blur-vertical.wgsl")
            },
        ))),
    });
    create_pipeline(
        device,
        &format!("liquid-glass bounded blur {shader_name}"),
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
) -> (
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
    wgpu::RenderPipeline,
) {
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
        None,
        output_format,
    );
    let shadow = create_pipeline(
        device,
        "liquid-glass analytic shadow pipeline",
        &glass_shader,
        Some(bind_group_layout),
        "fs_shadow",
        None,
        output_format,
    );

    (background, horizontal, vertical, shadow, glass)
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
        assert_eq!(std::mem::size_of::<GlassUniform>(), 368);
    }

    #[test]
    fn reference_fusion_circle_is_included_in_scissor_bounds() {
        let mut material = GlassMaterial::clear();
        material.show_shape1 = true;
        let node =
            GlassNode::new(GlassId(4), Rect::new(390.0, 220.0, 200.0, 200.0)).material(material);

        let bounds = node_optical_bounds(&node, GpuSize::new(640, 640));

        assert_eq!(bounds, Rect::new(220.0, 220.0, 370.0, 200.0));
    }

    #[test]
    fn whiteness_adds_a_neutral_layer_above_tint() {
        let mut material = GlassMaterial::clear();
        material.tint = liquid_glass_scene::Color::rgba(0.2, 0.4, 0.8, 0.10);
        material.whiteness = 0.20;

        let effective = tint_with_whiteness(material);

        assert!((effective[3] - 0.28).abs() < f32::EPSILON);
        let tint = srgb_to_linear_rgba([
            material.tint.r,
            material.tint.g,
            material.tint.b,
            material.tint.a,
        ]);
        assert!(effective[0] > tint[0]);
        assert!(effective[1] > tint[1]);
        assert!(effective[2] > tint[2]);
    }

    #[test]
    fn srgb_uniform_colors_are_decoded_to_linear_light() {
        let linear = srgb_to_linear_rgba([0.5, 0.5, 0.5, 0.4]);

        assert!((linear[0] - 0.214_041_14).abs() < 0.000_01);
        assert_eq!(linear[3], 0.4);
    }
}
