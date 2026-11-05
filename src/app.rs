//! App state and the library (bookshelf) screen.

use crate::epub::{self, Book};
use crate::model::*;
use crate::samples;
use crate::style::*;
use eframe::egui::{
    self, pos2, text::LayoutJob, vec2, Align, Align2, Color32, CornerRadius, FontId, Frame, Layout,
    Margin, Rect, RichText, ScrollArea, Sense, Stroke, StrokeKind, TextFormat, TextureHandle, Vec2,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    All,
    Reading,
    Unread,
    Finished,
    Favorites,
}

impl Tab {
    const ALL: [Tab; 5] = [Tab::All, Tab::Reading, Tab::Unread, Tab::Finished, Tab::Favorites];
    fn label(self) -> &'static str {
        match self {
            Tab::All => "All books",
            Tab::Reading => "Reading",
            Tab::Unread => "Not started",
            Tab::Finished => "Finished",
            Tab::Favorites => "Favourites",
        }
    }
    fn keep(self, e: &Entry) -> bool {
        match self {
            Tab::All => true,
            Tab::Reading => e.started(),
            Tab::Unread => e.last_opened == 0 && !e.finished,
            Tab::Finished => e.finished,
            Tab::Favorites => e.favorite,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Panel {
    None,
    Contents,
    Bookmarks,
    Display,
}

enum Action {
    Open(usize),
    ToggleFav(usize),
    ToggleFinished(usize),
    Remove(usize),
    PickFiles,
    PickFolder,
}

pub struct OpenBook {
    pub id: String,
    pub book: Book,
    pub chapter: usize,
    pub frac: f32,
    pub pending_frac: Option<f32>,
    pub settle: u8,
    pub scroll_to: Option<f32>,
    pub offset: f32,
    pub max: f32,
    pub view_h: f32,
    pub images: HashMap<String, Option<TextureHandle>>,
}

pub struct App {
    pub saved: Saved,
    pub open: Option<OpenBook>,
    pub panel: Panel,
    covers: HashMap<String, Option<TextureHandle>>,
    search: String,
    tab: Tab,
    toast: Option<(String, f64)>,
    style_key: Option<u8>,
    persist: bool,
    shot: Option<PathBuf>,
    frames: u64,
    shot_requested: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, screenshot: Option<String>, files: Vec<String>) -> Self {
        setup_fonts(&cc.egui_ctx);
        let persist = screenshot.is_none();
        let mut saved: Saved = if persist {
            cc.storage
                .and_then(|s| eframe::get_value(s, eframe::APP_KEY))
                .unwrap_or_default()
        } else {
            Saved::default()
        };
        if !saved.initialized {
            saved.initialized = true;
            let t = now();
            for (i, s) in samples::SAMPLES.iter().enumerate() {
                let b = samples::book(i);
                saved.entries.push(Entry {
                    id: format!("sample:{i}"),
                    source: Source::Builtin(i),
                    title: s.title.to_string(),
                    author: s.author.to_string(),
                    chapters: b.chapters.len(),
                    chapter: 0,
                    frac: 0.0,
                    added: t.saturating_sub(i as u64),
                    last_opened: 0,
                    favorite: false,
                    finished: false,
                    bookmarks: vec![],
                });
            }
        }
        let mut app = Self {
            saved,
            open: None,
            panel: Panel::None,
            covers: HashMap::new(),
            search: String::new(),
            tab: Tab::All,
            toast: None,
            style_key: None,
            persist,
            shot: screenshot.map(PathBuf::from),
            frames: 0,
            shot_requested: false,
        };
        // Testing hooks: sample reading state, list view, open a book / panel / theme straight away.
        if std::env::var("FOLIO_DEMO").is_ok() && app.saved.entries.len() >= 4 {
            let t = now();
            let es = &mut app.saved.entries;
            es[1].chapter = 1;
            es[1].frac = 0.4;
            es[1].last_opened = t - 3600;
            es[1].favorite = true;
            es[1].bookmarks.push(Bookmark { chapter: 0, frac: 0.5, label: "The keeper".into(), snippet: "Annet had kept the light for eleven years…".into(), created: t - 7200 });
            es[3].finished = true;
            es[3].last_opened = t - 86400 * 3;
            es[0].favorite = true;
        }
        if std::env::var("FOLIO_LIST").is_ok() {
            app.saved.settings.grid = false;
        }
        if let Some(i) = std::env::var("FOLIO_OPEN").ok().and_then(|v| v.parse::<usize>().ok()) {
            if i < app.saved.entries.len() {
                if let Some(t) = std::env::var("FOLIO_THEME").ok().and_then(|v| v.parse::<usize>().ok()) {
                    app.saved.settings.theme = t.min(THEMES.len() - 1);
                }
                app.open_book(i);
                app.panel = match std::env::var("FOLIO_PANEL").as_deref() {
                    Ok("contents") => Panel::Contents,
                    Ok("bookmarks") => Panel::Bookmarks,
                    Ok("display") => Panel::Display,
                    _ => Panel::None,
                };
            }
        }
        if !files.is_empty() {
            let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
            app.import(&cc.egui_ctx, &paths);
            let first = Source::File(paths[0].to_string_lossy().into_owned());
            if let Some(i) = app.saved.entries.iter().position(|e| e.source == first) {
                app.open_book(i);
            }
        }
        app
    }

    pub fn notify(&mut self, ctx: &egui::Context, msg: impl Into<String>) {
        let t = ctx.input(|i| i.time);
        self.toast = Some((msg.into(), t + 3.5));
    }

    // ----- library actions -----

    fn import(&mut self, ctx: &egui::Context, paths: &[PathBuf]) {
        let mut found = Vec::new();
        for p in paths {
            collect_epubs(p, &mut found, 0);
        }
        let (mut added, mut failed, mut dupes) = (0, 0, 0);
        for p in found {
            let key = p.to_string_lossy().into_owned();
            if self.saved.entries.iter().any(|e| e.source == Source::File(key.clone())) {
                dupes += 1;
                continue;
            }
            match epub::load(&p) {
                Ok(book) => {
                    let id = format!("file:{:016x}", hash(&key));
                    if let Some(c) = &book.cover {
                        let tex = decode_texture(ctx, &id, c, 420);
                        self.covers.insert(id.clone(), tex);
                    }
                    self.saved.entries.push(Entry {
                        id,
                        source: Source::File(key),
                        title: book.title,
                        author: book.author,
                        chapters: book.chapters.len(),
                        chapter: 0,
                        frac: 0.0,
                        added: now(),
                        last_opened: 0,
                        favorite: false,
                        finished: false,
                        bookmarks: vec![],
                    });
                    added += 1;
                }
                Err(_) => failed += 1,
            }
        }
        let mut msg = match added {
            0 => "No new books added".to_string(),
            1 => "Added 1 book".to_string(),
            n => format!("Added {n} books"),
        };
        if dupes > 0 {
            msg += &format!(" · {dupes} already on the shelf");
        }
        if failed > 0 {
            msg += &format!(" · {failed} couldn't be opened");
        }
        self.notify(ctx, msg);
    }

    pub fn open_book(&mut self, idx: usize) {
        let e = &self.saved.entries[idx];
        let book = match &e.source {
            Source::Builtin(i) => Ok(samples::book(*i)),
            Source::File(p) => epub::load(Path::new(p)),
        };
        match book {
            Ok(book) => {
                let e = &mut self.saved.entries[idx];
                e.last_opened = now();
                e.chapters = book.chapters.len();
                let (chapter, frac) = if e.finished { (0, 0.0) } else { (e.chapter.min(book.chapters.len().saturating_sub(1)), e.frac) };
                self.open = Some(OpenBook {
                    id: e.id.clone(),
                    book,
                    chapter,
                    frac,
                    pending_frac: Some(frac),
                    settle: 0,
                    scroll_to: None,
                    offset: 0.0,
                    max: 0.0,
                    view_h: 0.0,
                    images: HashMap::new(),
                });
            }
            Err(err) => {
                self.toast = Some((format!("Couldn't open “{}”: {err}", e.title), f64::MAX));
            }
        }
    }

    fn cover_tex(&mut self, ctx: &egui::Context, idx: usize) -> Option<(egui::TextureId, Vec2)> {
        let e = &self.saved.entries[idx];
        if !self.covers.contains_key(&e.id) {
            let tex = match &e.source {
                Source::Builtin(_) => None,
                Source::File(p) => epub::load_cover(Path::new(p)).and_then(|b| decode_texture(ctx, &e.id, &b, 420)),
            };
            self.covers.insert(e.id.clone(), tex);
        }
        self.covers
            .get(&e.id)
            .and_then(|t| t.as_ref().map(|t| (t.id(), t.size_vec2())))
    }

    fn visible(&self) -> Vec<usize> {
        let q = self.search.trim().to_lowercase();
        let es = &self.saved.entries;
        let mut v: Vec<usize> = (0..es.len())
            .filter(|&i| {
                let e = &es[i];
                self.tab.keep(e)
                    && (q.is_empty() || e.title.to_lowercase().contains(&q) || e.author.to_lowercase().contains(&q))
            })
            .collect();
        let sort = self.saved.settings.sort;
        v.sort_by(|&a, &b| {
            let (a, b) = (&es[a], &es[b]);
            match sort {
                Sort::Recent => b.last_opened.cmp(&a.last_opened).then(b.added.cmp(&a.added)),
                Sort::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                Sort::Author => a.author.to_lowercase().cmp(&b.author.to_lowercase()).then(a.title.cmp(&b.title)),
                Sort::Progress => b.progress().partial_cmp(&a.progress()).unwrap_or(std::cmp::Ordering::Equal),
                Sort::Added => b.added.cmp(&a.added),
            }
        });
        v
    }

    fn apply(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::Open(i) => self.open_book(i),
            Action::ToggleFav(i) => {
                let e = &mut self.saved.entries[i];
                e.favorite = !e.favorite;
            }
            Action::ToggleFinished(i) => {
                let e = &mut self.saved.entries[i];
                e.finished = !e.finished;
                if !e.finished {
                    e.chapter = 0;
                    e.frac = 0.0;
                }
            }
            Action::Remove(i) => {
                let e = self.saved.entries.remove(i);
                self.covers.remove(&e.id);
                self.notify(ctx, format!("Removed “{}” from the shelf (the file is untouched)", e.title));
            }
            Action::PickFiles => {
                if let Some(files) = rfd::FileDialog::new()
                    .set_title("Add EPUB books")
                    .add_filter("EPUB books", &["epub"])
                    .pick_files()
                {
                    self.import(ctx, &files);
                }
            }
            Action::PickFolder => {
                if let Some(dir) = rfd::FileDialog::new().set_title("Find books in a folder").pick_folder() {
                    self.import(ctx, &[dir]);
                }
            }
        }
    }

    // ----- library UI -----

    fn library_ui(&mut self, ctx: &egui::Context) {
        let mut actions: Vec<Action> = Vec::new();

        egui::TopBottomPanel::top("lib_top")
            .frame(Frame::default().fill(BG).inner_margin(Margin::symmetric(22, 14)))
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                    logo(ui, r);
                    ui.label(RichText::new("Folio").size(22.0).strong().color(TEXT));
                    ui.add_space(18.0);
                    for t in Tab::ALL {
                        let sel = self.tab == t;
                        let txt = RichText::new(t.label()).size(14.5).color(if sel { TEXT } else { MUTED });
                        if ui.selectable_label(sel, txt).clicked() {
                            self.tab = t;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let add = egui::Button::new(RichText::new("+  Add books").color(ON_ACCENT).strong()).fill(ACCENT);
                        let resp = ui.add(add);
                        let popup_id = ui.make_persistent_id("add_menu");
                        if resp.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(popup_id));
                        }
                        egui::popup_below_widget(ui, popup_id, &resp, egui::PopupCloseBehavior::CloseOnClick, |ui| {
                            ui.set_min_width(200.0);
                            if ui.button("Choose EPUB files…").clicked() {
                                actions.push(Action::PickFiles);
                            }
                            if ui.button("Find books in a folder…").clicked() {
                                actions.push(Action::PickFolder);
                            }
                            ui.label(RichText::new("Tip: you can also drop files on the window").small().color(MUTED));
                        });
                        ui.add_space(6.0);
                        let s = &mut self.saved.settings;
                        if ui.selectable_label(!s.grid, "List").clicked() {
                            s.grid = false;
                        }
                        if ui.selectable_label(s.grid, "Grid").clicked() {
                            s.grid = true;
                        }
                        ui.add_space(6.0);
                        egui::ComboBox::from_id_salt("sort")
                            .selected_text(format!("Sort: {}", s.sort.label()))
                            .width(170.0)
                            .show_ui(ui, |ui| {
                                for so in Sort::ALL {
                                    ui.selectable_value(&mut s.sort, so, so.label());
                                }
                            });
                        ui.add_space(6.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search)
                                .hint_text("Search title or author")
                                .desired_width(210.0),
                        );
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::default().fill(BG).inner_margin(Margin { left: 22, right: 22, top: 4, bottom: 0 }))
            .show(ctx, |ui| {
                let list = self.visible();
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.add_space(8.0);
                    if self.tab == Tab::All && self.search.trim().is_empty() {
                        if let Some(i) = self.continue_candidate() {
                            self.continue_card(ui, i, &mut actions);
                            ui.add_space(22.0);
                        }
                    }
                    let heading = if self.search.trim().is_empty() {
                        self.tab.label().to_string()
                    } else {
                        format!("Results for “{}”", self.search.trim())
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(heading).size(18.0).strong().color(TEXT));
                        ui.label(RichText::new(format!("{} book{}", list.len(), if list.len() == 1 { "" } else { "s" })).color(MUTED));
                    });
                    ui.add_space(14.0);
                    if list.is_empty() {
                        empty_state(ui, self.tab, !self.search.trim().is_empty(), &mut actions);
                    } else if self.saved.settings.grid {
                        self.grid(ui, &list, &mut actions);
                    } else {
                        self.list(ui, &list, &mut actions);
                    }
                    ui.add_space(30.0);
                });
            });

        for a in actions {
            self.apply(ctx, a);
        }
    }

    fn continue_candidate(&self) -> Option<usize> {
        self.saved
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.started())
            .max_by_key(|(_, e)| e.last_opened)
            .map(|(i, _)| i)
    }

    fn continue_card(&mut self, ui: &mut egui::Ui, idx: usize, actions: &mut Vec<Action>) {
        let tex = self.cover_tex(ui.ctx(), idx);
        let e = self.saved.entries[idx].clone();
        Frame::default()
            .fill(SURFACE)
            .corner_radius(CornerRadius::same(14))
            .inner_margin(Margin::same(18))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, resp) = ui.allocate_exact_size(vec2(84.0, 126.0), Sense::click());
                    draw_cover(ui, r, &e.title, &e.author, tex);
                    if resp.clicked() {
                        actions.push(Action::Open(idx));
                    }
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.add_space(4.0);
                        ui.label(RichText::new("CONTINUE READING").size(11.5).color(ACCENT).strong());
                        ui.label(RichText::new(&e.title).size(21.0).strong().color(TEXT));
                        ui.label(RichText::new(&e.author).color(MUTED));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            progress_bar(ui, 260.0, e.progress(), ACCENT, SURFACE2);
                            ui.label(
                                RichText::new(format!(
                                    "{}% · chapter {} of {} · {}",
                                    (e.progress() * 100.0).round(),
                                    e.chapter + 1,
                                    e.chapters.max(1),
                                    ago(e.last_opened)
                                ))
                                .color(MUTED)
                                .size(12.5),
                            );
                        });
                        ui.add_space(8.0);
                        let b = egui::Button::new(RichText::new("Resume reading  ›").color(ON_ACCENT).strong()).fill(ACCENT);
                        if ui.add(b).clicked() {
                            actions.push(Action::Open(idx));
                        }
                    });
                });
            });
    }

    fn grid(&mut self, ui: &mut egui::Ui, list: &[usize], actions: &mut Vec<Action>) {
        let gap = 26.0;
        let avail = ui.available_width();
        let min_w = 138.0;
        let cols = (((avail + gap) / (min_w + gap)).floor() as usize).max(1);
        let w = ((avail - gap * (cols as f32 - 1.0)) / cols as f32).min(176.0);
        for row in list.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for &i in row {
                    self.card(ui, i, w, actions);
                }
            });
            ui.add_space(22.0);
        }
    }

    fn card(&mut self, ui: &mut egui::Ui, idx: usize, w: f32, actions: &mut Vec<Action>) {
        let tex = self.cover_tex(ui.ctx(), idx);
        let e = self.saved.entries[idx].clone();
        let cover_h = w * 1.5;
        let (rect, resp) = ui.allocate_exact_size(vec2(w, cover_h + 76.0), Sense::click());
        let cover = Rect::from_min_size(rect.min, vec2(w, cover_h));
        let lift = if resp.hovered() { -3.0 } else { 0.0 };
        let cover_r = cover.translate(vec2(0.0, lift));
        draw_cover(ui, cover_r, &e.title, &e.author, tex);
        let p = ui.painter().clone();
        if resp.hovered() {
            p.rect_stroke(cover_r.expand(2.0), 3.0, Stroke::new(2.0, ACCENT), StrokeKind::Outside);
        }
        let mut bx = cover_r.right() - 14.0;
        if e.favorite {
            badge(&p, pos2(bx, cover_r.top() + 14.0), "★", Color32::from_rgb(255, 200, 70));
            bx -= 26.0;
        }
        if e.finished {
            badge(&p, pos2(bx, cover_r.top() + 14.0), "✔", Color32::from_rgb(120, 220, 150));
        }
        let mut job = LayoutJob::single_section(e.title.clone(), TextFormat::simple(FontId::proportional(14.0), TEXT));
        job.wrap.max_width = w;
        job.wrap.max_rows = 2;
        job.wrap.overflow_character = Some('…');
        let g = ui.fonts(|f| f.layout_job(job));
        let th = g.size().y;
        p.galley(pos2(rect.left(), cover.bottom() + 10.0), g, TEXT);
        let mut job = LayoutJob::single_section(e.author.clone(), TextFormat::simple(FontId::proportional(12.0), MUTED));
        job.wrap.max_width = w - 44.0;
        job.wrap.max_rows = 1;
        job.wrap.overflow_character = Some('…');
        let g = ui.fonts(|f| f.layout_job(job));
        p.galley(pos2(rect.left(), cover.bottom() + 13.0 + th), g, MUTED);
        let (status, col) = if e.finished {
            ("Done".to_string(), MUTED)
        } else if e.last_opened == 0 {
            ("New".to_string(), ACCENT)
        } else {
            (format!("{}%", (e.progress() * 100.0).round()), MUTED)
        };
        p.text(pos2(rect.right(), cover.bottom() + 13.0 + th), Align2::RIGHT_TOP, status, FontId::proportional(12.0), col);
        let bar = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 4.0), vec2(w, 3.0));
        p.rect_filled(bar, 2.0, SURFACE2);
        let pr = e.progress();
        if pr > 0.0 {
            p.rect_filled(Rect::from_min_size(bar.min, vec2(w * pr, 3.0)), 2.0, ACCENT);
        }
        if resp.clicked() {
            actions.push(Action::Open(idx));
        }
        resp.context_menu(|ui| book_menu(ui, idx, e.favorite, e.finished, actions));
        resp.on_hover_text(format!("{}\nClick to read · right-click for options", e.title));
    }

    fn list(&mut self, ui: &mut egui::Ui, list: &[usize], actions: &mut Vec<Action>) {
        for &idx in list {
            let tex = self.cover_tex(ui.ctx(), idx);
            let e = self.saved.entries[idx].clone();
            let w = ui.available_width();
            let (rect, resp) = ui.allocate_exact_size(vec2(w, 92.0), Sense::click());
            let p = ui.painter().clone();
            if resp.hovered() {
                p.rect_filled(rect, 10.0, SURFACE);
            }
            let cover = Rect::from_min_size(rect.min + vec2(10.0, 8.0), vec2(50.0, 76.0));
            draw_cover(ui, cover, &e.title, &e.author, tex);
            let x = cover.right() + 16.0;
            p.text(pos2(x, rect.top() + 16.0), Align2::LEFT_TOP, &e.title, FontId::proportional(16.0), TEXT);
            p.text(pos2(x, rect.top() + 40.0), Align2::LEFT_TOP, &e.author, FontId::proportional(13.0), MUTED);
            let mut tags = Vec::new();
            if e.favorite {
                tags.push("★ Favourite".to_string());
            }
            if e.finished {
                tags.push("✔ Finished".to_string());
            }
            tags.push(format!("{} chapters", e.chapters));
            if !e.bookmarks.is_empty() {
                tags.push(format!("{} bookmark{}", e.bookmarks.len(), if e.bookmarks.len() == 1 { "" } else { "s" }));
            }
            p.text(pos2(x, rect.top() + 62.0), Align2::LEFT_TOP, tags.join("   ·   "), FontId::proportional(12.0), MUTED);
            let right = rect.right() - 16.0;
            let pct = if e.last_opened == 0 && !e.finished { "New".to_string() } else { format!("{}%", (e.progress() * 100.0).round()) };
            p.text(pos2(right, rect.top() + 20.0), Align2::RIGHT_TOP, pct, FontId::proportional(15.0), TEXT);
            p.text(pos2(right, rect.top() + 44.0), Align2::RIGHT_TOP, format!("Opened {}", ago(e.last_opened)), FontId::proportional(12.0), MUTED);
            let bar = Rect::from_min_size(pos2(right - 160.0, rect.top() + 68.0), vec2(160.0, 4.0));
            p.rect_filled(bar, 2.0, SURFACE2);
            p.rect_filled(Rect::from_min_size(bar.min, vec2(160.0 * e.progress(), 4.0)), 2.0, ACCENT);
            if resp.clicked() {
                actions.push(Action::Open(idx));
            }
            resp.context_menu(|ui| book_menu(ui, idx, e.favorite, e.finished, actions));
            ui.add_space(4.0);
        }
    }

    // ----- misc -----

    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot.clone() else { return };
        self.frames += 1;
        ctx.request_repaint();
        if self.frames >= 120 && !self.shot_requested {
            self.shot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let img = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = img {
            let [w, h] = img.size;
            let mut rgba = Vec::with_capacity(w * h * 4);
            for c in &img.pixels {
                rgba.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
            }
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = image::save_buffer(&path, &rgba, w as u32, h as u32, image::ColorType::Rgba8) {
                eprintln!("could not save screenshot: {e}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.frames > 600 {
            eprintln!("screenshot timed out");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().filter_map(|f| f.path.clone()).collect());
        if !dropped.is_empty() {
            self.import(ctx, &dropped);
        }
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let screen = ctx.screen_rect();
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop")));
            p.rect_filled(screen, 0.0, Color32::from_black_alpha(170));
            p.rect_stroke(screen.shrink(24.0), 16.0, Stroke::new(2.0, ACCENT), StrokeKind::Inside);
            p.text(screen.center(), Align2::CENTER_CENTER, "Drop EPUB books or folders to add them", FontId::proportional(24.0), Color32::WHITE);
        }
    }

    fn toast_ui(&mut self, ctx: &egui::Context) {
        let now_t = ctx.input(|i| i.time);
        let mut clear = false;
        if let Some((msg, until)) = &self.toast {
            if now_t > *until {
                clear = true;
            } else {
                egui::Area::new(egui::Id::new("toast"))
                    .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -64.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        Frame::default()
                            .fill(Color32::from_rgb(48, 52, 62))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(Margin::symmetric(16, 10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(msg).color(Color32::WHITE));
                                    if ui.small_button("×").clicked() {
                                        clear = true;
                                    }
                                });
                            });
                    });
                ctx.request_repaint_after(std::time::Duration::from_millis(250));
            }
        }
        if clear {
            self.toast = None;
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.frames == 2 {
            if let Ok(chars) = std::env::var("FOLIO_GLYPHS") {
                for c in chars.chars() {
                    let ok = ctx.fonts(|f| f.has_glyph(&FontId::proportional(14.0), c));
                    eprintln!("U+{:04X} {}", c as u32, ok);
                }
            }
        }
        self.handle_screenshot(ctx);
        self.handle_drops(ctx);
        let key = match &self.open {
            None => 0,
            Some(_) => 1 + self.saved.settings.theme.min(THEMES.len() - 1) as u8,
        };
        if self.style_key != Some(key) {
            apply_visuals(ctx, key);
            self.style_key = Some(key);
        }
        if self.open.is_some() {
            self.reader_ui(ctx);
        } else {
            self.library_ui(ctx);
        }
        self.toast_ui(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.persist {
            eframe::set_value(storage, eframe::APP_KEY, &self.saved);
        }
    }

    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(10)
    }
}

