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
                        // ---
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
                        // ---

                        if let Some(tex) = &self.preview_tex {
                            ui.add(egui::Image::from_texture((tex.id(), tex.size_vec2())).max_size(ui.available_size()));
                        } else if self.loading {
                            ui.label("⏳ decoding…");
                        } else if let Some(e) = &self.last_err {
                            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), e);
                        }
                    });
            });
    }
}
