use egui::viewport::{ResizeDirection, ViewportCommand};
use egui::{Color32, Context, CursorIcon, Id, Order, Rect, Sense, Stroke, pos2, vec2};

/// Задать политики окна (зови в первый кадр или периодически).
pub fn apply_window_prefs(ctx: &Context) {
    ctx.send_viewport_cmd(ViewportCommand::Resizable(true)); // разрешаем ресайз
    ctx.send_viewport_cmd(ViewportCommand::ResizeIncrements(Some(vec2(8.0, 8.0)))); // шаг 8×8
    ctx.send_viewport_cmd(ViewportCommand::MinInnerSize(vec2(480.0, 320.0))); // мин. размер
    // при желании: MaxInnerSize, Decorations(false), и т.п.
}

/// Нижний-правый уголок, который запускает нативный ресайз по ЛКМ.
pub fn resize_corner_br(ctx: &Context) {
    let grip = 18.0;
    let screen = ctx.screen_rect();
    let corner = Rect::from_min_max(pos2(screen.right() - grip, screen.bottom() - grip), screen.right_bottom());

    egui::Area::new(Id::new("__br_resize_handle__"))
        .order(Order::Foreground)
        .fixed_pos(corner.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(corner.size(), Sense::click_and_drag());

            // визуальная "решётка"
            let p = ui.painter_at(rect);
            let stroke = Stroke::new(1.2, Color32::from_rgba_unmultiplied(220, 220, 220, 180));
            for i in 0..3 {
                let off = 3.0 + i as f32 * 4.0;
                p.line_segment([rect.right_bottom() - vec2(off, 0.0), rect.right_bottom() - vec2(0.0, off)], stroke);
            }

            if resp.hovered() || resp.dragged() {
                ui.ctx()
                    .set_cursor_icon(CursorIcon::ResizeSouthEast);
            }

            // важно: посылать в кадр нажатия или начала драга
            let pressed_now = ui.input(|i| i.pointer.primary_pressed());
            if (resp.hovered() && pressed_now) || resp.drag_started() {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::BeginResize(ResizeDirection::SouthEast));
            }
        });
}
