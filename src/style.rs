//! Colours, fonts, themes and cover drawing.

use eframe::egui::{
    self, pos2, text::LayoutJob, vec2, Align, Color32, CornerRadius, FontFamily, FontId, Rect,
    Stroke, StrokeKind, TextFormat, TextureHandle, Vec2,
};
use std::sync::{Arc, OnceLock};

pub const ACCENT: Color32 = Color32::from_rgb(110, 168, 254);
pub const ON_ACCENT: Color32 = Color32::from_rgb(12, 22, 40);
pub const BG: Color32 = Color32::from_rgb(19, 21, 25);
pub const SURFACE: Color32 = Color32::from_rgb(29, 32, 38);
pub const SURFACE2: Color32 = Color32::from_rgb(44, 48, 57);
pub const TEXT: Color32 = Color32::from_rgb(232, 234, 238);
pub const MUTED: Color32 = Color32::from_rgb(148, 155, 168);

pub struct Theme {
    pub name: &'static str,
    pub bg: Color32,
    pub fg: Color32,
    pub muted: Color32,
    pub dark: bool,
}

pub static THEMES: [Theme; 4] = [
    Theme { name: "Paper", bg: Color32::from_rgb(250, 250, 247), fg: Color32::from_rgb(33, 33, 33), muted: Color32::from_rgb(120, 120, 120), dark: false },
    Theme { name: "Sepia", bg: Color32::from_rgb(244, 236, 216), fg: Color32::from_rgb(91, 70, 54), muted: Color32::from_rgb(150, 128, 105), dark: false },
    Theme { name: "Dusk", bg: Color32::from_rgb(32, 34, 38), fg: Color32::from_rgb(214, 214, 210), muted: Color32::from_rgb(140, 142, 146), dark: true },
    Theme { name: "Night", bg: Color32::from_rgb(0, 0, 0), fg: Color32::from_rgb(178, 178, 178), muted: Color32::from_rgb(110, 110, 110), dark: true },
];

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

// ---------- fonts ----------

static ITALIC_FONT: OnceLock<bool> = OnceLock::new();

pub fn have_italic_font() -> bool {
    *ITALIC_FONT.get().unwrap_or(&false)
}

fn load_font(fonts: &mut egui::FontDefinitions, key: &str, paths: &[&str]) -> bool {
    for p in paths {
        if let Ok(bytes) = std::fs::read(p) {
            fonts
                .font_data
                .insert(key.to_string(), Arc::new(egui::FontData::from_owned(bytes)));
            return true;
        }
    }
    false
}

/// Uses the computer's own reading fonts when they exist (Georgia / Segoe UI on
/// Windows, DejaVu on Linux, Georgia / Arial on macOS) and falls back to the
/// built-in font otherwise.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let prop = fonts.families[&FontFamily::Proportional].clone();
    let sets: [(&str, &[&str]); 6] = [
        ("sans", &["C:/Windows/Fonts/segoeui.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/System/Library/Fonts/Supplemental/Arial.ttf"]),
        ("sans-bold", &["C:/Windows/Fonts/segoeuib.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", "/System/Library/Fonts/Supplemental/Arial Bold.ttf"]),
        ("sans-italic", &["C:/Windows/Fonts/segoeuii.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Oblique.ttf", "/System/Library/Fonts/Supplemental/Arial Italic.ttf"]),
        ("serif", &["C:/Windows/Fonts/georgia.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf", "/System/Library/Fonts/Supplemental/Georgia.ttf"]),
        ("serif-bold", &["C:/Windows/Fonts/georgiab.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf", "/System/Library/Fonts/Supplemental/Georgia Bold.ttf"]),
        ("serif-italic", &["C:/Windows/Fonts/georgiai.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf", "/System/Library/Fonts/Supplemental/Georgia Italic.ttf"]),
    ];
    let mut italic = true;
    for (name, paths) in sets {
        let ok = load_font(&mut fonts, name, paths);
        if name.ends_with("italic") && !ok {
            italic = false;
        }
        let mut list = Vec::new();
        if ok {
            list.push(name.to_string());
        }
        list.extend(prop.iter().cloned());
        fonts.families.insert(FontFamily::Name(name.into()), list);
    }
    let _ = ITALIC_FONT.set(italic);
    ctx.set_fonts(fonts);
}

pub fn family(serif: bool, bold: bool, italic: bool) -> FontFamily {
    let base = if serif { "serif" } else { "sans" };
    let name = if bold {
        format!("{base}-bold")
    } else if italic {
        format!("{base}-italic")
    } else {
        base.to_string()
    };
    FontFamily::Name(name.into())
}

// ---------- visuals ----------

