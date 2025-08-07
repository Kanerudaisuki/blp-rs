use crate::ui::viewer::ViewerApp;
use eframe::{App, NativeOptions};
use std::path::PathBuf;

pub fn run_ui(path: Option<PathBuf>) -> Result<(), eframe::Error> {
    let options = NativeOptions::default();

    eframe::run_native(
        "BLP Viewer",
        options,
        Box::new(|_cc| {
            let app: Box<dyn App> = Box::new(ViewerApp::new(path));
            Ok(app)
        }),
    )
}