// ---------- helpers ----------

fn collect_epubs(p: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if p.is_dir() {
        if depth > 8 {
            return;
        }
        if let Ok(rd) = std::fs::read_dir(p) {
            let mut items: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
            items.sort();
            for it in items {
                collect_epubs(&it, out, depth + 1);
            }
        }
    } else if p.extension().map(|e| e.eq_ignore_ascii_case("epub")).unwrap_or(false) {
        out.push(p.to_path_buf());
    }
}

fn logo(ui: &egui::Ui, r: Rect) {
    let p = ui.painter();
    p.circle_filled(r.center(), r.width() / 2.0, ACCENT);
    let c = r.center();
    let s = r.width() / 28.0;
    let page = ON_ACCENT;
    p.add(egui::Shape::convex_polygon(
        vec![pos2(c.x - 8.0 * s, c.y - 6.0 * s), pos2(c.x - 0.8 * s, c.y - 4.0 * s), pos2(c.x - 0.8 * s, c.y + 7.0 * s), pos2(c.x - 8.0 * s, c.y + 5.0 * s)],
        page,
        Stroke::NONE,
    ));
    p.add(egui::Shape::convex_polygon(
        vec![pos2(c.x + 0.8 * s, c.y - 4.0 * s), pos2(c.x + 8.0 * s, c.y - 6.0 * s), pos2(c.x + 8.0 * s, c.y + 5.0 * s), pos2(c.x + 0.8 * s, c.y + 7.0 * s)],
        page,
        Stroke::NONE,
    ));
}

