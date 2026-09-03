//! Small native hooks that are not part of the renderer or the UI model.
//!
//! The macOS hooks below reapply the window compositor's blur radius and
//! optionally capture the pixels below the window for a wgpu glass shader.

#![allow(unsafe_code)]

use raw_window_handle::RawWindowHandle;

/// A retained identity for the native window that owns a desktop backdrop.
///
/// The value is intentionally opaque and contains no borrowed `AppKit` object,
/// so it can live alongside a graphics compositor between frames.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DesktopBlurTarget(usize);

/// Gets a native desktop-blur target from a window handle, when supported.
#[must_use]
pub fn desktop_blur_target(handle: RawWindowHandle) -> Option<DesktopBlurTarget> {
    #[cfg(target_os = "macos")]
    return macos::desktop_blur_target(handle);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = handle;
        None
    }
}

/// Reapplies the desktop blur associated with a transparent application
/// window, when the current platform exposes that operation.
///
/// On macOS, Stage Manager can rebuild a window's compositor state while the
/// window is being restored. Reapplying the radius around frame submission
/// keeps transparent pixels blurred instead of briefly showing a sharp desktop.
pub fn refresh_desktop_blur(target: DesktopBlurTarget) {
    #[cfg(target_os = "macos")]
    macos::refresh_desktop_blur(target);

    #[cfg(not(target_os = "macos"))]
    let _ = target;
}

/// Captures the pixels below a transparent application window for shader use.
///
/// The native window compositor can blur the desktop behind transparent
/// pixels, but it cannot expose those pixels to a custom wgpu shader. This
/// bridge supplies the missing source texture for refraction and dispersion.
/// `None` means that the platform does not support capture or macOS denied it
/// because Screen Recording permission is unavailable.
#[must_use]
pub fn capture_desktop_backdrop(target: DesktopBlurTarget) -> Option<(u32, u32, Vec<u8>)> {
    #[cfg(target_os = "macos")]
    return macos::capture_desktop_backdrop(target);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = target;
        None
    }
}

/// Installs notification observers that reapply the desktop blur after the
/// window server rebuilds the window's compositor state.
///
/// Stage Manager and Mission Control space transitions reset the private CGS
/// blur radius asynchronously, so a reapplication made inside a redraw can be
/// overwritten a few hundred milliseconds later — that is the visible "flash"
/// where the translucent surface briefly loses its blur. The observers fire on
/// Stage Manager session changes, space switches, app activation, and window
/// occlusion changes, and reapply the radius immediately plus a few delayed
/// times so the last write lands after the window server settles.
///
/// The guard is process-global and lives until the app exits; installing it
/// more than once is harmless but unnecessary.
pub fn install_stage_manager_guard(target: DesktopBlurTarget) {
    #[cfg(target_os = "macos")]
    macos::install_stage_manager_guard(target);

    #[cfg(not(target_os = "macos"))]
    let _ = target;
}

/// Renders a system symbol (SF Symbol) into a one-page vector PDF and returns
/// its bytes, when the current platform exposes named system symbols.
///
/// This is an asset-authoring hook used by developer tooling to extract
/// icons from the operating system; it is not called at app runtime. The PDF
/// page matches the symbol's natural canvas size in points and contains pure
/// vector path data filled in black, so it can be recolored and rescaled
/// freely. Returns `None` when the platform or the symbol name is unknown.
#[must_use]
pub fn system_symbol_pdf(name: &str) -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    return macos::system_symbol_pdf(name);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = name;
        None
    }
}

/// Extracts a named image asset from a compiled `.car` asset catalog as PNG
/// bytes, when the current platform supports asset catalogs.
///
/// System Settings keeps its bespoke icons (the Bluetooth rune, the Software
/// Update loop, ...) inside per-feature `.car` bundles. This reads them
/// through `CoreUI`'s private `CUICatalog`, which is how those apps load them.
/// `scale` selects the rendition density (pass 2.0 for retina). Returns
/// `None` when the catalog or the asset name is unknown.
#[must_use]
pub fn named_asset_png(car_path: &str, name: &str, scale: f64) -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    return macos::named_asset_png(car_path, name, scale);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (car_path, name, scale);
        None
    }
}

