use eframe::egui::{self, CentralPanel, TopBottomPanel, ViewportBuilder};
use std::path::PathBuf;

pub fn run_ui(_path: Option<PathBuf>) {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("BLP Viewer")
            .with_inner_size([800.0, 600.0])
            .with_decorations(false), // 👈 без рамок и заголовка
                                      // .with_transparent(true)    // 👈 если надо прозрачное окно
                                      // .with_always_on_top(true)  // 👈 если нужно поверх всех
        ..Default::default()
    };

    eframe::run_native("blp-rs", options, Box::new(|_cc| -> Result<Box<dyn eframe::App>, _> { Ok(Box::new(App)) })).expect("failed to run eframe");
}

struct App;

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Импровизированная "титулка" для перетаскивания окна
        TopBottomPanel::top("drag_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("BLP Viewer");
                // ЛКМ по этой кнопке — начать перетаскивание окна
                if ui
                    .button("⠿")
                    .is_pointer_button_down_on()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        });

        CentralPanel::default().show(ctx, |ui| {
            if ui.button("🔄 Refresh").clicked() {
                // что-то делаем; при необходимости форсим перерисовку:
                // ctx.request_repaint();
            }
            if ui.button("❌ Exit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}
