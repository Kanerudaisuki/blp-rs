pub(crate) use crate::ui::viewer::app::App;
use crate::ui::viewer::theme::apply_cyberpunk_style::apply_cyberpunk_style;
use crate::ui::viewer::theme::paint_bg_neon_maze::paint_bg_neon_maze;
use eframe::Renderer;
use eframe::egui::{self, ViewportBuilder};
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
        apply_cyberpunk_style(ctx);
        paint_bg_neon_maze(ctx, self.bg_seed);
        self.draw_title_bar(ctx);
        self.draw_file_picker_bar(ctx);
        self.draw_left_right_panels(ctx);
    }
}
