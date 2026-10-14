#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod color;
#[cfg(target_os = "macos")]
mod macos;
mod picker;
#[cfg(debug_assertions)]
mod snapshot;
mod theme;

use color::{Format, Model, Rgba, Wcag};
use eframe::egui::{
    self, Align2, Color32, EventFilter, Id, Key, Margin, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, Vec2, ViewportCommand, WidgetInfo, WidgetType, epaint::Mesh, pos2, vec2,
};
use picker::Picker;
use theme::{Palette, bold, regular};

const WIDTH: f32 = 304.0;
const MARGIN: f32 = 12.0;
/// Width of the content column inside the margins.
const CONTENT: f32 = WIDTH - 2.0 * MARGIN;
/// Left edge of the content column; labels sit in the column before it.
const COL: f32 = 78.0;
const ICON: f32 = 22.0;
/// Header height when the traffic-light position is unknown (and off macOS).
const HEADER: f32 = 28.0;
/// Fallback room left for the macOS traffic-light buttons.
const TRAFFIC_LIGHTS: f32 = if cfg!(target_os = "macos") { 66.0 } else { 0.0 };

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Contrast")
            .with_inner_size([WIDTH, 260.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
                    .expect("bundled icon is a valid PNG"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "Contrast",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            #[allow(unused_mut)]
            let mut app = App::default();
            #[cfg(debug_assertions)]
            {
                app.snapshot = snapshot::Snapshot::from_env(&cc.egui_ctx);
                if app.snapshot.is_some() && snapshot::Snapshot::sliders() {
                    (app.fg.sliders, app.bg.sliders) = (true, true);
                }
            }
            Ok(Box::new(app))
        }),
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Which {
    Fg,
    Bg,
}

struct Slot {
    color: Rgba,
    text: String,
    text_valid: bool,
    sliders: bool,
    /// Last slider values, so hue/saturation survive passing through greys.
    cache: Option<(Model, [f32; 3])>,
}

impl Slot {
    fn new(color: Rgba) -> Self {
        Self { color, text: String::new(), text_valid: true, sliders: false, cache: None }
    }

    fn set_rgb(&mut self, rgb: [u8; 3], format: Format) {
        self.color.rgb = rgb;
        self.cache = None;
        self.text = color::format(self.color, format);
        self.text_valid = true;
    }
}

struct App {
    fg: Slot,
    bg: Slot,
    format: Format,
    model: Model,
    on_top: bool,
    picker: Picker,
    pick_target: Which,
    copied_at: Option<f64>,
    height: f32,
    /// Paper colour last applied to the native window (macOS).
    native_bg: Option<Color32>,
    /// Header height and title inset, fitted to the traffic lights.
    header: (f32, f32),
    header_measured: bool,
    #[cfg(debug_assertions)]
    snapshot: Option<snapshot::Snapshot>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            fg: Slot::new(Rgba::opaque(0x57, 0x33, 0x7A)),
            bg: Slot::new(Rgba::opaque(0x23, 0x83, 0x91)),
            format: Format::Hex,
            model: Model::Rgb,
            on_top: false,
            picker: Picker::default(),
            pick_target: Which::Fg,
            copied_at: None,
            height: 0.0,
            native_bg: None,
            header: (HEADER, TRAFFIC_LIGHTS),
            header_measured: false,
            #[cfg(debug_assertions)]
            snapshot: None,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        if let Some(rgb) = self.picker.take() {
            let slot = match self.pick_target {
                Which::Fg => &mut self.fg,
                Which::Bg => &mut self.bg,
            };
            slot.set_rgb(rgb, self.format);
        }

        let pal = theme::palette(ui);
        if self.native_bg != Some(pal.paper) {
            #[cfg(target_os = "macos")]
            if macos::pin_contents(frame, pal.paper) {
                self.native_bg = Some(pal.paper);
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = frame;
                self.native_bg = Some(pal.paper);
            }
        }
        #[cfg(target_os = "macos")]
        if !self.header_measured
            && let Some(t) = macos::traffic_lights(frame)
        {
            // Centre the header band on the buttons, and start the title one
            // button-gap after the last button.
            self.header = ((2.0 * t.center_y).round(), t.right + t.gap - MARGIN);
            self.header_measured = true;
        }
        ui.painter().rect_filled(ui.ctx().content_rect(), 0.0, pal.paper);
        let margin = Margin { left: 12, right: 12, top: 0, bottom: 10 };
        let used = egui::Frame::new()
            .inner_margin(margin)
            .show(ui, |ui| {
                ui.set_width(CONTENT);
                self.contents(ui, pal);
            })
            .response
            .rect
            .height()
            .ceil();

        // Keep the window exactly as tall as its content.
        if (used - self.height).abs() > 0.5 {
            self.height = used;
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::InnerSize(vec2(WIDTH, used)));
        }
        #[cfg(debug_assertions)]
        if let Some(s) = &mut self.snapshot {
            s.tick(ui.ctx());
        }
    }
}

