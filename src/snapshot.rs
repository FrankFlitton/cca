//! Debug-only: render the window to a BMP and exit, for checking the UI
//! without screen-recording permission.
//!
//! `CCA_SNAPSHOT=out.bmp [CCA_THEME=light|dark] [CCA_SLIDERS=1] cargo run`

use std::path::PathBuf;

use eframe::egui::{self, ColorImage, ViewportCommand};

pub struct Snapshot {
    path: PathBuf,
    frame: u32,
}

impl Snapshot {
    pub fn from_env(ctx: &egui::Context) -> Option<Self> {
        let path = std::env::var_os("CCA_SNAPSHOT")?.into();
        match std::env::var("CCA_THEME").as_deref() {
            Ok("light") => ctx.set_theme(egui::Theme::Light),
            Ok("dark") => ctx.set_theme(egui::Theme::Dark),
            _ => {}
        }
        Some(Self { path, frame: 0 })
    }

    pub fn sliders() -> bool {
        std::env::var_os("CCA_SLIDERS").is_some()
    }

    pub fn tick(&mut self, ctx: &egui::Context) {
        self.frame += 1;
        // Let the window settle on its auto-sized height first.
        if self.frame == 10 {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        ctx.request_repaint();
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = shot {
            std::fs::write(&self.path, bmp(&image)).expect("write snapshot");
            std::process::exit(0);
        }
    }
}

/// Uncompressed 24-bit BMP.
fn bmp(img: &ColorImage) -> Vec<u8> {
    let [w, h] = img.size;
    let stride = (w * 3).div_ceil(4) * 4;
    let size = 54 + stride * h;
    let mut out = Vec::with_capacity(size);
    let u32le = |out: &mut Vec<u8>, v: usize| out.extend_from_slice(&(v as u32).to_le_bytes());
    out.extend_from_slice(b"BM");
    u32le(&mut out, size);
    u32le(&mut out, 0);
    u32le(&mut out, 54);
    u32le(&mut out, 40);
    u32le(&mut out, w);
    u32le(&mut out, h);
    out.extend_from_slice(&[1, 0, 24, 0]);
    for _ in 0..6 {
        u32le(&mut out, 0);
    }
    for y in (0..h).rev() {
        for x in 0..w {
            let c = img.pixels[y * w + x];
            out.extend_from_slice(&[c.b(), c.g(), c.r()]);
        }
        out.resize(out.len() + stride - w * 3, 0);
    }
    out
}
