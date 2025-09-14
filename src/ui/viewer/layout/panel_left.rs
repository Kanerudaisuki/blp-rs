use crate::export::export_blp::export_blp;
use crate::export::export_png::export_png;
use crate::export::last_dir::{load_last_dir, save_last_dir};
use crate::ui::viewer::app::App;
use eframe::egui::{vec2, Button, Context, CursorIcon, Frame, Margin, ScrollArea, Sense, SidePanel};
use std::path::PathBuf;

impl App {
    fn default_names(&self) -> (String, String) {
        let stem = self
            .current_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "texture".to_string());
        (format!("{stem}.blp"), format!("{stem}.png"))
    }

    fn pick_save_path(&mut self, default_name: &str, ext: &str, desc: &str) -> Option<PathBuf> {
        let mut dlg = rfd::FileDialog::new()
            .set_file_name(default_name)
            .add_filter(desc, &[ext]);

        if let Some(dir) = load_last_dir().or_else(|| {
            self.current_path
                .as_ref()
                .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        }) {
            dlg = dlg.set_directory(dir);
        }

        let path = dlg.save_file()?;
        if let Some(parent) = path.parent() {
            let _ = save_last_dir(parent); // best-effort
        }
        Some(path)
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

                            let full_width = ui.available_width();

                            // Save as BLP…
                            ui.add_enabled_ui(!self.loading, |ui| {
                                if ui
                                    .add_sized([ui.available_width(), 0.0], Button::new(self.tr("save-as-blp")))
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    let (def_blp, _) = self.default_names();
                                    if let Some(path) = self.pick_save_path(&def_blp, "blp", "BLP texture") {
                                        let res = if let Some(img) = self.blp.as_ref() {
                                            export_blp(img, &path, 100)
                                        } else {
                                            Err("No image loaded".into())
                                        };

                                        match res {
                                            Ok(()) => self.err_clear(),
                                            Err(e) => self.err_set(e),
                                        }
                                    }
                                }

                                // Save as PNG…
                                if ui
                                    .add_sized([full_width, 0.0], Button::new(self.tr("save-as-png")))
                                    .on_hover_cursor(CursorIcon::PointingHand)
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
                                            Ok(()) => self.err_clear(),
                                            Err(e) => self.err_set(e),
                                        }
                                    }
                                }
                            });
                        });

                        let _ = ui.allocate_exact_size(vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}
