use crate::ui::viewer::app::App;
use egui::{self, Align, Color32, Context, CursorIcon, Frame, Margin, Pos2, Response, Sense, Shape, Stroke, TopBottomPanel, Ui, Vec2, ViewportCommand};

impl App {
    pub(crate) fn draw_title_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("custom_title_bar")
            .exact_height(28.0)
            .frame(Frame {
                fill: Color32::from_rgba_unmultiplied(10, 180, 250, 60), //
                inner_margin: Margin::symmetric(6, 2),
                outer_margin: Margin::ZERO,
                stroke: Stroke::NONE,
                ..Default::default()
            })
            .show(ctx, |ui| {
                let title_bar_rect = ui.max_rect();

                // справа налево: red → green → yellow
                let (close_resp, zoom_resp, min_resp) = ui
                    .with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        let close_resp = macos_dot(ui, TrafficKind::Close).on_hover_cursor(CursorIcon::PointingHand);
                        ui.add_space(6.0);
                        let zoom_resp = macos_dot_zoom(ui, self.maximized) // зелёная с треугольниками
                            .on_hover_cursor(CursorIcon::PointingHand);
                        ui.add_space(6.0);
                        let min_resp = macos_dot(ui, TrafficKind::Minimize).on_hover_cursor(CursorIcon::PointingHand);
                        (close_resp, zoom_resp, min_resp)
                    })
                    .inner;

                // действия
                if min_resp.clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                }
                if zoom_resp.clicked() {
                    self.maximized = !self.maximized;
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(self.maximized));
                }
                if close_resp.clicked() {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }

                // курсор "Move" над drag-зоной (всё, кроме кружков)
                if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
                    let over_btns = min_resp.rect.contains(p) || zoom_resp.rect.contains(p) || close_resp.rect.contains(p);
                    if title_bar_rect.contains(p) && !over_btns {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::Grab);
                    }
                }

                // drag-area — всё кроме кружков
                let pointer = ui.input(|i| i.pointer.clone());
                if pointer.primary_down() {
                    if let Some(pos) = pointer.interact_pos() {
                        let over = min_resp.rect.contains(pos) || zoom_resp.rect.contains(pos) || close_resp.rect.contains(pos);
                        if title_bar_rect.contains(pos) && !over {
                            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                        }
                    }
                }
            });
    }
}

#[derive(Copy, Clone)]
enum TrafficKind {
    Close,
    Minimize,
}

const MACOS_DOT_SIZE: f32 = 18.0;

fn macos_dot(ui: &mut Ui, kind: TrafficKind) -> Response {
    let size = Vec2::splat(MACOS_DOT_SIZE);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let center = rect.center();

    let (base, hover_stroke) = match kind {
        TrafficKind::Close => (Color32::from_rgb(255, 95, 86), Color32::from_rgba_unmultiplied(0, 0, 0, 100)),     // red
        TrafficKind::Minimize => (Color32::from_rgb(255, 189, 46), Color32::from_rgba_unmultiplied(0, 0, 0, 100)), // yellow
    };

    ui.painter()
        .circle_filled(center, MACOS_DOT_SIZE * 0.5, base);

    if resp.hovered() {
        ui.painter()
            .circle_stroke(center, MACOS_DOT_SIZE * 0.5, Stroke { width: 1.0, color: hover_stroke });

        match kind {
            TrafficKind::Close => {
                let r = MACOS_DOT_SIZE * 0.28;
                ui.painter()
                    .line_segment([Pos2::new(center.x - r, center.y - r), Pos2::new(center.x + r, center.y + r)], Stroke { width: 1.5, color: Color32::BLACK });
                ui.painter()
                    .line_segment([Pos2::new(center.x - r, center.y + r), Pos2::new(center.x + r, center.y - r)], Stroke { width: 1.5, color: Color32::BLACK });
            }
            TrafficKind::Minimize => {
                let r = MACOS_DOT_SIZE * 0.30;
                ui.painter()
                    .line_segment([Pos2::new(center.x - r, center.y), Pos2::new(center.x + r, center.y)], Stroke { width: 2.0, color: Color32::BLACK });
            }
        }
    }

    resp
}

fn macos_dot_zoom(ui: &mut Ui, inward: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(MACOS_DOT_SIZE), Sense::click());
    let c = rect.center();
    let r = MACOS_DOT_SIZE * 0.5;

    // круг
    ui.painter()
        .circle_filled(c, r, Color32::from_rgb(39, 201, 63));

    // показываем знак на hover (убери if, если нужно всегда видно)
    if resp.hovered() {
        ui.painter()
            .circle_stroke(c, r, Stroke { width: 1.0, color: Color32::from_rgba_unmultiplied(0, 0, 0, 100) });

        // диагональ ↘ (единичный вектор) и её перпендикуляр
        let u = Vec2::new(1.0, 1.0).normalized();
        let n = Vec2::new(-u.y, u.x);

        // параметры глифа
        let tip_off = if inward { 1. } else { r * 0.8 }; // смещение носика от центра вдоль диагонали
        let height = r * 0.6; // "длина" треугольника
        let base_w = r * 0.9; // ширина основания

        // рисовалка "по носику"
        let tri_tip = |tip: Pos2, dir: Vec2| {
            let d = dir.normalized();
            let base_c = tip - d * height;
            let a = base_c + n * (base_w * 0.5);
            let b = base_c - n * (base_w * 0.5);
            ui.painter()
                .add(Shape::convex_polygon(vec![tip, a, b], Color32::BLACK, Stroke::NONE));
        };

        // положения носиков на диагонали
        let tip_tl = c - u * tip_off; // верх-лево
        let tip_br = c + u * tip_off; // низ-право

        if inward {
            // внутрь: ↘ у верх-левого, ↖ у нижне-правого
            tri_tip(tip_tl, u); // к центру
            tri_tip(tip_br, -u); // к центру
        } else {
            // наружу: ↖ у верх-левого, ↘ у нижне-правого
            tri_tip(tip_tl, -u); // наружу
            tri_tip(tip_br, u); // наружу
        }
    }

    resp
}
