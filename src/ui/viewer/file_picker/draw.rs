use crate::decode::decode_input::{DecodeInput, decode_input};
use crate::ui::viewer::app::App;
use crate::ui::viewer::file_picker::all_image_exts::all_image_exts;
use arboard::Clipboard;
use egui::{self, Align, Color32, Context, CursorIcon, Event, Frame, Key, KeyboardShortcut, Layout, Margin, Modifiers, RichText, TopBottomPanel};
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

// ---------- хоткей-хелперы: максимально совместимо ----------
fn hotkey_pressed(ctx: &Context, key: Key) -> bool {
    // 1) Нормальный путь: шорткат (Cmd на mac / Ctrl на win/linux)
    let via_shortcut = ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, key)));
    if via_shortcut {
        return true;
    }

    // 2) Фолбэк: комбо (съедаем точную пару command+key, Shift допустим, Alt — нет)
    let via_combo = ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, key));
    if via_combo {
        return true;
    }

    // 3) Сырой ивент (на случай экзотики на macOS)
    ctx.input(|i| {
        i.raw.events.iter().any(|e| {
            matches!(
                e,
                Event::Key {
                    key: k,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if *k == key && modifiers.command && !modifiers.alt
            )
        })
    })
}

impl App {
    pub(crate) fn set_current_clipboard(&mut self) -> Result<(), String> {
        // Читаем картинку из буфера обмена
        let mut cb = Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
        let img = cb
            .get_image()
            .map_err(|e| format!("No image in clipboard: {e}"))?;

        let w = img.width as u32;
        let h = img.height as u32;
        #[allow(unused_mut)]
        let mut rgba = img.bytes.into_owned();

        // Windows: BGRA → RGBA
        #[cfg(target_os = "windows")]
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
        }

        let rgba_img = image::RgbaImage::from_raw(w, h, rgba).ok_or("Invalid clipboard image buffer")?;
        let dyn_img = DynamicImage::ImageRgba8(rgba_img);

        // Кодируем во временный PNG (твой декодер ест png/jpg/…)
        let mut buf = Vec::new();
        dyn_img
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .map_err(|e| format!("Encode PNG failed: {e}"))?;

        // Сброс состояния + запуск декодера в фоне
        self.picked_file = None;
        self.err_clear();
        self.blp = None;
        self.selected_mip = 0;
        self.mip_textures.fill_with(|| None);

        let (tx, rx) = mpsc::sync_channel(1);
        self.decode_rx = Some(rx);
        self.loading = true;

        thread::spawn(move || {
            let res = decode_input(DecodeInput::Bytes(buf));
            let _ = tx.send(res);
        });

        Ok(())
    }

    pub(crate) fn file_picker_draw(&mut self, ctx: &Context) {
        // --- Drag & Drop файлов ---
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            if let Some(path) = f.path {
                self.set_current_file(Some(path));
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
                outer_margin: Margin::symmetric(item_gap_i, item_gap_i),
                inner_margin: Margin::symmetric(item_gap_i, item_gap_i),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
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
            if let Err(e) = self.set_current_clipboard() {
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
            self.set_current_file(Some(path));
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
