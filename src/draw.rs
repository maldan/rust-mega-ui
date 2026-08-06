use glam::Vec2;

use super::font::push_solid;
use super::types::{DrawCommand, Rect};

/// `DrawCommand.kind` for GPU SDF rounded fills (one quad).
pub const KIND_SDF_ROUND: f32 = 2.0;

/// Corner mask packed into `params.w`: all / top only / bottom only.
const CORNER_ALL: f32 = 0.0;
const CORNER_TOP: f32 = 1.0;
const CORNER_BOT: f32 = 2.0;

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
    if r < 0.5 || (!round_top && !round_bot) {
        push_solid(out, rect, color, white_uv, clip);
        return;
    }

    let corner = if round_top && round_bot {
        CORNER_ALL
    } else if round_top {
        CORNER_TOP
    } else {
        CORNER_BOT
    };

    // Pad so SDF AA fringe is not clipped by the quad edges.
    let pad = 1.0;
    let outer = Rect {
        min: rect.min - Vec2::splat(pad),
        max: rect.max + Vec2::splat(pad),
    };
    let (draw, uv_min, uv_max) = match clip {
        Some(c) => {
            let Some(clipped) = outer.intersect(c) else {
                return;
            };
            let ow = outer.width().max(1e-3);
            let oh = outer.height().max(1e-3);
            let u0 = (clipped.min.x - outer.min.x) / ow;
            let u1 = (clipped.max.x - outer.min.x) / ow;
            let v0 = (clipped.min.y - outer.min.y) / oh;
            let v1 = (clipped.max.y - outer.min.y) / oh;
            // Map outer UV → content UV in [ -pad/w .. 1+pad/w ].
            let content_u0 = -pad / w + u0 * (w + pad * 2.0) / w;
            let content_u1 = -pad / w + u1 * (w + pad * 2.0) / w;
            let content_v0 = -pad / h + v0 * (h + pad * 2.0) / h;
            let content_v1 = -pad / h + v1 * (h + pad * 2.0) / h;
            (
                clipped,
                [content_u0, content_v0],
                [content_u1, content_v1],
            )
        }
        None => (
            outer,
            [-pad / w, -pad / h],
            [1.0 + pad / w, 1.0 + pad / h],
        ),
    };

    out.push(DrawCommand {
        rect: draw,
        uv_min,
        uv_max,
        colors: [color; 4],
        kind: KIND_SDF_ROUND,
        tex: 0,
        params: [w, h, r, corner],
    });
}

/// Ring sector: clockwise sweep from `start_rad` (math angle, 0=+x, CCW, y-up).
pub fn push_arc_cw(
    out: &mut Vec<DrawCommand>,
    center: Vec2,
    r_in: f32,
    r_out: f32,
    start_rad: f32,
    sweep_rad: f32,
    t0: f32,
    t1: f32,
    color: [f32; 4],
    white_uv: [f32; 2],
    clip: Option<Rect>,
) {
    let t0 = t0.clamp(0.0, 1.0);
    let t1 = t1.clamp(0.0, 1.0);
    if t1 <= t0 || r_out <= r_in || sweep_rad <= 0.0 {
        return;
    }
    let r_mid = (r_in + r_out) * 0.5;
    let thickness = r_out - r_in;
    let arc_len = sweep_rad * (t1 - t0) * r_mid;
    let n = ((arc_len / (thickness * 0.45)).ceil() as i32).clamp(8, 128);
    let stamp = thickness * 1.05;
    let half = stamp * 0.5;

    for i in 0..n {
        let u = t0 + (t1 - t0) * (i as f32 + 0.5) / n as f32;
        let a = start_rad - u * sweep_rad;
        let d = Vec2::new(a.cos(), -a.sin());
        let p = center + d * r_mid;
        push_round_rect(
            out,
            Rect::from_min_size(p - Vec2::splat(half), Vec2::splat(stamp)),
            half,
            color,
            true,
            true,
            white_uv,
            clip,
        );
    }
}

/// Axis-aligned stamps along a line (indicator needle).
pub fn push_line(
    out: &mut Vec<DrawCommand>,
    a: Vec2,
    b: Vec2,
    thickness: f32,
    color: [f32; 4],
    white_uv: [f32; 2],
    clip: Option<Rect>,
) {
    let delta = b - a;
    let len = delta.length();
    if len < 0.5 || thickness <= 0.0 {
        return;
    }
    let n = (len / (thickness * 0.5)).ceil() as i32;
    let n = n.clamp(2, 64);
    let stamp = thickness;
    let half = stamp * 0.5;
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let p = a + delta * t;
        push_round_rect(
            out,
            Rect::from_min_size(p - Vec2::splat(half), Vec2::splat(stamp)),
            half,
            color,
            true,
            true,
            white_uv,
            clip,
        );
    }
}
