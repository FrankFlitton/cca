//! Screen colour picker (eyedropper).
//!
//! On macOS this uses the system `NSColorSampler` loupe, which needs no
//! screen-recording permission. Other platforms have no picker yet.

#[cfg(target_os = "macos")]
mod imp {
    use std::cell::Cell;
    use std::rc::Rc;

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2_app_kit::{NSColor, NSColorSampler, NSColorSpace};

    #[derive(Default)]
    pub struct Picker {
        // Kept alive for the duration of the sampling session.
        sampler: Option<Retained<NSColorSampler>>,
        result: Rc<Cell<Option<[u8; 3]>>>,
    }

    impl Picker {
        pub const AVAILABLE: bool = true;

        /// Shows the system loupe; the picked colour is later returned by [`Self::take`].
        pub fn start(&mut self, ctx: &eframe::egui::Context) {
            let result = self.result.clone();
            let ctx = ctx.clone();
            let handler = RcBlock::new(move |color: *mut NSColor| {
                // SAFETY: AppKit passes either nil (cancelled) or a valid NSColor.
                if let Some(color) = unsafe { color.as_ref() }
                    && let Some(c) = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())
                {
                    let q = |v: f64| (v * 255.0).round().clamp(0.0, 255.0) as u8;
                    result.set(Some([
                        q(c.redComponent()),
                        q(c.greenComponent()),
                        q(c.blueComponent()),
                    ]));
                }
                ctx.request_repaint();
            });
            let sampler = NSColorSampler::new();
            // SAFETY: called on the main thread (the UI thread) with a valid block.
            unsafe { sampler.showSamplerWithSelectionHandler(&handler) };
            self.sampler = Some(sampler);
        }

        pub fn take(&mut self) -> Option<[u8; 3]> {
            self.result.take()
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    #[derive(Default)]
    pub struct Picker;

    impl Picker {
        pub const AVAILABLE: bool = false;

        pub fn start(&mut self, _ctx: &eframe::egui::Context) {}

        pub fn take(&mut self) -> Option<[u8; 3]> {
            None
        }
    }
}

pub use imp::Picker;
