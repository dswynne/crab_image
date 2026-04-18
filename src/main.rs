// Standard
use std::env;

// External
use eframe::egui::{self, Vec2};

// Local
mod equalize;
mod util;
mod gui;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // Run pipeline from YAML file
        let filepath = &args[1];
        match gui::run_pipeline_cli(filepath) {
            Ok(result) => {
                println!("Pipeline executed successfully:");
                println!("{}", result);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Launch GUI
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
}