impl App {
    fn contents(&mut self, ui: &mut Ui, pal: Palette) {
        self.header(ui, pal);
        rule(ui, 2.0, pal.ink);

        let format = self.format;
        let fg_seen = blend32(self.fg.color, self.bg.color.rgb);
        let bg32 = to32(self.bg.color.rgb);
        let bg_rgb = self.bg.color.rgb;
        let fg_spec = SlotSpec { label: "Foreground", swatch: fg_seen, alpha_over: Some(bg_rgb) };
        if slot_ui(ui, pal, fg_spec, &mut self.fg, format, &mut self.model) {
            self.pick_target = Which::Fg;
            self.picker.start(ui.ctx());
        }
        rule(ui, 1.0, pal.rule);
        let bg_spec = SlotSpec { label: "Background", swatch: bg32, alpha_over: None };
        if slot_ui(ui, pal, bg_spec, &mut self.bg, format, &mut self.model) {
            self.pick_target = Which::Bg;
            self.picker.start(ui.ctx());
        }

        ui.add_space(4.0);
        preview(ui, pal, fg_seen, bg32);
        ui.add_space(10.0);
        rule(ui, 2.0, pal.ink);
        ui.add_space(6.0);
        results(ui, pal, color::contrast(self.fg.color, bg_rgb));
        ui.add_space(8.0);
        rule(ui, 1.0, pal.rule);
        ui.add_space(6.0);
        self.footer(ui, pal);
    }

    fn header(&mut self, ui: &mut Ui, pal: Palette) {
        let (rect, resp) =
            ui.allocate_exact_size(vec2(CONTENT, self.header.0), Sense::click_and_drag());
        // The header replaces the title bar, so it has to move the window too.
        if resp.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }
        ui.painter().text(
            pos2(rect.left() + self.header.1, rect.center().y),
            Align2::LEFT_CENTER,
            "Contrast",
            bold(13.0),
            pal.ink,
        );

        let icon = |i: f32| {
            Rect::from_min_size(
                pos2(rect.right() - ICON * (i + 1.0) - 4.0 * i, rect.center().y - ICON / 2.0),
                Vec2::splat(ICON),
            )
        };
        if icon_button(ui, pal, icon(0.0), Icon::Pin, self.on_top, "Keep window on top").clicked() {
            self.on_top = !self.on_top;
            let level = if self.on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            };
            ui.ctx().send_viewport_cmd(ViewportCommand::WindowLevel(level));
        }
        let tip = "Swap foreground and background";
        if icon_button(ui, pal, icon(1.0), Icon::Swap, false, tip).clicked() {
            let (fg, bg) = (self.fg.color.rgb, self.bg.color.rgb);
            self.fg.set_rgb(bg, self.format);
            self.bg.set_rgb(fg, self.format);
        }
    }

    fn footer(&mut self, ui: &mut Ui, pal: Palette) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            for f in Format::ALL {
                let tip = "Format shown in the fields. Any format can be typed.";
                if text_button(ui, pal, f.label(), self.format == f).on_hover_text(tip).clicked() {
                    self.format = f;
                }
            }

            let now = ui.input(|i| i.time);
            let copied = self.copied_at.is_some_and(|t| now - t < 1.5);
            if copied {
                ui.ctx().request_repaint_after_secs(0.25);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if copied { "Copied" } else { "Copy results" };
                if text_button(ui, pal, label, false).clicked() {
                    ui.ctx().copy_text(self.report());
                    self.copied_at = Some(now);
                }
            });
        });
    }

    fn report(&self) -> String {
        let ratio = color::contrast(self.fg.color, self.bg.color.rgb);
        let w = Wcag::new(ratio);
        let pf = |ok: bool| if ok { "Pass" } else { "Fail" };
        format!(
            "Foreground: {}\nBackground: {}\nContrast ratio: {}\n\
             1.4.3 Contrast (Minimum) (AA): {} (regular text), {} (large text)\n\
             1.4.6 Contrast (Enhanced) (AAA): {} (regular text), {} (large text)\n\
             1.4.11 Non-text Contrast (AA): {} (UI components and graphical objects)\n",
            color::format(self.fg.color, self.format),
            color::format(self.bg.color, self.format),
            color::ratio_text(ratio),
            pf(w.aa_text),
            pf(w.aa_large),
            pf(w.aaa_text),
            pf(w.aaa_large),
            pf(w.non_text),
        )
    }
}

