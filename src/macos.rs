//! macOS window tweaks.
//!
//! Flicker: when the window grows or shrinks, Core Animation shows the previous
//! frame for a moment before the new one arrives. By default it stretches that
//! frame to the new size (`kCAGravityResize`), which looks like a flicker.
//! Pinning the layer contents to the top-left keeps the old frame sharp.
//! Painting the window background in the paper colour makes the newly revealed
//! strip blend in.
//!
//! Title bar: the header shares the title bar with the traffic-light buttons,
//! whose position varies between macOS versions, so it is read from AppKit.

use eframe::egui::Color32;
use objc2_app_kit::{NSColor, NSView, NSWindowButton};
use objc2_quartz_core::{CALayer, kCAGravityTopLeft};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

fn view(frame: &eframe::Frame) -> Option<&NSView> {
    let handle = frame.window_handle().ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // SAFETY: winit hands out a valid NSView that lives as long as the window,
    // and we are on the main thread (inside the UI callback).
    Some(unsafe { handle.ns_view.cast().as_ref() })
}

/// Returns true once the render layer was found and configured.
pub fn pin_contents(frame: &eframe::Frame, background: Color32) -> bool {
    let Some(view) = view(frame) else {
        return false;
    };

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

/// Where the traffic-light buttons sit, in points from the window's top-left.
#[derive(Clone, Copy, Debug)]
pub struct TrafficLights {
    /// Vertical centre of the buttons.
    pub center_y: f32,
    /// Right edge of the last (zoom) button.
    pub right: f32,
    /// Horizontal gap between neighbouring buttons.
    pub gap: f32,
}

pub fn traffic_lights(frame: &eframe::Frame) -> Option<TrafficLights> {
    let window = view(frame)?.window()?;
    let height = window.frame().size.height;
    // Each button's rect in window coordinates (origin bottom-left; with a
    // full-size content view the window and content share one coordinate space).
    let rect = |which| {
        let button = window.standardWindowButton(which)?;
        Some(button.convertRect_toView(button.bounds(), None))
    };
    let close = rect(NSWindowButton::CloseButton)?;
    let mini = rect(NSWindowButton::MiniaturizeButton)?;
    let zoom = rect(NSWindowButton::ZoomButton)?;
    Some(TrafficLights {
        center_y: (height - (close.origin.y + close.size.height / 2.0)) as f32,
        right: (zoom.origin.x + zoom.size.width) as f32,
        gap: (mini.origin.x - (close.origin.x + close.size.width)) as f32,
    })
}
