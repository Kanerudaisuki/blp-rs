use eframe::egui::{self, CentralPanel, ViewportBuilder};
use std::path::PathBuf;

pub fn run_ui(_path: Option<PathBuf>) {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("BLP Viewer")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "blp-rs", //
        options,
        Box::new(|_cc| -> Result<Box<dyn eframe::App>, _> { Ok(Box::new(App)) }),
    )
    .expect("failed to run eframe");
}

struct App;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            if ui.button("🔄 Refresh").clicked() {
                // по желанию:
                // ctx.request_repaint();
            }
            if ui.button("❌ Exit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}