fn to32([r, g, b]: [u8; 3]) -> Color32 {
    Color32::from_rgb(r, g, b)
}

fn blend32(fg: Rgba, bg: [u8; 3]) -> Color32 {
    to32(color::blend(fg, bg).map(|c| (c * 255.0).round() as u8))
}

/// A full-width horizontal rule.
fn rule(ui: &mut Ui, thickness: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(CONTENT, thickness), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, color);
}

/// Rect spanning `x0..x1` (relative to `row`'s left edge), `h` tall, centred vertically.
fn cell(row: Rect, x0: f32, x1: f32, h: f32) -> Rect {
    Rect::from_min_max(
        pos2(row.left() + x0, row.center().y - h / 2.0),
        pos2(row.left() + x1, row.center().y + h / 2.0),
    )
}

struct SlotSpec {
    label: &'static str,
    swatch: Color32,
    /// For the foreground: the background its alpha is composited over.
    alpha_over: Option<[u8; 3]>,
}

/// One colour row (label, swatch, field, buttons) plus its optional sliders.
/// Returns true when the eyedropper was clicked.
fn slot_ui(
    ui: &mut Ui,
    pal: Palette,
    spec: SlotSpec,
    slot: &mut Slot,
    format: Format,
    model: &mut Model,
) -> bool {
    let name = spec.label.to_lowercase();
    let w = CONTENT;
    let (row, _) = ui.allocate_exact_size(vec2(w, 36.0), Sense::hover());
    let p = ui.painter().clone();

    p.text(
        pos2(row.left(), row.center().y),
        Align2::LEFT_CENTER,
        spec.label,
        regular(11.5),
        pal.muted,
    );
    let swatch = cell(row, COL, COL + 14.0, 14.0);
    p.rect_filled(swatch, 0.0, spec.swatch);
    p.rect_stroke(swatch, 0.0, Stroke::new(1.0, pal.ink), StrokeKind::Inside);

    // Field.
    let field = cell(row, COL + 22.0, w - 2.0 * ICON - 12.0, 20.0);
    let text_color = if slot.text_valid { pal.ink } else { pal.accent };
    let edit = egui::TextEdit::singleline(&mut slot.text)
        .frame(egui::Frame::NONE)
        .font(regular(15.0))
        .text_color(text_color)
        .desired_width(field.width());
    let resp = ui.place(field, edit).on_hover_text(format!(
        "The {name} colour. Type hex, rgb(), hsl() or hsv(){}.",
        if spec.alpha_over.is_some() { ", optionally with alpha" } else { "" }
    ));
    // Underline only on hover/focus; at rest the row's hairline is enough.
    let underline = if resp.has_focus() || !slot.text_valid {
        Some(Stroke::new(2.0, text_color))
    } else if resp.hovered() {
        Some(Stroke::new(1.0, pal.ink))
    } else {
        None
    };
    if let Some(stroke) = underline {
        let y = field.bottom() + 1.0;
        p.line_segment([pos2(field.left(), y), pos2(field.right(), y)], stroke);
    }
    if resp.changed() {
        match color::parse(&slot.text) {
            Some(mut c) => {
                if spec.alpha_over.is_none() {
                    c.a = 1.0;
                }
                slot.color = c;
                slot.cache = None;
                slot.text_valid = true;
            }
            None => slot.text_valid = false,
        }
    }
    if !resp.has_focus() {
        slot.text = color::format(slot.color, format);
        slot.text_valid = true;
    }

    // Buttons.
    let tip = format!("Adjust the {name} with sliders");
    let sliders_rect = cell(row, w - ICON, w, ICON);
    if icon_button(ui, pal, sliders_rect, Icon::Sliders, slot.sliders, &tip).clicked() {
        slot.sliders = !slot.sliders;
    }
    let mut pick = false;
    if Picker::AVAILABLE {
        let tip = format!("Pick the {name} from the screen");
        let pick_rect = cell(row, w - 2.0 * ICON - 4.0, w - ICON - 4.0, ICON);
        pick = icon_button(ui, pal, pick_rect, Icon::Eyedropper, false, &tip).clicked();
    }

    if slot.sliders {
        sliders(ui, pal, spec.label, slot, model, spec.alpha_over);
        ui.add_space(8.0);
    }
    pick
}

