use egui::{self, Align2, Button, Response, TextStyle, Ui};

pub fn mipmap_button_row(ui: &mut Ui, on: &mut bool, i: usize, w: u32, h: u32) -> Response {
    let row_h = ui.spacing().interact_size.y;
    let pad_l = 8.0; // внешний слева
    let pad_r = 8.0; // внешний справа
    let gap = 12.0; // прозрачный зазор по центру
    let inner = 6.0; // внутренний отступ от кромки к тексту

    // сама кнопка (фон/hover/press — как обычно)
    let mut btn = Button::new("")
        .min_size(egui::vec2(ui.available_width(), row_h))
        .wrap();
    if *on {
        btn = btn.fill(ui.visuals().selection.bg_fill);
    }
    let resp = ui.add(btn);

    // области слева/справа от зазора
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

    // справа: "WxH" — выравниваем по ЛЕВОЙ кромке правой области (к центру)
    let right_pos = egui::pos2(right.left() + inner, right.center().y);
    ui.painter()
        .text(right_pos, Align2::LEFT_CENTER, format!("{w}×{h}"), font_id, text_col);

    if resp.clicked() {
        *on = !*on;
    }
    resp
}
