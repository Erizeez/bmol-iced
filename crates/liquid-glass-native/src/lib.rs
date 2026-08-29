//! Small native hooks that are not part of the renderer or the UI model.
//!
//! The macOS hook below deliberately only reapplies the window compositor's
//! blur radius. It does not install an `AppKit` visual-effect subview, which
//! would compete with a wgpu `CAMetalLayer` for the same content area.

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

#[cfg(target_os = "macos")]
mod macos {
    use std::{ffi::c_void, sync::OnceLock};

    use objc2_app_kit::NSView;
    use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};

    use super::DesktopBlurTarget;

    const DESKTOP_BLUR_RADIUS: i64 = 80;

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
        if window_number > 0
            && let Some((main_connection, set_blur)) = cgs_functions()
        {
            // A non-zero status is intentionally ignored: the next refresh
            // frame will retry while the Stage Manager transition settles.
            let _ = unsafe { set_blur(main_connection(), window_number, DESKTOP_BLUR_RADIUS) };
        }
    }
}