fn sliders(
    ui: &mut Ui,
    pal: Palette,
    salt: &str,
    slot: &mut Slot,
    model: &mut Model,
    alpha_over: Option<[u8; 3]>,
) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.add_space(COL + 22.0 - 10.0);
        for m in Model::ALL {
            if text_button(ui, pal, m.label(), *model == m).clicked() {
                *model = m;
            }
        }
    });
    ui.add_space(4.0);

    let mut vals = match slot.cache {
        Some((m, v)) if m == *model && m.to_rgb(v) == slot.color.rgb => v,
        _ => model.channels_of(slot.color.rgb),
    };

    // One slider row; `gradient` gives the colour at each position 0..=1.
    let row = |ui: &mut Ui,
               name: &str,
               value: &mut f32,
               max: f32,
               decimals: usize,
               gradient: &dyn Fn(f32) -> Color32| {
        let w = CONTENT;
        let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
        ui.painter().text(
            pos2(r.left() + COL + 3.0, r.center().y),
            Align2::LEFT_CENTER,
            name,
            bold(11.0),
            pal.muted,
        );
        let step = 10f32.powi(-(decimals as i32));
        let track = cell(r, COL + 22.0, w - 48.0, 12.0);
        let id = ui.id().with(("slider", salt, name));
        let mut changed = channel_slider(ui, pal, track, id, value, max, step, name, gradient);
        let drag = egui::DragValue::new(value)
            .range(0.0..=max)
            .fixed_decimals(decimals)
            .speed(step * if decimals == 0 { 1.0 } else { 0.5 });
        changed |= ui.place(cell(r, w - 38.0, w, 18.0), drag).changed();
        changed
    };

    let mut changed = false;
    for (i, (name, max)) in model.channels().into_iter().enumerate() {
        let (m, base) = (*model, vals);
        let gradient = move |t: f32| {
            let mut v = base;
            v[i] = t * max;
            to32(m.to_rgb(v))
        };
        changed |= row(ui, name, &mut vals[i], max, 0, &gradient);
    }
    if changed {
        slot.color.rgb = model.to_rgb(vals);
        slot.cache = Some((*model, vals));
    }
    if let Some(bg) = alpha_over {
        let rgb = slot.color.rgb;
        let gradient = move |t: f32| blend32(Rgba { rgb, a: t }, bg);
        row(ui, "A", &mut slot.color.a, 1.0, 2, &gradient);
    }
}

/// A flat slider whose track previews the colour along the channel.
#[allow(clippy::too_many_arguments)]
fn channel_slider(
    ui: &mut Ui,
    pal: Palette,
    track: Rect,
    id: Id,
    value: &mut f32,
    max: f32,
    step: f32,
    label: &str,
    gradient: &dyn Fn(f32) -> Color32,
) -> bool {
    let resp = ui.interact(track.expand2(vec2(3.0, 5.0)), id, Sense::click_and_drag());
    let mut v = *value;
    if resp.is_pointer_button_down_on()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        v = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0) * max;
    }
    if resp.has_focus() {
        let filter = EventFilter { horizontal_arrows: true, ..Default::default() };
        ui.memory_mut(|m| m.set_focus_lock_filter(id, filter));
        let (left, right, shift) = ui.input(|i| {
            (i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight), i.modifiers.shift)
        });
        let d = step * if shift { 10.0 } else { 1.0 };
        v += d * (f32::from(u8::from(right)) - f32::from(u8::from(left)));
    }
    if v != *value {
        v = ((v / step).round() * step).clamp(0.0, max);
    }
    let changed = v != *value;
    *value = v;
    resp.widget_info(|| WidgetInfo::slider(true, f64::from(v), label));
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }

    let p = ui.painter();
    let mut mesh = Mesh::default();
    const STEPS: u32 = 48;
    for i in 0..=STEPS {
        let t = i as f32 / STEPS as f32;
        let x = track.left() + t * track.width();
        let c = gradient(t);
        mesh.colored_vertex(pos2(x, track.top()), c);
        mesh.colored_vertex(pos2(x, track.bottom()), c);
        if i > 0 {
            let k = 2 * i;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k, k + 1);
        }
    }
    p.add(mesh);
    let border = if resp.has_focus() {
        Stroke::new(1.5, pal.ink)
    } else {
        Stroke::new(1.0, pal.rule)
    };
    p.rect_stroke(track, 0.0, border, StrokeKind::Outside);

    let x = track.left() + (v / max) * track.width();
    let handle = Rect::from_center_size(pos2(x, track.center().y), vec2(4.0, track.height() + 8.0));
    p.rect_filled(handle, 0.0, pal.ink);
    p.rect_stroke(handle, 0.0, Stroke::new(1.0, pal.paper), StrokeKind::Outside);
    changed
}

