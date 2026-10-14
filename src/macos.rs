//! macOS window tweaks so auto-resizing doesn't flicker.
//!
//! When the window grows or shrinks, Core Animation shows the previous frame
//! for a moment before the new one arrives. By default it stretches that frame
//! to the new size (`kCAGravityResize`), which looks like a flicker. Pinning the
//! layer contents to the top-left keeps the old frame sharp. Painting the window
//! background in the paper colour makes the newly revealed strip blend in.

use eframe::egui::Color32;
use objc2_app_kit::{NSColor, NSView};
use objc2_quartz_core::{CALayer, kCAGravityTopLeft};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Returns true once the render layer was found and configured.
pub fn pin_contents(frame: &eframe::Frame, background: Color32) -> bool {
    let Ok(handle) = frame.window_handle() else {
        return false;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return false;
    };
    // SAFETY: winit hands out a valid NSView that lives as long as the window,
    // and we are on the main thread (inside the UI callback).
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };

    if let Some(window) = view.window() {
        let c = |v: u8| f64::from(v) / 255.0;
        let (r, g, b) = (c(background.r()), c(background.g()), c(background.b()));
        let color = NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0);
        window.setBackgroundColor(Some(&color));
    }

    let Some(layer) = view.layer() else {
        return false;
    };
    let mut pinned = false;
    let mut pin = |layer: &CALayer| {
        // SAFETY: reading an immutable framework constant.
        layer.setContentsGravity(unsafe { kCAGravityTopLeft });
        pinned = true;
    };
    pin(&layer);
    // wgpu renders into a CAMetalLayer added as a sublayer of the view's layer.
    // SAFETY: only reads the layer tree; the array is not mutated while iterating.
    if let Some(sublayers) = unsafe { layer.sublayers() } {
        for sub in sublayers.iter() {
            pin(&sub);
        }
    }
    pinned
}
