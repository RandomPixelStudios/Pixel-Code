use std::sync::Arc;

use eframe::egui::{self, Color32, FontFamily, FontId, Stroke};

pub const BG: Color32 = Color32::from_rgb(0x09, 0x09, 0x0b);
pub const SURFACE: Color32 = Color32::from_rgb(0x0f, 0x0f, 0x12);
pub const PANE: Color32 = Color32::from_rgb(0x0f, 0x0f, 0x12);
pub const TERM_BG: Color32 = PANE;
pub const ELEVATED: Color32 = Color32::from_rgb(0x15, 0x15, 0x19);
pub const INPUT: Color32 = Color32::from_rgb(0x0b, 0x0b, 0x0e);
pub const BORDER: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x25);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x2c, 0x2c, 0x34);
pub const HOVER: Color32 = Color32::from_rgb(0x19, 0x19, 0x1e);
pub const SELECTED: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x26);
pub const ACCENT: Color32 = Color32::from_rgb(0xe4, 0xe4, 0xe7);
pub const GREEN: Color32 = Color32::from_rgb(0x34, 0xd3, 0x99);
pub const RED: Color32 = Color32::from_rgb(0xf8, 0x71, 0x71);
pub const AMBER: Color32 = Color32::from_rgb(0xfa, 0xcc, 0x15);
pub const BLUE: Color32 = Color32::from_rgb(0x60, 0xa5, 0xfa);
pub const TEXT: Color32 = Color32::from_rgb(0xed, 0xed, 0xef);
pub const MUTED: Color32 = Color32::from_rgb(0x8f, 0x8f, 0x99);
pub const FAINT: Color32 = Color32::from_rgb(0x5c, 0x5c, 0x66);

pub const RADIUS: u8 = 12;
/// Höhe der Kopfzeilen von Sidebar, Settings-Navigation und Terminals.
pub const HEAD: f32 = 36.0;

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}
pub fn medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("medium".into()))
}
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}
pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}
pub fn mono_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono-bold".into()))
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let data = [
        ("inter", &include_bytes!("../assets/fonts/Inter-Regular.ttf")[..]),
        ("inter-medium", &include_bytes!("../assets/fonts/Inter-Medium.ttf")[..]),
        ("inter-semibold", &include_bytes!("../assets/fonts/Inter-SemiBold.ttf")[..]),
        ("jbmono", &include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")[..]),
        ("jbmono-bold", &include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf")[..]),
    ];
    for (name, bytes) in data {
        fonts.font_data.insert(name.into(), Arc::new(egui::FontData::from_static(bytes)));
    }
    // Standard-Fonts von egui bleiben als Fallback (Symbole, Emoji) erhalten.
    let fallback_prop = fonts.families[&FontFamily::Proportional].clone();
    let fallback_mono = fonts.families[&FontFamily::Monospace].clone();
    let with = |first: &str, fb: &Vec<String>| {
        let mut v = vec![first.to_string()];
        v.extend(fb.iter().cloned());
        v
    };
    fonts.families.insert(FontFamily::Proportional, with("inter", &fallback_prop));
    fonts.families.insert(FontFamily::Name("medium".into()), with("inter-medium", &fallback_prop));
    fonts.families.insert(FontFamily::Name("semibold".into()), with("inter-semibold", &fallback_prop));
    fonts.families.insert(FontFamily::Monospace, with("jbmono", &fallback_mono));
    fonts.families.insert(FontFamily::Name("mono-bold".into()), with("jbmono-bold", &fallback_mono));
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    ctx.set_theme(egui::Theme::Dark);

    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = ELEVATED;
    v.window_stroke = Stroke::new(1.0, BORDER_STRONG);
    v.window_corner_radius = egui::CornerRadius::same(12);
    v.window_shadow = egui::Shadow { offset: [0, 10], blur: 30, spread: 0, color: Color32::from_black_alpha(140) };
    v.popup_shadow = egui::Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(120) };
    v.menu_corner_radius = egui::CornerRadius::same(10);
    v.extreme_bg_color = INPUT;
    v.faint_bg_color = SURFACE;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = Color32::from_rgb(0x3a, 0x3a, 0x42);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.text_cursor.stroke = Stroke::new(1.5, ACCENT);

    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    for (s, fill, stroke) in [
        (&mut w.inactive, Color32::TRANSPARENT, BORDER),
        (&mut w.hovered, HOVER, BORDER_STRONG),
        (&mut w.active, SELECTED, BORDER_STRONG),
        (&mut w.open, SELECTED, BORDER_STRONG),
    ] {
        s.weak_bg_fill = fill;
        s.bg_fill = if fill == Color32::TRANSPARENT { INPUT } else { fill };
        s.bg_stroke = Stroke::new(1.0, stroke);
        s.corner_radius = egui::CornerRadius::same(8);
        s.expansion = 0.0;
    }
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    w.active.fg_stroke = Stroke::new(1.0, TEXT);
    ctx.set_visuals(v);

    ctx.all_styles_mut(|s| {
        use egui::TextStyle::*;
        s.text_styles.insert(Body, font(13.5));
        s.text_styles.insert(Button, medium(13.0));
        s.text_styles.insert(Small, font(11.5));
        s.text_styles.insert(Heading, semibold(20.0));
        s.text_styles.insert(Monospace, mono(13.0));
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        s.spacing.menu_margin = egui::Margin::same(6);
        s.spacing.interact_size.y = 30.0;
    });
}
