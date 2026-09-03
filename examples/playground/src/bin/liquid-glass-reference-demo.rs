//! Minimal visual port of the liquid-glass-studio reference composition.
//!
//! The reference effect is intentionally one glass node: its shader combines
//! a circle and a rounded rectangle with smooth-min, then applies refraction,
//! dispersion, Fresnel, glare, blur, and shadow to the merged SDF boundary.

use std::{sync::Arc, time::Instant};

use liquid_glass::{
    Color, GlassId, GlassMaterial, GlassNode, GlassScene, GlassShape, GpuRenderer, GpuSize, Rect,
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

struct Playground {
    window: Option<Arc<Window>>,
    state: Option<WindowState>,
}

struct WindowState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: GpuRenderer,
    scene: GlassScene,
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
            .with_title("Liquid Glass Reference Fusion")
            .with_inner_size(Size::Physical(PhysicalSize::new(960, 640)));
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
            label: Some("liquid-glass reference demo device"),
            ..Default::default()
        }))
        .expect("create GPU device");

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("configure surface");
        config.format = preferred_surface_format(&surface.get_capabilities(&adapter));
        surface.configure(&device, &config);

        let background = background::reference_grid_texture(&device, &queue);
        let mut renderer = GpuRenderer::from_device_with_format(
            device,
            queue,
            GpuSize::new(config.width, config.height),
            config.format,
        );
        renderer.set_background_texture(background.0, background.1);
        let scene = reference_scene(config.width, config.height, None);

        self.window = Some(window);
        self.state =
            Some(WindowState { surface, config, renderer, scene, started_at: Instant::now() });
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
                let cursor = (position.x as f32, state.config.height as f32 - position.y as f32);
                state.scene =
                    reference_scene(state.config.width, state.config.height, Some(cursor));
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
        self.renderer.resize(GpuSize::new(size.width, size.height)).expect("resize GPU targets");
        self.surface.configure(self.renderer.device(), &self.config);
        self.scene = reference_scene(size.width, size.height, None);
    }

    fn render(&mut self) {
        let Ok(frame) = self.surface.get_current_texture() else { return };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.renderer
            .render_scene_to_view(&view, &self.scene, self.started_at.elapsed().as_secs_f32())
            .expect("render reference fusion");
        frame.present();
    }
}

#[allow(clippy::cast_precision_loss)]
fn reference_scene(width: u32, height: u32, cursor: Option<(f32, f32)>) -> GlassScene {
    // The source demo places its 100px-radius circle at the canvas center and
    // moves the rounded rectangle with the pointer. The initial overlap keeps
    // the comparison deterministic, while pointer movement exposes the same
    // fused / separated states as the source.
    let circle_x = width as f32 * 0.5;
    let circle_y = height as f32 * 0.5;
    let shape_width = 220.0;
    let shape_height = 220.0;
    let (shape_center_x, shape_center_y) = cursor.unwrap_or((circle_x + 185.0, circle_y));

    let mut material = GlassMaterial::clear();
    material.blur.radius = 1.0;
    material.tint = Color::transparent();
    material.merge_rate = 0.05;
    material.show_shape1 = true;
    material.refraction.thickness = 0.20;
    material.refraction.index = 1.40;
    material.dispersion.strength = 0.07;
    material.fresnel.range = 0.75;
    material.fresnel.hardness = 0.20;
    material.fresnel.strength = 0.20;
    material.opacity = 1.0;
    material.shadow.factor = 0.25;

    let node = GlassNode::new(
        GlassId(1),
        Rect::new(
            shape_center_x - shape_width * 0.5,
            height as f32 - shape_center_y - shape_height * 0.5,
            shape_width,
            shape_height,
        ),
    )
    .shape(GlassShape::Superellipse { exponent: 5.0 })
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

#[path = "../background.rs"]
mod background;

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.run_app(&mut Playground::new()).expect("run event loop");
}
