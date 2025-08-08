use egui::{Color32, Context, Id, LayerId, Order, Painter, Pos2, Rect, Stroke, StrokeKind};

// ───────────────────────── entry ─────────────────────────

pub fn paint_bg_neon_maze(ctx: &Context, seed: u64) {
    let rect = ctx.screen_rect();
    let p = ctx.layer_painter(LayerId::new(Order::Background, Id::new("bg.neon_maze")));
    draw_maze(&p, rect, seed);
    draw_grain(&p, rect, seed ^ 0xA5A5_5A5A_D3C3_F00D);
    draw_vignette(&p, rect);
}

// ──────────────────────── maze with glow ────────────────────────
// Коарсинг блоками даёт длинные участки; редкие повороты добавляют вариацию.

fn draw_maze(p: &Painter, rect: Rect, seed: u64) {
    // параметры стиля
    let tile = 18.0; // размер тайла
    let inset = 5.0; // отступ дорожки внутри тайла
    let block = 4; // размер коарс-блока (в тайлах)
    let core = Color32::from_rgba_unmultiplied(0, 220, 255, 56); // основной штрих

    let gx0 = (rect.left() / tile).floor() as i32 - 2;
    let gy0 = (rect.top() / tile).floor() as i32 - 2;
    let gx1 = (rect.right() / tile).ceil() as i32 + 2;
    let gy1 = (rect.bottom() / tile).ceil() as i32 + 2;

    for gy in gy0..gy1 {
        for gx in gx0..gx1 {
            // ориентация всего блока (гор/верт) — стабильна по seed
            let bx = div_floor(gx, block);
            let by = div_floor(gy, block);
            let orient_h = (hash2(seed, bx, by) & 1) == 0;

            // небольшая вариативность внутри блока
            let r = hash2(seed ^ 0x9E37_79B9, gx, gy);
            let turn_here = (r & 0xF) == 0; // ≈ 1/16
            let x0 = gx as f32 * tile;
            let y0 = gy as f32 * tile;
            let x1 = x0 + tile;
            let y1 = y0 + tile;

            // центры рёбер
            let top = Pos2::new((x0 + x1) * 0.5, y0);
            let right = Pos2::new(x1, (y0 + y1) * 0.5);
            let bottom = Pos2::new((x0 + x1) * 0.5, y1);
            let left = Pos2::new(x0, (y0 + y1) * 0.5);

            // «углы» дорожки
            let tl = Pos2::new(x0 + inset, y0 + inset);
            let tr = Pos2::new(x1 - inset, y0 + inset);
            let bl = Pos2::new(x0 + inset, y1 - inset);
            let br = Pos2::new(x1 - inset, y1 - inset);

            if orient_h && !turn_here {
                neon_line(p, left, right, core);
            } else if !orient_h && !turn_here {
                neon_line(p, top, bottom, core);
            } else {
                match (r >> 4) & 3 {
                    0 => {
                        neon_line(p, top, tl, core);
                        neon_line(p, tl, left, core);
                    } // ┌
                    1 => {
                        neon_line(p, top, tr, core);
                        neon_line(p, tr, right, core);
                    } // ┐
                    2 => {
                        neon_line(p, left, bl, core);
                        neon_line(p, bl, bottom, core);
                    } // └
                    _ => {
                        neon_line(p, right, br, core);
                        neon_line(p, br, bottom, core);
                    } // ┘
                }
            }

            // очень редкие «контактные площадки» (узлы) — для живости
            if (r & 0x3FF) == 0 {
                let c = Pos2::new((x0 + x1) * 0.5, (y0 + y1) * 0.5);
                let glow = Color32::from_rgba_unmultiplied(core.r(), core.g(), core.b(), 36);
                p.circle_filled(c, 1.8, glow);
            }
        }
    }
}

// неоновая линия: мягкий ореол + основной штрих
#[inline]
fn neon_line(p: &Painter, a: Pos2, b: Pos2, core: Color32) {
    let glow = Color32::from_rgba_unmultiplied(core.r(), core.g(), core.b(), 44);
    p.line_segment([a, b], Stroke::new(3.0, glow)); // ореол
    p.line_segment([a, b], Stroke::new(1.1, core)); // ядро
}

// ─────────────────────────── grain (зерно) ───────────────────────────

fn draw_grain(p: &Painter, rect: Rect, seed: u64) {
    let cell = 8.0;
    let gx0 = (rect.left() / cell).floor() as i32 - 1;
    let gy0 = (rect.top() / cell).floor() as i32 - 1;
    let gx1 = (rect.right() / cell).ceil() as i32 + 1;
    let gy1 = (rect.bottom() / cell).ceil() as i32 + 1;

    for gy in gy0..gy1 {
        for gx in gx0..gx1 {
            let r = hash2(seed, gx, gy);
            if (r & 0xFF) < 80 {
                // ~31% ячеек
                let n = 1 + ((r >> 8) & 1) as i32;
                for k in 0..n {
                    let rx = (((r >> (10 + k * 7)) & 0x7F) as f32) / 127.0;
                    let ry = (((r >> (17 + k * 7)) & 0x7F) as f32) / 127.0;
                    let x = gx as f32 * cell + rx * cell;
                    let y = gy as f32 * cell + ry * cell;
                    let alpha = 14 + ((r >> (24 + k * 3)) & 0x0F) as u8; // 14..29
                    p.circle_filled(Pos2::new(x, y), 0.6, Color32::from_rgba_unmultiplied(0, 220, 255, alpha));
                }
            }
        }
    }
}

// ─────────────────────────── vignette ───────────────────────────

fn draw_vignette(p: &Painter, rect: Rect) {
    // тонкая «глубина» по краям, без драматизма
    for (k, alpha) in [18u8, 12, 8].into_iter().enumerate() {
        let pad = 4.0 + (k as f32) * 6.0;
        let col = Color32::from_rgba_unmultiplied(0, 0, 0, alpha);
        p.rect_stroke(rect.shrink(pad), 0.0, Stroke::new(4.0, col), StrokeKind::Outside);
    }
}

// ────────────────────────── utils ──────────────────────────

#[inline]
fn div_floor(a: i32, b: i32) -> i32 {
    let (q, r) = (a / b, a % b);
    if (r != 0) && ((r > 0) != (b > 0)) { q - 1 } else { q }
}

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
