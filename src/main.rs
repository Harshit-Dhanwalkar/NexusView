mod file_scan;
mod graph;
mod physics_nodes;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Interactive File Graph"),
        ..Default::default()
    };

    eframe::run_native(
        "Interactive File Graph",
        options,
        Box::new(|cc| Ok(Box::new(ui::FileGraphApp::new(cc, "./dummy_dir")))),
    )
}
