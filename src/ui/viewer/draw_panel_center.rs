use crate::ui::viewer::app::App;
use egui::epaint::MarginF32;
use egui::{self, Align, CentralPanel, Color32, Frame, Label, Layout, Margin, ScrollArea, Sense, Stroke, Ui, UiBuilder};

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
                        let _ = ui.allocate_exact_size(egui::vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}

fn draw_mip_block_columns(ui: &mut Ui, i: usize, w: u32, h: u32, tex: Option<&egui::TextureHandle>) {
    let stroke: Stroke = ui
        .visuals()
        .widgets
        .noninteractive
        .bg_stroke;

    // Рамка блока БЕЗ внутренних отступов
    Frame {
        stroke, //
        ..Default::default()
    }
    .show(ui, |ui| {
        let spacing = &mut ui.style_mut().spacing;
        let ispy = spacing.item_spacing.y;
        let ispx = spacing.item_spacing.x;
        spacing.item_spacing.y = 0.0;

        let isy = ui.spacing().interact_size.y;

        let (hdr_rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), isy), Sense::hover());

        let mut hdr_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(hdr_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );

        hdr_ui.columns(2, |cols| {
            // Левая колонка: WxH
            cols[0].with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(8.0); // небольшой левый паддинг
                ui.add(
                    Label::new(egui::RichText::new(format!("{w}×{h}")).monospace()).truncate(), // однострочно с …
                );
            });

            // Правая колонка: #NN (выравниваем вправо)
            cols[1].with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0); // симметричный зазор от правого края
                ui.add(Label::new(egui::RichText::new(format!("#{i:02}")).monospace()).truncate());
            });
        });

        // Нижний бордер заголовка (pixel-snap, чтобы без «ступенек»)
        let ppp = ui.ctx().pixels_per_point();
        let y = (hdr_rect.bottom() * ppp).round() / ppp;
        ui.painter()
            .hline(hdr_rect.x_range(), y, stroke);

        // ── Content: картинка или "no image" ──────────────────────────────
        if let Some(tex) = tex {
            ui.add(egui::Image::from_texture((tex.id(), tex.size_vec2())).max_size(egui::vec2(ui.available_width(), f32::MAX)));
        } else {
            Frame {
                inner_margin: Margin::from(MarginF32 {
                    left: ispy, //
                    right: ispy,
                    top: ispx,
                    bottom: ispx,
                }),
                ..Default::default()
            }
            .show(ui, |ui| {
                ui.label("no image");
            });
        }
    });
}
