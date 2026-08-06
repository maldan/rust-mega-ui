//! Animation curve editor (Hermite segments, Unity-style).

use glam::Vec2;

use crate::plot_view::PlotView;
use crate::theme;
use crate::types::{CursorIcon, Rect};
use crate::{LayoutDir, Ui};

const HANDLE_LEN: f32 = 48.0;
const POINT_R: f32 = 5.0;
const HIT_R: f32 = 8.0;

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
    for i in 0..pts.len() - 1 {
        let p0 = &pts[i];
        let p1 = &pts[i + 1];
        if t >= p0.t && t <= p1.t {
            let dt = (p1.t - p0.t).max(1e-5);
            let u = (t - p0.t) / dt;
            let m0 = p0.tangent_out * dt;
            let m1 = if i + 1 < pts.len() {
                pts[i + 1].tangent_out * dt
            } else {
                0.0
            };
            return hermite(p0.v, p1.v, m0, m1, u);
        }
    }
    pts[pts.len() - 1].v
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
    Tangent(usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CurveEditState {
    pub selected: Option<usize>,
    pub drag: CurveDrag,
    pub preview_t: f32,
}

impl Default for CurveEditState {
    fn default() -> Self {
        Self {
            selected: None,
            drag: CurveDrag::None,
            preview_t: 0.0,
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
            apply_preset(curve, CurvePreset::EaseInOut);
        }

        let mut st = self
            .curve_edits
            .get(&widget_id)
            .copied()
            .unwrap_or_default();

        // Preset buttons
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

        let outer = self.allocate(Vec2::new(w, h));
        let radius = self.s(theme::BTN_RADIUS);
        let inner = outer.inset(self.s(4.0));
        let plot_rect = inner;
        let view = PlotView::default();
        self.round_rect(plot_rect, radius, theme::PLOT_BG);

        self.push_clip(plot_rect);
        draw_grid(self, plot_rect, &view);

        let samples = 64;
        let mut line_pts = Vec::with_capacity(samples + 1);
        for i in 0..=samples {
            let u = i as f32 / samples as f32;
            let t = view.t_min + u * (view.t_max - view.t_min);
            let v = sample_curve(curve, t);
            line_pts.push(view.plot_to_screen(plot_rect, t, v));
        }
        self.draw_polyline(&line_pts, self.s(2.0), theme::PLOT_LINE);

        // Tangents + points
        let n = curve.points.len();
        let smooth = auto_smooth_tangents(curve);
        for i in 0..n {
            let pt = &curve.points[i];
            let center = view.plot_to_screen(plot_rect, pt.t, pt.v);
            let tangent = if curve.preset == CurvePreset::Custom {
                pt.tangent_out
            } else {
                smooth[i]
            };
            let handle = center + Vec2::new(HANDLE_LEN * self.scale, -tangent * HANDLE_LEN * self.scale);
            if i < n - 1 || curve.preset == CurvePreset::Custom {
                self.draw_line_segment(center, handle, self.s(1.0), theme::TEXT_DIM);
                let grip = Rect::from_min_size(
                    handle - Vec2::splat(self.s(3.0)),
                    Vec2::splat(self.s(6.0)),
                );
                self.round_rect(grip, self.s(2.0), theme::SLIDER_THUMB);
            }
            let p_rect = Rect::from_min_size(
                center - Vec2::splat(POINT_R * self.scale),
                Vec2::splat(POINT_R * 2.0 * self.scale),
            );
            let sel = st.selected == Some(i);
            self.round_rect(
                p_rect,
                POINT_R * self.scale,
                if sel {
                    theme::ACCENT
                } else {
                    theme::SLIDER_THUMB_HOT
                },
            );
        }

        // Preview playhead
        let preview_v = sample_curve(curve, st.preview_t);
        let px = view.plot_to_screen(plot_rect, st.preview_t, preview_v);
        let vline_a = Vec2::new(px.x, plot_rect.min.y);
        let vline_b = Vec2::new(px.x, plot_rect.max.y);
        self.draw_line_segment(vline_a, vline_b, self.s(1.0), theme::ACCENT_DIM);
        self.pop_clip();

        out.sampled = Some(sample_curve(curve, st.preview_t));
        out.selected = st.selected;

        if !enabled {
            return out;
        }

        let hovered = self.hovered_rect(plot_rect);
        if hovered {
            self.want_capture = true;
        }

        // Interaction
        if hovered && self.input.mouse_pressed {
            let mp = self.input.mouse_pos;
            // Hit tangent grips first
            let mut hit = None;
            for i in 0..n {
                let pt = &curve.points[i];
                let center = view.plot_to_screen(plot_rect, pt.t, pt.v);
                let tangent = if curve.preset == CurvePreset::Custom {
                    pt.tangent_out
                } else {
                    smooth[i]
                };
                let handle = center + Vec2::new(HANDLE_LEN * self.scale, -tangent * HANDLE_LEN * self.scale);
                if mp.distance(handle) < HIT_R * self.scale {
                    hit = Some(CurveDrag::Tangent(i));
                    break;
                }
            }
            if hit.is_none() {
                for i in 0..n {
                    let pt = &curve.points[i];
                    let center = view.plot_to_screen(plot_rect, pt.t, pt.v);
                    if mp.distance(center) < HIT_R * self.scale {
                        hit = Some(CurveDrag::Point(i));
                        st.selected = Some(i);
                        break;
                    }
                }
            }
            if hit.is_none() {
                // Add point on curve
                let plot = view.screen_to_plot(plot_rect, mp);
                let t = plot.x.clamp(0.0, 1.0);
                let v = sample_curve(curve, t);
                curve.points.push(CurvePoint {
                    t,
                    v,
                    tangent_out: 0.0,
                });
                sort_points(curve);
                curve.preset = CurvePreset::Custom;
                apply_preset(curve, CurvePreset::Custom);
                if let Some(idx) = curve.points.iter().position(|p| (p.t - t).abs() < 1e-4) {
                    st.selected = Some(idx);
                }
                out.changed = true;
            }
            if let Some(d) = hit {
                st.drag = d;
                self.active_id = Some(widget_id);
            }
        }

        let active = self.active_id == Some(widget_id);
        if active && self.input.mouse_down {
            let mp = self.input.mouse_pos;
            match st.drag {
                CurveDrag::Point(i) => {
                    if i < curve.points.len() {
                        let plot = view.screen_to_plot(plot_rect, mp);
                        let mut t = plot.x;
                        let v = plot.y.clamp(0.0, 1.0);
                        if i == 0 {
                            t = 0.0;
                        } else if i == curve.points.len() - 1 {
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
                CurveDrag::Tangent(i) => {
                    if i < curve.points.len() {
                        curve.preset = CurvePreset::Custom;
                        let center = view.plot_to_screen(
                            plot_rect,
                            curve.points[i].t,
                            curve.points[i].v,
                        );
                        let dy = (center.y - mp.y) / (HANDLE_LEN * self.scale);
                        curve.points[i].tangent_out = dy;
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
        }

        // Shift+click deletes middle point
        if hovered && self.input.mouse_pressed && self.input.key_shift {
            if let Some(i) = st.selected {
                if i > 0 && i < curve.points.len() - 1 {
                    curve.points.remove(i);
                    st.selected = None;
                    curve.preset = CurvePreset::Custom;
                    apply_preset(curve, CurvePreset::Custom);
                    out.changed = true;
                }
            }
        }

        self.set_cursor(CursorIcon::Pointer);

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
