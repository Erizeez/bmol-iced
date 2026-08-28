use std::{sync::Arc, time::Instant};

use iced::Size as IcedSize;
use liquid_glass::{
    GlassContainer, GlassId, GlassMaterial, GlassNode, GlassScene, GlassShape, GpuRenderer,
    GpuSize, Rect,
};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes, WindowId},
};

enum RenderOutcome {
    Presented,
    Reconfigure,
    Skipped,
}

struct Playground {
    window: Option<Arc<Window>>,
    state: Option<WindowState>,
}

struct WindowState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: GpuRenderer,
    panel_widget: GlassContainer,
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
            .with_title("Liquid Glass Playground")
            .with_inner_size(Size::Physical(PhysicalSize::new(960, 640)));
        let window = Arc::new(event_loop.create_window(attributes).expect("create window"));
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("create wgpu surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .expect("find a compatible GPU adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("liquid-glass playground device"),
            ..Default::default()
        }))
        .expect("create GPU device");

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("configure surface");
        config.format = preferred_surface_format(&surface.get_capabilities(&adapter));
        surface.configure(&device, &config);

        let renderer = GpuRenderer::from_device_with_format(
            device,
            queue,
            GpuSize::new(config.width, config.height),
            config.format,
        );
        let mut panel_widget =
            GlassContainer::new(GlassId(1), Rect::new(220.0, 150.0, 520.0, 340.0))
                .shape(GlassShape::Superellipse { exponent: 4.5 })
                .material(GlassMaterial::regular());
        let panel = panel_widget.layout_scene_node(iced_viewport_size(config.width, config.height));
        let panel_bounds = panel.bounds;
        let scene = demo_scene(panel);
        let scene_nodes = scene.nodes().len();

        self.window = Some(window);
        self.state = Some(WindowState {
            surface,
            config,
            renderer,
            panel_widget,
            scene,
            started_at: Instant::now(),
        });
        println!(
            "Liquid Glass window initialized: {}x{}; Iced layout -> {} scene nodes; primary {:.0}x{:.0} at ({:.0}, {:.0})",
            size.width,
            size.height,
            scene_nodes,
            panel_bounds.width,
            panel_bounds.height,
            panel_bounds.x,
            panel_bounds.y,
        );
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
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window.id() != window_id {
            return;
        }

        let Some(state) = self.state.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::RedrawRequested => match state.render() {
                RenderOutcome::Reconfigure => state.reconfigure(),
                RenderOutcome::Presented | RenderOutcome::Skipped => {}
            },
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
        let panel =
            self.panel_widget.layout_scene_node(iced_viewport_size(size.width, size.height));
        self.scene = demo_scene(panel);
    }

    fn reconfigure(&mut self) {
        self.surface.configure(self.renderer.device(), &self.config);
    }

    fn render(&mut self) -> RenderOutcome {
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => {
                self.renderer
                    .render_scene_to_surface_texture(
                        frame,
                        &self.scene,
                        self.started_at.elapsed().as_secs_f32(),
                    )
                    .expect("render glass scene");
                RenderOutcome::Presented
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.renderer
                    .render_scene_to_surface_texture(
                        frame,
                        &self.scene,
                        self.started_at.elapsed().as_secs_f32(),
                    )
                    .expect("render glass scene");
                RenderOutcome::Reconfigure
            }
            wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost
            | wgpu::CurrentSurfaceTexture::Validation => RenderOutcome::Reconfigure,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                RenderOutcome::Skipped
            }
        }
    }
}

fn preferred_surface_format(capabilities: &wgpu::SurfaceCapabilities) -> wgpu::TextureFormat {
    capabilities
        .formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .unwrap_or(capabilities.formats[0])
}

fn demo_scene(primary_panel: GlassNode) -> GlassScene {
    let mut scene = GlassScene::default();

    let mut sidebar_material = GlassMaterial::thick();
    sidebar_material.tint = liquid_glass::Color::rgba(0.12, 0.34, 0.72, 0.20);
    let mut sidebar = GlassNode::new(GlassId(10), Rect::new(36.0, 36.0, 168.0, 568.0))
        .shape(GlassShape::Superellipse { exponent: 5.0 })
        .material(sidebar_material);
    sidebar.z_index = 0;
    scene.push(sidebar);

    let mut toolbar_material = GlassMaterial::clear();
    toolbar_material.tint = liquid_glass::Color::rgba(0.30, 0.58, 1.0, 0.16);
    let mut toolbar = GlassNode::new(GlassId(11), Rect::new(220.0, 36.0, 700.0, 82.0))
        .shape(GlassShape::RoundedRect { radius: 28.0 })
        .material(toolbar_material);
    toolbar.z_index = 0;
    scene.push(toolbar);

    let mut card_material = GlassMaterial::regular();
    card_material.tint = liquid_glass::Color::rgba(0.12, 0.72, 0.92, 0.15);
    for (index, x) in [220.0, 430.0, 640.0].into_iter().enumerate() {
        let mut card = GlassNode::new(GlassId(20 + index as u64), Rect::new(x, 138.0, 190.0, 86.0))
            .shape(GlassShape::Superellipse { exponent: 4.5 })
            .material(card_material);
        card.z_index = 0;
        scene.push(card);
    }

    let mut front_panel = primary_panel;
    front_panel.z_index = 1;
    scene.push(front_panel);

    let mut interactive_material = GlassMaterial::interactive();
    interactive_material.tint = liquid_glass::Color::rgba(0.52, 0.22, 0.95, 0.20);
    let mut secondary = GlassNode::new(GlassId(30), Rect::new(560.0, 270.0, 300.0, 190.0))
        .shape(GlassShape::RoundedRect { radius: 42.0 })
        .material(interactive_material);
    secondary.z_index = 2;
    scene.push(secondary);

    let mut footer_material = GlassMaterial::clear();
    footer_material.tint = liquid_glass::Color::rgba(0.08, 0.82, 0.64, 0.16);
    let mut footer = GlassNode::new(GlassId(31), Rect::new(220.0, 540.0, 700.0, 64.0))
        .shape(GlassShape::Capsule)
        .material(footer_material);
    footer.z_index = 1;
    scene.push(footer);

    scene
}

#[allow(clippy::cast_precision_loss)]
fn iced_viewport_size(width: u32, height: u32) -> IcedSize {
    IcedSize::new(width as f32, height as f32)
}

fn main() {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.run_app(&mut Playground::new()).expect("run event loop");
}
