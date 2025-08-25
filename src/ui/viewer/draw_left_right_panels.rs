use crate::ui::viewer::app::App;
use crate::ui::viewer::mipmap_button_row::mipmap_button_row;
use egui::{self, Align, Button, CentralPanel, Color32, CornerRadius, Frame, Layout, Margin, ScrollArea, Sense, SidePanel, Stroke, StrokeKind, UiBuilder};

impl App {
    pub(crate) fn draw_left_right_panels(&mut self, ctx: &egui::Context) {
        SidePanel::left("left_mips")
            .resizable(false)
            .exact_width(260.0)
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
                            for i in 0..16 {
                                let (w, h) = if self.is_blp {
                                    self.blp
                                        .as_ref()
                                        .and_then(|b| b.mipmaps.get(i))
                                        .map(|m| (m.width, m.height))
                                        .unwrap_or((0, 0))
                                } else if let Some(Some(tex)) = self.mip_textures.get(i) {
                                    let s = tex.size_vec2();
                                    (s.x as u32, s.y as u32)
                                } else if let Some(Some(base)) = self.mip_textures.get(0) {
                                    let s0 = base.size_vec2();
                                    ((s0.x as u32).max(1) >> i, (s0.y as u32).max(1) >> i)
                                } else {
                                    (0, 0)
                                };
                                mipmap_button_row(ui, &mut self.mip_visible[i], i, w, h);
                            }

                            let row_h = ui.spacing().interact_size.y;
                            ui.columns(2, |cols| {
                                if cols[0]
                                    .add_sized([cols[0].available_width(), row_h], Button::new("All").wrap())
                                    .clicked()
                                {
                                    self.mip_visible.fill(true);
                                }

                                if cols[1]
                                    .add_sized([cols[1].available_width(), row_h], Button::new("None").wrap())
                                    .clicked()
                                {
                                    self.mip_visible.fill(false);
                                }
                            });
                        });

                        let _ = ui.allocate_exact_size(egui::vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });

        // → RIGHT: все мипы блоками; если нет изображения — пишем "no image"
        CentralPanel::default()
            .frame(Frame::default())
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .id_salt("right_scroll_mips")
                    .show(ui, |ui| {
                        let _ = ui.allocate_exact_size(egui::vec2(ui.available_width(), 0.0), Sense::hover());
                        // ---

                        if self.loading {
                            ui.label("Decoding…");
                            return;
                        }
                        if let Some(err) = &self.last_err {
                            ui.colored_label(Color32::from_rgb(255, 120, 120), format!("Error: {err}"));
                            return;
                        }

                        for i in 0..16 {
                            if !self.mip_visible[i] {
                                continue;
                            }

                            // размеры уровня для заголовка
                            let (w, h) = if self.is_blp {
                                self.blp
                                    .as_ref()
                                    .and_then(|b| b.mipmaps.get(i))
                                    .map(|m| (m.width, m.height))
                                    .unwrap_or((0, 0))
                            } else if let Some(Some(tex)) = self.mip_textures.get(i) {
                                let s = tex.size_vec2();
                                (s.x as u32, s.y as u32)
                            } else if let Some(Some(base)) = self.mip_textures.get(0) {
                                let s0 = base.size_vec2();
                                ((s0.x as u32).max(1) >> i, (s0.y as u32).max(1) >> i)
                            } else {
                                (0, 0)
                            };

                            // ── карточка мипа ──────────────────────────────────────────────
                            // бордер вокруг блока
                            let block_w = ui.available_width();
                            let header_h = 24.0;

                            // выделяем общий прямоугольник блока (с небольшими внешними отступами)
                            let (block_rect, _resp) = ui.allocate_exact_size(egui::vec2(block_w, 0.0), Sense::hover()); // сначала «строка»
                            // растянем высоту позже, когда отрисуем содержимое

                            // рисуем внутри обычным лэйаутом:
                            // 1) Header-блок
                            let header_rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(block_w, header_h));
                            // фон хедера + нижняя линия
                            ui.painter()
                                .rect_filled(header_rect, CornerRadius::same(6u8), ui.visuals().panel_fill);
                            ui.painter().hline(
                                header_rect.x_range(),
                                header_rect.bottom(),
                                Stroke::new(
                                    1.0,
                                    ui.visuals()
                                        .widgets
                                        .noninteractive
                                        .bg_stroke
                                        .color,
                                ),
                            );

                            // текст в хедере: слева WxH, справа #NN
                            let mut h_ui = ui.new_child(
                                UiBuilder::new()
                                    .max_rect(header_rect)
                                    .layout(Layout::left_to_right(Align::Center)),
                            );
                            h_ui.set_clip_rect(header_rect);
                            h_ui.monospace(format!("{w}×{h}"));
                            // spacer
                            let spacer = h_ui.available_width();
                            h_ui.add_space(spacer);
                            h_ui.monospace(format!("#{i:02}"));

                            // 2) Контент-область (картинка или "no image")
                            //    небольшой внутренний паддинг
                            ui.add_space(6.0);
                            if let Some(Some(tex)) = self.mip_textures.get(i) {
                                // обрамим картинку тонким бордером
                                let img_w = ui.available_width();
                                let (img_rect, _r) = ui.allocate_exact_size(egui::vec2(img_w, 0.0), Sense::hover());
                                // рисуем рамку по периметру блока

                                // собственно картинка
                                ui.add(egui::Image::from_texture((tex.id(), tex.size_vec2())).max_size(egui::vec2(ui.available_width(), f32::MAX)));

                                // теперь рамка вокруг всего блока (после картинки),
                                // используем последний y курсора как низ блока
                                let block_bottom = ui.cursor().min.y;
                                let rect = egui::Rect::from_min_max(header_rect.min, egui::pos2(header_rect.right(), block_bottom));
                                ui.painter().rect_stroke(
                                    rect,
                                    CornerRadius::same(6u8),
                                    Stroke::new(
                                        1.0,
                                        ui.visuals()
                                            .widgets
                                            .noninteractive
                                            .bg_stroke
                                            .color,
                                    ),
                                    StrokeKind::Outside,
                                );
                            } else {
                                ui.label("no image");
                                // рамка вокруг «пустого» блока (header + text)
                                let block_bottom = ui.cursor().min.y;
                                let rect = egui::Rect::from_min_max(header_rect.min, egui::pos2(header_rect.right(), block_bottom));
                                ui.painter().rect_stroke(
                                    rect,
                                    CornerRadius::same(6u8),
                                    Stroke::new(
                                        1.0,
                                        ui.visuals()
                                            .widgets
                                            .noninteractive
                                            .bg_stroke
                                            .color,
                                    ),
                                    StrokeKind::Outside,
                                );
                            }

                            ui.add_space(12.0); // gap между карточками
                        }
                    });
            });
    }
}
