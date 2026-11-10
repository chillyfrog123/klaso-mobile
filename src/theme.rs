use crate::i18n::tr;
use eframe::egui::{self, Color32, Stroke, Vec2};
use serde::{Deserialize, Serialize};

pub const TEXT: Color32 = Color32::from_rgb(237, 240, 246);
pub const MUTED: Color32 = Color32::from_rgb(163, 174, 192);
pub const GREEN: Color32 = Color32::from_rgb(107, 224, 178);
pub const RED: Color32 = Color32::from_rgb(255, 140, 153);

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Accent {
    #[default]
    Cobalt,
    Emerald,
    Violet,
    Amber,
}
impl Accent {
    pub const ALL: [Self; 4] = [Self::Cobalt, Self::Emerald, Self::Violet, Self::Amber];
    pub fn name(self) -> &'static str {
        match self {
            Self::Cobalt => tr!("Kobalt", "Cobalt"),
            Self::Emerald => tr!("Szmaragd", "Emerald"),
            Self::Violet => tr!("Fiolet", "Violet"),
            Self::Amber => tr!("Bursztyn", "Amber"),
        }
    }
    pub fn color(self) -> Color32 {
        match self {
            Self::Cobalt => Color32::from_rgb(102, 164, 255),
            Self::Emerald => Color32::from_rgb(78, 219, 170),
            Self::Violet => Color32::from_rgb(182, 148, 255),
            Self::Amber => Color32::from_rgb(255, 194, 99),
        }
    }
    pub fn load() -> Self {
        config_path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
    pub fn save(self) -> std::io::Result<()> {
        if let Some(path) = config_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, serde_json::to_vec(&self)?)?;
        }
        Ok(())
    }
}
fn config_path() -> Option<std::path::PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|p| {
        std::path::PathBuf::from(p)
            .join("LibrusLite")
            .join("appearance.json")
    })
}
pub fn blend(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        (a.r() as f32 * (1. - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1. - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1. - t) + b.b() as f32 * t) as u8,
    )
}
pub fn glass(_accent: Accent) -> egui::Frame {
    egui::Frame::none()
        .fill(Color32::from_rgba_unmultiplied(25, 30, 41, 247))
        .stroke(Stroke::new(1.0_f32, Color32::from_white_alpha(18)))
        .rounding(8.)
        .inner_margin(18.)
}
pub fn inset() -> egui::Frame {
    egui::Frame::none()
        .fill(Color32::from_rgba_unmultiplied(18, 23, 33, 240))
        .stroke(Stroke::new(1.0_f32, Color32::from_white_alpha(12)))
        .rounding(5.)
        .inner_margin(14.)
}
pub fn apply(ctx: &egui::Context, accent: Accent) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(12., 8.);
    style.spacing.button_padding = Vec2::new(16., 10.);
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = Color32::from_rgb(20, 30, 48);
    style.visuals.extreme_bg_color = Color32::from_rgb(12, 21, 35);
    style.visuals.faint_bg_color = Color32::from_white_alpha(5);
    style.visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(1.0_f32, Color32::from_white_alpha(13));
    style.visuals.selection.bg_fill = accent.color().gamma_multiply(0.3);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, accent.color());
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widget.rounding = egui::Rounding::same(5.);
    }
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 43, 62);
    style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(30, 43, 62);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_white_alpha(22));
    style.visuals.widgets.hovered.bg_fill =
        blend(Color32::from_rgb(32, 46, 66), accent.color(), 0.17);
    style.visuals.widgets.hovered.weak_bg_fill = style.visuals.widgets.hovered.bg_fill;
    style.visuals.widgets.active.weak_bg_fill =
        blend(Color32::from_rgb(32, 46, 66), accent.color(), 0.26);
    style.visuals.widgets.hovered.bg_stroke =
        Stroke::new(1.0_f32, accent.color().gamma_multiply(0.7));
    style.visuals.hyperlink_color = accent.color();
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(15.));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(15.));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(25.));
    ctx.set_style(style);
}
pub fn fonts(ctx: &egui::Context) {
    if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\segoeui.ttf") {
        let mut fonts = egui::FontDefinitions::default();
        fonts
            .font_data
            .insert("segoe".into(), egui::FontData::from_owned(bytes));
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "segoe".into());
        ctx.set_fonts(fonts);
    }
}
pub fn background(ctx: &egui::Context, accent: Accent) {
    let rect = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    let base = Color32::from_rgb(14, 18, 26);
    let mut mesh = egui::Mesh::default();
    for (pos, color) in [
        (rect.left_top(), base),
        (rect.right_top(), blend(base, accent.color(), 0.045)),
        (rect.right_bottom(), base),
        (rect.left_bottom(), base),
    ] {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}
pub fn badge(ui: &mut egui::Ui, label: &str, color: Color32) {
    egui::Frame::none()
        .fill(color.gamma_multiply(0.13))
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.3)))
        .rounding(4.)
        .inner_margin(egui::Margin::symmetric(8., 4.))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).size(12.).color(color));
        });
}