fn preview(ui: &mut Ui, pal: Palette, fg: Color32, bg: Color32) {
    let (rect, resp) = ui.allocate_exact_size(vec2(CONTENT, 52.0), Sense::hover());
    resp.on_hover_text("Sample: regular text, large text and a graphical object");
    let p = ui.painter().with_clip_rect(rect);
    p.rect_filled(rect, 0.0, bg);
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, pal.rule), StrokeKind::Inside);

    let y = rect.center().y;
    let left = pos2(rect.left() + 12.0, y);
    let r = p.text(left, Align2::LEFT_CENTER, "Regular text", regular(14.0), fg);
    p.text(pos2(r.right() + 14.0, y), Align2::LEFT_CENTER, "Large", bold(24.0), fg);

    // A small bar chart as the "graphical object" sample.
    let base = pos2(rect.right() - 44.0, y + 11.0);
    for (i, h) in [9.0, 16.0, 12.0, 22.0].into_iter().enumerate() {
        let x = base.x + i as f32 * 8.0;
        let bar = Rect::from_min_max(pos2(x, base.y - h), pos2(x + 5.0, base.y));
        p.rect_filled(bar, 0.0, fg);
    }
}

fn results(ui: &mut Ui, pal: Palette, ratio: f32) {
    let w = Wcag::new(ratio);
    let (rect, resp) = ui.allocate_exact_size(vec2(CONTENT, 64.0), Sense::hover());
    let pf = |ok: bool| if ok { "pass" } else { "fail" };
    let summary = format!(
        "Contrast ratio {}. Text: AA {}, AAA {}. Large text: AA {}, AAA {}. Graphics: AA {}.",
        color::ratio_text(ratio),
        pf(w.aa_text),
        pf(w.aaa_text),
        pf(w.aa_large),
        pf(w.aaa_large),
        pf(w.non_text)
    );
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &summary));
    let p = ui.painter();

    // Ratio: the number in ink, the ":1" receding.
    let text = color::ratio_text(ratio);
    let (num, unit) = text.split_at(text.find(':').unwrap_or(text.len()));
    let origin = pos2(rect.left() - 1.0, rect.top() - 5.0);
    let r = p.text(origin, Align2::LEFT_TOP, num, bold(40.0), pal.ink);
    p.text(r.right_top(), Align2::LEFT_TOP, unit, bold(40.0), pal.muted);
    let caption = "Contrast ratio";
    p.text(rect.left_bottom(), Align2::LEFT_BOTTOM, caption, regular(10.5), pal.muted);

    // WCAG table, right-aligned to the content edge.
    const ROW: f32 = 16.0;
    let (c_label, c_aa, c_aaa) = (rect.right() - 136.0, rect.right() - 64.0, rect.right() - 26.0);
    for (x, head) in [(c_aa, "AA"), (c_aaa, "AAA")] {
        p.text(pos2(x, rect.top()), Align2::LEFT_TOP, head, regular(10.5), pal.muted);
    }
    type Check = (bool, &'static str);
    let rows: [(&str, Check, Option<Check>); 3] = [
        (
            "Text",
            (w.aa_text, "1.4.3 Contrast (Minimum) needs 4.5:1"),
            Some((w.aaa_text, "1.4.6 Contrast (Enhanced) needs 7:1")),
        ),
        (
            "Large text",
            (w.aa_large, "1.4.3 Contrast (Minimum) needs 3:1"),
            Some((w.aaa_large, "1.4.6 Contrast (Enhanced) needs 4.5:1")),
        ),
        (
            "Graphics",
            (w.non_text, "1.4.11 Non-text Contrast needs 3:1 for UI components and graphics"),
            None,
        ),
    ];
    for (i, (label, aa, aaa)) in rows.into_iter().enumerate() {
        let y = rect.top() + ROW * (i as f32 + 1.0);
        p.text(pos2(c_label, y), Align2::LEFT_TOP, label, regular(12.0), pal.ink);
        for (x, result) in [(c_aa, Some(aa)), (c_aaa, aaa)] {
            let Some((pass, criterion)) = result else {
                p.text(pos2(x, y), Align2::LEFT_TOP, "—", regular(12.0), pal.muted);
                continue;
            };
            let (word, color) = if pass { ("Pass", pal.ink) } else { ("Fail", pal.accent) };
            let r = p.text(pos2(x, y), Align2::LEFT_TOP, word, bold(12.0), color);
            ui.interact(r.expand(2.0), resp.id.with((i, x as i32)), Sense::hover())
                .on_hover_text(format!("{word}: {criterion}"));
        }
    }
}

