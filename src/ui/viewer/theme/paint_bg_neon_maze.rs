use egui::{Color32, Context, LayerId, Rect, Stroke, pos2};

pub fn paint_bg_neon_maze(ctx: &Context, seed: u64) {
    let painter = ctx.layer_painter(LayerId::background());
    let rect: Rect = ctx.screen_rect();

    let target = 22.0_f32.max(8.0);
    let cols = (rect.width() / target).floor().max(1.0) as i32;
    let rows = (rect.height() / target)
        .floor()
        .max(1.0) as i32;

    let size = (rect.width() / cols as f32).min(rect.height() / rows as f32);
    let offset = pos2(rect.left() + (rect.width() - cols as f32 * size) * 0.5, rect.top() + (rect.height() - rows as f32 * size) * 0.5);

    let line_width: f32 = 10.0;
    let stroke = Stroke { width: line_width, color: Color32::from_rgba_unmultiplied(0, 22, 25, 255) };
    let extend = line_width * 0.5;

    // диапазон -1..cols и -1..rows — чтобы нарисовать ещё по одной клетке за краями
    for y in -1..=rows {
        for x in -1..=cols {
            let x0 = offset.x + x as f32 * size;
            let y0 = offset.y + y as f32 * size;
            let x1 = x0 + size;
            let y1 = y0 + size;

            let h = mix64(seed ^ (((x as i64) as u64) << 32) ^ (y as i64 as u64));
            let slash = (h & 1) == 0;

            let (a, b) = if slash { (pos2(x0, y0), pos2(x1, y1)) } else { (pos2(x0, y1), pos2(x1, y0)) };

            let mut dir = b - a;
            let len = dir.length().max(1e-6);
            dir /= len;

            let a_ext = a - dir * extend;
            let b_ext = b + dir * extend;

            painter.line_segment([a_ext, b_ext], stroke);
        }
    }
}

// простая качественная мешалка (SplitMix64-подобная), без зависимостей
#[inline]
fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}
