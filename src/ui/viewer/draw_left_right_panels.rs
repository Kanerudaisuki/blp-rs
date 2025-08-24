use crate::ui::viewer::app::App;
use egui::{self, CentralPanel, Frame, ScrollArea, SidePanel};

impl App {
    pub(crate) fn draw_left_right_panels(&mut self, ctx: &egui::Context) {
        // ← Левая фиксированная панель со скроллом
        SidePanel::left("left_fixed")
            .resizable(true)
            .exact_width(260.0)
            .show_separator_line(false)
            .frame(Frame::default())
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .id_salt("left_scroll")
                    .show(ui, |ui| {
                        let _ = ui.allocate_exact_size(egui::vec2(ui.available_width(), 0.0), egui::Sense::hover());
                        for i in 0..80 {
                            ui.label(format!("• item {i:02}"));
                        }
                    });
            });

        // → Правая панель (занимает остаток) со скроллом
        CentralPanel::default()
            .frame(Frame::default())
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .id_salt("right_scroll")
                    .show(ui, |ui| {
                        let _ = ui.allocate_exact_size(egui::vec2(ui.available_width(), 0.0), egui::Sense::hover());
                        for i in 0..200 {
                            ui.monospace(format!("log[{i:04}] :: system ping ok;"));
                        }
                    });
            });
    }
}
