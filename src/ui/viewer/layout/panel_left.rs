use crate::error::error::BlpError;
use crate::ui::viewer::app::App;
use crate::ui::viewer::layout::file_saver::export_quality::export_quality_save;
use crate::ui::viewer::layout::file_saver::save_same_dir::save_same_dir_save;
use eframe::egui::{Button, Context, CursorIcon, Frame, Label, Margin, RichText, ScrollArea, Sense, SidePanel, Slider, vec2};

impl App {
    fn default_names(&self) -> (String, String) {
        if let Some(p) = self.picked_file.as_ref() {
            if let Some(stem) = p
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
            {
                return (format!("{stem}.blp"), format!("{stem}.png"));
            }
        }
        // «имя из буфера» если нет файла; иначе общий фолбек
        let stem = if self.picked_file.is_none() { "clipboard" } else { "texture" };
        (format!("{stem}.blp"), format!("{stem}.png"))
    }

    fn run_export<F>(&mut self, f: F)
    where
        F: FnOnce(&crate::core::image::ImageBlp) -> Result<(), BlpError>,
    {
        let res = if let Some(img) = self.blp.as_ref() { f(img) } else { Err(BlpError::new("error-save-no-image")) };
        self.error = res.err();
    }

    pub(crate) fn draw_panel_left(&mut self, ctx: &Context) {
        SidePanel::left("left_panel")
            .resizable(false)
            .exact_width(190.0)
            .show_separator_line(false)
            .frame(Frame { inner_margin: Margin::same(0), ..Default::default() })
            .show(ctx, |ui| {
                let spx_f = ui.spacing().item_spacing.x;
                let spx_i = spx_f.round() as i8;

                ScrollArea::vertical()
                    .id_salt("left_panel_scroll")
                    .show(ui, |ui| {
                        Frame { inner_margin: Margin { left: spx_i, right: spx_i, top: 0, bottom: 0 }, ..Default::default() }.show(ui, |ui| {
                            ui.add_space(ui.spacing().item_spacing.y * 2.0);

                            // ------- Переключатель «Выбрать путь / Сохранить рядом» -------
                            let (label_key, hint_key) = if self.save_same_dir {
                                ("save-location-same-dir", "save-location-hint-same-dir")
                            } else {
                                ("save-location-select-path", "save-location-hint-select-path")
                            };
                            let label = self.tr(label_key);
                            let hint = if self.picked_file.is_some() {
                                self.tr(hint_key)
                            } else {
                                self.tr("save-location-hint-disabled-no-source")
                            };

                            ui.add_enabled_ui(self.picked_file.is_some(), |ui| {
                                if ui
                                    .add_sized([ui.available_width(), 0.0], Button::new(label))
                                    .on_hover_text(hint)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    self.save_same_dir = !self.save_same_dir;
                                    let _ = save_same_dir_save(self.save_same_dir);
                                }
                            });

                            // ------- Кнопки сохранения с тултипом конечного пути -------
                            ui.add_enabled_ui(!self.loading, |ui| {
                                let full_width = ui.available_width();
                                // Save as BLP…
                                let (def_blp, _) = self.default_names();
                                let blp_preview = self.preview_save_path(&def_blp, "blp");
                                let blp_tt = self.save_preview_tooltip(&blp_preview);

                                if ui
                                    .add_sized([full_width, 0.0], Button::new(self.tr("save-as-blp")))
                                    .on_hover_text(blp_tt)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    if let Some(path) = self.pick_save_path(&def_blp, "blp", self.tr("blp-texture")) {
                                        let export_quality = self.export_quality;
                                        let mip_visible = self.mip_visible;
                                        self.run_export(|img| img.export_blp(&path, export_quality, &mip_visible));
                                    }
                                }

                                // Save as PNG…
                                let (_, def_png) = self.default_names();
                                let png_preview = self.preview_save_path(&def_png, "png");
                                let png_tt = self.save_preview_tooltip(&png_preview);

                                if ui
                                    .add_sized([full_width, 0.0], Button::new(self.tr("save-as-png")))
                                    .on_hover_text(png_tt)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    if let Some(path) = self.pick_save_path(&def_png, "png", self.tr("png-image")) {
                                        self.run_export(|img| img.export_png(img.mipmaps.get(0).unwrap(), &path));
                                    }
                                }
                            });

                            ui.add_space(ui.spacing().item_spacing.y);

                            let quality_label = self.tr("blp-quality");
                            ui.add(Label::new(quality_label).wrap());

                            let quality_hint = self.tr("blp-quality-hint");
                            const VALUE_WIDTH: f32 = 44.0;
                            let slider_width = (ui.available_width() - VALUE_WIDTH - ui.spacing().item_spacing.x).max(0.0);
                            let mut slider_changed = false;
                            ui.horizontal(|ui| {
                                let slider_scope = ui.scope(|ui| {
                                    ui.set_width(slider_width);
                                    ui.spacing_mut().slider_width = slider_width;
                                    ui.add(Slider::new(&mut self.export_quality, 0..=100).show_value(false))
                                });
                                let slider_resp = slider_scope
                                    .inner
                                    .on_hover_text(quality_hint.clone());
                                if slider_resp.changed() {
                                    slider_changed = true;
                                }

                                let value_text = format!("{:>3}", self.export_quality);
                                ui.add_sized([VALUE_WIDTH, 0.0], Label::new(RichText::new(value_text).monospace()).wrap());
                            });
                            if slider_changed {
                                let _ = export_quality_save(self.export_quality);
                            }
                        });

                        let _ = ui.allocate_exact_size(vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}
