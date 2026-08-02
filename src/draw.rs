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
        push_solid(out, rect, color, white_uv, clip);
        return;
    }

    let mut y = rect.min.y.floor();
    let y_end = rect.max.y.ceil();
    while y < y_end {
        let y0 = y.max(rect.min.y);
        let y1 = (y + 1.0).min(rect.max.y);
        if y1 > y0 {
            let yl = (y0 + y1) * 0.5 - rect.min.y;
            let inset = round_row_inset(yl, h, r, round_top, round_bot);
            push_solid(
                out,
                Rect {
                    min: Vec2::new(rect.min.x + inset, y0),
                    max: Vec2::new(rect.max.x - inset, y1),
                },
                color,
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
