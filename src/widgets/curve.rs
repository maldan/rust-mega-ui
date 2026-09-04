//! Animation curve editor (Hermite segments, Unity-style).

use glam::Vec2;

use crate::plot_view::PlotView;
use crate::theme;
use crate::types::{CursorIcon, Rect};
use crate::{LayoutDir, Ui};

const POINT_R: f32 = 6.0;
const HIT_R: f32 = 16.0;
const DBL_CLICK_SEC: f32 = 0.35;
const DBL_CLICK_PX: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvePreset {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Custom,
}

#[derive(Clone, Debug)]
pub struct CurvePoint {
    pub t: f32,
    pub v: f32,
    /// Outgoing derivative dv/dt.
    pub tangent_out: f32,
}

#[derive(Clone, Debug)]
pub struct AnimationCurve {
    pub points: Vec<CurvePoint>,
    pub preset: CurvePreset,
}

impl Default for AnimationCurve {
    fn default() -> Self {
        ease_in_out()
    }
}

pub fn flat_pass_curve() -> AnimationCurve {
    AnimationCurve {
        points: vec![
            CurvePoint {
                t: 0.0,
                v: 1.0,
                tangent_out: 0.0,
            },
            CurvePoint {
                t: 1.0,
                v: 1.0,
                tangent_out: 0.0,
            },
        ],
        preset: CurvePreset::Custom,
    }
}

pub fn ease_in_out() -> AnimationCurve {
    let mut c = AnimationCurve {
        points: vec![
            CurvePoint {
                t: 0.0,
                v: 0.0,
                tangent_out: 0.0,
            },
            CurvePoint {
                t: 1.0,
                v: 1.0,
                tangent_out: 0.0,
            },
        ],
        preset: CurvePreset::EaseInOut,
    };
    apply_preset(&mut c, CurvePreset::EaseInOut);
    c
}

pub fn apply_preset(curve: &mut AnimationCurve, preset: CurvePreset) {
    curve.preset = preset;
    match preset {
        CurvePreset::Linear => {
            curve.points = vec![
                CurvePoint {
                    t: 0.0,
                    v: 0.0,
                    tangent_out: 1.0,
                },
                CurvePoint {
                    t: 1.0,
                    v: 1.0,
                    tangent_out: 1.0,
                },
            ];
        }
        CurvePreset::EaseIn => {
            curve.points = vec![
                CurvePoint {
                    t: 0.0,
                    v: 0.0,
                    tangent_out: 0.0,
                },
                CurvePoint {
                    t: 1.0,
                    v: 1.0,
                    tangent_out: 2.0,
                },
            ];
        }
        CurvePreset::EaseOut => {
            curve.points = vec![
                CurvePoint {
                    t: 0.0,
                    v: 0.0,
                    tangent_out: 2.0,
                },
                CurvePoint {
                    t: 1.0,
                    v: 1.0,
                    tangent_out: 0.0,
                },
            ];
        }
        CurvePreset::EaseInOut | CurvePreset::Custom => {
            if curve.points.len() < 2 {
                curve.points = vec![
                    CurvePoint {
                        t: 0.0,
                        v: 0.0,
                        tangent_out: 0.0,
                    },
                    CurvePoint {
                        t: 1.0,
                        v: 1.0,
                        tangent_out: 0.0,
                    },
                ];
            }
            if preset == CurvePreset::EaseInOut {
                auto_smooth_tangents(curve);
            }
        }
    }
}

fn auto_smooth_tangents(curve: &AnimationCurve) -> Vec<f32> {
    let n = curve.points.len();
    let mut tangents = vec![0.0; n];
    if n < 2 {
        return tangents;
    }
    for i in 0..n {
        if i == 0 {
            let dt = curve.points[1].t - curve.points[0].t;
            tangents[i] = if dt > 1e-5 {
                (curve.points[1].v - curve.points[0].v) / dt
            } else {
                0.0
            };
        } else if i == n - 1 {
            let dt = curve.points[i].t - curve.points[i - 1].t;
            tangents[i] = if dt > 1e-5 {
                (curve.points[i].v - curve.points[i - 1].v) / dt
            } else {
                0.0
            };
        } else {
            let dt = curve.points[i + 1].t - curve.points[i - 1].t;
            tangents[i] = if dt > 1e-5 {
                (curve.points[i + 1].v - curve.points[i - 1].v) / dt
            } else {
                0.0
            };
        }
    }
    tangents
}