/// key 0 = library look, 1..=4 = reading theme.
pub fn apply_visuals(ctx: &egui::Context, key: u8) {
    let (bg, fg, dark) = if key == 0 {
        (BG, TEXT, true)
    } else {
        let t = &THEMES[(key - 1) as usize];
        (t.bg, t.fg, t.dark)
    };
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let surf = if key == 0 { SURFACE } else { mix(bg, fg, 0.06) };
    v.panel_fill = bg;
    v.window_fill = surf;
    v.extreme_bg_color = if key == 0 { Color32::from_rgb(14, 15, 18) } else { mix(bg, fg, 0.04) };
    v.faint_bg_color = mix(bg, fg, 0.03);
    v.hyperlink_color = ACCENT;
    v.window_stroke = Stroke::new(1.0, mix(bg, fg, 0.12));
    v.selection.bg_fill = if dark {
        Color32::from_rgba_unmultiplied(110, 168, 254, 70)
    } else {
        Color32::from_rgba_unmultiplied(60, 120, 230, 60)
    };
    v.selection.stroke = Stroke::new(1.0, fg);
    let r = CornerRadius::same(7);
    let w = &mut v.widgets;
    w.noninteractive.fg_stroke.color = fg;
    w.noninteractive.bg_stroke = Stroke::new(1.0, mix(bg, fg, 0.10));
    w.noninteractive.corner_radius = r;
    w.inactive.weak_bg_fill = mix(bg, fg, 0.07);
    w.inactive.bg_fill = mix(bg, fg, 0.12);
    w.inactive.fg_stroke.color = fg;
    w.inactive.corner_radius = r;
    w.hovered.weak_bg_fill = mix(bg, fg, 0.13);
    w.hovered.bg_fill = mix(bg, fg, 0.16);
    w.hovered.fg_stroke.color = fg;
    w.hovered.corner_radius = r;
    w.active.weak_bg_fill = mix(bg, fg, 0.18);
    w.active.fg_stroke.color = fg;
    w.active.corner_radius = r;
    w.open.weak_bg_fill = mix(bg, fg, 0.12);
    w.open.corner_radius = r;
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.button_padding = vec2(11.0, 6.0);
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.interact_size.y = 28.0;
    });
}

// ---------- covers ----------

const COVER_PALETTE: [(Color32, Color32); 8] = [
    (Color32::from_rgb(52, 89, 149), Color32::from_rgb(24, 40, 72)),
    (Color32::from_rgb(170, 74, 68), Color32::from_rgb(88, 30, 36)),
    (Color32::from_rgb(64, 130, 109), Color32::from_rgb(26, 60, 52)),
    (Color32::from_rgb(196, 145, 64), Color32::from_rgb(110, 66, 26)),
    (Color32::from_rgb(112, 84, 160), Color32::from_rgb(48, 34, 82)),
    (Color32::from_rgb(70, 140, 170), Color32::from_rgb(28, 64, 88)),
    (Color32::from_rgb(160, 90, 130), Color32::from_rgb(76, 36, 66)),
    (Color32::from_rgb(98, 110, 124), Color32::from_rgb(40, 46, 56)),
];

pub fn hash(s: &str) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

/// Draws a book's cover picture, or a generated gradient cover with the title.
pub fn draw_cover(ui: &egui::Ui, rect: Rect, title: &str, author: &str, tex: Option<(egui::TextureId, Vec2)>) {
    let p = ui.painter();
    p.rect_filled(rect.translate(vec2(0.0, 3.0)).expand(1.0), 3.0, Color32::from_black_alpha(80));
    if let Some((id, size)) = tex {
        let ar_t = size.x / size.y.max(1.0);
        let ar_r = rect.width() / rect.height();
        let uv = if ar_t > ar_r {
            let w = ar_r / ar_t;
            Rect::from_min_max(pos2((1.0 - w) / 2.0, 0.0), pos2((1.0 + w) / 2.0, 1.0))
        } else {
            let h = ar_t / ar_r;
            Rect::from_min_max(pos2(0.0, (1.0 - h) / 2.0), pos2(1.0, (1.0 + h) / 2.0))
        };
        p.image(id, rect, uv, Color32::WHITE);
        return;
    }
    let (c1, c2) = COVER_PALETTE[(hash(title) % COVER_PALETTE.len() as u64) as usize];
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), c1);
    mesh.colored_vertex(rect.right_top(), c1);
    mesh.colored_vertex(rect.right_bottom(), c2);
    mesh.colored_vertex(rect.left_bottom(), c2);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
    p.rect_filled(
        Rect::from_min_size(rect.min, vec2(rect.width() * 0.06, rect.height())),
        0.0,
        Color32::from_black_alpha(45),
    );
    let s = rect.width() / 140.0;
    let inner = rect.shrink(9.0 * s);
    p.rect_stroke(inner, 0.0, Stroke::new(1.0, Color32::from_white_alpha(55)), StrokeKind::Inside);

    let mut job = LayoutJob::single_section(
        title.to_string(),
        TextFormat::simple(FontId::new(15.0 * s, family(true, true, false)), Color32::WHITE),
    );
    job.halign = Align::Center;
    job.wrap.max_width = inner.width() - 12.0 * s;
    job.wrap.max_rows = 4;
    job.wrap.overflow_character = Some('…');
    let g = ui.fonts(|f| f.layout_job(job));
    p.galley(pos2(rect.center().x, inner.top() + 18.0 * s), g, Color32::WHITE);
    let ly = rect.center().y + 14.0 * s;
    p.line_segment(
        [pos2(rect.center().x - 14.0 * s, ly), pos2(rect.center().x + 14.0 * s, ly)],
        Stroke::new(1.0, Color32::from_white_alpha(120)),
    );
    let mut job = LayoutJob::single_section(
        author.to_string(),
        TextFormat::simple(FontId::new(10.5 * s, family(false, false, false)), Color32::from_white_alpha(210)),
    );
    job.halign = Align::Center;
    job.wrap.max_width = inner.width() - 10.0;
    job.wrap.max_rows = 2;
    job.wrap.overflow_character = Some('…');
    let g = ui.fonts(|f| f.layout_job(job));
    let h = g.size().y;
    p.galley(pos2(rect.center().x, inner.bottom() - 12.0 * s - h), g, Color32::WHITE);
}

pub fn decode_texture(ctx: &egui::Context, name: &str, bytes: &[u8], max: u32) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?;
    let img = if img.width() > max || img.height() > max { img.thumbnail(max, max) } else { img };
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let ci = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(name, ci, egui::TextureOptions::LINEAR))
}
