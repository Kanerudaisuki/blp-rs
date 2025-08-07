use crate::ui::viewer::ViewerApp;
use std::path::PathBuf;

use eframe::egui::ViewportBuilder;
use eframe::NativeOptions;

pub fn run_ui(path: Option<PathBuf>) -> Result<(), eframe::Error> {
    let options = NativeOptions {
        viewport: ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "BLP Viewer",
        options,
        Box::new(move |cc| {
            let app = ViewerApp::new(path, &cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
