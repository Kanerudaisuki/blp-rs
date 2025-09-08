use crate::ui::viewer::app::App;
use eframe::Renderer;
use egui::ViewportBuilder;
use std::path::PathBuf;

pub fn run_native(path: Option<PathBuf>) {
    eframe::run_native(
        "blp-rs",
        eframe::NativeOptions {
            persist_window: true,
            viewport: ViewportBuilder {
                title: Some("blp-rs".to_string()), //
                app_id: Some("org.warraft.blp-rs".to_string()),
                inner_size: Some(egui::vec2(800.0, 680.0)),
                clamp_size_to_monitor_size: Some(true),
                decorations: Some(false),
                resizable: Some(true),
                ..Default::default()
            },
            renderer: Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(move |cc| -> Result<Box<dyn eframe::App>, _> {
            // Ваш обычный App
            let mut app = App::new(&cc.egui_ctx);
            app.pick_from_file(path);
            Ok(Box::new(app))
        }),
    )
    .expect("Failed to run eframe.");
}
