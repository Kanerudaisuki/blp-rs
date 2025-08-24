use crate::ui::viewer::app::App;
use eframe::epaint::Color32;
use egui::{self, Align, CornerRadius, Frame, Key, Layout, Margin, RichText, Stroke, TopBottomPanel};
use std::path::Path;

impl App {
    pub(crate) fn draw_file_picker_bar(&mut self, ctx: &egui::Context) {
        for f in ctx.input(|i| i.raw.dropped_files.clone()) {
            if let Some(path) = f.path {
                self.set_initial_file(Some(path));
            }
        }

        // Cmd/Ctrl+O
        let open_hotkey = ctx.input(|i| i.key_pressed(Key::O) && i.modifiers.command);
        if open_hotkey {
            self.open_file_dialog();
        }

        TopBottomPanel::top("file_picker_bar")
            .show_separator_line(false)
            .frame(Frame {
                fill: Color32::from_rgba_unmultiplied(8, 32, 44, 192), //
                stroke: Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 220, 255, 128)),
                outer_margin: Margin { top: 10, left: 10, right: 10, bottom: 0 },
                corner_radius: CornerRadius::same(8),
                inner_margin: Margin::symmetric(10, 8), //
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    if ui.button("Select…").clicked() {
                        self.open_file_dialog();
                    }
                    ui.add_space(8.0);

                    if let Some(p) = &self.picked_file {
                        ui.label(RichText::new(Self::path_short(p, 72)).monospace());
                    } else {
                        ui.label(RichText::new("Drag the file here or click “Select...”").italics());
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label("Cmd/Ctrl+O");
                    });
                });
            });
    }

    fn open_file_dialog(&mut self) {
        let mut dlg = rfd::FileDialog::new()
            .set_title("Select file")
            .add_filter("BLP/PNG/JPG", &["blp", "png", "jpg", "jpeg"])
            .add_filter("All files", &["*"]);

        // стартовая директория: рядом с уже выбранным файлом, иначе cwd
        if let Some(dir) = self
            .picked_file
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .or_else(|| std::env::current_dir().ok())
        {
            dlg = dlg.set_directory(dir);
        }

        if let Some(path) = dlg.pick_file() {
            self.set_initial_file(Some(path));
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
}