/// A typographic toggle: bold label, muted until hovered or selected, underlined when selected.
fn text_button(ui: &mut Ui, pal: Palette, text: &str, selected: bool) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), bold(11.0), pal.ink);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + vec2(0.0, 5.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, text));
    let color = if selected || resp.hovered() { pal.ink } else { pal.muted };
    let p = ui.painter();
    p.galley_with_override_text_color(rect.left_top(), galley, color);
    if selected {
        let bar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.right_bottom());
        p.rect_filled(bar, 0.0, pal.ink);
    } else if resp.has_focus() {
        p.rect_stroke(rect.expand(2.0), 0.0, Stroke::new(1.0, pal.ink), StrokeKind::Outside);
    }
    resp
}

#[derive(Clone, Copy)]
enum Icon {
    Swap,
    Pin,
    Eyedropper,
    Sliders,
}

/// Point at the fractional position `(x, y)` inside `r`.
fn at(r: Rect, x: f32, y: f32) -> Pos2 {
    pos2(r.left() + x * r.width(), r.top() + y * r.height())
}

/// Square icon button; icons are drawn as vectors (egui's bundled fonts lack these glyphs).
fn icon_button(
    ui: &mut Ui,
    pal: Palette,
    rect: Rect,
    icon: Icon,
    selected: bool,
    tip: &str,
) -> Response {
    let resp = ui.interact(rect, ui.id().with(tip), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, tip));

    let p = ui.painter();
    let color = if selected {
        p.rect_filled(rect, 0.0, pal.ink);
        pal.paper
    } else {
        if resp.hovered() || resp.is_pointer_button_down_on() {
            p.rect_filled(rect, 0.0, pal.wash);
        }
        pal.ink
    };
    if resp.has_focus() {
        p.rect_stroke(rect, 0.0, Stroke::new(1.5, pal.ink), StrokeKind::Outside);
    }

    let r = Rect::from_center_size(rect.center(), Vec2::splat(12.0));
    let s = Stroke::new(1.4, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([at(r, a.0, a.1), at(r, b.0, b.1)], s);
    };
    match icon {
        Icon::Swap => {
            line((0.28, 1.0), (0.28, 0.05));
            line((0.03, 0.3), (0.28, 0.03));
            line((0.53, 0.3), (0.28, 0.03));
            line((0.72, 0.0), (0.72, 0.95));
            line((0.47, 0.7), (0.72, 0.97));
            line((0.97, 0.7), (0.72, 0.97));
        }
        Icon::Pin => {
            line((0.2, 0.05), (0.8, 0.05));
            p.rect_filled(Rect::from_min_max(at(r, 0.32, 0.05), at(r, 0.68, 0.5)), 0.0, color);
            line((0.1, 0.55), (0.9, 0.55));
            line((0.5, 0.55), (0.5, 1.05));
        }
        Icon::Eyedropper => {
            // Bulb top-right, barrel along the diagonal, fine tip bottom-left.
            let bulb = Rect::from_center_size(at(r, 0.82, 0.18), Vec2::splat(4.6));
            p.rect_filled(bulb, 0.0, color);
            p.line_segment([at(r, 0.5, 0.2), at(r, 0.8, 0.5)], Stroke::new(1.8, color));
            p.line_segment([at(r, 0.7, 0.3), at(r, 0.25, 0.75)], Stroke::new(3.0, color));
            line((0.25, 0.75), (0.02, 0.98));
        }
        Icon::Sliders => {
            for (y, knob) in [(0.12, 0.7), (0.5, 0.28), (0.88, 0.6)] {
                line((0.0, y), (1.0, y));
                let k = Rect::from_center_size(at(r, knob, y), vec2(3.0, 5.0));
                p.rect_filled(k, 0.0, color);
            }
        }
    }
    resp.on_hover_text(tip)
}
