use crate::ui::viewer::app::App;
use crate::ui::viewer::file_picker::all_image_exts::all_image_exts;
use crate::ui::viewer::file_picker::hotkey_pressed::hotkey_pressed;
use egui::{self, Color32, CornerRadius, Galley, StrokeKind, TextStyle};
use std::path::Path;
use std::sync::Arc;

impl App {
    pub(crate) fn draw_file_picker(&mut self, ctx: &egui::Context) {
        // DnD
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            if let Some(path) = f.path {
                self.pick_from_file(Some(path));
            }
        }

        let open_hotkey = hotkey_pressed(ctx, egui::Key::O);
        let paste_hotkey = hotkey_pressed(ctx, egui::Key::V);

        let style = ctx.style();
        let spacing = &style.spacing;
        let gap = spacing.item_spacing.x;
        let gap_i = gap as i8;

        let mut click_select = false;
        let mut click_paste = false;

        egui::TopBottomPanel::top("file_picker_bar")
            .show_separator_line(false)
            .frame(egui::Frame {
                fill: Color32::TRANSPARENT, //
                stroke: egui::Stroke::NONE,
                outer_margin: egui::Margin { left: gap_i, right: gap_i, top: gap_i, bottom: 0 },
                inner_margin: egui::Margin::same(0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.add_enabled_ui(!self.loading, |ui| {
                        if ui
                            .button("Open")
                            .on_hover_text("Open a file (Cmd/Ctrl+O)")
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            click_select = true;
                        }
                        ui.add_space(gap);
                        if ui
                            .button("Paste")
                            .on_hover_text("Paste image from clipboard (Cmd/Ctrl+V)")
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            click_paste = true;
                        }
                    });

                    ui.add_space(gap);

                    let row_h = ui.spacing().interact_size.y;
                    let w = ui.available_width();

                    if let Some(path) = self.picked_file.clone() {
                        let mut s = path_abs(&path);
                        let te = egui::TextEdit::singleline(&mut s)
                            .font(TextStyle::Monospace)
                            .cursor_at_end(true)
                            .desired_width(w); // тянем на всю оставшуюся ширину
                        let resp = ui.add_sized([w, row_h], te);
                        resp.on_hover_text(&s);
                    } else {
                        let s = if self.blp.is_some() { "Pasted image (clipboard)" } else { "Drag a file here or use Open / Paste" };
                        let style = ui.style().clone();
                        let spacing = style.spacing.clone();
                        let pad = spacing.button_padding;
                        let font_id = TextStyle::Button.resolve(&style);

                        let galley: Arc<Galley> = ui.fonts(|f| f.layout_no_wrap(s.to_owned(), font_id.clone(), style.visuals.text_color()));

                        let size = egui::vec2(galley.size().x + pad.x * 2.0, galley.size().y + pad.y * 2.0);
                        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::click());

                        let stroke = style.visuals.widgets.inactive.bg_stroke;
                        let fill = Color32::from_rgba_unmultiplied(8, 32, 44, 192);
                        ui.painter()
                            .rect(rect, CornerRadius::same(0), fill, stroke, StrokeKind::Inside);

                        let text_pos = egui::pos2(rect.min.x + pad.x, rect.min.y + pad.y);
                        ui.painter()
                            .galley(text_pos, galley, style.visuals.text_color());
                    }
                });
            });

        // Действия
        if open_hotkey || click_select {
            self.file_dialog_open();
        }
        if click_paste || paste_hotkey {
            if let Err(e) = self.pick_from_clipboard() {
                self.err_set(e);
            }
        }
    }

    fn file_dialog_open(&mut self) {
        let mut dlg = rfd::FileDialog::new()
            .set_title("Select image")
            .add_filter("All images", all_image_exts());

        if let Some(dir) = self
            .picked_file
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .or_else(|| std::env::current_dir().ok())
        {
            dlg = dlg.set_directory(dir);
        }

        if let Some(path) = dlg.pick_file() {
            self.pick_from_file(Some(path));
        }
    }
}

// Абсолютный путь (копируемый)
fn path_abs(p: &Path) -> String {
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(p)
    } else {
        p.to_path_buf()
    };
    abs.to_string_lossy().into_owned()
}
