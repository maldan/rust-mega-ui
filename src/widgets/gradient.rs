//! Horizontal gradient editor: draggable color stops along a 0..1 bar.

use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, DrawCommand, Rect};
use crate::{LayoutDir, Ui};

const POINT_R: f32 = 6.0;
const HIT_R: f32 = 14.0;
const DBL_CLICK_SEC: f32 = 0.35;
const DBL_CLICK_PX: f32 = 8.0;

/// A single color stop in a [`Ui::gradient_editor`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    /// Position along the bar, 0..1.
    pub t: f32,
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GradientEditorResponse {
    pub changed: bool,
    pub selected: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum GradientDrag {
    None,
    Point(usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GradientEditState {
    pub selected: Option<usize>,
    pub drag: GradientDrag,
    pub last_press_pos: Vec2,
    pub since_press: f32,
}

impl Default for GradientEditState {
    fn default() -> Self {
        Self {
            selected: None,
            drag: GradientDrag::None,
            last_press_pos: Vec2::ZERO,
            since_press: 10.0,
        }
    }
}

fn sort_stops(stops: &mut Vec<GradientStop>, selected: &mut Option<usize>) {
    let Some(sel) = *selected else {
        stops.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        return;
    };
    let key = stops.get(sel).copied();
    stops.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(k) = key {
        *selected = stops
            .iter()
            .position(|s| s.t == k.t && s.color == k.color)
            .or(Some(sel.min(stops.len().saturating_sub(1))));
    }
}

/// Sample the gradient at `t`, clamping to the outer stops.
pub fn sample_gradient(stops: &[GradientStop], t: f32) -> [f32; 4] {
    if stops.is_empty() {
        return [1.0, 1.0, 1.0, 1.0];
    }
    if stops.len() == 1 {
        return stops[0].color;
    }
    let t = t.clamp(0.0, 1.0);
    let mut ordered: Vec<&GradientStop> = stops.iter().collect();
    ordered.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    if t <= ordered[0].t {
        return ordered[0].color;
    }
    let last = *ordered.last().unwrap();
    if t >= last.t {
        return last.color;
    }
    for w in ordered.windows(2) {
        let a = w[0];
        let b = w[1];
        if t >= a.t && t <= b.t {
            let span = (b.t - a.t).max(1e-5);
            let u = (t - a.t) / span;
            return [
                a.color[0] + (b.color[0] - a.color[0]) * u,
                a.color[1] + (b.color[1] - a.color[1]) * u,
                a.color[2] + (b.color[2] - a.color[2]) * u,
                a.color[3] + (b.color[3] - a.color[3]) * u,
            ];
        }
    }
    last.color
}

impl Ui {
    /// Edit a color gradient (list of stops at normalized position 0..1).
    ///
    /// Drag stops to move them, double-click / Ctrl+click the bar to add one,
    /// Del/Backspace or right-click a stop to remove it. Use the returned
    /// `selected` index with [`Ui::color_edit`] to edit that stop's color.
    pub fn gradient_editor(
        &mut self,
        id: &str,
        stops: &mut Vec<GradientStop>,
        size: Vec2,
    ) -> GradientEditorResponse {
        let enabled = self.enabled();
        let widget_id = self.current_id(id);
        let mut out = GradientEditorResponse::default();

        if stops.is_empty() {
            stops.push(GradientStop {
                t: 0.0,
                color: [1.0, 1.0, 1.0, 1.0],
            });
            stops.push(GradientStop {
                t: 1.0,
                color: [0.0, 0.0, 0.0, 1.0],
            });
        } else if stops.len() == 1 {
            let c = stops[0].color;
            stops[0].t = 0.0;
            stops.push(GradientStop { t: 1.0, color: c });
        }
        stops.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));

        let fill_w = self.layer().fill_w;
        let w = if size.x <= 0.0 && fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical)
        {
            fill_w
        } else {
            self.s(size.x.max(120.0))
        };
        let h = self.s(size.y.max(28.0));

        let mut st = self
            .gradient_edits
            .get(&widget_id)
            .copied()
            .unwrap_or_default();

        let outer = self.allocate(Vec2::new(w, h));
        let radius = self.s(theme::BTN_RADIUS);
        let marker_pad = self.s(POINT_R + 2.0);
        let bar = Rect {
            min: Vec2::new(outer.min.x + marker_pad, outer.min.y + marker_pad),
            max: Vec2::new(outer.max.x - marker_pad, outer.max.y - marker_pad),
        };

        self.round_rect(outer, radius, theme::BTN_BORDER);
        draw_checker(self, bar);
        self.push_clip(bar);
        for w2 in stops.windows(2) {
            let a = w2[0];
            let b = w2[1];
            let xa = bar.min.x + a.t.clamp(0.0, 1.0) * bar.width();
            let xb = bar.min.x + b.t.clamp(0.0, 1.0) * bar.width();
            if xb <= xa {
                continue;
            }
            let seg = Rect {
                min: Vec2::new(xa, bar.min.y),
                max: Vec2::new(xb, bar.max.y),
            };
            self.gradient_rect(seg, [a.color, b.color, b.color, a.color]);
        }
        self.pop_clip();

        let n = stops.len();
        let mp = self.input.mouse_pos;
        let hit_r = HIT_R * self.scale;
        let hover_pt = nearest_stop(stops, bar, mp, hit_r);

        for i in 0..n {
            let center = stop_screen(stops, bar, i);
            let sel = st.selected == Some(i);
            let hot = hover_pt == Some(i);
            let r = POINT_R * self.scale * if sel { 1.3 } else { 1.0 };
            let ring = Rect::from_min_size(center - Vec2::splat(r + 1.5), Vec2::splat((r + 1.5) * 2.0));
            self.round_rect(
                ring,
                r + 1.5,
                if sel {
                    theme::ACCENT
                } else if hot {
                    theme::SLIDER_THUMB_HOT
                } else {
                    theme::BTN_BORDER
                },
            );
            let fill = Rect::from_min_size(center - Vec2::splat(r), Vec2::splat(r * 2.0));
            self.round_rect(fill, r, [stops[i].color[0], stops[i].color[1], stops[i].color[2], 1.0]);
        }

        out.selected = st.selected;

        if !enabled {
            return out;
        }

        let full_hit = Rect {
            min: Vec2::new(outer.min.x, outer.min.y),
            max: Vec2::new(outer.max.x, outer.max.y),
        };
        let hovered = self.hovered_rect(full_hit);
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
            if delete_stop(stops, &mut st.selected) {
                out.changed = true;
            }
        }

        if hovered && self.input.mouse_right_pressed {
            self.focus_id = Some(widget_id);
            if let Some(i) = hover_pt {
                st.selected = Some(i);
                if delete_stop(stops, &mut st.selected) {
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
                hit = Some(GradientDrag::Point(i));
                st.selected = Some(i);
            }

            let want_add = hit.is_none() && (self.input.key_ctrl || is_double) && bar.width() > 1.0;
            if want_add {
                let t = ((mp.x - bar.min.x) / bar.width()).clamp(0.0, 1.0);
                let color = sample_gradient(stops, t);
                stops.push(GradientStop { t, color });
                let mut sel = Some(stops.len() - 1);
                sort_stops(stops, &mut sel);
                st.selected = sel;
                st.drag = GradientDrag::Point(sel.unwrap_or(0));
                self.active_id = Some(widget_id);
                out.changed = true;
            } else if let Some(d) = hit {
                st.drag = d;
                self.active_id = Some(widget_id);
            } else {
                st.selected = None;
                st.drag = GradientDrag::None;
            }
        }

        let active = self.active_id == Some(widget_id);
        if active && self.input.mouse_down {
            match st.drag {
                GradientDrag::Point(i) => {
                    if i < stops.len() && bar.width() > 1.0 {
                        let t = ((mp.x - bar.min.x) / bar.width()).clamp(0.0, 1.0);
                        stops[i].t = t;
                        out.changed = true;
                    }
                }
                GradientDrag::None => {}
            }
        }

        if active && self.input.mouse_released {
            st.drag = GradientDrag::None;
            sort_stops(stops, &mut st.selected);
        }

        out.selected = st.selected;
        self.gradient_edits.insert(widget_id, st);
        out
    }

    /// Filled quad with independent corner colors (TL, TR, BR, BL). Respects
    /// the current overlay/normal draw target; no clipping (caller must keep
    /// the rect within bounds).
    pub(crate) fn gradient_rect(&mut self, rect: Rect, colors: [[f32; 4]; 4]) {
        let uv = self.font.white_uv();
        let cmd = DrawCommand::gradient(rect, uv, uv, colors, 0.0, 0);
        let out = if self.draw_to_overlay {
            &mut self.overlay
        } else {
            &mut self.draw_list
        };
        out.push(cmd);
    }
}

