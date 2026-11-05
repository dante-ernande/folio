//! The reading screen: page, contents, bookmarks and text settings.

use crate::app::{App, OpenBook, Panel};
use crate::html::{Block, Kind, Span};
use crate::model::*;
use crate::style::*;
use eframe::egui::{
    self, pos2, text::LayoutJob, vec2, Align, Align2, Color32, CornerRadius, FontId, Frame, Key,
    Layout, Margin, Rect, RichText, ScrollArea, Sense, Stroke, TextFormat, TextureHandle,
};
use std::collections::HashMap;

impl App {
    pub fn reader_ui(&mut self, ctx: &egui::Context) {
        let Some(mut ob) = self.open.take() else { return };
        let theme = &THEMES[self.saved.settings.theme.min(THEMES.len() - 1)];
        let n = ob.book.chapters.len();
        let mut close = false;
        let mut goto: Option<(usize, f32)> = None;
        let entry_idx = self.saved.entries.iter().position(|e| e.id == ob.id);

        // keyboard paging
        if !ctx.wants_keyboard_input() {
            let (fwd, back, esc) = ctx.input(|i| {
                (
                    i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::PageDown) || i.key_pressed(Key::Space),
                    i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::PageUp),
                    i.key_pressed(Key::Escape),
                )
            });
            if fwd && ob.pending_frac.is_none() {
                if ob.offset >= ob.max - 2.0 {
                    if ob.chapter + 1 < n {
                        goto = Some((ob.chapter + 1, 0.0));
                    }
                } else {
                    ob.scroll_to = Some((ob.offset + ob.view_h * 0.9).min(ob.max));
                }
            }
            if back && ob.pending_frac.is_none() {
                if ob.offset <= 2.0 {
                    if ob.chapter > 0 {
                        goto = Some((ob.chapter - 1, 1.0));
                    }
                } else {
                    ob.scroll_to = Some((ob.offset - ob.view_h * 0.9).max(0.0));
                }
            }
            if esc {
                if self.panel != Panel::None {
                    self.panel = Panel::None;
                } else {
                    close = true;
                }
            }
        }

        let here_bm = entry_idx.and_then(|i| {
            self.saved.entries[i]
                .bookmarks
                .iter()
                .position(|b| b.chapter == ob.chapter && (b.frac - ob.frac).abs() < 0.03)
        });

        // ---- top bar ----
        egui::TopBottomPanel::top("rd_top")
            .frame(Frame::default().fill(theme.bg).inner_margin(Margin::symmetric(14, 9)))
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("‹  Library").on_hover_text("Back to your books (Esc)").clicked() {
                        close = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let toggle = |p: &mut Panel, v: Panel| *p = if *p == v { Panel::None } else { v };
                        if ui.selectable_label(self.panel == Panel::Display, RichText::new(" Aa ").strong()).on_hover_text("Text size, typeface and theme").clicked() {
                            toggle(&mut self.panel, Panel::Display);
                        }
                        if ui.selectable_label(self.panel == Panel::Bookmarks, "Bookmarks").clicked() {
                            toggle(&mut self.panel, Panel::Bookmarks);
                        }
                        if ui.selectable_label(self.panel == Panel::Contents, "☰ Contents").clicked() {
                            toggle(&mut self.panel, Panel::Contents);
                        }
                        let (txt, tip) = if here_bm.is_some() { ("★ Bookmarked", "Remove this bookmark") } else { ("☆ Bookmark", "Save this place") };
                        if ui.button(txt).on_hover_text(tip).clicked() {
                            if let Some(ei) = entry_idx {
                                let bms = &mut self.saved.entries[ei].bookmarks;
                                if let Some(bi) = here_bm {
                                    bms.remove(bi);
                                } else {
                                    let ch = &ob.book.chapters[ob.chapter];
                                    bms.push(Bookmark {
                                        chapter: ob.chapter,
                                        frac: ob.frac,
                                        label: ch.title.clone(),
                                        snippet: snippet_at(&ch.blocks, ob.frac),
                                        created: now(),
                                    });
                                    bms.sort_by(|a, b| (a.chapter, a.frac).partial_cmp(&(b.chapter, b.frac)).unwrap_or(std::cmp::Ordering::Equal));
                                }
                            }
                        }
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new(format!("{}  ·  {}", ob.book.title, ob.book.author)).color(theme.muted));
                        });
                    });
                });
            });

        // ---- bottom bar ----
        let progress = if n == 0 { 0.0 } else { ((ob.chapter as f32 + ob.frac) / n as f32).clamp(0.0, 1.0) };
        egui::TopBottomPanel::bottom("rd_bottom")
            .frame(Frame::default().fill(theme.bg).inner_margin(Margin::symmetric(14, 8)))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let w = ui.available_width();
                let (r, resp) = ui.allocate_exact_size(vec2(w, 12.0), Sense::click());
                let bar = Rect::from_center_size(r.center(), vec2(w, 3.0));
                let p = ui.painter();
                p.rect_filled(bar, 2.0, mix(theme.bg, theme.fg, 0.12));
                p.rect_filled(Rect::from_min_size(bar.min, vec2(w * progress, 3.0)), 2.0, ACCENT);
                for c in 1..n {
                    let x = bar.left() + w * c as f32 / n as f32;
                    p.line_segment([pos2(x, bar.top()), pos2(x, bar.bottom())], Stroke::new(1.5, theme.bg));
                }
                if resp.clicked() {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        let f = ((pos.x - bar.left()) / w).clamp(0.0, 0.9999) * n as f32;
                        goto = Some((f.floor() as usize, f.fract()));
                    }
                }
                resp.on_hover_text("Click to jump anywhere in the book");
                ui.horizontal(|ui| {
                    if ui.add_enabled(ob.chapter > 0, egui::Button::new("‹ Previous")).clicked() {
                        goto = Some((ob.chapter - 1, 0.0));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add_enabled(ob.chapter + 1 < n, egui::Button::new("Next ›")).clicked() {
                            goto = Some((ob.chapter + 1, 0.0));
                        }
                        ui.centered_and_justified(|ui| {
                            let ch = &ob.book.chapters[ob.chapter];
                            let mins = ((ch.words as f32 * (1.0 - ob.frac)) / 230.0).ceil() as u32;
                            let left = if mins <= 1 { "under a minute left in chapter".to_string() } else { format!("{mins} min left in chapter") };
                            ui.label(
                                RichText::new(format!(
                                    "{}   ·   chapter {} of {}   ·   {}% of book   ·   {}",
                                    ch.title,
                                    ob.chapter + 1,
                                    n,
                                    (progress * 100.0).round(),
                                    left
                                ))
                                .color(theme.muted)
                                .size(12.5),
                            );
                        });
                    });
                });
            });

        // ---- side panels ----
        let side_frame = Frame::default().fill(mix(theme.bg, theme.fg, 0.04)).inner_margin(Margin::same(14));
        match self.panel {
            Panel::Contents => {
                egui::SidePanel::left("toc").frame(side_frame).default_width(290.0).resizable(true).show(ctx, |ui| {
                    ui.label(RichText::new("Contents").size(17.0).strong());
                    ui.label(RichText::new(format!("{n} chapters")).color(theme.muted).small());
                    ui.add_space(8.0);
                    let current = ob.book.toc.iter().rposition(|t| t.chapter <= ob.chapter);
                    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        for (ti, t) in ob.book.toc.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.add_space(t.depth as f32 * 16.0);
                                let sel = Some(ti) == current;
                                let txt = RichText::new(&t.label).color(if sel { ACCENT } else { theme.fg });
                                if ui.add(egui::Button::new(txt).frame(sel).wrap()).clicked() {
                                    goto = Some((t.chapter, 0.0));
                                }
                            });
                        }
                    });
                });
            }
            Panel::Bookmarks => {
                egui::SidePanel::left("bms").frame(side_frame).default_width(290.0).resizable(true).show(ctx, |ui| {
                    ui.label(RichText::new("Bookmarks").size(17.0).strong());
                    ui.add_space(6.0);
                    let Some(ei) = entry_idx else { return };
                    let mut remove = None;
                    let bms = &self.saved.entries[ei].bookmarks;
                    if bms.is_empty() {
                        ui.label(RichText::new("No bookmarks yet. Press “☆ Bookmark” at the top to save the place you're reading.").color(theme.muted));
                    }
                    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        for (bi, b) in bms.iter().enumerate() {
                            Frame::default()
                                .fill(mix(theme.bg, theme.fg, 0.07))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::same(10))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&b.label).strong());
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if ui.small_button("×").on_hover_text("Delete bookmark").clicked() {
                                                remove = Some(bi);
                                            }
                                        });
                                    });
                                    if !b.snippet.is_empty() {
                                        ui.label(RichText::new(format!("“{}”", b.snippet)).italics().color(theme.muted).size(12.5));
                                    }
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("Chapter {} · {}% in · {}", b.chapter + 1, (b.frac * 100.0).round(), ago(b.created)))
                                                .small()
                                                .color(theme.muted),
                                        );
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            if ui.small_button("Go ›").clicked() {
                                                goto = Some((b.chapter, b.frac));
                                            }
                                        });
                                    });
                                });
                            ui.add_space(6.0);
                        }
                    });
                    if let Some(bi) = remove {
                        self.saved.entries[ei].bookmarks.remove(bi);
                    }
                });
            }
            Panel::Display => {
                egui::SidePanel::right("display").frame(side_frame).default_width(280.0).resizable(false).show(ctx, |ui| {
                    display_panel(ui, &mut self.saved.settings, theme);
                });
            }
            Panel::None => {}
        }

        // ---- page ----
        let settings = self.saved.settings.clone();
        egui::CentralPanel::default()
            .frame(Frame::default().fill(theme.bg))
            .show(ctx, |ui| {
                chapter_view(ui, &mut ob, &settings, theme, &mut goto, &mut close);
            });

        if let Some((c, f)) = goto {
            if c < n {
                ob.chapter = c;
                ob.frac = f;
                ob.pending_frac = Some(f);
                ob.settle = 0;
                ob.scroll_to = None;
            }
        }

        if let Some(ei) = entry_idx {
            let e = &mut self.saved.entries[ei];
            e.chapter = ob.chapter;
            e.frac = ob.frac;
            e.chapters = n;
            let at_end = ob.chapter + 1 == n && ob.pending_frac.is_none() && ob.scroll_to.is_none() && ob.view_h > 0.0 && ob.offset >= ob.max - 2.0;
            if at_end && ob.max > 0.0 {
                e.finished = true;
            }
        }

        if close {
            self.panel = Panel::None;
        } else {
            self.open = Some(ob);
        }
    }
}

