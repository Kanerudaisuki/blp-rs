use crate::export::blp::export_blp;
use crate::export::png::export_png;
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

                            // Save as BLP…
                            if ui
                                .add_sized([full_width, 0.0], egui::Button::new("Save as BLP"))
                                .clicked()
                            {
                                // 1) сначала выбираем путь (требуется &mut self)
                                let (def_blp, _) = self.default_names();
                                if let Some(path) = self.pick_save_path(&def_blp, "blp", "BLP texture") {
                                    // 2) теперь берём img (immut borrow живёт только внутри этого if)
                                    let res = if let Some(img) = self.blp.as_ref() {
                                        export_blp(img, &path, 100)
                                    } else {
                                        Err("No image loaded".into())
                                    };

                                    match res {
                                        Ok(()) => self.save_err = None,
                                        Err(e) => self.save_err = Some(format!("Save BLP failed: {e}")),
                                    }
                                }
                            }

                            // Save as PNG…
                            if ui
                                .add_sized([full_width, 0.0], egui::Button::new("Save as PNG"))
                                .clicked()
                            {
                                let (_, def_png) = self.default_names();
                                if let Some(path) = self.pick_save_path(&def_png, "png", "PNG image") {
                                    let res = if let Some(img) = self.blp.as_ref() {
                                        export_png(img, &path)
                                    } else {
                                        Err("No image loaded".into())
                                    };

                                    match res {
                                        Ok(()) => self.save_err = None,
                                        Err(e) => self.save_err = Some(format!("Save PNG failed: {e}")),
                                    }
                                }
                            }

                            if let Some(err) = &self.save_err {
                                ui.colored_label(egui::Color32::from_rgb(255, 120, 120), err);
                            }
                        });

                        let _ = ui.allocate_exact_size(egui::vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}
