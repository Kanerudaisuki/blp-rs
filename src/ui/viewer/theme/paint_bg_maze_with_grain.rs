use egui::{Color32, Context, Id, LayerId, Order, Painter, Pos2, Rect, Stroke};

pub fn paint_bg_maze_with_grain(ctx: &Context, seed: u64) {
    let rect = ctx.screen_rect();
    let p = ctx.layer_painter(LayerId::new(Order::Background, Id::new("bg.maze")));
    paint_maze(&p, rect, seed);
    paint_grain(&p, rect, seed ^ 0xA5A5_5A5A_D3C3_F00D);
}

// ───────────────────── Maze (длинные линии, минимум «каши») ─────────────────────
// Идея: поле «ориентаций» с коарсингом: блоки 4×4 тайлов делят одну ориентацию
// (горизонтальная или вертикальная), поэтому получаются длинные участки.
// На границах блоков — повороты «┌┐└┘» без точек по прямой.

fn paint_maze(p: &Painter, rect: Rect, seed: u64) {
    let tile  = 18.0;              // размер тайла
    let inset = 5.0;               // отступ дорожки от углов тайла
    let block = 4;                 // размер коарс-блока (в тайлах)
    let w     = Stroke::new(1.1, Color32::from_rgba_unmultiplied(0, 220, 255, 56));

    let gx0 = (rect.left()   / tile).floor() as i32 - 2;
    let gy0 = (rect.top()    / tile).floor() as i32 - 2;
    let gx1 = (rect.right()  / tile).ceil()  as i32 + 2;
    let gy1 = (rect.bottom() / tile).ceil()  as i32 + 2;

    for gy in gy0..gy1 {
        for gx in gx0..gx1 {
            // Коарс-ориентация блока: 0 — горизонталь, 1 — вертикаль
            let bx = div_floor(gx, block) as i32;
            let by = div_floor(gy, block) as i32;
            let orient = (hash2(seed, bx, by) & 1) as u8;

            // Немного вариативности внутри блока (редкие повороты)
            let r = hash2(seed ^ 0x9E37_79B9, gx, gy);
            let turn_here = (r & 0xF) == 0; // ≈ 1/16 тайлов

            let x0 = gx as f32 * tile;
            let y0 = gy as f32 * tile;
            let x1 = x0 + tile;
            let y1 = y0 + tile;

            // центры рёбер
            let top    = Pos2::new((x0 + x1) * 0.5, y0);
            let right  = Pos2::new(x1, (y0 + y1) * 0.5);
            let bottom = Pos2::new((x0 + x1) * 0.5, y1);
            let left   = Pos2::new(x0, (y0 + y1) * 0.5);

            // «углы» дорожки (чтоб поворот был округлым в пределах тайла)
            let tl = Pos2::new(x0 + inset, y0 + inset);
            let tr = Pos2::new(x1 - inset, y0 + inset);
            let bl = Pos2::new(x0 + inset, y1 - inset);
            let br = Pos2::new(x1 - inset, y1 - inset);

            if orient == 0 && !turn_here {
                // длинная горизонталь
                p.line_segment([left, right], w);
            } else if orient == 1 && !turn_here {
                // длинная вертикаль
                p.line_segment([top, bottom], w);
            } else {
                // поворот — выбираем один из четырёх ┌┐└┘
                match (r >> 4) & 3 {
                    0 => { p.line_segment([top, tl], w);    p.line_segment([tl, left], w); }   // ┌
                    1 => { p.line_segment([top, tr], w);    p.line_segment([tr, right], w); }  // ┐
                    2 => { p.line_segment([left, bl], w);   p.line_segment([bl, bottom], w); } // └
                    _ => { p.line_segment([right, br], w);  p.line_segment([br, bottom], w); } // ┘
                }
            }
        }
    }
}

// Целочисл. деление вниз (работает и для отрицательных индексов)
#[inline] fn div_floor(a: i32, b: i32) -> i32 {
    let (q, r) = (a / b, a % b);
    if (r != 0) && ((r > 0) != (b > 0)) { q - 1 } else { q }
}

// ───────────────────────────── Grain (зерно/шум) ─────────────────────────────
// Супердешёвое зерно: редкие точки разной прозрачности на сетке 8×8 px.
// Детерминированно от seed, не мешает UI.

fn paint_grain(p: &Painter, rect: Rect, seed: u64) {
    let cell = 8.0;
    let gx0 = (rect.left()   / cell).floor() as i32 - 1;
    let gy0 = (rect.top()    / cell).floor() as i32 - 1;
    let gx1 = (rect.right()  / cell).ceil()  as i32 + 1;
    let gy1 = (rect.bottom() / cell).ceil()  as i32 + 1;

    for gy in gy0..gy1 {
        for gx in gx0..gx1 {
            let r = hash2(seed, gx, gy);
            // около 35% ячеек получат 1–2 точки
            if (r & 0xFF) < 90 {
                let n = 1 + ((r >> 8) & 1) as i32;
                for k in 0..n {
                    let rx = (((r >> (10 + k*7)) & 0x7F) as f32) / 127.0;
                    let ry = (((r >> (17 + k*7)) & 0x7F) as f32) / 127.0;
                    let x = gx as f32 * cell + rx * cell;
                    let y = gy as f32 * cell + ry * cell;
                    let alpha = 18 + ((r >> (24 + k*3)) & 0x0F) as u8; // 18..33
                    p.circle_filled(Pos2::new(x, y), 0.7, Color32::from_rgba_unmultiplied(0, 220, 255, alpha));
                }
            }
        }
    }
}

// ───────────────────────────── small hash utils ─────────────────────────────

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[inline]
fn hash2(seed: u64, x: i32, y: i32) -> u64 {
    let xu = x as i64 as u64;
    let yu = y as i64 as u64;
    splitmix64(seed ^ xu.wrapping_mul(0x9E37_79B9) ^ yu.wrapping_mul(0xC2B2_AE3D))
}
