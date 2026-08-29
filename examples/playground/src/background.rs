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

/// Builds an optional wallpaper fallback for renderer previews.
///
/// The Tahoe wallpapers are kept in the checked-in reference project so the
/// standalone renderer can be exercised without an OS desktop backdrop. The
/// Settings example does not use this fallback: its transparent window leaves
/// the actual desktop to the platform compositor.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
#[allow(dead_code)]
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
    let pixels = image.into_raw();

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("liquid-glass wallpaper fallback"),
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
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    (texture, width as f32 / height as f32)
}
