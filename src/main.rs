// src/main.rs
use eframe::{NativeOptions, egui};
use std::path::PathBuf;

mod file_scan;
mod graph;
mod physics_nodes;
mod ui;

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    let scan_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        eprintln!("Usage: {} <path_to_directory_to_scan>", args[0]);
        eprintln!("Scanning current directory as no path was provided.");
        std::env::current_dir().expect("Failed to get current directory")
    };

    let app_name = "NexusView";
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(1200.0, 800.0))
            .with_title(app_name),
        ..Default::default()
    };

    eframe::run_native(
        app_name,
        options,
        Box::new(|_cc| Ok(Box::new(ui::FileGraphApp::new(scan_dir)))),
    )
}
