pub(crate) use crate::ui::viewer::app::App;
use crate::ui::viewer::theme::apply_cyberpunk_style::apply_cyberpunk_style;
use crate::ui::viewer::theme::paint_bg_neon_maze::paint_bg_neon_maze;
use eframe::Renderer;
use eframe::egui::{self, ViewportBuilder};
use egui::SidePanel;
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
        paint_bg_neon_maze(ctx, self.bg_seed); // стабильный фон по сидy

        // 💠 Топбар — твой кастомный
        self.draw_title_bar(ctx);

        // ◀ Левая панель: фикс ширина, свой цвет/рамка
        let left_frame = egui::Frame {
            //fill: egui::Color32::from_rgba_unmultiplied(14, 24, 36, 230), //
            //stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 220, 255)),
            inner_margin: egui::Margin::same(8),
            outer_margin: egui::Margin::same(6),
            ..Default::default()
        };

        SidePanel::left("left_panel")
            .resizable(false)
            .exact_width(260.0)
            .frame(left_frame)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("◉ Modules");
                    ui.separator();
                    for i in 0..24 {
                        ui.add(egui::Button::new(format!("> Item #{i}")));
                    }
                });
            });

        // ▶ Центральный контент: фон‑сетка + скролл
        egui::CentralPanel::default()
            .frame(egui::Frame { fill: egui::Color32::from_rgba_unmultiplied(8, 14, 20, 220), inner_margin: egui::Margin::same(10), ..Default::default() })
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("🧩 Dashboard");
                    ui.add_space(6.0);

                    if ui
                        .add(egui::Button::new("⚡ Refresh"))
                        .clicked()
                    {
                        // ...
                    }
                    ui.add_space(8.0);

                    for i in 0..120 {
                        ui.label(format!("log[{i:03}] :: system ping ok;"));
                    }

                    ui.add_space(8.0);
                    if ui
                        .add(egui::Button::new("⏻ Exit"))
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
    }
}
