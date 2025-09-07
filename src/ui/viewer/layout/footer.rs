use crate::ui::viewer::app::App;
use egui::{Align, Color32, CornerRadius, Frame, Label, Layout, Margin, RichText, ScrollArea, Stroke, TopBottomPanel};

impl App {
    pub(crate) fn draw_footer(&mut self, ctx: &egui::Context) {
        if let Some(err) = self.err.clone().as_ref() {
            TopBottomPanel::bottom("footer_error")
                .resizable(true)
                .show_separator_line(false)
                .frame(Frame {
                    fill: Color32::from_rgba_unmultiplied(18, 8, 12, 230),
                    stroke: Stroke::new(1.0, Color32::from_rgb(255, 70, 70)),
                    inner_margin: Margin::symmetric(8, 8), // см. твои предпочтения по Margin
                    outer_margin: Margin::symmetric(0, 0),
                    corner_radius: CornerRadius::same(8u8), // и по CornerRadius
                    ..Default::default()
                })
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("⚠ Error")
                                .strong()
                                .color(Color32::from_rgb(255, 120, 120)),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Clear").clicked() {
                                self.err_clear();
                            }
                            if ui.button("Copy").clicked() {
                                ui.ctx()
                                    .copy_text(format!("```\n{}\n```", err));
                            }
                        });
                    });

                    ui.add_space(6.0);

                    ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add(
                                Label::new(RichText::new(err).monospace())
                                    .wrap()
                                    .selectable(true),
                            );
                        });
                });
        }
    }
}
