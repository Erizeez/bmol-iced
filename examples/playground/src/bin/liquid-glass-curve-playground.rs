//! Standalone Iced GUI playground with Side-by-Side real-time comparison.
//! Left: macOS 27 Native Official capture.
//! Right: liquid-rs Live GPU rendering.

#[path = "../iced_backend.rs"]
mod iced_backend;

use std::sync::Arc;

use iced::{
    Alignment, Color, Element, Length, Task, Theme,
    widget::{button, column, container, image as iced_image, row, slider, text},
};
use iced_backend::Renderer;
use image::{ImageBuffer, Rgba};
use liquid_glass_render::{
    ContentGlassAppearance, ContentGlassMaterial, ContentGlassNode, ContentGlassRenderer, GpuSize,
};

const W: u32 = 1280;
const H: u32 = 800;

type RgbaImg = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackgroundPreset {
    Sonoma,
    Gray,
    Checkerboard,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum MacRefMode {
    Clear,
    Regular,
}

#[derive(Debug, Clone)]
enum Message {
    SetAmount(f32),
    SetWidthX(f32),
    SetHeightY(f32),
    SetBlur(f32),
    SetP1(f32),
    SetP2(f32),
    SetP3(f32),
    SetClarity(f32),
    ToggleMacMode,
    SetBezierPreset(f32, f32, f32),
    ApplyCalibrated,
    ApplyMilkyJade,
    SetSmoothing(f32),
    SetGlassOpacity(f32),
    SetTintOpacity(f32),
    SetBackground(BackgroundPreset),
    ResetDefaults,
    CopyCode,
}

struct GpuPipeline {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: ContentGlassRenderer,
    target: wgpu::Texture,
    readback: wgpu::Buffer,
    bytes_per_row: u32,
    nodes: Vec<ContentGlassNode>,
}

impl GpuPipeline {
    fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("iced playground gpu"),
            ..Default::default()
        }))
        .expect("device");

        let renderer = ContentGlassRenderer::from_device(
            device.clone(),
            queue.clone(),
            GpuSize::new(W, H),
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );

        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("playground target"),
            size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let bytes_per_row = (W * 4).div_ceil(256) * 256;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback buffer"),
            size: u64::from(bytes_per_row) * u64::from(H),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // 5 Standard Panels in 1280x800 logical canvas:
        let panels = [
            (120.0, 120.0, 320.0, 120.0, 60.0),
            (520.0, 100.0, 260.0, 260.0, 48.0),
            (880.0, 150.0, 300.0, 110.0, 55.0),
            (420.0, 480.0, 300.0, 200.0, 40.0),
            (880.0, 500.0, 260.0, 100.0, 50.0),
        ];
        let nodes: Vec<ContentGlassNode> = panels
            .iter()
            .map(|&(x, y, w, h, r)| {
                ContentGlassNode::new(x, y, w, h)
                    .corner_radius(r)
                    .appearance(ContentGlassAppearance::LIGHT)
            })
            .collect();

        Self { device, queue, renderer, target, readback, bytes_per_row, nodes }
    }

    #[allow(clippy::large_types_passed_by_value)]
    fn render(&mut self, material: ContentGlassMaterial, opacity: f32, backdrop: &[u8]) -> RgbaImg {
        self.renderer.set_material(material);
        let _ = self.renderer.set_backdrop_rgba8(W, H, backdrop);

        for node in &mut self.nodes {
            node.diffusion = opacity;
        }

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            backdrop,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(W * 4),
                rows_per_image: Some(H),
            },
            wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        );

        let view = self.target.create_view(&wgpu::TextureViewDescriptor::default());
        self.renderer.render_to_view(&view, &self.nodes, wgpu::LoadOp::Load);

        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.bytes_per_row),
                    rows_per_image: Some(H),
                },
            },
            wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = self.readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
            .expect("poll");
        rx.recv().expect("recv").expect("map");

        let mut pixels = vec![0u8; (W * H * 4) as usize];
        {
            let data = slice.get_mapped_range();
            for y in 0..H {
                let src_start = (y * self.bytes_per_row) as usize;
                let src_end = src_start + (W * 4) as usize;
                let dst_start = (y * W * 4) as usize;
                let dst_end = dst_start + (W * 4) as usize;
                pixels[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
            }
        }
        self.readback.unmap();

        ImageBuffer::from_raw(W, H, pixels).expect("rgba image buffer")
    }
}

