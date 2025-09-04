use crate::ui::viewer::app::App;
use egui::{self, Frame, Margin, ScrollArea, Sense, SidePanel};

impl App {
    pub(crate) fn draw_panel_left(&mut self, ctx: &egui::Context) {
        SidePanel::left("left_mips")
            .resizable(false)
            .exact_width(180.0)
            .show_separator_line(false)
            .frame(Frame { inner_margin: Margin::same(0), ..Default::default() })
            .show(ctx, |ui| {
                let spx_f = ui.spacing().item_spacing.x;
                let spx_i = spx_f.round() as i8;

                ScrollArea::vertical()
                    .id_salt("left_scroll_mips")
                    .show(ui, |ui| {
                        Frame { inner_margin: Margin { left: spx_i, right: spx_i, top: 0, bottom: 0 }, ..Default::default() }.show(ui, |ui| {
                            ui.add_space(ui.spacing().item_spacing.y * 2.0);

                            let full_width = ui.available_width();

                            if ui
                                .add_sized([full_width, 0.0], egui::Button::new("Convert to BLP"))
                                .clicked()
                            {
                                // blp
                            }

                            if ui
                                .add_sized([full_width, 0.0], egui::Button::new("Convert to PNG"))
                                .clicked()
                            {
                                // png
                            }
                        });

                        let _ = ui.allocate_exact_size(egui::vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}
