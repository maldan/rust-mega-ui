use glam::Vec2;

use crate::types::Rect;

pub(crate) const NODE_MIN_W: f32 = 168.0;
pub(crate) const NODE_PAD: f32 = 10.0;
pub(crate) const PIN_R: f32 = 5.5;
pub(crate) const PIN_HIT: f32 = 10.0;
pub(crate) const FRAME_PAD: f32 = 20.0;
pub(crate) const FRAME_LABEL: f32 = 20.0;
pub(crate) const ZOOM_MIN: f32 = 0.1;
pub(crate) const ZOOM_MAX: f32 = 8.0;

pub(crate) fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

/// Widget scale inside a node. Must track zoom (no 0.45 floor): chrome uses `z`
/// for the box, so a higher floor made ports/layout sit in a different space.
pub(crate) fn node_content_scale(ui_scale: f32, zoom: f32) -> f32 {
    (ui_scale * zoom).clamp(0.05, 16.0)
}

pub(crate) fn link_handle(from: Vec2, to: Vec2, zoom: f32) -> f32 {
    ((to.x - from.x).abs() * 0.5).max(48.0 * zoom.clamp(ZOOM_MIN, 1.0))
}

pub(crate) fn link_controls(from: Vec2, to: Vec2, zoom: f32) -> (Vec2, Vec2) {
    let dx = link_handle(from, to, zoom);
    (from + Vec2::new(dx, 0.0), to - Vec2::new(dx, 0.0))
}

/// Screen AABB of the cubic (endpoints + horizontal handles), inflated by `pad`.
pub(crate) fn link_aabb(from: Vec2, to: Vec2, zoom: f32, pad: f32) -> Rect {
    let (c1, c2) = link_controls(from, to, zoom);
    let min = from.min(to).min(c1).min(c2) - Vec2::splat(pad);
    let max = from.max(to).max(c1).max(c2) + Vec2::splat(pad);
    Rect { min, max }
}

pub(crate) fn link_seg_count(from: Vec2, to: Vec2, zoom: f32) -> usize {
    let dx = link_handle(from, to, zoom);
    let approx = from.distance(to) + dx * 0.35;
    ((approx / 14.0).ceil() as usize).clamp(4, 18)
}

pub(crate) fn for_link_segments(from: Vec2, to: Vec2, zoom: f32, mut emit: impl FnMut(Vec2, Vec2)) {
    let (c1, c2) = link_controls(from, to, zoom);
    let n = link_seg_count(from, to, zoom);
    let mut prev = from;
    for i in 1..=n {
        let p = cubic_bezier(from, c1, c2, to, i as f32 / n as f32);
        emit(prev, p);
        prev = p;
    }
}

pub(crate) fn dist_point_link(p: Vec2, from: Vec2, to: Vec2, zoom: f32) -> f32 {
    let mut best = f32::MAX;
    for_link_segments(from, to, zoom, |a, b| {
        best = best.min(dist_point_segment(p, a, b));
    });
    best
}

pub(crate) fn dist_point_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-8 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

pub(crate) fn dist_point_polyline(p: Vec2, pts: &[Vec2]) -> f32 {
    let mut best = f32::MAX;
    for w in pts.windows(2) {
        best = best.min(dist_point_segment(p, w[0], w[1]));
    }
    best
}

pub(crate) fn rect_from_points(a: Vec2, b: Vec2) -> Rect {
    Rect {
        min: Vec2::new(a.x.min(b.x), a.y.min(b.y)),
        max: Vec2::new(a.x.max(b.x), a.y.max(b.y)),
    }
}

pub(crate) fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
}

pub(crate) fn snap_vec(p: Vec2, snap: f32) -> Vec2 {
    if snap <= 1e-6 {
        p
    } else {
        Vec2::new((p.x / snap).round() * snap, (p.y / snap).round() * snap)
    }
}