struct AppState {
    amount: f32,
    width_x: f32,
    height_y: f32,
    blur: f32,
    p1: f32,
    p2: f32,
    p3: f32,
    clarity: f32,
    mac_mode: MacRefMode,
    smoothing: f32,
    glass_opacity: f32,
    tint_opacity: f32,
    bg_preset: BackgroundPreset,

    gpu: GpuPipeline,

    bg_sonoma: Arc<RgbaImg>,
    mac_sonoma_clear: Arc<RgbaImg>,
    mac_sonoma_regular: Arc<RgbaImg>,
    bg_gray: Arc<RgbaImg>,
    mac_gray_clear: Arc<RgbaImg>,
    mac_gray_regular: Arc<RgbaImg>,
    bg_test: Arc<RgbaImg>,
    mac_test_clear: Arc<RgbaImg>,
    mac_test_regular: Arc<RgbaImg>,

    mac_handle: iced_image::Handle,
    liq_handle: iced_image::Handle,
    curve_handle: iced_image::Handle,
}

fn generate_bezier_preview(p1: f32, p2: f32, p3: f32) -> iced_image::Handle {
    const W: u32 = 180;
    const H: u32 = 48;
    let mut raw = vec![0u8; (W * H * 4) as usize];

    for px in raw.chunks_exact_mut(4) {
        px[0] = 24;
        px[1] = 26;
        px[2] = 32;
        px[3] = 255;
    }

    let floor_y = H as f32 - 6.0;
    let top_y = 6.0;
    let height_span = floor_y - top_y;

    let floor_row = floor_y as usize;
    for x in 0..W as usize {
        let idx = (floor_row * W as usize + x) * 4;
        raw[idx] = 50;
        raw[idx + 1] = 55;
        raw[idx + 2] = 65;
    }

    for x in 0..W {
        let t = x as f32 / (W - 1) as f32;
        let u = 1.0 - t;
        let u2 = u * u;
        let u3 = u2 * u;
        let u4 = u2 * u2;
        let t2 = t * t;
        let t3 = t2 * t;
        let y_val = u4 * 1.0 + 4.0 * u3 * t * p1 + 6.0 * u2 * t2 * p2 + 4.0 * u * t3 * p3;

        let py = (floor_y - y_val * height_span).round() as i32;
        for dy in -1..=1 {
            let yy = py + dy;
            if yy >= 0 && yy < H as i32 {
                let idx = ((yy as usize) * (W as usize) + (x as usize)) * 4;
                raw[idx] = 94;
                raw[idx + 1] = 200;
                raw[idx + 2] = 255;
            }
        }
    }

    iced_image::Handle::from_rgba(W, H, raw)
}

fn load_rgba_file(path: &str, w: u32, h: u32) -> RgbaImg {
    if let Ok(bytes) = std::fs::read(path) {
        if let Ok(img) = ::image::load_from_memory(&bytes) {
            let resized = img.resize_exact(w, h, ::image::imageops::FilterType::Lanczos3);
            return resized.to_rgba8();
        }
    }
    ImageBuffer::from_pixel(w, h, Rgba([128, 128, 128, 255]))
}

impl AppState {
    fn new() -> (Self, Task<Message>) {
        let bg_sonoma =
            Arc::new(load_rgba_file("/tmp/lg-harness/assets/bg_sonoma2x_srgb.png", W, H));
        let mac_sonoma_clear =
            Arc::new(load_rgba_file("/tmp/lg-macos-glass/out/srgb/px_sonoma2x.png", W, H));
        let mac_sonoma_regular =
            Arc::new(load_rgba_file("/tmp/lg-macos-glass/out/srgb/macos_r5_sonoma.png", W, H));

        let bg_gray = Arc::new(load_rgba_file("/tmp/lg-harness/assets/bg_gray2x.png", W, H));
        let mac_gray_clear =
            Arc::new(load_rgba_file("/tmp/lg-macos-glass/out/srgb/px_gray2x.png", W, H));
        let mac_gray_regular =
            Arc::new(load_rgba_file("/tmp/lg-macos-glass/out/t2_r5_gray.png", W, H));

        let bg_test = Arc::new(load_rgba_file("/tmp/lg-harness/assets/bg_test2x.png", W, H));
        let mac_test_clear =
            Arc::new(load_rgba_file("/tmp/lg-macos-glass/out/srgb/px_test2x.png", W, H));
        let mac_test_regular =
            Arc::new(load_rgba_file("/tmp/lg-macos-glass/out/srgb/macos_r5_test.png", W, H));

        let gpu = GpuPipeline::new();

        let mut s = Self {
            amount: -56.0,
            width_x: 28.0,
            height_y: 32.0,
            blur: 14.0,
            p1: 1.02,
            p2: 0.16,
            p3: 0.00,
            clarity: 0.0,
            mac_mode: MacRefMode::Clear,
            smoothing: 0.00,
            glass_opacity: 1.0,
            tint_opacity: 0.0,
            bg_preset: BackgroundPreset::Sonoma,
            gpu,
            bg_sonoma,
            mac_sonoma_clear,
            mac_sonoma_regular,
            bg_gray,
            mac_gray_clear,
            mac_gray_regular,
            bg_test,
            mac_test_clear,
            mac_test_regular,
            mac_handle: iced_image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            liq_handle: iced_image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
            curve_handle: iced_image::Handle::from_rgba(1, 1, vec![0, 0, 0, 255]),
        };

        s.render_frame();
        (s, Task::none())
    }