fn snippet_at(blocks: &[Block], frac: f32) -> String {
    let texts: Vec<&Block> = blocks.iter().filter(|b| b.words() > 3).collect();
    if texts.is_empty() {
        return String::new();
    }
    let i = ((texts.len() as f32 - 1.0) * frac).round() as usize;
    let t = texts[i.min(texts.len() - 1)].text().replace('\n', " ");
    let words: Vec<&str> = t.split_whitespace().take(14).collect();
    let mut s = words.join(" ");
    if t.split_whitespace().count() > 14 {
        s.push('…');
    }
    s
}

fn display_panel(ui: &mut egui::Ui, s: &mut Settings, theme: &Theme) {
    ui.label(RichText::new("Text & theme").size(17.0).strong());
    ui.add_space(10.0);
    ui.label(RichText::new("Theme").color(theme.muted));
    ui.horizontal(|ui| {
        for (i, t) in THEMES.iter().enumerate() {
            ui.vertical(|ui| {
                let (r, resp) = ui.allocate_exact_size(vec2(54.0, 54.0), Sense::click());
                let p = ui.painter();
                p.circle_filled(r.center(), 22.0, t.bg);
                p.circle_stroke(r.center(), 22.0, Stroke::new(1.0, mix(theme.bg, theme.fg, 0.25)));
                if s.theme == i {
                    p.circle_stroke(r.center(), 25.5, Stroke::new(2.5, ACCENT));
                }
                p.text(r.center(), Align2::CENTER_CENTER, "Aa", FontId::new(16.0, family(true, false, false)), t.fg);
                if resp.clicked() {
                    s.theme = i;
                }
                ui.add_sized(vec2(54.0, 14.0), egui::Label::new(RichText::new(t.name).size(11.5).color(theme.muted)));
            });
        }
    });
    ui.add_space(14.0);
    ui.label(RichText::new("Typeface").color(theme.muted));
    ui.horizontal(|ui| {
        if ui.selectable_label(s.serif, RichText::new("Serif").family(family(true, false, false)).size(16.0)).clicked() {
            s.serif = true;
        }
        if ui.selectable_label(!s.serif, RichText::new("Sans").family(family(false, false, false)).size(16.0)).clicked() {
            s.serif = false;
        }
    });
    ui.add_space(14.0);
    ui.label(RichText::new("Text size").color(theme.muted));
    ui.horizontal(|ui| {
        if ui.button(RichText::new("A−").size(13.0)).clicked() {
            s.font_size = (s.font_size - 1.0).max(12.0);
        }
        ui.add(egui::Slider::new(&mut s.font_size, 12.0..=34.0).step_by(1.0));
        if ui.button(RichText::new("A+").size(17.0)).clicked() {
            s.font_size = (s.font_size + 1.0).min(34.0);
        }
    });
    ui.add_space(10.0);
    ui.label(RichText::new("Line spacing").color(theme.muted));
    ui.add(egui::Slider::new(&mut s.line_spacing, 1.0..=2.2).step_by(0.05));
    ui.add_space(10.0);
    ui.label(RichText::new("Page width").color(theme.muted));
    ui.add(egui::Slider::new(&mut s.column_width, 420.0..=1100.0).step_by(10.0));
    ui.add_space(10.0);
    ui.checkbox(&mut s.justify, "Justify text");
    ui.checkbox(&mut s.indent, "Indent paragraphs like a printed book");
    ui.add_space(16.0);
    if ui.button("Reset to defaults").clicked() {
        let (grid, sort) = (s.grid, s.sort);
        *s = Settings { grid, sort, ..Default::default() };
    }
    ui.add_space(16.0);
    ui.label(RichText::new("Keys: Right arrow or Space for the next page, Left arrow for the previous page, Esc to go back").small().color(theme.muted));
}

