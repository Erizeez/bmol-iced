use std::{
    fmt,
    sync::atomic::{AtomicU8, Ordering},
    time::{Duration, Instant},
};

use iced_wgpu::{Engine, Renderer as IcedRenderer, graphics, wgpu};
use liquid_glass::{GlassId, GlassNode, GlassRole, GlassScene, GpuRenderer, GpuSize, Rect, UiColorScheme, UiTheme};

#[path = "background.rs"]
mod background;

#[cfg(target_os = "macos")]
pub const CONTENT_TOP_INSET: f32 = 32.0;
#[cfg(not(target_os = "macos"))]
pub const CONTENT_TOP_INSET: f32 = 0.0;

static ACTIVE_COLOR_SCHEME: AtomicU8 = AtomicU8::new(1);

pub fn set_color_scheme(scheme: UiColorScheme) {
    ACTIVE_COLOR_SCHEME.store(
        match scheme {
            UiColorScheme::Light => 0,
            UiColorScheme::Dark => 1,
        },
        Ordering::Relaxed,
    );
}

fn active_color_scheme() -> UiColorScheme {
    match ACTIVE_COLOR_SCHEME.load(Ordering::Relaxed) {
        0 => UiColorScheme::Light,
        _ => UiColorScheme::Dark,
    }
}

pub struct Renderer {
    inner: IcedRenderer,
    foreground: Option<IcedRenderer>,
    overlay: Option<IcedRenderer>,
    active_layer: RenderLayer,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum RenderLayer {
    #[default]
    Source,
    Foreground,
    Overlay,
}

impl fmt::Debug for Renderer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("LiquidIcedRenderer").finish_non_exhaustive()
    }
}

impl Renderer {
    fn new(engine: Engine, default_font: iced::Font, default_text_size: iced::Pixels) -> Self {
        Self {
            inner: IcedRenderer::new(engine.clone(), default_font, default_text_size),
            foreground: Some(IcedRenderer::new(engine.clone(), default_font, default_text_size)),
            overlay: Some(IcedRenderer::new(
                engine,
                default_font,
                default_text_size,
            )),
            active_layer: RenderLayer::Source,
        }
    }

    fn active_mut(&mut self) -> &mut IcedRenderer {
        match self.active_layer {
            RenderLayer::Foreground => {
                if let Some(foreground) = self.foreground.as_mut() {
                    return foreground;
                }
            }
            RenderLayer::Overlay => {
                if let Some(overlay) = self.overlay.as_mut() {
                    return overlay;
                }
            }
            RenderLayer::Source => {}
        }
        &mut self.inner
    }
}

impl liquid_glass::GlassForegroundRenderer for Renderer {
    fn begin_glass_foreground(&mut self) {
        self.active_layer = RenderLayer::Foreground;
    }

    fn end_glass_foreground(&mut self) {
        self.active_layer = RenderLayer::Source;
    }

    fn begin_glass_overlay(&mut self) {
        self.active_layer = RenderLayer::Overlay;
    }

    fn end_glass_overlay(&mut self) {
        self.active_layer = RenderLayer::Source;
    }
}

