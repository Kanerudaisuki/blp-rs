use crate::ui::viewer::app::App;
use egui::{self, Align2, Button, Frame, Margin, Response, ScrollArea, Sense, SidePanel, TextStyle, Ui};

impl App {
    pub(crate) fn draw_panel_left(&mut self, ctx: &egui::Context) {
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
                        // горизонтальный внутренний отступ слева/справа
                        Frame { inner_margin: Margin { left: spx_i, right: spx_i, top: 0, bottom: 0 }, ..Default::default() }.show(ui, |ui| {
                            ui.add_space(ui.spacing().item_spacing.y * 2.0);

                            for i in 0..16 {
                                // ЧИТАЕМ ТОЛЬКО ИЗ ImageBlp
                                let (w, h) = self
                                    .blp
                                    .as_ref()
                                    .and_then(|b| b.mipmaps.get(i))
                                    .map(|m| (m.width, m.height))
                                    .unwrap_or((0, 0));

                                mipmap_button_row(ui, &mut self.mip_visible[i], i, w, h);
                            }

                            // Кнопки All / None, поровну по ширине
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

                        // невидимый растягивающий спейсер, чтобы скролл работал корректно
                        let _ = ui.allocate_exact_size(egui::vec2(ui.available_width(), 0.0), Sense::hover());
                    });
            });
    }
}

// Кнопка-ряд: пустая кнопка как фон/hover/press; текст рисуем поверх painter’ом.
// Слева "#NN" прижат к центру справа, справа "WxH" прижат к центру слева.
pub fn mipmap_button_row(ui: &mut Ui, on: &mut bool, i: usize, w: u32, h: u32) -> Response {
    let row_h = ui.spacing().interact_size.y;
    let pad_l = 8.0; // внешний слева
    let pad_r = 8.0; // внешний справа
    let gap = 12.0; // прозрачный зазор по центру
    let inner = 6.0; // отступ текста от кромки своей половины

    // фон/hover/press — как у обычной кнопки
    let mut btn = Button::new("")
        .min_size(egui::vec2(ui.available_width(), row_h))
        .wrap();
    if *on {
        btn = btn.fill(ui.visuals().selection.bg_fill);
    }
    let resp = ui.add(btn);

    // зоны слева/справа от центрального зазора
    let rect = resp.rect;
    let cx = rect.center().x;
    let left = egui::Rect::from_min_max(egui::pos2(rect.left() + pad_l, rect.top()), egui::pos2((cx - gap * 0.5).max(rect.left() + pad_l), rect.bottom()));
    let right = egui::Rect::from_min_max(egui::pos2((cx + gap * 0.5).min(rect.right() - pad_r), rect.top()), egui::pos2(rect.right() - pad_r, rect.bottom()));

    // цвет/шрифт под состояние
    let vis = &ui.style().visuals.widgets;
    let text_col = if *on {
        vis.active.fg_stroke.color
    } else if resp.hovered() {
        vis.hovered.fg_stroke.color
    } else {
        vis.inactive.fg_stroke.color
    };
    let font_id = TextStyle::Monospace.resolve(ui.style());

    // слева: "#NN" — выравниваем по ПРАВОЙ кромке левой области (к центру)
    let left_pos = egui::pos2(left.right() - inner, left.center().y);
    ui.painter()
        .text(left_pos, Align2::RIGHT_CENTER, format!("#{i:02}"), font_id.clone(), text_col);

    // справа: "WxH" — по ЛЕВОЙ кромке правой области (к центру)
    let right_pos = egui::pos2(right.left() + inner, right.center().y);
    ui.painter()
        .text(right_pos, Align2::LEFT_CENTER, format!("{w}×{h}"), font_id, text_col);

    if resp.clicked() {
        *on = !*on;
    }
    resp
}