/// Renders a named vector glyph stored inside a compiled `.car` asset catalog
/// into a one-page vector PDF and returns its bytes.
///
/// Private SF Symbols (e.g. the Settings app's `appearance` glyph) are not
/// resolvable through `NSImage systemSymbolName:`; they ship as
/// `CUINamedVectorGlyph` renditions in `CoreGlyphsPrivate.bundle`. This reads
/// them through `CoreUI`'s private `CUICatalog` and draws the glyph into a
/// PDF context, yielding pure vector path data like [`system_symbol_pdf`].
/// Returns `None` when the catalog or the glyph name is unknown.
#[must_use]
pub fn glyph_pdf(car_path: &str, name: &str) -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    return macos::glyph_pdf(car_path, name);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (car_path, name);
        None
    }
}

/// Resolves a `com.apple.graphic-icon.*` type identifier to its icon and
/// returns PNG bytes of the largest rendition (1024×1024).
///
/// Some System Settings panes (Notifications, ...) declare their sidebar icon
/// as a graphic-icon type identifier instead of a local asset. `IconServices`
/// resolves those at runtime; `NSWorkspace iconForFileType:` is the public
/// funnel that reaches the same artwork. This is an asset-authoring hook used
/// by developer tooling, not called at app runtime. Returns `None` when the
/// identifier is unknown.
#[must_use]
pub fn graphic_icon_png(type_identifier: &str) -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    return macos::graphic_icon_png(type_identifier);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = type_identifier;
        None
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::{
        ffi::c_void,
        ptr::NonNull,
        sync::{
            OnceLock,
            atomic::{AtomicU64, Ordering},
        },
    };

    use block2::RcBlock;
    use objc2::{
        AllocAnyThread, MainThreadMarker, msg_send,
        rc::Retained,
        runtime::{AnyClass, AnyObject},
        sel,
    };
    use objc2_app_kit::{
        NSApplication, NSApplicationDidBecomeActiveNotification, NSBitmapImageFileType,
        NSBitmapImageRep, NSImage, NSView, NSWindowDidChangeOcclusionStateNotification, NSWorkspace,
        NSWorkspaceActiveSpaceDidChangeNotification, NSWorkspaceSessionDidBecomeActiveNotification,
    };
    use objc2_core_foundation::{
        CGPoint, CGRect, CGSize, CFRetained, CFString, CFURL, CFURLPathStyle,
    };
    use objc2_core_graphics::{
        CGContext, CGImage, CGPDFContextBeginPage, CGPDFContextClose, CGPDFContextCreateWithURL,
        CGPDFContextEndPage,
    };
    use objc2_foundation::{NSDictionary, NSNotification, NSNotificationCenter, NSString, NSURL};
    use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};

    use super::DesktopBlurTarget;
    use core_graphics::{
        base::kCGImageAlphaPremultipliedLast,
        color_space::CGColorSpace,
        context::CGContext as LegacyCGContext,
        display::CGDisplay,
        geometry::{CGPoint as LegacyCGPoint, CGRect as LegacyCGRect, CGSize as LegacyCGSize},
        window::{
            create_image, kCGWindowImageBestResolution,
            kCGWindowListOptionOnScreenBelowWindow,
        },
    };

    // Keep the OS backdrop readable. The glass shader supplies the local
    // optical blur; a large window-wide compositor radius makes the entire
    // demo look like a frosted screenshot before the shader even runs.
    const DESKTOP_BLUR_RADIUS: i64 = 28;

    // These are the same private compositor entry points used by winit's
    // Window::set_blur(true). They are intentionally isolated to this native
    // adapter so the rest of the workspace remains safe Rust. Loading the
    // symbols at runtime also keeps builds independent of an SDK's private
    // SkyLight link stub.
    unsafe extern "C" {
        fn dlopen(filename: *const i8, flags: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
    }

    type CgsMainConnectionId = unsafe extern "C" fn() -> *mut c_void;
    type CgsSetWindowBackgroundBlurRadius = unsafe extern "C" fn(*mut c_void, isize, i64) -> i32;

    unsafe extern "C" {
        fn CGPreflightScreenCaptureAccess() -> bool;
    }

    static CGS_FUNCTIONS: OnceLock<
        Option<(CgsMainConnectionId, CgsSetWindowBackgroundBlurRadius)>,
    > = OnceLock::new();

    fn cgs_functions() -> Option<(CgsMainConnectionId, CgsSetWindowBackgroundBlurRadius)> {
        *CGS_FUNCTIONS.get_or_init(|| unsafe {
            let framework = dlopen(
                c"/System/Library/PrivateFrameworks/SkyLight.framework/SkyLight".as_ptr().cast(),
                1,
            );
            if framework.is_null() {
                return None;
            }

            let main_connection = dlsym(framework, c"CGSMainConnectionID".as_ptr().cast());
            let set_blur = dlsym(framework, c"CGSSetWindowBackgroundBlurRadius".as_ptr().cast());
            if main_connection.is_null() || set_blur.is_null() {
                return None;
            }

            Some((
                std::mem::transmute::<*mut c_void, CgsMainConnectionId>(main_connection),
                std::mem::transmute::<*mut c_void, CgsSetWindowBackgroundBlurRadius>(set_blur),
            ))
        })
    }

    pub fn desktop_blur_target(handle: RawWindowHandle) -> Option<DesktopBlurTarget> {
        let RawWindowHandle::AppKit(appkit_handle) = handle else {
            return None;
        };

        let AppKitWindowHandle { ns_view, .. } = appkit_handle;
        Some(DesktopBlurTarget(ns_view.as_ptr() as usize))
    }

    pub fn refresh_desktop_blur(target: DesktopBlurTarget) {
        let Some(ns_view) = std::ptr::NonNull::new(target.0 as *mut c_void) else {
            return;
        };
        // The target originates from winit's AppKit handle and is used only
        // on the AppKit main thread while its window is alive.
        let view: &NSView = unsafe { ns_view.cast().as_ref() };
        let Some(window) = view.window() else {
            return;
        };

        // objc2 marks this selector unsafe because it relies on the object
        // being a live NSWindow. `view.window()` supplies that live object.
        let window_number = window.windowNumber();
        reapply_window_blur(window_number);
    }

    pub fn capture_desktop_backdrop(target: DesktopBlurTarget) -> Option<(u32, u32, Vec<u8>)> {
        // Avoid synchronously entering ScreenCaptureKit when the user has
        // not granted Screen Recording permission. On recent macOS releases
        // that call can otherwise block the UI event loop for several seconds.
        if !unsafe { CGPreflightScreenCaptureAccess() } {
            return None;
        }
        let ns_view = std::ptr::NonNull::new(target.0 as *mut c_void)?;
        let view: &NSView = unsafe { ns_view.cast().as_ref() };
        let window = view.window()?;
        let frame = window.frame();
        if frame.size.width <= 0.0 || frame.size.height <= 0.0 {
            return None;
        }

        // Capture only windows below this one. This avoids feeding the
        // previous frame of the app back into its own glass texture.
        // AppKit reports window frames in a bottom-left coordinate system,
        // while CGWindowListCreateImage uses Quartz's top-left global
        // coordinates. Passing the AppKit y value directly captures a
        // different desktop strip, which is especially obvious along the
        // sidebar where the wallpaper has strong vertical features.
        let display_bounds = CGDisplay::main().bounds();
        let quartz_y = display_bounds.origin.y + display_bounds.size.height
            - frame.origin.y
            - frame.size.height;
        let bounds = LegacyCGRect::new(
            &LegacyCGPoint::new(frame.origin.x, quartz_y),
            &LegacyCGSize::new(frame.size.width, frame.size.height),
        );
        let image = create_image(
            bounds,
            kCGWindowListOptionOnScreenBelowWindow,
            window.windowNumber().try_into().ok()?,
            kCGWindowImageBestResolution,
        )?;
        let width = image.width();
        let height = image.height();
        if width == 0 || height == 0 {
            return None;
        }

        // Normalize CoreGraphics' native pixel format to tightly packed RGBA
        // so the renderer does not need platform-specific channel handling.
        let color_space = CGColorSpace::create_device_rgb();
        let context = LegacyCGContext::create_bitmap_context(
            None,
            width,
            height,
            8,
            width.saturating_mul(4),
            &color_space,
            kCGImageAlphaPremultipliedLast,
        );
        context.draw_image(
            LegacyCGRect::new(
                &LegacyCGPoint::new(0.0, 0.0),
                &LegacyCGSize::new(width as f64, height as f64),
            ),
            &image,
        );
        let normalized = context.create_image()?;
        Some((
            u32::try_from(width).ok()?,
            u32::try_from(height).ok()?,
            normalized.data().bytes().to_vec(),
        ))
    }

    /// Writes the blur radius through the private CGS entry points.
    ///
    /// This only touches the window-server connection, never an `AppKit`
    /// object, so it is safe to call from any thread.
    fn reapply_window_blur(window_number: isize) {
        if window_number > 0
            && let Some((main_connection, set_blur)) = cgs_functions()
        {
            // A non-zero status is intentionally ignored: the next refresh
            // frame will retry while the Stage Manager transition settles.
            let _ = unsafe { set_blur(main_connection(), window_number, DESKTOP_BLUR_RADIUS) };
        }
    }

    /// Monotonic token that lets a newer blur reapplication burst supersede
    /// the delayed writes of an older one.
    static REAPPLICATION_GENERATION: AtomicU64 = AtomicU64::new(0);

    /// Reapplies the blur now and again after short delays.
    ///
    /// The window server rebuilds a window's compositor state asynchronously
    /// after a Stage Manager or space transition, so an immediate write can be
    /// overwritten before the transition settles. The delayed burst makes the
    /// last write win.
    fn schedule_blur_reapplication(window_number: isize) {
        reapply_window_blur(window_number);
        let generation = REAPPLICATION_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        std::thread::spawn(move || {
            for delay in [150_u64, 250, 400] {
                std::thread::sleep(std::time::Duration::from_millis(delay));
                if REAPPLICATION_GENERATION.load(Ordering::SeqCst) != generation {
                    return;
                }
                reapply_window_blur(window_number);
            }
        });
    }

    pub fn install_stage_manager_guard(target: DesktopBlurTarget) {
        let Some(ns_view) = std::ptr::NonNull::new(target.0 as *mut c_void) else {
            return;
        };
        // The target originates from winit's AppKit handle and is used only
        // on the AppKit main thread while its window is alive.
        let view: &NSView = unsafe { ns_view.cast().as_ref() };
        let Some(window) = view.window() else {
            return;
        };
        let window_number = window.windowNumber();
        if window_number <= 0 {
            return;
        }

        // The block captures only the window number, so it never dereferences
        // an `AppKit` object from the notification's posting thread.
        let handler = RcBlock::new(move |_notification: NonNull<NSNotification>| {
            schedule_blur_reapplication(window_number);
        });

        let workspace_center = NSWorkspace::sharedWorkspace().notificationCenter();
        let default_center = NSNotificationCenter::defaultCenter();

        // Safety: every notification name is a valid AppKit constant and the
        // block signature matches the observer contract. The observers are
        // deliberately leaked: the guard must outlive every window-server
        // transition for the whole app lifetime.
        let observers = unsafe {
            [
                workspace_center.addObserverForName_object_queue_usingBlock(
                    Some(NSWorkspaceActiveSpaceDidChangeNotification),
                    None,
                    None,
                    &handler,
                ),
                workspace_center.addObserverForName_object_queue_usingBlock(
                    Some(NSWorkspaceSessionDidBecomeActiveNotification),
                    None,
                    None,
                    &handler,
                ),
                default_center.addObserverForName_object_queue_usingBlock(
                    Some(NSApplicationDidBecomeActiveNotification),
                    None,
                    None,
                    &handler,
                ),
                default_center.addObserverForName_object_queue_usingBlock(
                    Some(NSWindowDidChangeOcclusionStateNotification),
                    None,
                    None,
                    &handler,
                ),
            ]
        };
        std::mem::forget(observers);
    }

    /// Draws a `CUINamedVectorGlyph` into a one-page PDF and returns its
    /// bytes. The glyph draws in native pixel space (natural size × its
    /// `scale`), so the context is pre-scaled to land inside `page_size`.
    fn render_glyph_pdf(glyph: &AnyObject, page_size: CGSize) -> Option<Vec<u8>> {
        let scale: f64 = unsafe { msg_send![glyph, scale] };
        if !(scale.is_finite() && scale > 0.0) {
            return None;
        }

        let path = std::env::temp_dir().join(format!(
            "liquid-glass-glyph-{}.pdf",
            std::process::id(),
        ));
        let path_string = CFString::from_str(path.to_str()?);
        let url = CFURL::with_file_system_path(
            None,
            Some(&path_string),
            CFURLPathStyle::CFURLPOSIXPathStyle,
            false,
        )?;

        let page = CGRect::new(CGPoint::new(0.0, 0.0), page_size);
        // Safety: `url` and `page` are valid; the returned context is released
        // by the CFRetained wrapper. `glyph` is a live CUINamedVectorGlyph and
        // the draw happens between balanced page calls on the main thread.
        let context: CFRetained<_> =
            unsafe { CGPDFContextCreateWithURL(Some(&url), &raw const page, None) }?;
        unsafe {
            CGPDFContextBeginPage(Some(&context), None);
            CGContext::scale_ctm(Some(&context), 1.0 / scale, 1.0 / scale);
            let () = msg_send![glyph, drawInContext: CFRetained::as_ptr(&context).as_ptr()];
            CGPDFContextEndPage(Some(&context));
            CGPDFContextClose(Some(&context));
        }
        drop(context);

        let bytes = std::fs::read(&path).ok();
        let _ = std::fs::remove_file(&path);
        bytes
    }

    pub fn system_symbol_pdf(name: &str) -> Option<Vec<u8>> {
        // Symbol image lookup works without a running event loop, but AppKit
        // still expects the shared application object to exist. Both this and
        // the draw below must happen on the main thread.
        let main_thread = MainThreadMarker::new()?;
        let _app = NSApplication::sharedApplication(main_thread);

        let symbol_name = NSString::from_str(name);
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            &symbol_name,
            None,
        )?;
        let representations = image.representations();
        let rep = representations.firstObject()?;

        // NSSymbolImageRep rasterizes when drawn. The CoreUI vector glyph it
        // keeps in the private `_vectorGlyph` ivar does not: drawing it into a
        // PDF context emits pure path operators. This is the same entry point
        // the MIT-licensed `sfsym` tool uses, stable on macOS 13+.
        let key = NSString::from_str("_vectorGlyph");
        let glyph: Option<Retained<AnyObject>> =
            unsafe { msg_send![&*rep, valueForKey: &*key] };
        let glyph = glyph?;

        render_glyph_pdf(&glyph, rep.size())
    }

    /// Opens a compiled `.car` asset catalog through `CoreUI`'s private
    /// `CUICatalog`, which is how system apps load them. Loading the framework
    /// at runtime keeps linking clean.
    fn open_catalog(car_path: &str) -> Option<Retained<AnyObject>> {
        let framework = unsafe {
            dlopen(c"/System/Library/PrivateFrameworks/CoreUI.framework/CoreUI".as_ptr(), 1)
        };
        if framework.is_null() {
            return None;
        }
        let class = AnyClass::get(c"CUICatalog")?;

        let path = NSString::from_str(car_path);
        let url = NSURL::fileURLWithPath_isDirectory(&path, false);

        // CUICatalog has both initWithURL: and initWithURL:error: depending on
        // the OS release; prefer the throwing variant when present.
        // Safety: `class` is the live CUICatalog class; alloc/init return +1
        // objects, and `from_raw` adopts that retain count. When init fails it
        // has already consumed the allocated object per Cocoa convention.
        unsafe {
            let raw: *mut AnyObject = msg_send![class, alloc];
            if raw.is_null() {
                return None;
            }
            let use_error_variant: bool =
                msg_send![class, instancesRespondToSelector: sel!(initWithURL:error:)];
            let catalog: *mut AnyObject = if use_error_variant {
                let mut error: *mut AnyObject = std::ptr::null_mut();
                msg_send![raw, initWithURL: &*url, error: &raw mut error]
            } else {
                msg_send![raw, initWithURL: &*url]
            };
            if catalog.is_null() {
                return None;
            }
            Retained::from_raw(catalog)
        }
    }

    pub fn glyph_pdf(car_path: &str, name: &str) -> Option<Vec<u8>> {
        let main_thread = MainThreadMarker::new()?;
        let _app = NSApplication::sharedApplication(main_thread);
        let catalog = open_catalog(car_path)?;

        let asset_name = NSString::from_str(name);
        // Private glyphs carry no idiom/size/weight rendition matrix in
        // practice; probe the sensible combinations and take the first hit.
        // Regular weight is 0 in CoreUI's glyph key; size 1 is medium.
        // Safety: `catalog` is a live CUICatalog; returned objects are +0 and
        // retained through Retained's msg_send handling.
        let glyph = [(0_isize, 1_isize, 0_isize), (0, 0, 0), (0, 2, 0), (5, 1, 0)]
            .into_iter()
            .find_map(|(idiom, size, weight)| unsafe {
                msg_send![
                    &*catalog,
                    namedVectorGlyphWithName: &*asset_name,
                    scaleFactor: 2.0_f64,
                    deviceIdiom: idiom,
                    layoutDirection: 0_isize,
                    glyphSize: size,
                    glyphWeight: weight,
                    glyphPointSize: 64.0_f64,
                    appearanceName: Option::<&NSString>::None,
                    locale: Option::<&NSString>::None
                ]
            });
        let glyph: Retained<AnyObject> = glyph?;

        // The SVG downstream hugs the ink bounds, so a generous canvas is
        // fine; 64pt artwork at 2x lands well inside it.
        render_glyph_pdf(&glyph, CGSize::new(160.0, 160.0))
    }

    pub fn named_asset_png(car_path: &str, name: &str, scale: f64) -> Option<Vec<u8>> {
        let main_thread = MainThreadMarker::new()?;
        let _app = NSApplication::sharedApplication(main_thread);
        let catalog = open_catalog(car_path)?;

        let asset_name = NSString::from_str(name);
        // `imageWithName:` returns a CUINamedImage, which is NOT an NSImage
        // subclass: it wraps a CGImage vendored through its `image` property.
        // Icon-set assets ("Icon Image" type) need the icon variant instead.
        // Fall back to scale 1.0 when the requested density has no rendition.
        // Safety: `catalog` is a live CUICatalog; every returned object is
        // retained by msg_send's return handling.
        let named = unsafe {
            let named: Option<Retained<AnyObject>> =
                msg_send![&*catalog, imageWithName: &*asset_name, scaleFactor: scale];
            let named = match named {
                Some(named) => Some(named),
                None => msg_send![&*catalog, imageWithName: &*asset_name, scaleFactor: 1.0],
            };
            if let Some(named) = named {
                Some(named)
            } else {
                msg_send![
                    &*catalog,
                    iconImageWithName: &*asset_name,
                    scaleFactor: scale,
                    displayGamut: 0_isize,
                    layoutDirection: 0_isize,
                    desiredSize: CGSize::new(64.0, 64.0)
                ]
            }
        }?;
        // Safety: `image` is a live CUINamedImage; the returned CGImage is
        // owned by it and used only within this scope.
        let cg_image: *mut CGImage = unsafe { msg_send![&*named, image] };
        if cg_image.is_null() {
            return None;
        }
        let bitmap = NSBitmapImageRep::initWithCGImage(
            NSBitmapImageRep::alloc(),
            unsafe { &*cg_image },
        );
        let properties = NSDictionary::<NSString>::new();
        let data = unsafe {
            bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
        }?;
        Some(data.to_vec())
    }

    pub fn graphic_icon_png(type_identifier: &str) -> Option<Vec<u8>> {
        let main_thread = MainThreadMarker::new()?;
        let _app = NSApplication::sharedApplication(main_thread);

        let workspace = NSWorkspace::sharedWorkspace();
        let identifier = NSString::from_str(type_identifier);
        // iconForFileType: is deprecated in favor of UTType-based lookups, but
        // those do not cover com.apple.graphic-icon.* identifiers; this still
        // reaches the same IconServices artwork.
        // Safety: `workspace` is the live shared NSWorkspace; the returned
        // NSImage is retained through Retained's msg_send handling.
        let image: Option<Retained<NSImage>> =
            unsafe { msg_send![&*workspace, iconForFileType: &*identifier] };
        let image = image?;
        // TIFFRepresentation flattens the best (1024px) rendition.
        let tiff = image.TIFFRepresentation()?;
        let bitmap = NSBitmapImageRep::initWithData(NSBitmapImageRep::alloc(), &tiff)?;
        let properties = NSDictionary::<NSString>::new();
        let data = unsafe {
            bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
        }?;
        Some(data.to_vec())
    }
}
