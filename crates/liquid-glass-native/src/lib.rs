//! Small native hooks that are not part of the renderer or the UI model.
//!
//! The macOS hook below deliberately only reapplies the window compositor's
//! blur radius. It does not install an `AppKit` visual-effect subview, which
//! would compete with a wgpu `CAMetalLayer` for the same content area.

#![allow(unsafe_code)]

use raw_window_handle::RawWindowHandle;

/// Reapplies the desktop blur associated with a transparent application
/// window, when the current platform exposes that operation.
///
/// On macOS, Stage Manager can rebuild a window's compositor state while the
/// window is being restored. Reapplying the radius after focus changes keeps
/// transparent pixels blurred instead of briefly showing a sharp desktop.
pub fn refresh_desktop_blur(handle: RawWindowHandle) {
    #[cfg(target_os = "macos")]
    macos::refresh_desktop_blur(handle);

    #[cfg(not(target_os = "macos"))]
    let _ = handle;
}

#[cfg(target_os = "macos")]
mod macos {
    use std::{ffi::c_void, sync::OnceLock};

    use objc2_app_kit::NSView;
    use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};

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

    pub fn refresh_desktop_blur(handle: RawWindowHandle) {
        let RawWindowHandle::AppKit(appkit_handle) = handle else {
            return;
        };

        let AppKitWindowHandle { ns_view, .. } = appkit_handle;
        // The handle is provided by winit and is valid for the duration of
        // this synchronous callback on the AppKit main thread.
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
