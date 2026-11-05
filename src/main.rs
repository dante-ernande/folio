#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod epub;
mod html;
mod model;
mod reader;
mod samples;
mod style;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut screenshot: Option<String> = None;
    let mut files: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--screenshot" {
            screenshot = args.get(i + 1).cloned();
            i += 2;
        } else {
            files.push(args[i].clone());
            i += 1;
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Folio — e-book reader")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([640.0, 480.0])
            .with_drag_and_drop(true),
        persist_window: screenshot.is_none(),
        ..Default::default()
    };

    eframe::run_native(
        "Folio Reader",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, screenshot, files)))),
    )
}