struct ReadStyle<'a> {
    s: &'a Settings,
    theme: &'a Theme,
    width: f32,
}

#[allow(clippy::too_many_arguments)]
fn text_job(spans: &[Span], st: &ReadStyle, size: f32, bold: bool, italic: bool, color: Color32, indent: f32, justify: bool, line: f32, width: f32) -> LayoutJob {
    let have_italic = have_italic_font();
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    job.justify = justify;
    for (i, sp) in spans.iter().enumerate() {
        let b = bold || sp.bold;
        let it = italic || sp.italic;
        let fmt = TextFormat {
            font_id: FontId::new(size, family(st.s.serif, b, it)),
            color,
            line_height: Some(size * line),
            italics: it && (b || !have_italic),
            ..Default::default()
        };
        job.append(&sp.text, if i == 0 { indent } else { 0.0 }, fmt);
    }
    job
}

fn render_block(ui: &mut egui::Ui, b: &Block, st: &ReadStyle, images: &mut HashMap<String, Option<TextureHandle>>, data: &HashMap<String, Vec<u8>>, first: bool) {
    let fs = st.s.font_size;
    let fg = st.theme.fg;
    let w = st.width;
    match &b.kind {
        Kind::Heading(l) => {
            let scale = [1.75, 1.45, 1.25, 1.12, 1.05, 1.0][(*l as usize).clamp(1, 6) - 1];
            if !first {
                ui.add_space(fs * 1.4);
            }
            let job = text_job(&b.spans, st, fs * scale, true, false, fg, 0.0, false, 1.25, w);
            ui.add(egui::Label::new(job));
            ui.add_space(fs * 0.9);
        }
        Kind::Para => {
            let indent = if st.s.indent { fs * 1.6 } else { 0.0 };
            let job = text_job(&b.spans, st, fs, false, false, fg, indent, st.s.justify, st.s.line_spacing, w);
            ui.add(egui::Label::new(job));
            ui.add_space(if st.s.indent { fs * 0.2 } else { fs * 0.8 });
        }
        Kind::Quote => {
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(fs * 1.2, 1.0), Sense::hover());
                let job = text_job(&b.spans, st, fs * 0.95, false, true, mix(st.theme.bg, fg, 0.8), 0.0, st.s.justify, st.s.line_spacing, w - fs * 2.4);
                let resp = ui.add(egui::Label::new(job));
                ui.painter().line_segment(
                    [pos2(r.left() + 2.0, resp.rect.top()), pos2(r.left() + 2.0, resp.rect.bottom())],
                    Stroke::new(3.0, mix(st.theme.bg, fg, 0.25)),
                );
            });
            ui.add_space(fs * 0.8);
        }
        Kind::Item => {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_sized(vec2(fs * 1.5, fs * st.s.line_spacing), egui::Label::new(RichText::new("•").size(fs).color(st.theme.muted)));
                let job = text_job(&b.spans, st, fs, false, false, fg, 0.0, st.s.justify, st.s.line_spacing, w - fs * 1.5);
                ui.add(egui::Label::new(job));
            });
            ui.add_space(fs * 0.4);
        }
        Kind::Pre => {
            let mut job = LayoutJob::default();
            job.wrap.max_width = w;
            job.append(&b.text(), 0.0, TextFormat::simple(FontId::monospace(fs * 0.8), fg));
            ui.add(egui::Label::new(job));
            ui.add_space(fs * 0.8);
        }
        Kind::Rule => {
            ui.add_space(fs * 0.6);
            let (r, _) = ui.allocate_exact_size(vec2(w, fs), Sense::hover());
            ui.painter().text(r.center(), Align2::CENTER_CENTER, "*   *   *", FontId::new(fs, family(true, false, false)), st.theme.muted);
            ui.add_space(fs * 1.2);
        }
        Kind::Image { src, alt } => {
            if !images.contains_key(src) {
                let tex = data.get(src).and_then(|bytes| decode_texture(ui.ctx(), src, bytes, 1600));
                images.insert(src.clone(), tex);
            }
            if let Some(Some(t)) = images.get(src) {
                let size = t.size_vec2();
                let scale = (w / size.x).min(640.0 / size.y).min(1.5);
                let disp = size * scale;
                ui.vertical_centered(|ui| {
                    ui.add(egui::Image::from_texture(egui::load::SizedTexture::new(t.id(), disp)));
                });
                ui.add_space(fs);
            } else if !alt.is_empty() {
                ui.label(RichText::new(format!("[{alt}]")).italics().color(st.theme.muted));
                ui.add_space(fs * 0.5);
            }
        }
    }
}

