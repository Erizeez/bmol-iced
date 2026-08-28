use iced_wgpu::wgpu;
use liquid_glass::UiColorScheme;

#[allow(dead_code)]
const REFERENCE_GRID: &[u8] = include_bytes!("../../../liquid-glass-studio/src/assets/bg-grid.png");

#[allow(clippy::cast_precision_loss)]
#[allow(dead_code)]
pub fn reference_grid_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> (wgpu::Texture, f32) {
    let image = image::load_from_memory(REFERENCE_GRID)
        .expect("reference bg-grid.png must decode")
        .to_rgba8();
    let (width, height) = image.dimensions();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("liquid-glass reference bg-grid texture"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        image.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    (texture, width as f32 / height as f32)
}

/// Builds the neutral split-view background used by the Settings example.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
#[allow(dead_code)]
pub fn settings_background_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scheme: UiColorScheme,
) -> (wgpu::Texture, f32) {
    const WIDTH: u32 = 1_320;
    const HEIGHT: u32 = 760;
    const SIDEBAR_WIDTH: u32 = 232;

    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let sidebar = x < SIDEBAR_WIDTH;
            let vertical = y as f32 / HEIGHT as f32;
            let grain = (((x.wrapping_mul(17) ^ y.wrapping_mul(31)) & 7) as f32 - 3.5) / 255.0;
            let (mut red, mut green, mut blue) = match (scheme, sidebar) {
                (UiColorScheme::Light, true) => (0.900, 0.900, 0.920),
                (UiColorScheme::Light, false) => (0.955, 0.955, 0.970),
                (UiColorScheme::Dark, true) => (0.145, 0.145, 0.155),
                (UiColorScheme::Dark, false) => (0.105, 0.105, 0.115),
            };
            let gradient = match scheme {
                UiColorScheme::Light => (0.5 - vertical) * 0.018,
                UiColorScheme::Dark => (0.5 - vertical) * 0.012,
            };
            red += gradient + grain * 0.28;
            green += gradient + grain * 0.25;
            blue += gradient + grain * 0.22;

            if x == SIDEBAR_WIDTH - 1 {
                let divider = match scheme {
                    UiColorScheme::Light => 0.78,
                    UiColorScheme::Dark => 0.22,
                };
                red = divider;
                green = divider;
                blue = divider;
            }

            pixels.extend_from_slice(&[
                (red.clamp(0.0, 1.0) * 255.0).round() as u8,
                (green.clamp(0.0, 1.0) * 255.0).round() as u8,
                (blue.clamp(0.0, 1.0) * 255.0).round() as u8,
                255,
            ]);
        }
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("liquid-glass settings split-view background"),
        size: wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * WIDTH),
            rows_per_image: Some(HEIGHT),
        },
        wgpu::Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
    );
    (texture, WIDTH as f32 / HEIGHT as f32)
}
