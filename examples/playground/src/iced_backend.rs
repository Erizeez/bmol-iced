use std::{fmt, time::Instant};

use iced_wgpu::{Engine, Renderer as IcedRenderer, graphics, wgpu};
use liquid_glass::{
    Color as GlassColor, GlassId, GlassMaterial, GlassNode, GlassScene, GlassShape, GpuRenderer,
    GpuSize, Rect,
};

pub struct Renderer {
    inner: IcedRenderer,
}

impl fmt::Debug for Renderer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("LiquidIcedRenderer").finish_non_exhaustive()
    }
}

impl Renderer {
    fn new(inner: IcedRenderer) -> Self {
        Self { inner }
    }
}

impl iced::advanced::Renderer for Renderer {
    fn start_layer(&mut self, bounds: iced::Rectangle) {
        self.inner.start_layer(bounds);
    }

    fn end_layer(&mut self) {
        self.inner.end_layer();
    }

    fn start_transformation(&mut self, transformation: iced::Transformation) {
        self.inner.start_transformation(transformation);
    }

    fn end_transformation(&mut self) {
        self.inner.end_transformation();
    }

    fn fill_quad(
        &mut self,
        quad: iced::advanced::renderer::Quad,
        background: impl Into<iced::Background>,
    ) {
        self.inner.fill_quad(quad, background);
    }

    fn reset(&mut self, new_bounds: iced::Rectangle) {
        self.inner.reset(new_bounds);
    }

    fn allocate_image(
        &mut self,
        handle: &iced::advanced::image::Handle,
        callback: impl FnOnce(Result<iced::advanced::image::Allocation, iced::advanced::image::Error>)
        + Send
        + 'static,
    ) {
        self.inner.allocate_image(handle, callback);
    }
}

impl iced::advanced::text::Renderer for Renderer {
    type Font = <IcedRenderer as iced::advanced::text::Renderer>::Font;
    type Paragraph = <IcedRenderer as iced::advanced::text::Renderer>::Paragraph;
    type Editor = <IcedRenderer as iced::advanced::text::Renderer>::Editor;

    const ICON_FONT: Self::Font = <IcedRenderer as iced::advanced::text::Renderer>::ICON_FONT;
    const CHECKMARK_ICON: char = <IcedRenderer as iced::advanced::text::Renderer>::CHECKMARK_ICON;
    const ARROW_DOWN_ICON: char = <IcedRenderer as iced::advanced::text::Renderer>::ARROW_DOWN_ICON;
    const SCROLL_UP_ICON: char = <IcedRenderer as iced::advanced::text::Renderer>::SCROLL_UP_ICON;
    const SCROLL_DOWN_ICON: char =
        <IcedRenderer as iced::advanced::text::Renderer>::SCROLL_DOWN_ICON;
    const SCROLL_LEFT_ICON: char =
        <IcedRenderer as iced::advanced::text::Renderer>::SCROLL_LEFT_ICON;
    const SCROLL_RIGHT_ICON: char =
        <IcedRenderer as iced::advanced::text::Renderer>::SCROLL_RIGHT_ICON;
    const ICED_LOGO: char = <IcedRenderer as iced::advanced::text::Renderer>::ICED_LOGO;

    fn default_font(&self) -> Self::Font {
        self.inner.default_font()
    }

    fn default_size(&self) -> iced::Pixels {
        self.inner.default_size()
    }

    fn fill_paragraph(
        &mut self,
        text: &Self::Paragraph,
        position: iced::Point,
        color: iced::Color,
        clip_bounds: iced::Rectangle,
    ) {
        self.inner.fill_paragraph(text, position, color, clip_bounds);
    }

    fn fill_editor(
        &mut self,
        editor: &Self::Editor,
        position: iced::Point,
        color: iced::Color,
        clip_bounds: iced::Rectangle,
    ) {
        self.inner.fill_editor(editor, position, color, clip_bounds);
    }

    fn fill_text(
        &mut self,
        text: iced::advanced::text::Text<String, Self::Font>,
        position: iced::Point,
        color: iced::Color,
        clip_bounds: iced::Rectangle,
    ) {
        self.inner.fill_text(text, position, color, clip_bounds);
    }
}

impl graphics::text::Renderer for Renderer {
    fn fill_raw(&mut self, raw: graphics::text::Raw) {
        self.inner.fill_raw(raw);
    }
}

impl graphics::mesh::Renderer for Renderer {
    fn draw_mesh(&mut self, mesh: graphics::Mesh) {
        self.inner.draw_mesh(mesh);
    }

    fn draw_mesh_cache(&mut self, cache: graphics::mesh::Cache) {
        self.inner.draw_mesh_cache(cache);
    }
}

impl graphics::geometry::Renderer for Renderer {
    type Geometry = <IcedRenderer as graphics::geometry::Renderer>::Geometry;
    type Frame = <IcedRenderer as graphics::geometry::Renderer>::Frame;

    fn new_frame(&self, bounds: iced::Rectangle) -> Self::Frame {
        self.inner.new_frame(bounds)
    }

