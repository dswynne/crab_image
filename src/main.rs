// Standard

// External
use eframe::egui::{self, Vec2};

// Local
mod equalize;
mod util;
mod gui;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(Vec2::new(900.0, 600.0)),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "crab_image Flow GUI",
        options,
        Box::new(|_cc| Ok(Box::new(gui::FlowApp::default()))),
    ) {
        eprintln!("Failed to start GUI: {err}");
    }
}