fn badge(p: &egui::Painter, c: egui::Pos2, txt: &str, col: Color32) {
    p.circle_filled(c, 11.0, Color32::from_black_alpha(160));
    p.text(c, Align2::CENTER_CENTER, txt, FontId::proportional(13.0), col);
}

fn progress_bar(ui: &mut egui::Ui, w: f32, v: f32, fill: Color32, bg: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 14.0), Sense::hover());
    let bar = Rect::from_center_size(r.center(), vec2(w, 5.0));
    ui.painter().rect_filled(bar, 3.0, bg);
    ui.painter().rect_filled(Rect::from_min_size(bar.min, vec2(w * v.clamp(0.0, 1.0), 5.0)), 3.0, fill);
}

fn book_menu(ui: &mut egui::Ui, idx: usize, fav: bool, fin: bool, actions: &mut Vec<Action>) {
    if ui.button("Open").clicked() {
        actions.push(Action::Open(idx));
        ui.close_menu();
    }
    if ui.button(if fav { "Remove from favourites" } else { "★ Add to favourites" }).clicked() {
        actions.push(Action::ToggleFav(idx));
        ui.close_menu();
    }
    if ui.button(if fin { "Mark as not finished" } else { "✔ Mark as finished" }).clicked() {
        actions.push(Action::ToggleFinished(idx));
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Remove from shelf").clicked() {
        actions.push(Action::Remove(idx));
        ui.close_menu();
    }
}

fn empty_state(ui: &mut egui::Ui, tab: Tab, searching: bool, actions: &mut Vec<Action>) {
    ui.add_space(60.0);
    ui.vertical_centered(|ui| {
        let msg = if searching {
            "Nothing matches your search."
        } else {
            match tab {
                Tab::All => "Your shelf is empty.",
                Tab::Reading => "You aren't reading anything right now.",
                Tab::Unread => "You've started every book on your shelf.",
                Tab::Finished => "No finished books yet.",
                Tab::Favorites => "No favourites yet. Right-click a book to add it.",
            }
        };
        ui.label(RichText::new(msg).size(17.0).color(TEXT));
        ui.add_space(10.0);
        if tab == Tab::All && !searching {
            if ui.button("Choose EPUB files…").clicked() {
                actions.push(Action::PickFiles);
            }
            if ui.button("Find books in a folder…").clicked() {
                actions.push(Action::PickFolder);
            }
        }
    });
}
