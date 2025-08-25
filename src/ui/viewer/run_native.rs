use crate::ui::viewer::app::App;
use eframe::Renderer;
use egui::ViewportBuilder;
use std::path::PathBuf;

pub fn run_native(path: Option<PathBuf>) {
    eframe::run_native(
        "blp-rs",
        eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_title("blp-rs")
                .with_inner_size([800.0, 684.0])
                .with_decorations(false), // 👈 без рамок и заголовка
            // .with_transparent(true)    // 👈 если надо прозрачное окно
            // .with_always_on_top(true)  // 👈 если нужно поверх всех
            renderer: Renderer::Wgpu, // << Metal по умолчанию на macOS
            ..Default::default()
        },
        Box::new(|cc| -> Result<Box<dyn eframe::App>, _> {
            let mut app = App::new(&cc.egui_ctx);
            app.set_initial_file(path);
            Ok(Box::new(app))
        }),
    )
    .expect("failed to run eframe");
}