pub fn sample_curve(curve: &AnimationCurve, t: f32) -> f32 {
    let pts = &curve.points;
    if pts.is_empty() {
        return 0.0;
    }
    if pts.len() == 1 {
        return pts[0].v;
    }
    if t <= pts[0].t {
        return pts[0].v;
    }
    if t >= pts[pts.len() - 1].t {
        return pts[pts.len() - 1].v;
    }
    let smooth = auto_smooth_tangents(curve);
    for i in 0..pts.len() - 1 {
        let p0 = &pts[i];
        let p1 = &pts[i + 1];
        if t >= p0.t && t <= p1.t {
            let dt = (p1.t - p0.t).max(1e-5);
            let u = (t - p0.t) / dt;
            let m0 = sample_tangent(curve, &smooth, i) * dt;
            let m1 = sample_tangent(curve, &smooth, i + 1) * dt;
            return hermite(p0.v, p1.v, m0, m1, u);
        }
    }
    pts[pts.len() - 1].v
}

fn sample_tangent(curve: &AnimationCurve, smooth: &[f32], i: usize) -> f32 {
    if curve.preset == CurvePreset::Custom {
        smooth.get(i).copied().unwrap_or(0.0)
    } else {
        curve.points.get(i).map(|p| p.tangent_out).unwrap_or(0.0)
    }
}

fn hermite(p0: f32, p1: f32, m0: f32, m1: f32, u: f32) -> f32 {
    let u2 = u * u;
    let u3 = u2 * u;
    let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
    let h10 = u3 - 2.0 * u2 + u;
    let h01 = -2.0 * u3 + 3.0 * u2;
    let h11 = u3 - u2;
    h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1
}

