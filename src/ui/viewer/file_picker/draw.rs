use crate::decode::decode_input::{decode_input, DecodeInput};
use crate::ui::viewer::app::App;
use arboard::Clipboard;
use egui::{self};
use image::{DynamicImage, ImageFormat};
use std::collections::BTreeSet;
use std::io::Cursor;
use std::path::Path;
use std::sync::{mpsc, OnceLock};
use std::thread;

impl App {
    pub(crate) fn set_current_clipboard(&mut self) -> Result<(), String> {
        // Берём пиксели из буфера обмена
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

        // Кодируем в PNG в память (ImageBlp::from_bytes ест png/jpg/…)
        let mut buf = Vec::new();
        dyn_img
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .map_err(|e| format!("Encode PNG failed: {e}"))?;

        // Сбрасываем состояние и шлём в фоновый декодер
        self.picked_file = None;
        self.last_err = None;
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

    pub(crate) fn file_picker_draw(&mut self, ctx: &egui::Context) {
        // drag & drop
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            if let Some(path) = f.path {
                self.set_current_file(Some(path));
            }
        }

        // hotkeys → только флаги
        let open_hotkey = ctx.input(|i| i.key_pressed(egui::Key::O) && i.modifiers.command);
        let paste_hotkey = ctx.input(|i| i.key_pressed(egui::Key::V) && i.modifiers.command);

        // подпись заранее (не трогаем &self в UI-блоке)
        let picked_opt = self.picked_file.clone();
        let (picked_str, has_file) = if let Some(p) = &picked_opt {
            (path_short(p, 72), true)
        } else {
            ("Drag a file here or use Select / Paste".to_string(), false)
        };

        let style = ctx.style();
        let spx_f = style.spacing.item_spacing.x;
        let spx_i = spx_f as i8;

        let mut click_select = false;
        let mut click_paste = false;

        egui::TopBottomPanel::top("file_picker_bar")
            .show_separator_line(false)
            .frame(egui::Frame { fill: egui::Color32::from_rgba_unmultiplied(8, 32, 44, 192), stroke: style.visuals.widgets.inactive.bg_stroke, outer_margin: egui::Margin { top: spx_i, left: spx_i, right: spx_i, bottom: 0 }, inner_margin: egui::Margin::symmetric(spx_i, spx_i), ..Default::default() })
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    // --- Select ---
                    let select_resp = ui
                        .button("Select")
                        .on_hover_text("Open a file… (Cmd/Ctrl+O)")
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if select_resp.clicked() {
                        click_select = true;
                    }

                    ui.add_space(8.0);

                    // --- Paste ---
                    let paste_resp = ui
                        .button("Paste")
                        .on_hover_text("Paste image from clipboard (Cmd/Ctrl+V)")
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if paste_resp.clicked() {
                        click_paste = true;
                    }

                    ui.add_space(12.0);

                    if has_file {
                        ui.label(egui::RichText::new(picked_str.clone()).monospace());
                    } else {
                        ui.label(egui::RichText::new(picked_str.clone()).italics());
                    }
                });
            });

        // действия — после UI-блока (без конфликтов заимствований)
        if open_hotkey || click_select {
            self.file_dialog_open();
        }
        if paste_hotkey || click_paste {
            if let Err(e) = self.set_current_clipboard() {
                self.last_err = Some(e);
            }
        }
    }

    fn file_dialog_open(&mut self) {
        let mut dlg = rfd::FileDialog::new()
            .set_title("Select image")
            .add_filter("All images", all_image_exts());

        // Доп. фильтры по вкусу (необязательно)
        dlg = dlg
            .add_filter("BLP", &["blp"])
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg"]);

        // Стартовая директория: рядом с текущим файлом, иначе CWD
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

/// Кэшируем один раз на процесс
static ALL_IMAGE_EXTS: OnceLock<Vec<&'static str>> = OnceLock::new();

fn all_image_exts() -> &'static [&'static str] {
    ALL_IMAGE_EXTS
        .get_or_init(|| {
            let mut set: BTreeSet<&'static str> = BTreeSet::new();

            // Все форматы, известные crate `image` (зависят от включённых фич)
            for fmt in ImageFormat::all() {
                for &ext in fmt.extensions_str() {
                    set.insert(ext);
                }
            }

            // Плюс наши кастомные
            set.insert("blp");

            // Если хочешь явно добавить редкие, которые у тебя точно поддержаны, раскомментируй:
            // set.insert("dds");
            // set.insert("tga");
            // set.insert("qoi");
            // set.insert("avif");

            set.into_iter().collect::<Vec<_>>()
        })
        .as_slice()
}