    fn draw_geometry(&mut self, geometry: Self::Geometry) {
        self.inner.draw_geometry(geometry);
    }
}

impl iced_wgpu::primitive::Renderer for Renderer {
    fn draw_primitive(
        &mut self,
        bounds: iced::Rectangle,
        primitive: impl iced_wgpu::primitive::Primitive,
    ) {
        self.inner.draw_primitive(bounds, primitive);
    }
}

impl iced::advanced::renderer::Headless for Renderer {
    async fn new(
        default_font: iced::Font,
        default_text_size: iced::Pixels,
        backend: Option<&str>,
    ) -> Option<Self> {
        <IcedRenderer as iced::advanced::renderer::Headless>::new(
            default_font,
            default_text_size,
            backend,
        )
        .await
        .map(Self::new)
    }

    fn name(&self) -> String {
        self.inner.name()
    }

    fn screenshot(
        &mut self,
        size: iced::Size<u32>,
        scale_factor: f32,
        background_color: iced::Color,
    ) -> Vec<u8> {
        let viewport = graphics::Viewport::with_physical_size(size, scale_factor);
        self.inner.screenshot(&viewport, background_color)
    }
}

pub struct Compositor {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    format: wgpu::TextureFormat,
    alpha_mode: wgpu::CompositeAlphaMode,
    engine: Engine,
    settings: iced_wgpu::Settings,
    device: wgpu::Device,
    liquid: GpuRenderer,
    started_at: Instant,
}

impl fmt::Debug for Compositor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("LiquidIcedCompositor").finish_non_exhaustive()
    }
}

impl graphics::compositor::Default for Renderer {
    type Compositor = Compositor;
}

impl graphics::Compositor for Compositor {
    type Renderer = Renderer;
    type Surface = wgpu::Surface<'static>;

    async fn with_backend(
        settings: graphics::Settings,
        _display: impl graphics::compositor::Display,
        compatible_window: impl graphics::compositor::Window + Clone,
        shell: graphics::Shell,
        backend: Option<&str>,
    ) -> Result<Self, graphics::Error> {
        if backend.is_some_and(|backend| backend != "wgpu") {
            return Err(graphics::Error::GraphicsAdapterNotFound {
                backend: "wgpu",
                reason: graphics::error::Reason::DidNotMatch {
                    preferred_backend: backend.unwrap_or_default().to_owned(),
                },
            });
        }

        let settings = iced_wgpu::Settings::from(settings);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: settings.backends,
            ..Default::default()
        });
        let surface = instance.create_surface(compatible_window).map_err(|error| {
            graphics::Error::GraphicsAdapterNotFound {
                backend: "wgpu",
                reason: graphics::error::Reason::RequestFailed(error.to_string()),
            }
        })?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|error| graphics::Error::GraphicsAdapterNotFound {
                backend: "wgpu",
                reason: graphics::error::Reason::RequestFailed(error.to_string()),
            })?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or_else(|| graphics::Error::GraphicsAdapterNotFound {
                backend: "wgpu",
                reason: graphics::error::Reason::RequestFailed("surface has no formats".to_owned()),
            })?;
        let alpha_mode =
            capabilities.alpha_modes.first().copied().unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("liquid-glass iced compositor device"),
                ..Default::default()
            })
            .await
            .map_err(|error| graphics::Error::GraphicsAdapterNotFound {
                backend: "wgpu",
                reason: graphics::error::Reason::RequestFailed(error.to_string()),
            })?;
        let liquid = GpuRenderer::from_device_with_format(
            device.clone(),
            queue.clone(),
            GpuSize::new(1, 1),
            format,
        );
        let engine =
            Engine::new(&adapter, device.clone(), queue, format, settings.antialiasing, shell);

        Ok(Self {
            instance,
            adapter,
            format,
            alpha_mode,
            engine,
            settings,
            device,
            liquid,
            started_at: Instant::now(),
        })
    }

    fn create_renderer(&self) -> Self::Renderer {
        Renderer::new(IcedRenderer::new(
            self.engine.clone(),
            self.settings.default_font,
            self.settings.default_text_size,
        ))
    }

    fn create_surface<W: graphics::compositor::Window + Clone>(
        &mut self,
        window: W,
        width: u32,
        height: u32,
    ) -> Self::Surface {
        let mut surface = self.instance.create_surface(window).expect("create Iced surface");
        if width > 0 && height > 0 {
            self.configure_surface(&mut surface, width, height);
        }
        surface
    }

    fn configure_surface(&mut self, surface: &mut Self::Surface, width: u32, height: u32) {
        surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                present_mode: self.settings.present_mode,
                width,
                height,
                alpha_mode: self.alpha_mode,
                view_formats: vec![],
                desired_maximum_frame_latency: 1,
            },
        );
    }

    fn information(&self) -> graphics::compositor::Information {
        let info = self.adapter.get_info();
        graphics::compositor::Information {
            adapter: info.name,
            backend: format!("{:?}", info.backend),
        }
    }

    fn present(
        &mut self,
        renderer: &mut Self::Renderer,
        surface: &mut Self::Surface,
        viewport: &graphics::Viewport,
        background_color: iced::Color,
        on_pre_present: impl FnOnce(),
    ) -> Result<(), graphics::compositor::SurfaceError> {
        let frame = surface.get_current_texture().map_err(|error| map_surface_error(&error))?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let physical = viewport.physical_size();
        let size = GpuSize::new(physical.width.max(1), physical.height.max(1));
        if self.liquid.size() != size {
            self.liquid.resize(size).map_err(|_| graphics::compositor::SurfaceError::Other)?;
        }
        let scene = scene_for_size(size);
        self.liquid
            .render_scene_to_view(&view, &scene, self.started_at.elapsed().as_secs_f32())
            .map_err(|_| graphics::compositor::SurfaceError::Other)?;
        renderer.inner.present(None, frame.texture.format(), &view, viewport);
        on_pre_present();
        frame.present();
        let _ = background_color;
        Ok(())
    }

    fn screenshot(
        &mut self,
        renderer: &mut Self::Renderer,
        viewport: &graphics::Viewport,
        background_color: iced::Color,
    ) -> Vec<u8> {
        renderer.inner.screenshot(viewport, background_color)
    }
}

