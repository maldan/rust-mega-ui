//! Photoshop-style gradient editor: color stops below, opacity above.

use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect};
use crate::{LayoutDir, Ui};

const MARKER_W: f32 = 11.0;
const MARKER_H: f32 = 14.0;
const TIP_H: f32 = 5.0;
const HIT_PAD: f32 = 4.0;
const DBL_CLICK_SEC: f32 = 0.35;
const DBL_CLICK_PX: f32 = 8.0;

/// Color stop (RGB; alpha in `color[3]` is ignored — use [`OpacityStop`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    /// Position along the bar, 0..1.
    pub t: f32,
    pub color: [f32; 4],
}

/// Opacity stop (alpha only).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpacityStop {
    /// Position along the bar, 0..1.
    pub t: f32,
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GradientEditorResponse {
    pub changed: bool,
    pub selected_color: Option<usize>,
    pub selected_opacity: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GradKind {
    Color,
    Opacity,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum GradientDrag {
    None,
    Color(usize),
    Opacity(usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GradientEditState {
    pub sel_kind: Option<GradKind>,
    pub sel_idx: Option<usize>,
    pub drag: GradientDrag,
    pub last_press_pos: Vec2,
    pub since_press: f32,
}

impl Default for GradientEditState {
    fn default() -> Self {
        Self {
            sel_kind: None,
            sel_idx: None,
            drag: GradientDrag::None,
            last_press_pos: Vec2::ZERO,
            since_press: 10.0,
        }
    }
}

fn ensure_color_stops(stops: &mut Vec<GradientStop>) {
    if stops.is_empty() {
        stops.push(GradientStop {
            t: 0.0,
            color: [1.0, 0.0, 0.0, 1.0],
        });
        stops.push(GradientStop {
            t: 1.0,
            color: [0.0, 0.0, 1.0, 1.0],
        });
    } else if stops.len() == 1 {
        let c = stops[0].color;
        stops[0].t = 0.0;
        stops.push(GradientStop { t: 1.0, color: c });
    }
}

fn ensure_opacity_stops(stops: &mut Vec<OpacityStop>) {
    if stops.is_empty() {
        stops.push(OpacityStop { t: 0.0, alpha: 1.0 });
        stops.push(OpacityStop { t: 1.0, alpha: 1.0 });
    } else if stops.len() == 1 {
        let a = stops[0].alpha;
        stops[0].t = 0.0;
        stops.push(OpacityStop { t: 1.0, alpha: a });
    }
}

fn sort_color_stops(stops: &mut [GradientStop], selected: &mut Option<usize>) {
    let key = selected.and_then(|i| stops.get(i).copied());
    stops.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(k) = key {
        *selected = stops
            .iter()
            .position(|s| s.t == k.t && s.color == k.color)
            .or(Some(
                selected.unwrap_or(0).min(stops.len().saturating_sub(1)),
            ));
    }
}

fn sort_opacity_stops(stops: &mut [OpacityStop], selected: &mut Option<usize>) {
    let key = selected.and_then(|i| stops.get(i).copied());
    stops.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(k) = key {
        *selected = stops
            .iter()
            .position(|s| s.t == k.t && (s.alpha - k.alpha).abs() < 1e-6)
            .or(Some(
                selected.unwrap_or(0).min(stops.len().saturating_sub(1)),
            ));
    }
}

fn sample_rgb(stops: &[GradientStop], t: f32) -> [f32; 3] {
    if stops.is_empty() {
        return [1.0, 1.0, 1.0];
    }
    if stops.len() == 1 {
        let c = stops[0].color;
        return [c[0], c[1], c[2]];
    }
    let t = t.clamp(0.0, 1.0);
    let mut ordered: Vec<&GradientStop> = stops.iter().collect();
    ordered.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    if t <= ordered[0].t {
        let c = ordered[0].color;
        return [c[0], c[1], c[2]];
    }
    let last = *ordered.last().unwrap();
    if t >= last.t {
        return [last.color[0], last.color[1], last.color[2]];
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
            ];
        }
    }
    [last.color[0], last.color[1], last.color[2]]
}

fn sample_alpha(stops: &[OpacityStop], t: f32) -> f32 {
    if stops.is_empty() {
        return 1.0;
    }
    if stops.len() == 1 {
        return stops[0].alpha.clamp(0.0, 1.0);
    }
    let t = t.clamp(0.0, 1.0);
    let mut ordered: Vec<&OpacityStop> = stops.iter().collect();
    ordered.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    if t <= ordered[0].t {
        return ordered[0].alpha.clamp(0.0, 1.0);
    }
    let last = *ordered.last().unwrap();
    if t >= last.t {
        return last.alpha.clamp(0.0, 1.0);
    }
    for w in ordered.windows(2) {
        let a = w[0];
        let b = w[1];
        if t >= a.t && t <= b.t {
            let span = (b.t - a.t).max(1e-5);
            let u = (t - a.t) / span;
            return (a.alpha + (b.alpha - a.alpha) * u).clamp(0.0, 1.0);
        }
    }
    last.alpha.clamp(0.0, 1.0)
}

/// Sample RGB from color stops and alpha from opacity stops.
pub fn sample_gradient(colors: &[GradientStop], opacities: &[OpacityStop], t: f32) -> [f32; 4] {
    let rgb = sample_rgb(colors, t);
    let a = sample_alpha(opacities, t);
    [rgb[0], rgb[1], rgb[2], a]
}

fn knot_ts(colors: &[GradientStop], opacities: &[OpacityStop]) -> Vec<f32> {
    let mut ts: Vec<f32> = Vec::with_capacity(colors.len() + opacities.len() + 2);
    ts.push(0.0);
    ts.push(1.0);
    for s in colors {
        ts.push(s.t.clamp(0.0, 1.0));
    }
    for s in opacities {
        ts.push(s.t.clamp(0.0, 1.0));
    }
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    ts.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
    ts
}

impl Ui {
    /// Photoshop-style gradient editor.
    ///
    /// Color stops sit below the bar, opacity stops above. Drag to move,
    /// double-click / Ctrl+click a rail to add, Del/Backspace or RMB to remove.
    /// Built-in Color / Opacity controls edit the selection.
    pub fn gradient_editor(
        &mut self,
        id: &str,
        colors: &mut Vec<GradientStop>,
        opacities: &mut Vec<OpacityStop>,
        size: Vec2,
    ) -> GradientEditorResponse {
        let enabled = self.enabled();
        let widget_id = self.current_id(id);
        let mut out = GradientEditorResponse::default();

        ensure_color_stops(colors);
        ensure_opacity_stops(opacities);
        colors.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        opacities.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));

        let fill_w = self.layer().fill_w;
        let w = if size.x <= 0.0 && fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical)
        {
            fill_w
        } else {
            self.s(size.x.max(160.0))
        };
        let bar_h = self.s(size.y.max(28.0));
        let marker_h = self.s(MARKER_H);
        let rail = marker_h + self.s(2.0);
        let total_h = rail + bar_h + rail;

        let mut st = self
            .gradient_edits
            .get(&widget_id)
            .copied()
            .unwrap_or_default();

        let outer = self.allocate(Vec2::new(w, total_h));
        let pad_x = self.s(MARKER_W * 0.5 + 2.0);
        let bar = Rect {
            min: Vec2::new(outer.min.x + pad_x, outer.min.y + rail),
            max: Vec2::new(outer.max.x - pad_x, outer.min.y + rail + bar_h),
        };
        let opacity_rail = Rect {
            min: Vec2::new(bar.min.x, outer.min.y),
            max: Vec2::new(bar.max.x, bar.min.y),
        };
        let color_rail = Rect {
            min: Vec2::new(bar.min.x, bar.max.y),
            max: Vec2::new(bar.max.x, bar.max.y + rail),
        };

        // Bar: checker + rgba segments
        self.round_rect(bar.inset(-1.0), self.s(2.0), theme::BTN_BORDER);
        draw_checker(self, bar);
        self.push_clip(bar);
        let knots = knot_ts(colors, opacities);
        for w2 in knots.windows(2) {
            let t0 = w2[0];
            let t1 = w2[1];
            if t1 - t0 < 1e-6 {
                continue;
            }
            let c0 = sample_gradient(colors, opacities, t0);
            let c1 = sample_gradient(colors, opacities, t1);
            let xa = bar.min.x + t0 * bar.width();
            let xb = bar.min.x + t1 * bar.width();
            let seg = Rect {
                min: Vec2::new(xa, bar.min.y),
                max: Vec2::new(xb, bar.max.y),
            };
            self.gradient_rect(seg, [c0, c1, c1, c0]);
        }
        self.pop_clip();

        let mp = self.input.mouse_pos;
        let hit_color = nearest_color(colors, bar, mp, self.scale);
        let hit_opacity = nearest_opacity(opacities, bar, mp, self.scale);

        for (i, stop) in opacities.iter().enumerate() {
            let x = bar.min.x + stop.t.clamp(0.0, 1.0) * bar.width();
            let a = stop.alpha.clamp(0.0, 1.0);
            let fill = [a, a, a, 1.0];
            let sel = st.sel_kind == Some(GradKind::Opacity) && st.sel_idx == Some(i);
            let hot = hit_opacity == Some(i);
            draw_house_marker(self, x, bar.min.y, false, fill, sel, hot);
        }
        for (i, stop) in colors.iter().enumerate() {
            let x = bar.min.x + stop.t.clamp(0.0, 1.0) * bar.width();
            let c = stop.color;
            let fill = [c[0], c[1], c[2], 1.0];
            let sel = st.sel_kind == Some(GradKind::Color) && st.sel_idx == Some(i);
            let hot = hit_color == Some(i);
            draw_house_marker(self, x, bar.max.y, true, fill, sel, hot);
        }

        // Footer controls (continue in parent layout below the bar)
        self.horizontal(|ui| match (st.sel_kind, st.sel_idx) {
            (Some(GradKind::Color), Some(i)) if i < colors.len() => {
                ui.label("Color");
                ui.space(4.0);
                if ui
                    .color_edit(&format!("{id}_color"), &mut colors[i].color)
                    .changed()
                {
                    colors[i].color[3] = 1.0;
                    out.changed = true;
                }
            }
            (Some(GradKind::Opacity), Some(i)) if i < opacities.len() => {
                ui.label("Opacity");
                ui.space(4.0);
                if ui
                    .slider(&format!("{id}_op"), &mut opacities[i].alpha, 0.0..=1.0)
                    .changed()
                {
                    opacities[i].alpha = opacities[i].alpha.clamp(0.0, 1.0);
                    out.changed = true;
                }
            }
            _ => {
                ui.label("Select a stop");
            }
        });

        out.selected_color = match (st.sel_kind, st.sel_idx) {
            (Some(GradKind::Color), Some(i)) => Some(i),
            _ => None,
        };
        out.selected_opacity = match (st.sel_kind, st.sel_idx) {
            (Some(GradKind::Opacity), Some(i)) => Some(i),
            _ => None,
        };

        if !enabled {
            self.gradient_edits.insert(widget_id, st);
            return out;
        }

        let hit_area = Rect {
            min: outer.min,
            max: Vec2::new(outer.max.x, color_rail.max.y),
        };
        let hovered = self.hovered_rect(hit_area);
        if hovered {
            self.want_capture = true;
            if st.sel_idx.is_some() {
                self.focus_id = Some(widget_id);
            }
            let over_marker = hit_color.is_some() || hit_opacity.is_some();
            self.set_cursor(if over_marker {
                CursorIcon::Move
            } else {
                CursorIcon::Pointer
            });
        }

        st.since_press = (st.since_press + self.input.dt).min(10.0);

        let can_delete = hovered || self.focus_id == Some(widget_id);
        if can_delete
            && (self.input.key_delete || self.input.key_backspace)
            && delete_selected(colors, opacities, &mut st)
        {
            out.changed = true;
        }

        if hovered && self.input.mouse_right_pressed {
            self.focus_id = Some(widget_id);
            if let Some(i) = hit_color {
                st.sel_kind = Some(GradKind::Color);
                st.sel_idx = Some(i);
                if delete_selected(colors, opacities, &mut st) {
                    out.changed = true;
                }
            } else if let Some(i) = hit_opacity {
                st.sel_kind = Some(GradKind::Opacity);
                st.sel_idx = Some(i);
                if delete_selected(colors, opacities, &mut st) {
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

            if let Some(i) = hit_color {
                st.sel_kind = Some(GradKind::Color);
                st.sel_idx = Some(i);
                st.drag = GradientDrag::Color(i);
                self.active_id = Some(widget_id);
            } else if let Some(i) = hit_opacity {
                st.sel_kind = Some(GradKind::Opacity);
                st.sel_idx = Some(i);
                st.drag = GradientDrag::Opacity(i);
                self.active_id = Some(widget_id);
            } else {
                let want_add = self.input.key_ctrl || is_double;
                let bar_mid_y = (bar.min.y + bar.max.y) * 0.5;
                let on_opacity =
                    opacity_rail.contains(mp) || (bar.contains(mp) && mp.y < bar_mid_y);
                let on_color = color_rail.contains(mp) || (bar.contains(mp) && mp.y >= bar_mid_y);
                if want_add && bar.width() > 1.0 && (on_opacity || on_color) {
                    let t = ((mp.x - bar.min.x) / bar.width()).clamp(0.0, 1.0);
                    if on_opacity {
                        let alpha = sample_alpha(opacities, t);
                        opacities.push(OpacityStop { t, alpha });
                        let mut sel = Some(opacities.len() - 1);
                        sort_opacity_stops(opacities, &mut sel);
                        st.sel_kind = Some(GradKind::Opacity);
                        st.sel_idx = sel;
                        st.drag = GradientDrag::Opacity(sel.unwrap_or(0));
                    } else {
                        let rgb = sample_rgb(colors, t);
                        colors.push(GradientStop {
                            t,
                            color: [rgb[0], rgb[1], rgb[2], 1.0],
                        });
                        let mut sel = Some(colors.len() - 1);
                        sort_color_stops(colors, &mut sel);
                        st.sel_kind = Some(GradKind::Color);
                        st.sel_idx = sel;
                        st.drag = GradientDrag::Color(sel.unwrap_or(0));
                    }
                    self.active_id = Some(widget_id);
                    out.changed = true;
                } else {
                    st.sel_kind = None;
                    st.sel_idx = None;
                    st.drag = GradientDrag::None;
                }
            }
        }

        let active = self.active_id == Some(widget_id);
        if active && self.input.mouse_down && bar.width() > 1.0 {
            let t = ((mp.x - bar.min.x) / bar.width()).clamp(0.0, 1.0);
            match st.drag {
                GradientDrag::Color(i) => {
                    if i < colors.len() {
                        colors[i].t = t;
                        out.changed = true;
                    }
                }
                GradientDrag::Opacity(i) => {
                    if i < opacities.len() {
                        opacities[i].t = t;
                        out.changed = true;
                    }
                }
                GradientDrag::None => {}
            }
        }

        if active && self.input.mouse_released {
            st.drag = GradientDrag::None;
            match st.sel_kind {
                Some(GradKind::Color) => sort_color_stops(colors, &mut st.sel_idx),
                Some(GradKind::Opacity) => sort_opacity_stops(opacities, &mut st.sel_idx),
                None => {}
            }
        }

        out.selected_color = match (st.sel_kind, st.sel_idx) {
            (Some(GradKind::Color), Some(i)) => Some(i),
            _ => None,
        };
        out.selected_opacity = match (st.sel_kind, st.sel_idx) {
            (Some(GradKind::Opacity), Some(i)) => Some(i),
            _ => None,
        };
        self.gradient_edits.insert(widget_id, st);
        out
    }

    /// Filled quad with independent corner colors (TL, TR, BR, BL).
    /// Respects the current clip stack (unless drawing to overlay).
    pub(crate) fn gradient_rect(&mut self, rect: Rect, colors: [[f32; 4]; 4]) {
        let uv = self.font.white_uv();
        let clip = if self.draw_to_overlay {
            None
        } else {
            self.clip()
        };
        let out = if self.draw_to_overlay {
            &mut self.overlay
        } else {
            &mut self.draw_list
        };
        crate::font::push_gradient(out, rect, colors, uv, clip);
    }
}

fn marker_rect(x: f32, bar_edge_y: f32, tip_up: bool, scale: f32) -> Rect {
    let mw = MARKER_W * scale;
    let mh = MARKER_H * scale;
    if tip_up {
        Rect {
            min: Vec2::new(x - mw * 0.5, bar_edge_y),
            max: Vec2::new(x + mw * 0.5, bar_edge_y + mh),
        }
    } else {
        Rect {
            min: Vec2::new(x - mw * 0.5, bar_edge_y - mh),
            max: Vec2::new(x + mw * 0.5, bar_edge_y),
        }
    }
}

fn nearest_color(stops: &[GradientStop], bar: Rect, mp: Vec2, scale: f32) -> Option<usize> {
    let pad = HIT_PAD * scale;
    let mut best: Option<(f32, usize)> = None;
    for (i, s) in stops.iter().enumerate() {
        let x = bar.min.x + s.t.clamp(0.0, 1.0) * bar.width();
        let r = marker_rect(x, bar.max.y, true, scale).inset(-pad);
        if r.contains(mp) {
            let d = (mp.x - x).abs();
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, i));
            }
        }
    }
    best.map(|(_, i)| i)
}

