pub(crate) use crate::ui::viewer::app::App;
use eframe::Renderer;
use eframe::egui::{self, CentralPanel, ViewportBuilder};
use std::path::PathBuf;

pub fn run_ui(_path: Option<PathBuf>) {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("BLP Viewer")
            .with_inner_size([800.0, 600.0])
            .with_decorations(false), // 👈 без рамок и заголовка
        // .with_transparent(true)    // 👈 если надо прозрачное окно
        // .with_always_on_top(true)  // 👈 если нужно поверх всех
        renderer: Renderer::Wgpu, // << Metal по умолчанию на macOS
        ..Default::default()
    };

    eframe::run_native(
        "blp-rs",
        options,
        Box::new(|cc| -> Result<Box<dyn eframe::App>, _> {
            Ok(Box::new(App::new(&cc.egui_ctx))) //
        }),

    )
    .expect("failed to run eframe");
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Рисуем шапку
        self.draw_title_bar(ctx);

        // Основное содержимое
        CentralPanel::default().show(ctx, |ui| {
            if ui.button("🔄 Refresh Хуй пизда").clicked() {
                // ctx.request_repaint();
            }
            if ui.button("❌ Exit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}
