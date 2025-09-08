use crate::ui::viewer::app::App;
use crate::ui::viewer::file_picker::all_image_exts::all_image_exts;
use crate::ui::viewer::file_picker::hotkey_pressed::hotkey_pressed;
use egui::{self, Align, Color32, Context, CursorIcon, Frame, Key, Layout, Margin, RichText, TopBottomPanel};
use std::path::Path;

impl App {
    pub(crate) fn draw_file_picker(&mut self, ctx: &Context) {
        // --- Drag & Drop файлов ---
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            if let Some(path) = f.path {
                self.pick_from_file(Some(path));
            }
        }

        // Хоткеи (надёжно): сработают и когда в буфере ТОЛЬКО картинка
        let open_hotkey = hotkey_pressed(ctx, Key::O);
        let paste_hotkey = hotkey_pressed(ctx, Key::V);

        // Подпись файла заранее
        let picked_opt = self.picked_file.clone();
        let (picked_str, has_file) = if let Some(p) = &picked_opt {
            (path_short(p, 72), true)
        } else {
            ("Drag a file here or use Select / Paste".to_string(), false)
        };

        // --- Отступы строго из темы ---
        let style = ctx.style();
        let spacing = &style.spacing; // egui::style::Spacing
        let item_gap_f = spacing.item_spacing.x; // f32
        let item_gap_i = item_gap_f as i8; // Margin требует i8

        let mut click_select = false;
        let mut click_paste = false;

        TopBottomPanel::top("file_picker_bar")
            .show_separator_line(false)
            .frame(Frame {
                fill: Color32::from_rgba_unmultiplied(8, 32, 44, 192), //
                stroke: style.visuals.widgets.inactive.bg_stroke,
                outer_margin: Margin { left: item_gap_i, right: item_gap_i, top: item_gap_i, bottom: 0 },
                inner_margin: Margin::symmetric(item_gap_i, item_gap_i),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add_enabled_ui(!self.loading, |ui| {
                        // --- Select ---
                        let select_resp = ui
                            .button("Select")
                            .on_hover_text("Open a file… (Cmd/Ctrl+O)")
                            .on_hover_cursor(CursorIcon::PointingHand);
                        if select_resp.clicked() {
                            click_select = true;
                        }

                        ui.add_space(item_gap_f);

                        // --- Paste ---
                        let paste_resp = ui
                            .button("Paste")
                            .on_hover_text("Paste image from clipboard (Cmd/Ctrl+V)")
                            .on_hover_cursor(CursorIcon::PointingHand);
                        if paste_resp.clicked() {
                            click_paste = true;
                        }
                    });

                    ui.add_space(item_gap_f * 1.5);

                    if has_file {
                        ui.label(RichText::new(picked_str.clone()).monospace());
                    } else {
                        ui.label(RichText::new(picked_str.clone()).italics());
                    }
                });
            });

        // --- Действия после UI ---
        if open_hotkey || click_select {
            self.file_dialog_open();
        }

        // Вставка: кнопка или хоткей. НЕ фильтруем через wants_keyboard_input —
        // иначе шорткат часто "гасится" у TextEdit на macOS.
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

fn path_short(p: &Path, max: usize) -> String {
    let s = p.display().to_string();
    if s.len() <= max {
        return s;
    }
    let tail = max.saturating_sub(3);
    format!("…{}", &s[s.len().saturating_sub(tail)..])
}