fn chapter_view(ui: &mut egui::Ui, ob: &mut OpenBook, s: &Settings, theme: &Theme, goto: &mut Option<(usize, f32)>, close: &mut bool) {
    let avail = ui.available_width();
    let col = s.column_width.min(avail - 56.0).max(220.0);
    let margin = ((avail - col) / 2.0).max(0.0);
    let n = ob.book.chapters.len();
    let mut sa = ScrollArea::vertical().id_salt(("chapter", ob.chapter)).auto_shrink([false, false]);
    if let Some(off) = ob.scroll_to.take() {
        sa = sa.vertical_scroll_offset(off);
    }
    let st = ReadStyle { s, theme, width: col };
    let chapter = ob.chapter;
    let book = &ob.book;
    let images = &mut ob.images;
    let out = sa.show(ui, |ui| {
        ui.add_space(36.0);
        ui.horizontal_top(|ui| {
            ui.add_space(margin);
            ui.vertical(|ui| {
                ui.set_width(col);
                ui.spacing_mut().item_spacing.y = 0.0;
                let ch = &book.chapters[chapter];
                for (i, b) in ch.blocks.iter().enumerate() {
                    render_block(ui, b, &st, images, &book.images, i == 0);
                }
                if ch.blocks.is_empty() {
                    ui.label(RichText::new("(This section is empty.)").italics().color(theme.muted));
                }
                ui.add_space(s.font_size * 2.5);
                ui.vertical_centered(|ui| {
                    if chapter + 1 < n {
                        ui.label(RichText::new(format!("End of “{}”", ch.title)).color(theme.muted).size(13.0));
                        ui.add_space(8.0);
                        let next = &book.chapters[chapter + 1].title;
                        if ui.add(egui::Button::new(RichText::new(format!("Next: {next}  ›")).size(16.0)).min_size(vec2(0.0, 38.0))).clicked() {
                            *goto = Some((chapter + 1, 0.0));
                        }
                    } else {
                        ui.label(RichText::new("The end").size(22.0).color(theme.fg).family(family(true, false, true)));
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("You've finished “{}”.", book.title)).color(theme.muted));
                        ui.add_space(8.0);
                        if ui.button("Back to your library").clicked() {
                            *close = true;
                        }
                    }
                });
            });
        });
        ui.add_space(70.0);
    });
    let view_h = out.inner_rect.height();
    let max = (out.content_size.y - view_h).max(0.0);
    ob.view_h = view_h;
    ob.max = max;
    ob.offset = out.state.offset.y;
    if let Some(f) = ob.pending_frac {
        // keep re-applying for a few frames while the layout settles
        ob.frac = f;
        if f > 0.0 {
            ob.scroll_to = Some(f * max);
        }
        ob.settle += 1;
        if ob.settle >= 4 {
            ob.pending_frac = None;
            ob.settle = 0;
        }
        ui.ctx().request_repaint();
    } else if ob.scroll_to.is_none() {
        ob.frac = if max > 1.0 { (ob.offset / max).clamp(0.0, 1.0) } else { 0.0 };
    }
}