fn sort_points(curve: &mut AnimationCurve) {
    curve.points.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CurveEditorResponse {
    pub changed: bool,
    pub selected: Option<usize>,
    pub sampled: Option<f32>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum CurveDrag {
    None,
    Point(usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CurveEditState {
    pub selected: Option<usize>,
    pub drag: CurveDrag,
    pub preview_t: f32,
    pub last_press_pos: Vec2,
    pub since_press: f32,
}

impl Default for CurveEditState {
    fn default() -> Self {
        Self {
            selected: None,
            drag: CurveDrag::None,
            preview_t: 0.0,
            last_press_pos: Vec2::ZERO,
            since_press: 10.0,
        }
    }
}

impl Ui {
    /// Edit an animation curve in normalized time 0..1.
    pub fn curve_editor(
        &mut self,
        id: &str,
        curve: &mut AnimationCurve,
        size: Vec2,
    ) -> CurveEditorResponse {
        self.curve_editor_opts(id, curve, size, true, true, &[])
    }

    /// Frequency-response editor: X = log Hz (0..1), Y = pass (1) / cut (0).
    pub fn eq_curve_editor(
        &mut self,
        id: &str,
        curve: &mut AnimationCurve,
        size: Vec2,
        ticks: &[(f32, &str)],
    ) -> CurveEditorResponse {
        self.curve_editor_opts(id, curve, size, false, false, ticks)
    }

    fn curve_editor_opts(
        &mut self,
        id: &str,
        curve: &mut AnimationCurve,
        size: Vec2,
        presets: bool,
        hint: bool,
        ticks: &[(f32, &str)],
    ) -> CurveEditorResponse {
        let enabled = self.enabled();
        let widget_id = self.current_id(id);
        let mut out = CurveEditorResponse::default();

        let fill_w = self.layer().fill_w;
        let w = if size.x <= 0.0 && fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical)
        {
            fill_w
        } else {
            self.s(size.x.max(120.0))
        };
        let h = self.s(size.y.max(80.0));

        if curve.points.len() < 2 {
            if presets {
                apply_preset(curve, CurvePreset::EaseInOut);
            } else {
                *curve = flat_pass_curve();
            }
        }

        let mut st = self
            .curve_edits
            .get(&widget_id)
            .copied()
            .unwrap_or_default();

        if presets {
            self.row(|ui| {
                let presets = [
                    ("Linear", CurvePreset::Linear),
                    ("In", CurvePreset::EaseIn),
                    ("Out", CurvePreset::EaseOut),
                    ("InOut", CurvePreset::EaseInOut),
                ];
                for (label, p) in presets {
                    if ui.button(label).clicked() {
                        apply_preset(curve, p);
                        out.changed = true;
                        curve.preset = p;
                    }
                }
            });
        }

        let outer = self.allocate(Vec2::new(w, h));
        let radius = self.s(theme::BTN_RADIUS);
        let inner = outer.inset(self.s(4.0));
        let axis_h = if ticks.is_empty() { 0.0 } else { self.s(13.0) };
        let plot_rect = Rect {
            min: inner.min,
            max: Vec2::new(inner.max.x, (inner.max.y - axis_h).max(inner.min.y + 8.0)),
        };
        let view = PlotView::default();
        self.round_rect(inner, radius, theme::PLOT_BG);

        self.push_clip(plot_rect);
        if ticks.is_empty() {
            draw_grid(self, plot_rect, &view);
        } else {
            crate::widgets::plot::draw_tick_grid_x(self, plot_rect, ticks);
        }

        let samples = 64;
        let mut line_pts = Vec::with_capacity(samples + 1);
        for i in 0..=samples {
            let u = i as f32 / samples as f32;
            let t = view.t_min + u * (view.t_max - view.t_min);
            let v = sample_curve(curve, t);
            line_pts.push(view.plot_to_screen(plot_rect, t, v));
        }
        self.draw_polyline(&line_pts, self.s(2.0), theme::PLOT_LINE);

        let n = curve.points.len();
        let mp = self.input.mouse_pos;
        let hit_r = HIT_R * self.scale;
        let hover_pt = nearest_point(curve, &view, plot_rect, mp, hit_r);

        for i in 0..n {
            let center = point_screen(curve, &view, plot_rect, i);
            let sel = st.selected == Some(i);
            let hot = hover_pt == Some(i);
            let r = POINT_R * self.scale * if sel { 1.25 } else { 1.0 };
            let p_rect = Rect::from_min_size(center - Vec2::splat(r), Vec2::splat(r * 2.0));
            self.round_rect(
                p_rect,
                r,
                if sel {
                    theme::ACCENT
                } else if hot {
                    theme::SLIDER_THUMB_HOT
                } else {
                    theme::SLIDER_THUMB
                },
            );
        }

        let preview_v = sample_curve(curve, st.preview_t);
        let px = view.plot_to_screen(plot_rect, st.preview_t, preview_v);
        if hint {
            let vline_a = Vec2::new(px.x, plot_rect.min.y);
            let vline_b = Vec2::new(px.x, plot_rect.max.y);
            self.draw_line_segment(vline_a, vline_b, self.s(1.0), theme::ACCENT_DIM);
        }
        self.pop_clip();
        if !ticks.is_empty() {
            crate::widgets::plot::draw_tick_labels_x(self, inner, plot_rect, ticks);
        }

        if hint {
            self.label_styled(
                "Drag keys · double-click / Ctrl+click add · Del or RMB delete",
                crate::widgets::label::TextStyle {
                    color: theme::TEXT_DIM,
                    size: 11.0,
                },
            );
        }

        out.sampled = Some(sample_curve(curve, st.preview_t));
        out.selected = st.selected;

        if !enabled {
            return out;
        }

        let hovered = self.hovered_rect(plot_rect);
        if hovered {
            self.want_capture = true;
            if st.selected.is_some() {
                self.focus_id = Some(widget_id);
            }
            self.set_cursor(if hover_pt.is_some() {
                CursorIcon::Move
            } else {
                CursorIcon::Pointer
            });
        }

        st.since_press = (st.since_press + self.input.dt).min(10.0);

        let can_delete = hovered || self.focus_id == Some(widget_id);
        if can_delete && (self.input.key_delete || self.input.key_backspace) {
            if delete_middle(curve, &mut st.selected) {
                out.changed = true;
            }
        }

        if hovered && self.input.mouse_right_pressed {
            self.focus_id = Some(widget_id);
            if let Some(i) = hover_pt {
                st.selected = Some(i);
                if delete_middle(curve, &mut st.selected) {
                    out.changed = true;
                }
            }
        }

        if hovered && self.input.mouse_pressed {
            self.focus_id = Some(widget_id);
            let is_double = st.since_press < DBL_CLICK_SEC
                && mp.distance(st.last_press_pos) < DBL_CLICK_PX * self.scale;
            st.last_press_pos = mp;
            st.since_press = 0.0;

            let mut hit = None;
            if let Some(i) = hover_pt {
                hit = Some(CurveDrag::Point(i));
                st.selected = Some(i);
            }

            let want_add = hit.is_none() && (self.input.key_ctrl || is_double);
            if want_add {
                let plot = view.screen_to_plot(plot_rect, mp);
                if let Some(idx) = insert_key(curve, plot.x.clamp(0.02, 0.98), plot.y.clamp(0.0, 1.0))
                {
                    st.selected = Some(idx);
                    st.drag = CurveDrag::Point(idx);
                    self.active_id = Some(widget_id);
                    out.changed = true;
                }
            } else if let Some(d) = hit {
                st.drag = d;
                self.active_id = Some(widget_id);
            } else {
                st.selected = None;
                st.drag = CurveDrag::None;
            }
        }

        let active = self.active_id == Some(widget_id);
        if active && self.input.mouse_down {
            match st.drag {
                CurveDrag::Point(i) => {
                    if i < curve.points.len() {
                        let plot = view.screen_to_plot(plot_rect, mp);
                        let mut t = plot.x;
                        let v = plot.y.clamp(0.0, 1.0);
                        if i == 0 {
                            t = 0.0;
                        } else if i + 1 == curve.points.len() {
                            t = 1.0;
                        } else {
                            let lo = curve.points[i - 1].t + 0.01;
                            let hi = curve.points[i + 1].t - 0.01;
                            t = t.clamp(lo, hi);
                        }
                        curve.points[i].t = t;
                        curve.points[i].v = v;
                        curve.preset = CurvePreset::Custom;
                        out.changed = true;
                    }
                }
                CurveDrag::None => {}
            }
        }

        if active && self.input.mouse_released {
            st.drag = CurveDrag::None;
            sort_points(curve);
            if curve.preset == CurvePreset::Custom {
                apply_preset(curve, CurvePreset::Custom);
            }
            if let Some(sel) = st.selected {
                st.selected = Some(sel.min(curve.points.len().saturating_sub(1)));
            }
        }

        out.selected = st.selected;
        self.curve_edits.insert(widget_id, st);
        out
    }

    /// Scrub preview time on the curve editor (0..1).
    pub fn curve_preview_time(&mut self, id: &str, t: f32) {
        let widget_id = self.current_id(id);
        if let Some(st) = self.curve_edits.get_mut(&widget_id) {
            st.preview_t = t.clamp(0.0, 1.0);
        }
    }
}

fn point_screen(curve: &AnimationCurve, view: &PlotView, plot_rect: Rect, i: usize) -> Vec2 {
    let p = &curve.points[i];
    view.plot_to_screen(plot_rect, p.t, p.v)
}

fn nearest_point(
    curve: &AnimationCurve,
    view: &PlotView,
    plot_rect: Rect,
    mp: Vec2,
    max_dist: f32,
) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for i in 0..curve.points.len() {
        let d = mp.distance(point_screen(curve, view, plot_rect, i));
        if d <= max_dist && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    best.map(|(_, i)| i)
}

fn insert_key(curve: &mut AnimationCurve, t: f32, v: f32) -> Option<usize> {
    let t = t.clamp(0.02, 0.98);
    if curve.points.iter().any(|p| (p.t - t).abs() < 0.02) {
        return None;
    }
    curve.points.push(CurvePoint {
        t,
        v: v.clamp(0.0, 1.0),
        tangent_out: 0.0,
    });
    sort_points(curve);
    curve.preset = CurvePreset::Custom;
    apply_preset(curve, CurvePreset::Custom);
    curve.points.iter().position(|p| (p.t - t).abs() < 1e-4)
}

fn delete_middle(curve: &mut AnimationCurve, selected: &mut Option<usize>) -> bool {
    let Some(i) = *selected else {
        return false;
    };
    if i == 0 || i + 1 >= curve.points.len() {
        return false;
    }
    curve.points.remove(i);
    *selected = Some((i - 1).max(1).min(curve.points.len().saturating_sub(2)));
    curve.preset = CurvePreset::Custom;
    apply_preset(curve, CurvePreset::Custom);
    true
}

fn draw_grid(ui: &mut Ui, rect: Rect, view: &PlotView) {
    for i in 1..4 {
        let t = view.t_min + (view.t_max - view.t_min) * (i as f32 / 4.0);
        let x = view.plot_to_screen(rect, t, view.v_min).x;
        let a = Vec2::new(x, rect.min.y);
        let b = Vec2::new(x, rect.max.y);
        ui.draw_line_segment(a, b, 1.0, theme::PLOT_GRID);
    }
    for i in 1..4 {
        let v = view.v_min + (view.v_max - view.v_min) * (i as f32 / 4.0);
        let y = view.plot_to_screen(rect, view.t_min, v).y;
        let a = Vec2::new(rect.min.x, y);
        let b = Vec2::new(rect.max.x, y);
        ui.draw_line_segment(a, b, 1.0, theme::PLOT_GRID);
    }
}
