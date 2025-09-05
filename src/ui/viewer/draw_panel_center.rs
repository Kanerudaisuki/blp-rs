use crate::ui::viewer::app::App;
use egui::{self, Align, CentralPanel, Color32, Frame, Image, Label, Layout, Margin, RichText, ScrollArea, Sense, Ui, vec2};

impl App {
    pub(crate) fn draw_panel_center(&mut self, ctx: &egui::Context) {
        CentralPanel::default()
            .frame(Frame::default())
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .id_salt("right_scroll_mips")
                    .show(ui, |ui| {
                        let spy = ui.spacing().item_spacing.y;

                        ui.add_space(spy * 2.0);

                        if self.loading {
                            ui.label("Decoding…");
                            return;
                        }
                        if let Some(err) = &self.last_err {
                            ui.colored_label(Color32::from_rgb(255, 120, 120), format!("Error: {err}"));
                            return;
                        }

                        let pad_lr: i8 = ui.spacing().item_spacing.x.round() as i8;
                        for i in 0..16 {
                            if !self.mip_visible[i] {
                                continue;
                            }

                            let (w, h) = self
                                .blp
                                .as_ref()
                                .and_then(|b| b.mipmaps.get(i))
                                .map(|m| (m.width, m.height))
                                .unwrap_or((0, 0));

                            let tex_opt = self
                                .mip_textures
                                .get(i)
                                .and_then(|t| t.as_ref());

                            // внешний горизонтальный паддинг
                            Frame { inner_margin: Margin { left: pad_lr, right: pad_lr, top: 0, bottom: 0 }, ..Default::default() }.show(ui, |ui| {
                                draw_mip_block_columns(ui, i, w, h, tex_opt);
                            });

                            ui.add_space(spy);
                        }

                        // растяжка-строка
                        let _ = ui.allocate_exact_size(vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}

fn draw_mip_block_columns(ui: &mut Ui, i: usize, w: u32, h: u32, tex: Option<&egui::TextureHandle>) {
    let title = format!("#{i:02} {w}×{h}");

    // Считаем ширину правого текста (моно), чтобы оставить ему место справа
    let right_w = ui.fonts(|f| {
        let style = egui::TextStyle::Monospace.resolve(ui.style());
        let galley = f.layout_no_wrap(title.clone(), style, ui.style().visuals.text_color());
        galley.size().x
    });

    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
        let spacing = ui.spacing().item_spacing.x;

        // Сколько можно отдать левому блоку (картинке/тексту)
        let left_w = (ui.available_width() - right_w - spacing).max(0.0);

        // Левый блок фиксированной ширины; высота задаётся контентом (по верху)
        ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
            if let Some(tex) = tex {
                let tex_size = tex.size_vec2();
                if tex_size.x > 0.0 && tex_size.y > 0.0 && left_w > 0.0 {
                    // Не апскейлим: реальная ширина = min(left_w, исходная ширина)
                    let draw_w = left_w.min(tex_size.x);
                    let draw_h = draw_w * (tex_size.y / tex_size.x); // высота из ширины
                    ui.add(Image::from_texture((tex.id(), tex_size)).fit_to_exact_size(vec2(draw_w, draw_h)));
                } else {
                    ui.label(RichText::new("no image").italics());
                }
            } else {
                ui.label(RichText::new("no image").italics());
            }
        });

        // Спейсер, чтобы правый текст уехал к правому краю строки
        let rem = ui.available_width();
        if rem > right_w {
            ui.add_space(rem - right_w);
        }

        // Правый блок: подпись сверху справа
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            ui.add(Label::new(RichText::new(title).monospace()).truncate());
        });
    });
}
