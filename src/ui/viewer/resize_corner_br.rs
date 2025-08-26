use egui::viewport::{ResizeDirection, ViewportCommand};
use egui::{Color32, Context, CursorIcon, Id, Order, Rect, Sense, Stroke, pos2, vec2};

/// Ручка ресайза в правом-нижнем углу для borderless окна (egui 0.32).
pub fn resize_corner_br(ctx: &Context) {
    let screen = ctx.screen_rect();
    if screen.is_negative() {
        return;
    }

    let grip = 18.0_f32;
    let corner = Rect::from_min_max(pos2(screen.right() - grip, screen.bottom() - grip), screen.right_bottom());

    egui::Area::new(Id::new("__br_resize_handle__"))
        .order(Order::Foreground)
        .interactable(true)
        .movable(false)
        .fixed_pos(corner.min)
        .show(ctx, |ui| {
            // Хитбокс
            let (rect, response) = ui.allocate_exact_size(corner.size(), Sense::click_and_drag());

            // Визуальные "бороздки"
            let painter = ui.painter_at(rect);
            let stroke = Stroke::new(1.2, Color32::from_rgba_unmultiplied(220, 220, 220, 180));
            let pad = 3.0;
            for i in 0..3 {
                let off = pad + i as f32 * 4.0;
                let a = rect.right_bottom() - vec2(off, 0.0);
                let b = rect.right_bottom() - vec2(0.0, off);
                painter.line_segment([a, b], stroke);
            }

            // Курсор
            if response.hovered() || response.dragged() {
                ui.ctx()
                    .set_cursor_icon(CursorIcon::ResizeSouthEast);
            }

            // Нативный ресайз: вызывать в кадр нажатия ЛКМ
            let pressed_now = ui.input(|i| i.pointer.primary_pressed());
            if (response.hovered() && pressed_now) || response.drag_started() {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::BeginResize(ResizeDirection::SouthEast));
            }

            // Fallback: ручное изменение размера, если BeginResize не сработал
            #[derive(Clone, Copy, Default)]
            struct DragState {
                start_w: f32,
                start_h: f32,
                start_x: f32,
                start_y: f32,
                active: bool,
            }

            let mem_id = ui.make_persistent_id("__br_resize_state__");
            let mut st = ui
                .ctx()
                .memory_mut(|m| {
                    m.data
                        .get_persisted::<DragState>(mem_id)
                })
                .unwrap_or_default();

            if response.drag_started() {
                st.start_w = screen.width();
                st.start_h = screen.height();
                if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                    st.start_x = p.x;
                    st.start_y = p.y;
                }
                st.active = true;
                ui.ctx()
                    .memory_mut(|m| m.data.insert_persisted(mem_id, st));
            }

            if st.active && response.dragged() {
                if let Some(cur) = ui.input(|i| i.pointer.interact_pos()) {
                    let dw = cur.x - st.start_x;
                    let dh = cur.y - st.start_y;
                    let new_w = (st.start_w + dw).max(320.0).round();
                    let new_h = (st.start_h + dh).max(240.0).round();
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::InnerSize(vec2(new_w, new_h)));
                }
            }

            if response.drag_stopped() {
                st.active = false;
                ui.ctx()
                    .memory_mut(|m| m.data.insert_persisted(mem_id, st));
            }
        });
}
