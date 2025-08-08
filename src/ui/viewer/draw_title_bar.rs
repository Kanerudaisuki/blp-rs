use crate::ui::viewer::app::App;
use egui::{self, Frame, Margin, RichText, TopBottomPanel}; // проверь путь: App обычно не в run, а в app

impl App {
    pub(crate) fn draw_title_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("custom_title_bar")
            .exact_height(28.0)
            .frame(
                Frame::default()
                    .inner_margin(Margin::symmetric(8, 4))
                    .outer_margin(Margin::ZERO), // без внешних зазоров
                                                 // .fill(ui.visuals().panel_fill) // если нужен фон
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    //let title = ui.add(egui::Label::new(RichText::new("blp-rs").font(font(FontPreset::Title))).sense(egui::Sense::click()));
                    let title = ui.add(egui::Label::new(RichText::new("blp-rs")).sense(egui::Sense::click()));

                    if title.hovered() && ui.input(|i| i.pointer.primary_down()) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    if title.double_clicked() {
                        self.maximized = !self.maximized;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(self.maximized));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("✖")).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button(RichText::new("⬜")).clicked() {
                            self.maximized = !self.maximized;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(self.maximized));
                        }
                        if ui.button(RichText::new("➖")).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
    }
}