fn map_surface_error(error: &wgpu::SurfaceError) -> graphics::compositor::SurfaceError {
    match error {
        wgpu::SurfaceError::Timeout => graphics::compositor::SurfaceError::Timeout,
        wgpu::SurfaceError::Outdated => graphics::compositor::SurfaceError::Outdated,
        wgpu::SurfaceError::Lost => graphics::compositor::SurfaceError::Lost,
        wgpu::SurfaceError::OutOfMemory => graphics::compositor::SurfaceError::OutOfMemory,
        wgpu::SurfaceError::Other => graphics::compositor::SurfaceError::Other,
    }
}

#[allow(clippy::cast_precision_loss)]
fn scene_for_size(size: GpuSize) -> GlassScene {
    let width = size.width as f32;
    let height = size.height as f32;
    let margin = (width * 0.028).clamp(24.0, 48.0);
    let sidebar_width = (width * 0.17).clamp(168.0, 240.0);
    let content_x = margin + sidebar_width + margin;
    let content_width = (width - content_x - margin).max(360.0);
    let mut scene = GlassScene::default();

    let mut sidebar_material = GlassMaterial::thick();
    sidebar_material.tint = GlassColor::rgba(0.12, 0.34, 0.72, 0.20);
    scene.push(
        GlassNode::new(
            GlassId(10),
            Rect::new(margin, margin, sidebar_width, height - margin * 2.0),
        )
        .shape(GlassShape::Superellipse { exponent: 5.0 })
        .material(sidebar_material),
    );

    let mut toolbar_material = GlassMaterial::clear();
    toolbar_material.tint = GlassColor::rgba(0.30, 0.58, 1.0, 0.16);
    scene.push(
        GlassNode::new(GlassId(11), Rect::new(content_x, margin, content_width, 82.0))
            .shape(GlassShape::Superellipse { exponent: 4.5 })
            .material(toolbar_material),
    );

    let card_width = (content_width - 28.0) / 3.0;
    for (index, tint) in [
        GlassColor::rgba(0.15, 0.62, 1.0, 0.18),
        GlassColor::rgba(0.48, 0.28, 1.0, 0.18),
        GlassColor::rgba(0.04, 0.82, 0.64, 0.16),
    ]
    .into_iter()
    .enumerate()
    {
        let mut material = GlassMaterial::regular();
        material.tint = tint;
        scene.push(
            GlassNode::new(
                GlassId(20 + index as u64),
                Rect::new(
                    content_x + (card_width + 14.0) * index as f32,
                    margin + 98.0,
                    card_width,
                    112.0,
                ),
            )
            .shape(GlassShape::Superellipse { exponent: 4.5 })
            .material(material),
        );
    }

    let lower_y = margin + 224.0;
    let lower_height = (height - lower_y - margin - 98.0).max(180.0);
    for (index, x) in [content_x, content_x + content_width * 0.51].into_iter().enumerate() {
        let mut material =
            if index == 0 { GlassMaterial::regular() } else { GlassMaterial::clear() };
        material.tint = if index == 0 {
            GlassColor::rgba(0.10, 0.36, 0.80, 0.14)
        } else {
            GlassColor::rgba(0.08, 0.78, 0.68, 0.12)
        };
        scene.push(
            GlassNode::new(
                GlassId(30 + index as u64),
                Rect::new(x, lower_y, content_width * 0.49 - 7.0, lower_height),
            )
            .shape(GlassShape::Superellipse { exponent: 4.5 })
            .material(material),
        );
    }

    let mut footer_material = GlassMaterial::clear();
    footer_material.tint = GlassColor::rgba(0.30, 0.48, 0.90, 0.13);
    scene.push(
        GlassNode::new(
            GlassId(32),
            Rect::new(content_x, height - margin - 82.0, content_width, 82.0),
        )
        .shape(GlassShape::Superellipse { exponent: 4.5 })
        .material(footer_material),
    );
    scene
}
