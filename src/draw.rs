use glam::Vec2;

use super::font::push_solid;
use super::types::{DrawCommand, Rect};

pub fn push_round_rect(
    out: &mut Vec<DrawCommand>,
    rect: Rect,
    radius: f32,
    color: [f32; 4],
    round_top: bool,
    round_bot: bool,
    white_uv: [f32; 2],
    clip: Option<Rect>,
) {
    let w = rect.width();
    let h = rect.height();
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let r = radius.min(w * 0.5).min(h * 0.5).max(0.0);
    if r < 0.5 {
        // Sharp rects: one quad (per-row cost only for rounded).
        push_solid(out, rect, color, white_uv, clip);
        return;
    }

    // Scanline coverage AA: integer pixel quads, alpha = horizontal × vertical coverage.
    let mut y = rect.min.y.floor();
    let y_end = rect.max.y.ceil();
    while y < y_end {
        let y0 = y.max(rect.min.y);
        let y1 = (y + 1.0).min(rect.max.y);
        let row_a = y1 - y0;
        if row_a > 0.0 {
            let yl = y + 0.5 - rect.min.y;
            let inset = round_row_inset(yl, h, r, round_top, round_bot);
            push_span_aa(
                out,
                y,
                rect.min.x + inset,
                rect.max.x - inset,
                color,
                row_a,
                white_uv,
                clip,
            );
        }
        y += 1.0;
    }
}

fn round_row_inset(y_local: f32, h: f32, r: f32, round_top: bool, round_bot: bool) -> f32 {
    if round_top && y_local < r {
        let dy = r - y_local;
        r - (r * r - dy * dy).max(0.0).sqrt()
    } else if round_bot && y_local > h - r {
        let dy = r - (h - y_local);
        r - (r * r - dy * dy).max(0.0).sqrt()
    } else {
        0.0
    }
}

/// One pixel row `y .. y+1`: solid middle + fractional edge pixels via alpha.
fn push_span_aa(
    out: &mut Vec<DrawCommand>,
    y: f32,
    x0: f32,
    x1: f32,
    color: [f32; 4],
    row_a: f32,
    white_uv: [f32; 2],
    clip: Option<Rect>,
) {
    if x1 <= x0 || row_a <= 0.0 {
        return;
    }

    let y0 = y;
    let y1 = y + 1.0;
    let left = x0.ceil();
    let right = x1.floor();

    if left <= right {
        if x0 < left {
            let a = (left - x0).clamp(0.0, 1.0) * row_a;
            push_solid_alpha(
                out,
                Rect {
                    min: Vec2::new(left - 1.0, y0),
                    max: Vec2::new(left, y1),
                },
                color,
                a,
                white_uv,
                clip,
            );
        }
        if right > left {
            let mut c = color;
            c[3] *= row_a.clamp(0.0, 1.0);
            if c[3] > 0.001 {
                push_solid(
                    out,
                    Rect {
                        min: Vec2::new(left, y0),
                        max: Vec2::new(right, y1),
                    },
                    c,
                    white_uv,
                    clip,
                );
            }
        }
        if x1 > right {
            let a = (x1 - right).clamp(0.0, 1.0) * row_a;
            push_solid_alpha(
                out,
                Rect {
                    min: Vec2::new(right, y0),
                    max: Vec2::new(right + 1.0, y1),
                },
                color,
                a,
                white_uv,
                clip,
            );
        }
    } else {
        // Entire span inside one pixel column.
        let a = (x1 - x0).clamp(0.0, 1.0) * row_a;
        let px = x0.floor();
        push_solid_alpha(
            out,
            Rect {
                min: Vec2::new(px, y0),
                max: Vec2::new(px + 1.0, y1),
            },
            color,
            a,
            white_uv,
            clip,
        );
    }
}

fn push_solid_alpha(
    out: &mut Vec<DrawCommand>,
    rect: Rect,
    color: [f32; 4],
    alpha: f32,
    white_uv: [f32; 2],
    clip: Option<Rect>,
) {
    if alpha <= 0.001 {
        return;
    }
    let mut c = color;
    c[3] *= alpha.clamp(0.0, 1.0);
    push_solid(out, rect, c, white_uv, clip);
}