fn stop_screen(stops: &[GradientStop], bar: Rect, i: usize) -> Vec2 {
    Vec2::new(
        bar.min.x + stops[i].t.clamp(0.0, 1.0) * bar.width(),
        bar.min.y + bar.height() * 0.5,
    )
}

fn nearest_stop(stops: &[GradientStop], bar: Rect, mp: Vec2, max_dist: f32) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for i in 0..stops.len() {
        let d = mp.distance(stop_screen(stops, bar, i));
        if d <= max_dist && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    best.map(|(_, i)| i)
}

fn delete_stop(stops: &mut Vec<GradientStop>, selected: &mut Option<usize>) -> bool {
    let Some(i) = *selected else {
        return false;
    };
    if stops.len() <= 1 || i >= stops.len() {
        return false;
    }
    stops.remove(i);
    *selected = None;
    true
}

fn draw_checker(ui: &mut Ui, rect: Rect) {
    ui.round_rect(rect, 0.0, [0.22, 0.22, 0.22, 1.0]);
    let cell = ui.s(6.0).max(4.0);
    let mut y = rect.min.y;
    let mut row = 0i32;
    while y < rect.max.y {
        let y1 = (y + cell).min(rect.max.y);
        let mut x = rect.min.x;
        let mut col = 0i32;
        while x < rect.max.x {
            let x1 = (x + cell).min(rect.max.x);
            if (row + col) % 2 == 0 {
                ui.round_rect(
                    Rect {
                        min: Vec2::new(x, y),
                        max: Vec2::new(x1, y1),
                    },
                    0.0,
                    [0.32, 0.32, 0.32, 1.0],
                );
            }
            x = x1;
            col += 1;
        }
        y = y1;
        row += 1;
    }
}