    fn current_bg_arcs(&self) -> (Arc<RgbaImg>, Arc<RgbaImg>) {
        match (self.bg_preset, self.mac_mode) {
            (BackgroundPreset::Sonoma, MacRefMode::Clear) => {
                (self.bg_sonoma.clone(), self.mac_sonoma_clear.clone())
            }
            (BackgroundPreset::Sonoma, MacRefMode::Regular) => {
                (self.bg_sonoma.clone(), self.mac_sonoma_regular.clone())
            }
            (BackgroundPreset::Gray, MacRefMode::Clear) => {
                (self.bg_gray.clone(), self.mac_gray_clear.clone())
            }
            (BackgroundPreset::Gray, MacRefMode::Regular) => {
                (self.bg_gray.clone(), self.mac_gray_regular.clone())
            }
            (BackgroundPreset::Checkerboard, MacRefMode::Clear) => {
                (self.bg_test.clone(), self.mac_test_clear.clone())
            }
            (BackgroundPreset::Checkerboard, MacRefMode::Regular) => {
                (self.bg_test.clone(), self.mac_test_regular.clone())
            }
        }
    }

    fn render_frame(&mut self) {
        let (bg, mac) = self.current_bg_arcs();
        let bg_bytes = bg.as_raw().clone();

        // Scale 0.5: converts 2x Retina parameters (-60px, 26px, 14px) to 1x (1280x800) logical canvas
        let scale = 0.5;
        // Blend along the official clarity axis (transparent t=0 to tinted t=1):
        let base = ContentGlassMaterial::blend(self.clarity);
        let mut mat = base;

        let wx = self.width_x * scale;
        let hy = self.height_y * scale;
        let base_h = (wx * hy).sqrt();

        mat.inner_refract = [self.amount * scale, 1.0 / base_h, self.p1, self.p2];
        mat.refract_params = [0.0, self.smoothing, self.tint_opacity, self.p3];
        mat.displacement = [wx / base_h, 0.0, 0.0, hy / base_h];
        // Blur radius smoothly scales from calibrated self.blur to 65px (at 2x)
        mat.blur_radius = (self.blur + self.clarity * (65.0 - self.blur)) * scale;

        // Render live on GPU with real liquid-rs pipeline
        let liq = self.gpu.render(mat, self.glass_opacity, &bg_bytes);

        self.curve_handle = generate_bezier_preview(self.p1, self.p2, self.p3);
        self.liq_handle = iced_image::Handle::from_rgba(W, H, liq.into_raw());
        self.mac_handle = iced_image::Handle::from_rgba(W, H, mac.as_raw().clone());
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetClarity(c) => {
                self.clarity = c;
                self.render_frame();
            }
            Message::ToggleMacMode => {
                self.mac_mode = match self.mac_mode {
                    MacRefMode::Clear => MacRefMode::Regular,
                    MacRefMode::Regular => MacRefMode::Clear,
                };
                self.render_frame();
            }
            Message::SetAmount(v) => {
                self.amount = v;
                self.render_frame();
            }
            Message::SetWidthX(w) => {
                self.width_x = w;
                self.render_frame();
            }
            Message::SetHeightY(h) => {
                self.height_y = h;
                self.render_frame();
            }
            Message::SetBlur(v) => {
                self.blur = v;
                self.render_frame();
            }
            Message::SetP1(v) => {
                self.p1 = v;
                self.render_frame();
            }
            Message::SetP2(v) => {
                self.p2 = v;
                self.render_frame();
            }
            Message::SetP3(v) => {
                self.p3 = v;
                self.render_frame();
            }
            Message::SetBezierPreset(p1, p2, p3) => {
                self.p1 = p1;
                self.p2 = p2;
                self.p3 = p3;
                self.render_frame();
            }
            Message::ApplyCalibrated => {
                self.amount = -56.0;
                self.width_x = 28.0;
                self.height_y = 32.0;
                self.p1 = 1.02;
                self.p2 = 0.16;
                self.p3 = 0.00;
                self.clarity = 0.0;
                self.mac_mode = MacRefMode::Clear;
                self.smoothing = 0.00;
                self.blur = 14.0;
                self.glass_opacity = 1.0;
                self.tint_opacity = 0.0;
                self.render_frame();
            }
            Message::ApplyMilkyJade => {
                self.amount = -56.0;
                self.width_x = 28.0;
                self.height_y = 32.0;
                self.p1 = 1.02;
                self.p2 = 0.16;
                self.p3 = 0.00;
                self.smoothing = 0.00;
                self.blur = 14.0;
                self.glass_opacity = 1.0;
                self.tint_opacity = 0.28;
                self.render_frame();
            }
            Message::SetSmoothing(s) => {
                self.smoothing = s;
                self.render_frame();
            }
            Message::SetGlassOpacity(o) => {
                self.glass_opacity = o;
                self.render_frame();
            }
            Message::SetTintOpacity(t) => {
                self.tint_opacity = t;
                self.render_frame();
            }
            Message::SetBackground(b) => {
                self.bg_preset = b;
                self.render_frame();
            }
            Message::ResetDefaults => {
                self.amount = -56.0;
                self.width_x = 28.0;
                self.height_y = 32.0;
                self.blur = 14.0;
                self.p1 = 1.02;
                self.p2 = 0.16;
                self.p3 = 0.00;
                self.clarity = 0.0;
                self.mac_mode = MacRefMode::Clear;
                self.smoothing = 0.00;
                self.glass_opacity = 1.0;
                self.tint_opacity = 0.0;
                self.render_frame();
            }
            Message::CopyCode => {
                let code = format!(
                    "// Optimal calibrated values from Iced Playground:\n\
                     let mut mat = ContentGlassMaterial::transparent();\n\
                     let wx = {:.1}; // Bevel Width X\n\
                     let hy = {:.1}; // Bevel Height Y\n\
                     let base_h = (wx * hy).sqrt();\n\
                     mat.inner_refract = [{:.1}, 1.0 / base_h, {:.2}, {:.2}]; // amount, inv_height, P1, P2\n\
                     mat.refract_params = [0.0, {:.2}, {:.2}, {:.2}]; // [0.0, smoothing, pure_milkiness, P3]\n\
                     mat.displacement = [wx / base_h, 0.0, 0.0, hy / base_h];\n\
                     mat.blur_radius = {:.1};\n\
                     // Opacity = {:.2}\n",
                    self.width_x,
                    self.height_y,
                    self.amount,
                    self.p1,
                    self.p2,
                    self.smoothing,
                    self.tint_opacity,
                    self.p3,
                    self.blur,
                    self.glass_opacity,
                );
                return iced::clipboard::write(code);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message, Theme, Renderer> {
        let title_row = row![
            text("Liquid Glass 贝塞尔物理曲线调校台").size(14).color(Color::WHITE),
            button("官方对齐").on_press(Message::ApplyCalibrated),
            button(match self.mac_mode {
                MacRefMode::Clear => "左视口: 真机清晰态(t=0)",
                MacRefMode::Regular => "左视口: 真机浓郁态(t=1)",
            })
            .on_press(Message::ToggleMacMode),
            button("饱满水珠").on_press(Message::SetBezierPreset(1.25, 0.95, 0.40)),
            button("平缓S角").on_press(Message::SetBezierPreset(0.80, 0.30, 0.05)),
            button("温润白玉").on_press(Message::ApplyMilkyJade),
            button("Sonoma").on_press(Message::SetBackground(BackgroundPreset::Sonoma)),
            button("纯灰").on_press(Message::SetBackground(BackgroundPreset::Gray)),
            button("棋盘").on_press(Message::SetBackground(BackgroundPreset::Checkerboard)),
            button("重置").on_press(Message::ResetDefaults),
            button("复制代码").on_press(Message::CopyCode),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let clarity_col = column![
            text(format!("官方清晰度: {:.0}%", (1.0 - self.clarity) * 100.0))
                .size(11)
                .color(Color::from_rgb(0.95, 0.85, 0.45)),
            slider(0.0..=1.0, self.clarity, Message::SetClarity).step(0.01),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        // Bézier Control Curve: P1 (Shoulder), P2 (Mid-Belly), P3 (Landing) + 2D Plot
        let p1_col = column![
            text(format!("P1 (肩部凸度): {:.2}", self.p1)).size(11),
            slider(0.0..=2.0, self.p1, Message::SetP1).step(0.02),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let p2_col = column![
            text(format!("P2 (腰身弧度): {:.2}", self.p2)).size(11),
            slider(0.0..=2.0, self.p2, Message::SetP2).step(0.02),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let p3_col = column![
            text(format!("P3 (落底平滑): {:.2}", self.p3)).size(11),
            slider(0.0..=1.0, self.p3, Message::SetP3).step(0.02),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let curve_plot_col = column![
            text("截面剖面实时曲线:").size(11).color(Color::from_rgb(0.7, 0.8, 0.9)),
            container(iced_image(&self.curve_handle))
                .width(Length::Fixed(180.0))
                .height(Length::Fixed(48.0)),
        ]
        .spacing(2);

        let amount_col = column![
            text(format!("折射量级: {:.0}px", self.amount)).size(11),
            slider(-120.0..=-20.0, self.amount, Message::SetAmount).step(1.0),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let width_x_col = column![
            text(format!("左右宽度: {:.0}px", self.width_x)).size(11),
            slider(8.0..=60.0, self.width_x, Message::SetWidthX).step(1.0),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let height_y_col = column![
            text(format!("上下高度: {:.0}px", self.height_y)).size(11),
            slider(8.0..=60.0, self.height_y, Message::SetHeightY).step(1.0),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let smoothing_col = column![
            text(format!("圆角平滑(G2): {:.2}", self.smoothing)).size(11),
            slider(0.0..=1.0, self.smoothing, Message::SetSmoothing).step(0.05),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let blur_col = column![
            text(format!("模糊标准差: {:.1}px", self.blur)).size(11),
            slider(6.0..=30.0, self.blur, Message::SetBlur).step(0.5),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let opacity_col = column![
            text(format!("显隐透明度: {:.0}%", self.glass_opacity * 100.0)).size(11),
            slider(0.0..=1.0, self.glass_opacity, Message::SetGlassOpacity).step(0.01),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let tint_col = column![
            text(format!("纯乳化度: {:.0}%", self.tint_opacity * 100.0)).size(11),
            slider(0.0..=1.0, self.tint_opacity, Message::SetTintOpacity).step(0.01),
        ]
        .spacing(2)
        .width(Length::FillPortion(1));

        let sliders_row1 = row![clarity_col, p1_col, p2_col, p3_col, curve_plot_col]
            .spacing(12)
            .align_y(Alignment::Center)
            .width(Length::Fill);

        let sliders_row2 = row![
            amount_col,
            width_x_col,
            height_y_col,
            smoothing_col,
            blur_col,
            opacity_col,
            tint_col
        ]
        .spacing(8)
        .width(Length::Fill);

        let header = column![title_row, sliders_row1, sliders_row2].spacing(6);

        // Left Viewport: macOS 27 Native
        let left_viewport = column![
            text(match self.mac_mode {
                MacRefMode::Clear => "macOS 27 Native (官方真机·清晰态 t=0.0)",
                MacRefMode::Regular => "macOS 27 Native (官方真机·浓郁磨砂态 t=1.0)",
            })
            .size(13)
            .color(Color::from_rgb(0.35, 0.78, 0.98)),
            container(iced_image(&self.mac_handle).width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(6)
        .width(Length::FillPortion(1))
        .height(Length::Fill);

        // Right Viewport: liquid-rs Live
        let right_viewport = column![
            text("liquid-rs (实时 GPU 真实管线渲染)")
                .size(13)
                .color(Color::from_rgb(1.0, 0.45, 0.45)),
            container(iced_image(&self.liq_handle).width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(6)
        .width(Length::FillPortion(1))
        .height(Length::Fill);

        let viewports = row![left_viewport, right_viewport]
            .spacing(12)
            .width(Length::Fill)
            .height(Length::Fill);

        column![header, viewports]
            .spacing(6)
            .padding(iced::Padding { top: 36.0, right: 16.0, bottom: 16.0, left: 16.0 })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn theme(_state: &AppState) -> Theme {
    Theme::Dark
}

fn boot() -> (AppState, Task<Message>) {
    AppState::new()
}

fn main() -> iced::Result {
    iced::application::<AppState, Message, Theme, Renderer>(boot, AppState::update, AppState::view)
        .title("Liquid Glass Side-by-Side Comparison")
        .theme(theme)
        .window_size(iced::Size::new(1680.0, 780.0))
        .run()
}
