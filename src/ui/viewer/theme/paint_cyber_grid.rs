pub fn paint_cyber_grid(ui: &mut egui::Ui) {
    use egui::{Color32, Stroke};
    let r = ui.max_rect();
    let p = ui.painter();

    // мягкое внешнее свечение
    p.rect_filled(r.expand(20.0), 0.0, Color32::from_rgba_unmultiplied(0, 220, 255, 10));

    // сетка
    let step = 22.0;
    let grid = Color32::from_rgba_unmultiplied(0, 220, 255, 25);
    let bold = Color32::from_rgba_unmultiplied(0, 220, 255, 60);
    let mut x = (r.left() / step).floor() * step;
    while x < r.right() {
        p.line_segment([egui::pos2(x, r.top()), egui::pos2(x, r.bottom())], Stroke::new(if (x / step) as i32 % 4 == 0 { 1.2 } else { 0.6 }, if (x / step) as i32 % 4 == 0 { bold } else { grid }));
        x += step;
    }
    let mut y = (r.top() / step).floor() * step;
    while y < r.bottom() {
        p.line_segment([egui::pos2(r.left(), y), egui::pos2(r.right(), y)], Stroke::new(if (y / step) as i32 % 4 == 0 { 1.2 } else { 0.6 }, if (y / step) as i32 % 4 == 0 { bold } else { grid }));
        y += step;
    }
}