fn nearest_opacity(stops: &[OpacityStop], bar: Rect, mp: Vec2, scale: f32) -> Option<usize> {
    let pad = HIT_PAD * scale;
    let mut best: Option<(f32, usize)> = None;
    for (i, s) in stops.iter().enumerate() {
        let x = bar.min.x + s.t.clamp(0.0, 1.0) * bar.width();
        let r = marker_rect(x, bar.min.y, false, scale).inset(-pad);
        if r.contains(mp) {
            let d = (mp.x - x).abs();
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, i));
            }
        }
    }
    best.map(|(_, i)| i)
}

fn delete_selected(
    colors: &mut Vec<GradientStop>,
    opacities: &mut Vec<OpacityStop>,
    st: &mut GradientEditState,
) -> bool {
    let (Some(kind), Some(i)) = (st.sel_kind, st.sel_idx) else {
        return false;
    };
    match kind {
        GradKind::Color => {
            if colors.len() <= 2 || i >= colors.len() {
                return false;
            }
            colors.remove(i);
        }
        GradKind::Opacity => {
            if opacities.len() <= 2 || i >= opacities.len() {
                return false;
            }
            opacities.remove(i);
        }
    }
    st.sel_kind = None;
    st.sel_idx = None;
    st.drag = GradientDrag::None;
    true
}

