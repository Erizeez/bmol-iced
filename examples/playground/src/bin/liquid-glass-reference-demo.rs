//! Unmodified liquid-glass-studio rendering baseline.
//!
//! This executable runs the shaders and four-pass graph from source revision
//! d13c3e5 directly. Project-specific enhancements belong in the separate
//! comparison demo and must not enter this baseline.

use std::sync::Arc;

use liquid_glass::GpuSize;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[path = "../upstream_renderer.rs"]
#[allow(dead_code)]
mod upstream_renderer;

use upstream_renderer::UpstreamRenderer;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 640;

struct Playground {
    window: Option<Arc<Window>>,
    state: Option<WindowState>,
}

struct WindowState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: UpstreamRenderer,
    pointer: [f32; 2],
}

impl Playground {
    fn new() -> Self {
        Self { window: None, state: None }
    }

    fn initialize_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Liquid Glass Studio d13c3e5 — Unmodified Baseline")
            .with_inner_size(Size::Physical(PhysicalSize::new(WIDTH, HEIGHT)));
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
            label: Some("upstream liquid-glass baseline device"),
            ..wgpu::DeviceDescriptor::default()
        }))
        .expect("create GPU device");

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("configure surface");
        config.format = preferred_surface_format(&surface.get_capabilities(&adapter));
        surface.configure(&device, &config);

        let render_size = GpuSize::new(config.width, config.height);
        let renderer = UpstreamRenderer::new(device, queue, render_size, config.format);
        let pointer = default_pointer(render_size);
        self.window = Some(window);
        self.state = Some(WindowState { surface, config, renderer, pointer });
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
                #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                let x = position.x as f32;
                #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
                let y = state.config.height as f32 - position.y as f32;
                state.pointer = [
                    x.clamp(0.0, state.config.width as f32),
                    y.clamp(0.0, state.config.height as f32),
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
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(self.renderer.device(), &self.config);
        let render_size = GpuSize::new(size.width, size.height);
        self.renderer.resize(render_size);
        self.pointer = default_pointer(render_size);
    }

    fn render(&mut self) {
        let Ok(frame) = self.surface.get_current_texture() else { return };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.renderer.render_to_view(self.pointer, &view);
        frame.present();
    }
}

#[allow(clippy::cast_precision_loss)]
fn default_pointer(size: GpuSize) -> [f32; 2] {
    [size.width as f32 * 0.5 + 170.0, size.height as f32 * 0.5]
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