impl iced::advanced::Renderer for Renderer {
    fn start_layer(&mut self, bounds: iced::Rectangle) {
        // Scrollable widgets establish their clip layer before drawing the
        // child. The child may be routed to the foreground renderer, so both
        // renderer instances must enter the same layer or stale pixels can
        // remain visible after a scroll.
        self.inner.start_layer(bounds);
        if let Some(foreground) = self.foreground.as_mut() {
            foreground.start_layer(bounds);
        }
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.start_layer(bounds);
        }
    }

    fn end_layer(&mut self) {
        self.inner.end_layer();
        if let Some(foreground) = self.foreground.as_mut() {
            foreground.end_layer();
        }
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.end_layer();
        }
    }

    fn start_transformation(&mut self, transformation: iced::Transformation) {
        // Keep scroll translations identical across the source and
        // foreground render targets. A foreground widget can be drawn after
        // the parent scrollable has already pushed this transformation.
        self.inner.start_transformation(transformation);
        if let Some(foreground) = self.foreground.as_mut() {
            foreground.start_transformation(transformation);
        }
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.start_transformation(transformation);
        }
    }

    fn end_transformation(&mut self) {
        self.inner.end_transformation();
        if let Some(foreground) = self.foreground.as_mut() {
            foreground.end_transformation();
        }
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.end_transformation();
        }
    }

    fn fill_quad(
        &mut self,
        quad: iced::advanced::renderer::Quad,
        background: impl Into<iced::Background>,
    ) {
        self.active_mut().fill_quad(quad, background);
    }

    fn reset(&mut self, new_bounds: iced::Rectangle) {
        self.inner.reset(new_bounds);
        if let Some(foreground) = self.foreground.as_mut() {
            foreground.reset(new_bounds);
        }
        if let Some(overlay) = self.overlay.as_mut() {
            overlay.reset(new_bounds);
        }
    }

    fn allocate_image(
        &mut self,
        handle: &iced::advanced::image::Handle,
        callback: impl FnOnce(Result<iced::advanced::image::Allocation, iced::advanced::image::Error>)
        + Send
        + 'static,
    ) {
        self.active_mut().allocate_image(handle, callback);
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
        self.active_mut().fill_paragraph(text, position, color, clip_bounds);
    }

    fn fill_editor(
        &mut self,
        editor: &Self::Editor,
        position: iced::Point,
        color: iced::Color,
        clip_bounds: iced::Rectangle,
    ) {
        self.active_mut().fill_editor(editor, position, color, clip_bounds);
    }

    fn fill_text(
        &mut self,
        text: iced::advanced::text::Text<String, Self::Font>,
        position: iced::Point,
        color: iced::Color,
        clip_bounds: iced::Rectangle,
    ) {
        self.active_mut().fill_text(text, position, color, clip_bounds);
    }
}

impl graphics::text::Renderer for Renderer {
    fn fill_raw(&mut self, raw: graphics::text::Raw) {
        self.active_mut().fill_raw(raw);
    }
}

impl graphics::mesh::Renderer for Renderer {
    fn draw_mesh(&mut self, mesh: graphics::Mesh) {
        self.active_mut().draw_mesh(mesh);
    }

    fn draw_mesh_cache(&mut self, cache: graphics::mesh::Cache) {
        self.active_mut().draw_mesh_cache(cache);
    }
}

impl graphics::geometry::Renderer for Renderer {
    type Geometry = <IcedRenderer as graphics::geometry::Renderer>::Geometry;
    type Frame = <IcedRenderer as graphics::geometry::Renderer>::Frame;

    fn new_frame(&self, bounds: iced::Rectangle) -> Self::Frame {
        match self.active_layer {
            RenderLayer::Foreground => {
                if let Some(foreground) = self.foreground.as_ref() {
                    return foreground.new_frame(bounds);
                }
            }
            RenderLayer::Overlay => {
                if let Some(overlay) = self.overlay.as_ref() {
                    return overlay.new_frame(bounds);
                }
            }
            RenderLayer::Source => {}
        }
        self.inner.new_frame(bounds)
    }

    fn draw_geometry(&mut self, geometry: Self::Geometry) {
        self.active_mut().draw_geometry(geometry);
    }
}

impl iced_wgpu::primitive::Renderer for Renderer {
    fn draw_primitive(
        &mut self,
        bounds: iced::Rectangle,
        primitive: impl iced_wgpu::primitive::Primitive,
    ) {
        self.active_mut().draw_primitive(bounds, primitive);
    }
}

impl iced::advanced::svg::Renderer for Renderer {
    fn measure_svg(&self, handle: &iced::advanced::svg::Handle) -> iced::Size<u32> {
        iced::advanced::svg::Renderer::measure_svg(&self.inner, handle)
    }

    fn draw_svg(
        &mut self,
        svg: iced::advanced::svg::Svg,
        bounds: iced::Rectangle,
        clip_bounds: iced::Rectangle,
    ) {
        iced::advanced::svg::Renderer::draw_svg(self.active_mut(), svg, bounds, clip_bounds);
    }
}

impl iced::advanced::image::Renderer for Renderer {
    type Handle = iced::advanced::image::Handle;

    fn load_image(
        &self,
        handle: &Self::Handle,
    ) -> Result<iced::advanced::image::Allocation, iced::advanced::image::Error> {
        iced::advanced::image::Renderer::load_image(&self.inner, handle)
    }

    fn measure_image(&self, handle: &Self::Handle) -> Option<iced::Size<u32>> {
        iced::advanced::image::Renderer::measure_image(&self.inner, handle)
    }