fn draw_house_marker(
    ui: &mut Ui,
    x: f32,
    bar_edge_y: f32,
    tip_up: bool,
    fill: [f32; 4],
    selected: bool,
    hot: bool,
) {
    let mw = ui.s(MARKER_W);
    let mh = ui.s(MARKER_H);
    let tip_h = ui.s(TIP_H);
    let border = if selected {
        theme::ACCENT
    } else if hot {
        theme::SLIDER_THUMB_HOT
    } else {
        theme::BTN_BORDER
    };

    let (body, tip_apex, tip_base_y) = if tip_up {
        // tip points up toward bar
        let apex = Vec2::new(x, bar_edge_y);
        let body = Rect {
            min: Vec2::new(x - mw * 0.5, bar_edge_y + tip_h),
            max: Vec2::new(x + mw * 0.5, bar_edge_y + mh),
        };
        (body, apex, bar_edge_y + tip_h)
    } else {
        let apex = Vec2::new(x, bar_edge_y);
        let body = Rect {
            min: Vec2::new(x - mw * 0.5, bar_edge_y - mh),
            max: Vec2::new(x + mw * 0.5, bar_edge_y - tip_h),
        };
        (body, apex, bar_edge_y - tip_h)
    };

    // Border slightly larger
    let br = body.inset(-1.5);
    ui.round_rect(br, ui.s(1.5), border);
    fill_wedge(ui, tip_apex, tip_base_y, mw * 0.5 + 1.5, border);
    ui.round_rect(body, ui.s(1.0), fill);
    fill_wedge(ui, tip_apex, tip_base_y, mw * 0.5, fill);

    if selected {
        let ring = body.inset(-2.5);
        ui.round_rect(
            Rect {
                min: Vec2::new(ring.min.x, ring.min.y),
                max: Vec2::new(ring.max.x, ring.min.y + ui.s(2.0)),
            },
            0.0,
            theme::ACCENT,
        );
    }
}

