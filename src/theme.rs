//! Swiss typographic theme: one grotesk (Inter), ink on paper, a single red
//! accent, hairline rules and square corners.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
    Theme, Visuals, style::HandleShape,
};

#[derive(Clone, Copy)]
pub struct Palette {
    pub paper: Color32,
    pub ink: Color32,
    /// Secondary text (labels, units).
    pub muted: Color32,
    /// Hairline rules and input underlines.
    pub rule: Color32,
    /// Hover/pressed backgrounds.
    pub wash: Color32,
    /// Failing results and invalid input.
    pub accent: Color32,
}

pub const LIGHT: Palette = Palette {
    paper: Color32::from_rgb(0xF6, 0xF5, 0xF1),
    ink: Color32::from_rgb(0x11, 0x11, 0x11),
    muted: Color32::from_rgb(0x66, 0x66, 0x66),
    rule: Color32::from_rgb(0xC9, 0xC7, 0xC0),
    wash: Color32::from_rgb(0xE6, 0xE4, 0xDE),
    accent: Color32::from_rgb(0xD0, 0x02, 0x1B),
};

pub const DARK: Palette = Palette {
    paper: Color32::from_rgb(0x14, 0x14, 0x14),
    ink: Color32::from_rgb(0xF2, 0xF1, 0xED),
    muted: Color32::from_rgb(0x9A, 0x9A, 0x9A),
    rule: Color32::from_rgb(0x3C, 0x3C, 0x3C),
    wash: Color32::from_rgb(0x2A, 0x2A, 0x2A),
    accent: Color32::from_rgb(0xFF, 0x45, 0x3A),
};

pub fn palette(ui: &egui::Ui) -> Palette {
    if ui.visuals().dark_mode { DARK } else { LIGHT }
}

const BOLD: &str = "bold";

pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD.into()))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in [
        ("Inter-Regular", &include_bytes!("../assets/fonts/Inter-Regular.ttf")[..]),
        ("Inter-Bold", &include_bytes!("../assets/fonts/Inter-Bold.ttf")[..]),
    ] {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
    }
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "Inter-Regular".into());
    let mut bold_family = proportional.clone();
    bold_family[0] = "Inter-Bold".into();
    fonts.families.insert(FontFamily::Name(BOLD.into()), bold_family);
    ctx.set_fonts(fonts);

    ctx.set_visuals_of(Theme::Light, visuals(LIGHT, false));
    ctx.set_visuals_of(Theme::Dark, visuals(DARK, true));
    ctx.all_styles_mut(|s| {
        s.text_styles = [
            (TextStyle::Small, regular(10.5)),
            (TextStyle::Body, regular(12.5)),
            (TextStyle::Button, regular(12.5)),
            (TextStyle::Heading, bold(15.0)),
            (TextStyle::Monospace, FontId::monospace(12.0)),
        ]
        .into();
        s.spacing.item_spacing = egui::vec2(6.0, 0.0);
        s.spacing.button_padding = egui::vec2(4.0, 1.0);
        s.spacing.interact_size.y = 18.0;
        s.interaction.tooltip_delay = 0.4;
    });
}

fn visuals(p: Palette, dark: bool) -> Visuals {
    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = p.paper;
    v.window_fill = p.paper;
    v.extreme_bg_color = p.paper;
    v.faint_bg_color = p.wash;
    v.window_stroke = Stroke::new(1.0, p.ink);
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.selection.bg_fill = p.accent.gamma_multiply(0.3);
    v.selection.stroke = Stroke::new(1.5, p.ink);
    v.text_cursor.stroke = Stroke::new(1.5, p.ink);
    v.error_fg_color = p.accent;
    v.handle_shape = HandleShape::Rect { aspect_ratio: 0.4 };

    let w = &mut v.widgets;
    for (state, fill, stroke) in [
        (&mut w.noninteractive, p.paper, Stroke::new(1.0, p.rule)),
        (&mut w.inactive, p.paper, Stroke::new(1.0, p.rule)),
        (&mut w.hovered, p.wash, Stroke::new(1.0, p.ink)),
        (&mut w.active, p.wash, Stroke::new(1.5, p.ink)),
        (&mut w.open, p.wash, Stroke::new(1.0, p.ink)),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = stroke;
        state.fg_stroke = Stroke::new(1.0, p.ink);
        state.corner_radius = CornerRadius::ZERO;
        state.expansion = 0.0;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::{Rgba, contrast};

    fn ratio(a: Color32, b: Color32) -> f32 {
        contrast(Rgba::opaque(a.r(), a.g(), a.b()), [b.r(), b.g(), b.b()])
    }

    /// A contrast checker should pass its own audit.
    #[test]
    fn palettes_meet_wcag_aa() {
        for p in [LIGHT, DARK] {
            assert!(ratio(p.ink, p.paper) >= 7.0);
            assert!(ratio(p.muted, p.paper) >= 4.5);
            assert!(ratio(p.accent, p.paper) >= 4.5);
            assert!(ratio(p.ink, p.wash) >= 7.0);
            assert!(ratio(p.rule, p.paper) >= 1.4);
        }
    }
}
