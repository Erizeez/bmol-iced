//! Side-by-side liquid-glass algorithm comparison (original vs enhanced).
//!
//! Left: the checked-in liquid-glass-studio shader and four-pass renderer at
//! commit d13c3e5. Right: this workspace's enhanced physical compositor. Both
//! receive the same geometry, pointer and source control values.

use std::{sync::Arc, time::Instant};

use liquid_glass::{
    AdaptiveStyle, Color, GlareStyle, GlassId, GlassMaterial, GlassNode, GlassScene, GlassShape,
    GlassVariant, GpuRenderer, GpuSize, Rect, ShadowStyle,
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

#[path = "../upstream_renderer.rs"]
mod upstream_renderer;

use upstream_renderer::{ComparisonPresenter, UpstreamRenderer};

const PANE_WIDTH: u32 = 640;
const PANE_HEIGHT: u32 = 640;
const SOURCE_SHAPE_SIZE: f32 = 200.0;

struct Playground {
    window: Option<Arc<Window>>,
    state: Option<WindowState>,
}

struct WindowState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    upstream: UpstreamRenderer,
    enhanced: GpuRenderer,
    presenter: ComparisonPresenter,
    pointer: [f32; 2],
    started_at: Instant,
}

impl Playground {
    fn new() -> Self {
        Self { window: None, state: None }
    }

    fn initialize_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes: WindowAttributes = Window::default_attributes()
            .with_title("Upstream d13c3e5 (left) | Enhanced renderer (right)")
            .with_inner_size(Size::Physical(PhysicalSize::new(PANE_WIDTH * 2, PANE_HEIGHT)));
        let window = Arc::new(event_loop.create_window(attributes).expect("create window"));
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("create wgpu surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("find a compatible GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("liquid-glass comparison device"),
            ..wgpu::DeviceDescriptor::default()
        }))
        .expect("create GPU device");

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(2), size.height.max(1))
            .expect("configure surface");
        config.format = preferred_surface_format(&surface.get_capabilities(&adapter));
        surface.configure(&device, &config);

        let pane_size = comparison_pane_size(config.width, config.height);
        let upstream =
            UpstreamRenderer::new(device.clone(), queue.clone(), pane_size, config.format);
        let enhanced = GpuRenderer::from_device_with_format(
            device.clone(),
            queue.clone(),
            pane_size,
            config.format,
        );
        let presenter = ComparisonPresenter::new(
            device,
            queue,
            config.format,
            upstream.output_texture(),
            enhanced.output_texture(),
        );
        let pointer = default_pointer(pane_size);

        self.window = Some(window);
        self.state = Some(WindowState {
            surface,
            config,
            upstream,
            enhanced,
            presenter,
            pointer,
            started_at: Instant::now(),
        });
    }
}

impl ApplicationHandler for Playground {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.initialize_window(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else { return };
        if window.id() != window_id {
            return;
        }
        let Some(state) = self.state.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::CursorMoved { position, .. } => {
                let pane_size = comparison_pane_size(state.config.width, state.config.height);
                #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                let local_x = position.x as f32 % pane_size.width as f32;
                #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                let bottom_y = state.config.height as f32 - position.y as f32;
                state.pointer = [
                    local_x.clamp(0.0, pane_size.width as f32),
                    bottom_y.clamp(0.0, pane_size.height as f32),
                ];
            }
            WindowEvent::RedrawRequested => state.render(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

impl WindowState {
    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width < 2 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(self.enhanced.device(), &self.config);
        let pane_size = comparison_pane_size(size.width, size.height);
        self.upstream.resize(pane_size);
        self.enhanced.resize(pane_size).expect("resize enhanced GPU targets");
        self.presenter
            .update_sources(self.upstream.output_texture(), self.enhanced.output_texture());
        self.pointer = default_pointer(pane_size);
    }

    fn render(&mut self) {
        self.upstream.render(self.pointer);
        let scene = enhanced_scene(self.enhanced.size(), self.pointer);
        self.enhanced
            .render_scene(&scene, self.started_at.elapsed().as_secs_f32())
            .expect("render enhanced comparison pane");

        let Ok(frame) = self.surface.get_current_texture() else { return };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.presenter.present(&view);
        frame.present();
    }
}

fn comparison_pane_size(width: u32, height: u32) -> GpuSize {
    GpuSize::new((width / 2).max(1), height.max(1))
}

#[allow(clippy::cast_precision_loss)]
fn default_pointer(size: GpuSize) -> [f32; 2] {
    [size.width as f32 * 0.5 + 170.0, size.height as f32 * 0.5]
}

#[allow(clippy::cast_precision_loss)]
fn enhanced_scene(size: GpuSize, pointer: [f32; 2]) -> GlassScene {
    let mut material = GlassMaterial::clear();
    material.variant = GlassVariant::Regular;
    material.blur.radius = 1.0;
    material.tint = Color::transparent();
    material.merge_rate = 0.05;
    material.show_shape1 = true;
    material.refraction.thickness = 0.20;
    material.refraction.index = 1.40;
    material.refraction.strength = 1.0;
    material.dispersion.strength = 0.07;
    material.fresnel.range = 0.75;
    material.fresnel.hardness = 0.20;
    material.fresnel.strength = 0.20;
    material.glare = GlareStyle {
        range: 30.0,
        hardness: 0.20,
        convergence: 0.50,
        opposite_factor: 0.80,
        factor: 0.90,
    };
    material.opacity = 1.0;
    material.shadow = ShadowStyle { expand: 25.0, factor: 0.15, offset: [0.0, 10.0] };
    material.adaptive = AdaptiveStyle { tint: 1.0, ambient: 0.0, shadow: 1.0, size: 0.0 };

    let node = GlassNode::new(
        GlassId(1),
        Rect::new(
            pointer[0] - SOURCE_SHAPE_SIZE * 0.5,
            size.height as f32 - pointer[1] - SOURCE_SHAPE_SIZE * 0.5,
            SOURCE_SHAPE_SIZE,
            SOURCE_SHAPE_SIZE,
        ),
    )
    .shape(GlassShape::RoundedRect { radius: 80.0 })
    .material(material);

    let mut scene = GlassScene::default();
    scene.push(node);
    scene
}

fn preferred_surface_format(capabilities: &wgpu::SurfaceCapabilities) -> wgpu::TextureFormat {
    capabilities
        .formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .unwrap_or(capabilities.formats[0])
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.run_app(&mut Playground::new()).expect("run event loop");
}