    fn draw_image(
        &mut self,
        image: iced::advanced::image::Image,
        bounds: iced::Rectangle,
        clip_bounds: iced::Rectangle,
    ) {
        iced::advanced::image::Renderer::draw_image(self.active_mut(), image, bounds, clip_bounds);
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
        .map(|inner| Self {
            inner,
            foreground: None,
            overlay: None,
            active_layer: RenderLayer::Source,
        })
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
    iced_source: Option<wgpu::Texture>,
    iced_source_size: GpuSize,
    iced_foreground: Option<wgpu::Texture>,
    iced_foreground_size: GpuSize,
    iced_overlay: Option<wgpu::Texture>,
    iced_overlay_size: GpuSize,
    native_backdrop: Option<liquid_glass_native::DesktopBlurTarget>,
    last_backdrop_capture: Instant,
    color_scheme: UiColorScheme,
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

        apply_native_backdrop(&compatible_window);
        let native_backdrop = compatible_window
            .window_handle()
            .ok()
            .and_then(|handle| liquid_glass_native::desktop_blur_target(handle.as_raw()));
        if let Some(target) = native_backdrop {
            liquid_glass_native::refresh_desktop_blur(target);
            // Stage Manager resets the blur asynchronously between redraws;
            // the guard reapplies it on every workspace transition.
            liquid_glass_native::install_stage_manager_guard(target);
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
        let alpha_mode = preferred_transparent_alpha_mode(&capabilities);
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
        let mut liquid = GpuRenderer::from_device_with_format(
            device.clone(),
            queue.clone(),
            GpuSize::new(1, 1),
            format,
        );
        // The window owns the real desktop backdrop. The renderer writes
        // transparent pixels outside glass surfaces. Non-macOS builds may add
        // a deterministic wallpaper source below; macOS waits for a real
        // desktop capture instead of inventing a misaligned backdrop.
        liquid.set_transparent_background(true);
        // On macOS the native compositor already owns the real desktop
        // backdrop. Do not put a bundled wallpaper into the shader when
        // Screen Recording permission is unavailable: that creates a second,
        // visibly misaligned desktop behind the glass. Other platforms keep
        // the deterministic wallpaper source for the demo.
        #[cfg(not(target_os = "macos"))]
        {
            let (fallback, fallback_ratio) =
                background::settings_background_texture(&device, &queue, active_color_scheme());
            liquid.set_background_texture(fallback, fallback_ratio);
        }
        let engine = Engine::new(
            &adapter,
            device.clone(),
            queue.clone(),
            format,
            settings.antialiasing,
            shell,
        );

        Ok(Self {
            instance,
            adapter,
            format,
            alpha_mode,
            engine,
            settings,
            device,
            liquid,
            iced_source: None,
            iced_source_size: GpuSize::new(0, 0),
            iced_foreground: None,
            iced_foreground_size: GpuSize::new(0, 0),
            iced_overlay: None,
            iced_overlay_size: GpuSize::new(0, 0),
            native_backdrop,
            last_backdrop_capture: Instant::now() - Duration::from_secs(1),
            color_scheme: active_color_scheme(),
            started_at: Instant::now(),
        })
    }

    fn create_renderer(&self) -> Self::Renderer {
        Renderer::new(
            self.engine.clone(),
            self.settings.default_font,
            self.settings.default_text_size,
        )
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
        // Stage Manager can rebuild the native window compositor between two
        // redraws. Reapply the blur immediately around the transparent
        // surface submission so no sharp desktop frame can slip through.
        if let Some(target) = self.native_backdrop {
            liquid_glass_native::refresh_desktop_blur(target);
        }
        let frame = surface.get_current_texture().map_err(|error| map_surface_error(&error))?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let physical = viewport.physical_size();
        let size = GpuSize::new(physical.width.max(1), physical.height.max(1));
        if self.liquid.size() != size {
            self.liquid.resize(size).map_err(|_| graphics::compositor::SurfaceError::Other)?;
        }
        // CGWindowListCreateImage is synchronous on macOS. Sampling it every
        // 50 ms made an otherwise GPU-smooth scroll periodically block the UI
        // thread. The desktop is visually stable enough for a 120 ms source
        // cadence while the UI itself can continue presenting at display rate.
        if self.last_backdrop_capture.elapsed() >= Duration::from_millis(120) {
            if let Some(target) = self.native_backdrop
                && let Some((width, height, rgba8)) =
                    liquid_glass_native::capture_desktop_backdrop(target)
            {
                let stride = width.saturating_mul(4);
                let _ = self.liquid.set_background_rgba8(width, height, stride, &rgba8);
            }
            self.last_backdrop_capture = Instant::now();
        }
        if self.iced_source_size != size {
            self.iced_source = Some(self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("liquid-glass Iced source texture"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            }));
            self.iced_source_size = size;
        }
        if self.iced_foreground_size != size {
            self.iced_foreground = Some(self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("liquid-glass Iced foreground texture"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }));
            self.iced_foreground_size = size;
        }
        if self.iced_overlay_size != size {
            self.iced_overlay = Some(self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("liquid-glass Iced overlay texture"),
                size: wgpu::Extent3d {
                    width: size.width,
                    height: size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }));
            self.iced_overlay_size = size;
        }
        let source_texture = self.iced_source.as_ref().expect("Iced source texture is initialized");
        let source_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let color_scheme = active_color_scheme();
        let sidebar_medium = match color_scheme {
            UiColorScheme::Light => [0.89, 0.90, 0.91, 1.0],
            UiColorScheme::Dark => [0.21, 0.23, 0.27, 1.0],
        };
        self.liquid.render_background_to_view(&source_view);
        let scale = viewport.scale_factor().max(1.0);
        let right_x = (232.0 * scale).round() as u32;
        // Above the search field the sidebar uses one fixed, strong blur.
        // Only the search field's own height is the fade band: it starts at
        // the field's top edge and reaches zero at its bottom edge. The
        // search field is composited afterward, above this entire treatment.
        let sidebar_gradient_y = 0;
        let search_top_y = ((CONTENT_TOP_INSET + 10.0) * scale).round() as u32;
        let search_bottom_y = ((CONTENT_TOP_INSET + 46.0) * scale).round() as u32;
        let sidebar_gradient_height = search_bottom_y.saturating_sub(sidebar_gradient_y);
        // The sidebar is a flat, opaque medium. Seed it before Iced draws its
        // transparent rows so every later blur sample has a real background
        // instead of transparent black RGB from the window compositor.
        self.liquid.render_solid_region_to_view(
            &source_view,
            (0, 0, right_x, size.height),
            sidebar_medium,
        );
        self.liquid.render_solid_region_to_view(
            &source_view,
            (right_x, 0, size.width.saturating_sub(right_x), size.height),
            [0.97, 0.97, 0.98, 1.0],
        );
        renderer.inner.present(None, frame.texture.format(), &source_view, viewport);
        if self.color_scheme != color_scheme {
            self.color_scheme = color_scheme;
        }
        let scene = scene_for_viewport(size, viewport.scale_factor(), color_scheme);
        self.liquid
            .render_scene_to_view_with_source_and_blur_region(
                &view,
                source_texture,
                (
                    0,
                    0,
                    right_x,
                    size.height,
                ),
                (64.0 * scale).round() as u32,
                match color_scheme {
                    UiColorScheme::Light => [0.89, 0.90, 0.91, 0.78],
                    UiColorScheme::Dark => [0.21, 0.23, 0.27, 0.76],
                },
                &scene,
                self.started_at.elapsed().as_secs_f32(),
            )
            .map_err(|_| graphics::compositor::SurfaceError::Other)?;
        if let Some(foreground_texture) = self.iced_foreground.as_ref()
            && let Some(foreground) = renderer.foreground.as_mut()
        {
            let foreground_view =
                foreground_texture.create_view(&wgpu::TextureViewDescriptor::default());
            foreground.present(
                Some(iced::Color::TRANSPARENT),
                frame.texture.format(),
                &foreground_view,
                viewport,
            );
            self.liquid.composite_texture_to_output(foreground_texture);
            // Keep the top region at a fixed radius of 128. Within the
            // search field bounds only the overlay opacity changes, ending
            // fully transparent at the field's lower edge.
            self.liquid.render_vertical_blur_with_flat_top_to_output(
                (0, sidebar_gradient_y, right_x, sidebar_gradient_height.max(1)),
                search_top_y,
                (128.0 * scale).round() as u32,
                sidebar_medium,
            );
        }
        let search_scene = search_scene_for_viewport(size, viewport.scale_factor(), color_scheme);
        self.liquid
            .render_scene_over_output(
                &view,
                &search_scene,
                self.started_at.elapsed().as_secs_f32(),
            )
            .map_err(|_| graphics::compositor::SurfaceError::Other)?;
        if let Some(overlay_texture) = self.iced_overlay.as_ref()
            && let Some(overlay) = renderer.overlay.as_mut()
        {
            let overlay_view = overlay_texture.create_view(&wgpu::TextureViewDescriptor::default());
            overlay.present(
                Some(iced::Color::TRANSPARENT),
                frame.texture.format(),
                &overlay_view,
                viewport,
            );
            self.liquid.composite_texture_to_output(overlay_texture);
        }
        self.liquid.copy_output_to_view(&view);
        on_pre_present();
        if let Some(target) = self.native_backdrop {
            liquid_glass_native::refresh_desktop_blur(target);
        }
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