fn fill_wedge(ui: &mut Ui, apex: Vec2, base_y: f32, half_w: f32, color: [f32; 4]) {
    let h = (base_y - apex.y).abs();
    if h < 0.5 || half_w < 0.5 {
        return;
    }
    let steps = (h / 0.75).ceil().max(2.0) as i32;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        let y0 = apex.y + (base_y - apex.y) * t0;
        let y1 = apex.y + (base_y - apex.y) * t1;
        let w = half_w * ((t0 + t1) * 0.5);
        let y_min = y0.min(y1);
        let y_max = y0.max(y1).max(y_min + 0.75);
        ui.round_rect(
            Rect {
                min: Vec2::new(apex.x - w, y_min),
                max: Vec2::new(apex.x + w, y_max),
            },
            0.0,
            color,
        );
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cstop(t: f32, r: f32, g: f32, b: f32) -> GradientStop {
        GradientStop {
            t,
            color: [r, g, b, 1.0],
        }
    }

    fn ostop(t: f32, alpha: f32) -> OpacityStop {
        OpacityStop { t, alpha }
    }

    #[test]
    fn sample_gradient_empty_stops_returns_opaque_white() {
        let c = sample_gradient(&[], &[], 0.5);
        assert_eq!(c, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn sample_gradient_single_stop_is_constant_everywhere() {
        let colors = vec![cstop(0.5, 0.2, 0.4, 0.6)];
        let opacities = vec![ostop(0.5, 0.3)];
        assert_eq!(
            sample_gradient(&colors, &opacities, 0.0)[..3],
            [0.2, 0.4, 0.6]
        );
        assert_eq!(
            sample_gradient(&colors, &opacities, 1.0)[..3],
            [0.2, 0.4, 0.6]
        );
        assert_eq!(sample_gradient(&colors, &opacities, 1.0)[3], 0.3);
    }

    #[test]
    fn sample_gradient_hits_endpoints_exactly() {
        let colors = vec![cstop(0.0, 1.0, 0.0, 0.0), cstop(1.0, 0.0, 0.0, 1.0)];
        let opacities = vec![ostop(0.0, 1.0), ostop(1.0, 0.0)];
        assert_eq!(
            sample_gradient(&colors, &opacities, 0.0),
            [1.0, 0.0, 0.0, 1.0]
        );
        assert_eq!(
            sample_gradient(&colors, &opacities, 1.0),
            [0.0, 0.0, 1.0, 0.0]
        );
    }

    #[test]
    fn sample_gradient_interpolates_linearly_at_midpoint() {
        let colors = vec![cstop(0.0, 0.0, 0.0, 0.0), cstop(1.0, 1.0, 1.0, 1.0)];
        let opacities = vec![ostop(0.0, 0.0), ostop(1.0, 1.0)];
        let mid = sample_gradient(&colors, &opacities, 0.5);
        for v in mid {
            assert!((v - 0.5).abs() < 1e-4, "expected ~0.5, got {v}");
        }
    }

    #[test]
    fn sample_gradient_clamps_t_outside_0_1() {
        let colors = vec![cstop(0.0, 1.0, 0.0, 0.0), cstop(1.0, 0.0, 0.0, 1.0)];
        assert_eq!(sample_gradient(&colors, &[], -5.0)[..3], [1.0, 0.0, 0.0]);
        assert_eq!(sample_gradient(&colors, &[], 5.0)[..3], [0.0, 0.0, 1.0]);
    }

    #[test]
    fn sample_gradient_stops_out_of_order_are_sorted_before_sampling() {
        // Stops inserted out of `t` order must not break interpolation.
        let colors = vec![cstop(1.0, 0.0, 0.0, 1.0), cstop(0.0, 1.0, 0.0, 0.0)];
        let c = sample_gradient(&colors, &[], 0.25);
        assert!(c[0] > c[2], "expected closer to red at t=0.25, got {c:?}");
    }

    #[test]
    fn sample_alpha_defaults_to_opaque_without_opacity_stops() {
        assert_eq!(
            sample_gradient(&[cstop(0.0, 0.0, 0.0, 0.0)], &[], 0.5)[3],
            1.0
        );
    }

    #[test]
    fn sample_gradient_three_color_stops_middle_value_exact() {
        let colors = vec![
            cstop(0.0, 1.0, 0.0, 0.0),
            cstop(0.5, 0.0, 1.0, 0.0),
            cstop(1.0, 0.0, 0.0, 1.0),
        ];
        let c = sample_gradient(&colors, &[], 0.5);
        assert_eq!(c[..3], [0.0, 1.0, 0.0]);
    }

    #[test]
    fn ensure_color_stops_seeds_default_pair_when_empty() {
        let mut stops = Vec::new();
        ensure_color_stops(&mut stops);
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0].t, 0.0);
        assert_eq!(stops[1].t, 1.0);
    }

    #[test]
    fn ensure_color_stops_duplicates_single_stop_to_pair() {
        let mut stops = vec![cstop(0.7, 0.1, 0.2, 0.3)];
        ensure_color_stops(&mut stops);
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0].t, 0.0);
        assert_eq!(stops[1].t, 1.0);
        assert_eq!(stops[0].color, stops[1].color);
    }

    #[test]
    fn ensure_color_stops_leaves_existing_pair_untouched() {
        let mut stops = vec![cstop(0.2, 1.0, 1.0, 1.0), cstop(0.8, 0.0, 0.0, 0.0)];
        let before = stops.clone();
        ensure_color_stops(&mut stops);
        assert_eq!(stops, before);
    }

    #[test]
    fn ensure_opacity_stops_seeds_default_pair_when_empty() {
        let mut stops = Vec::new();
        ensure_opacity_stops(&mut stops);
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0].alpha, 1.0);
        assert_eq!(stops[1].alpha, 1.0);
    }

    #[test]
    fn sort_color_stops_reorders_and_keeps_selection_on_same_stop() {
        let mut stops = vec![cstop(1.0, 0.0, 0.0, 1.0), cstop(0.0, 1.0, 0.0, 0.0)];
        let mut selected = Some(0); // points at the t=1.0 stop before sort
        sort_color_stops(&mut stops, &mut selected);
        assert_eq!(stops[0].t, 0.0);
        assert_eq!(stops[1].t, 1.0);
        assert_eq!(
            selected,
            Some(1),
            "selection should follow the stop, not the index"
        );
    }
}
