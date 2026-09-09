use iced_wgpu::wgpu;
use liquid_glass::UiColorScheme;

/// Uploads tightly packed RGBA8 pixels into a fresh 2D texture.
fn upload_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
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
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    texture
}

/// Reference grid backdrop used by the upstream comparison demos.
///
/// Requires the `studio-assets` feature, which embeds assets from a local
/// `liquid-glass-studio` checkout (see the crate README). Without that feature
/// the crate compiles standalone and this entry point is not built.
#[cfg(feature = "studio-assets")]
#[allow(clippy::cast_precision_loss, dead_code)]
pub fn reference_grid_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> (wgpu::Texture, f32) {
    const REFERENCE_GRID: &[u8] =
        include_bytes!("../../../liquid-glass-studio/src/assets/bg-grid.png");
    let image = image::load_from_memory(REFERENCE_GRID)
        .expect("reference bg-grid.png must decode")
        .to_rgba8();
    let (width, height) = image.dimensions();
    let texture = upload_rgba8(
        device,
        queue,
        "liquid-glass reference bg-grid texture",
        width,
        height,
        image.as_raw(),
    );
    (texture, width as f32 / height as f32)
}

/// Builds a wallpaper fallback for renderer previews.
///
/// The non-macOS Settings example uses this as a deterministic shader-source
/// fallback when a platform cannot provide a captured desktop frame to the
/// GPU. macOS deliberately leaves the source transparent until it has a real
/// desktop capture, so it cannot show a mismatched wallpaper through glass.
///
/// With the `studio-assets` feature the Tahoe wallpapers from the local
/// `liquid-glass-studio` checkout are embedded. Without it a procedurally
/// generated vertical gradient is used, so the crate has no third-party
/// binary asset dependency by default.
#[cfg(feature = "studio-assets")]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
pub fn settings_background_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scheme: UiColorScheme,
) -> (wgpu::Texture, f32) {
    const LIGHT_WALLPAPER: &[u8] =
        include_bytes!("../../../liquid-glass-studio/src/assets/bg-tahoe-light.webp");
    const DARK_WALLPAPER: &[u8] =
        include_bytes!("../../../liquid-glass-studio/src/assets/bg-tahoe-dark.webp");
    let wallpaper = match scheme {
        UiColorScheme::Light => LIGHT_WALLPAPER,
        UiColorScheme::Dark => DARK_WALLPAPER,
    };
    let image =
        image::load_from_memory(wallpaper).expect("bundled Tahoe wallpaper must decode").to_rgba8();
    let (width, height) = image.dimensions();
    let texture = upload_rgba8(
        device,
        queue,
        "liquid-glass wallpaper fallback",
        width,
        height,
        image.as_raw(),
    );
    (texture, width as f32 / height as f32)
}

/// Builds a procedural wallpaper fallback for renderer previews.
///
/// See [`settings_background_texture`] for the role of this backdrop; this
/// variant generates the gradient in memory so the crate builds without the
/// `studio-assets` feature and without any checked-in third-party imagery.
#[cfg(not(feature = "studio-assets"))]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
pub fn settings_background_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scheme: UiColorScheme,
) -> (wgpu::Texture, f32) {
    const WIDTH: u32 = 512;
    const HEIGHT: u32 = 512;

    let (top, bottom): ([f32; 3], [f32; 3]) = match scheme {
        UiColorScheme::Light => ([0.949, 0.961, 0.980], [0.596, 0.678, 0.804]),
        UiColorScheme::Dark => ([0.129, 0.141, 0.169], [0.024, 0.027, 0.039]),
    };

    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for y in 0..HEIGHT {
        let t = y as f32 / (HEIGHT - 1) as f32;
        let row = [
            top[0] + (bottom[0] - top[0]) * t,
            top[1] + (bottom[1] - top[1]) * t,
            top[2] + (bottom[2] - top[2]) * t,
        ];
        for _ in 0..WIDTH {
            pixels.push((row[0] * 255.0) as u8);
            pixels.push((row[1] * 255.0) as u8);
            pixels.push((row[2] * 255.0) as u8);
            pixels.push(255);
        }
    }

    let texture = upload_rgba8(
        device,
        queue,
        "liquid-glass procedural wallpaper fallback",
        WIDTH,
        HEIGHT,
        &pixels,
    );
    (texture, WIDTH as f32 / HEIGHT as f32)
}