fn preferred_transparent_alpha_mode(
    capabilities: &wgpu::SurfaceCapabilities,
) -> wgpu::CompositeAlphaMode {
    capabilities
        .alpha_modes
        .iter()
        .copied()
        .find(|mode| matches!(mode, wgpu::CompositeAlphaMode::PostMultiplied))
        .or_else(|| {
            capabilities
                .alpha_modes
                .iter()
                .copied()
                .find(|mode| matches!(mode, wgpu::CompositeAlphaMode::PreMultiplied))
        })
        .unwrap_or(wgpu::CompositeAlphaMode::Auto)
}

fn apply_native_backdrop<W: graphics::compositor::Window>(window: &W) {
    #[cfg(target_os = "windows")]
    {
        if let Err(error) = window_vibrancy::apply_acrylic(window, Some((18, 18, 22, 110))) {
            eprintln!("liquid-glass: Windows Acrylic unavailable: {error}");
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
    }
}

#[allow(clippy::cast_precision_loss)]
fn scene_for_viewport(size: GpuSize, scale_factor: f32, color_scheme: UiColorScheme) -> GlassScene {
    let scale_factor = scale_factor.max(1.0);
    let logical_width = size.width as f32 / scale_factor;
    let sidebar_width = 232.0;
    let content_x = sidebar_width;
    let content_width = (logical_width - content_x).max(1.0);
    let content_y = CONTENT_TOP_INSET;
    let theme = UiTheme::new(color_scheme);
    let mut scene = GlassScene::default();

    let mut toolbar = GlassNode::new(
        GlassId(10),
        Rect::new(content_x, content_y, content_width, 56.0),
    )
    .shape(theme.glass_shape(GlassRole::Toolbar))
    .material(theme.glass_material(GlassRole::Toolbar));
    toolbar.z_index = 10;
    scene.push(toolbar);

    let mut navigation = GlassNode::new(
        GlassId(12),
        Rect::new(content_x + 8.0, content_y + 10.0, 72.0, 36.0),
    )
    .shape(theme.glass_shape(GlassRole::FloatingControl))
    .material(theme.glass_material(GlassRole::FloatingControl));
    navigation.z_index = 20;
    scene.push(navigation);
    scale_scene(&mut scene, scale_factor);
    scene
}

fn search_scene_for_viewport(
    size: GpuSize,
    scale_factor: f32,
    color_scheme: UiColorScheme,
) -> GlassScene {
    let scale_factor = scale_factor.max(1.0);
    let content_y = CONTENT_TOP_INSET;
    let theme = UiTheme::new(color_scheme);
    let mut scene = GlassScene::default();
    let mut search = GlassNode::new(
        GlassId(11),
        Rect::new(10.0, content_y + 10.0, 212.0, 36.0),
    )
    .shape(theme.glass_shape(GlassRole::InputField))
    .material(theme.glass_material(GlassRole::InputField));
    search.z_index = 30;
    scene.push(search);
    scale_scene(&mut scene, scale_factor);
    let _ = size;
    scene
}

fn scale_scene(scene: &mut GlassScene, scale_factor: f32) {
    for node in scene.nodes_mut() {
        node.bounds.x *= scale_factor;
        node.bounds.y *= scale_factor;
        node.bounds.width *= scale_factor;
        node.bounds.height *= scale_factor;
        node.backdrop.bounds = node.bounds;
        node.backdrop.padding *= scale_factor;
        node.backdrop.blur_radius *= scale_factor;
        node.material.blur.radius *= scale_factor;
    }
}
