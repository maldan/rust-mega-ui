use glam::Vec2;

use crate::plot_view::PlotView;
use crate::types::{DrawCommand, Rect, Response};
use crate::{LayoutDir, Ui};

impl Ui {
    /// Host RGBA texture slot (`kind = 1`). `size` in screen pixels.
    /// Returns the allocated screen-space rect (for hit-testing / picking).
    pub fn texture(&mut self, slot: u32, size: Vec2) -> Rect {
        let fill_w = self.layer().fill_w;
        let w = if size.x > 0.0 {
            size.x
        } else if fill_w > 0.0 {
            fill_w
        } else {
            1.0
        };
        let h = size.y.max(1.0);
        let rect = self.allocate(Vec2::new(w, h));
        push_tex_cmd(self, rect, slot);
        rect
    }

    /// Host texture in an explicit screen rect, sampling `uv_min`..`uv_max` (0..1).
    /// Clipped to the current clip rect; UVs follow the visible portion.
    pub fn texture_rect(&mut self, slot: u32, rect: Rect, uv_min: [f32; 2], uv_max: [f32; 2]) {
        let (rect, uv_min, uv_max) = match self.clip() {
            Some(clip) => {
                let Some(visible) = rect.intersect(clip) else {
                    return;
                };
                let (uv_min, uv_max) = remap_uv(rect, visible, uv_min, uv_max);
                (visible, uv_min, uv_max)
            }
            None => (rect, uv_min, uv_max),
        };
        self.draw_list.push(DrawCommand::solid(
            rect,
            uv_min,
            uv_max,
            [1.0, 1.0, 1.0, 1.0],
            1.0,
            slot,
        ));
    }

    /// Host texture as a (possibly rotated) quad. Corners are TL, TR, BR, BL.
    /// Clipped to the current clip rect with UV interpolation.
    pub fn texture_quad(&mut self, slot: u32, pos: [Vec2; 4], uv: [[f32; 2]; 4]) {
        let mut poly: Vec<(Vec2, [f32; 2])> = pos
            .into_iter()
            .zip(uv)
            .map(|(p, u)| (p, u))
            .collect();
        if let Some(clip) = self.clip() {
            poly = clip_poly_to_rect(poly, clip);
        }
        if poly.len() < 3 {
            return;
        }
        let origin = poly[0];
        for i in 1..poly.len().saturating_sub(1) {
            let a = poly[i];
            let b = poly[i + 1];
            self.draw_list.push(DrawCommand::textured_quad(
                [
                    [origin.0.x, origin.0.y],
                    [a.0.x, a.0.y],
                    [b.0.x, b.0.y],
                    [origin.0.x, origin.0.y],
                ],
                [origin.1, a.1, b.1, origin.1],
                slot,
            ));
        }
    }

    /// Convenience: texture slot 0, size in UI points (scaled).
    pub fn image(&mut self, size: Vec2) {
        let fill_w = self.layer().fill_w;
        let w = if size.x <= 0.0 && fill_w > 0.0 {
            fill_w
        } else {
            self.s(size.x.max(1.0))
        };
        let h = self.s(size.y.max(1.0));
        self.texture(0, Vec2::new(w, h));
    }

    /// Simple line plot (auto-scaled values, t = index).
    pub fn plot(&mut self, size: Vec2, values: &[f32]) {
        let view = PlotView::fit_values(values);
        self.plot_with_view("plot", size, values, &view);
    }

    /// Line plot with explicit view mapping.
    pub fn plot_with_view(
        &mut self,
        id: &str,
        size: Vec2,
        values: &[f32],
        view: &PlotView,
    ) -> Response {
        let fill_w = self.layer().fill_w;
        let filling =
            size.x <= 0.0 && fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical);
        let w = if filling {
            fill_w
        } else {
            self.s(size.x.max(40.0))
        };
        let h = self.s(size.y.max(40.0));
        let radius = self.s(self.theme.metrics.button_radius);
        let rect = if filling {
            self.allocate_fill_x(Vec2::new(w, h))
        } else {
            self.allocate(Vec2::new(w, h))
        };
        self.round_rect(rect, radius, self.theme.plot.bg);
        self.push_clip(rect);
        draw_plot_grid(self, rect, view);
        draw_plot_series(self, rect, view, values, self.theme.plot.line);
        self.pop_clip();
        self.interact_rect(self.current_id(id), rect)
    }

    /// Vertical bars, values in `0..=1`, left = first bin.
    /// `ticks` are `(0..=1, label)` along the X axis (left → right).
    pub fn plot_bars(
        &mut self,
        id: &str,
        size: Vec2,
        values: &[f32],
        ticks: &[(f32, &str)],
    ) -> Response {
        let (rect, radius) = plot_rect(self, size);
        self.round_rect(rect, radius, self.theme.plot.bg);
        let axis_h = self.s(13.0);
        let plot = Rect {
            min: rect.min,
            max: Vec2::new(rect.max.x, (rect.max.y - axis_h).max(rect.min.y + 8.0)),
        };
        self.push_clip(plot);
        if !values.is_empty() {
            let n = values.len() as f32;
            let gap = self.s(1.0).min(plot.width() / n * 0.25);
            let bw = ((plot.width() - gap * (n - 1.0)) / n).max(1.0);
            for (i, v) in values.iter().enumerate() {
                let h = plot.height() * v.clamp(0.0, 1.0);
                if h < 0.5 {
                    continue;
                }
                let x = plot.min.x + i as f32 * (bw + gap);
                let bar = Rect {
                    min: Vec2::new(x, plot.max.y - h),
                    max: Vec2::new(x + bw, plot.max.y),
                };
                self.fill_rect(bar, self.theme.plot.line);
            }
        }
        draw_tick_grid_x(self, plot, ticks);
        self.pop_clip();
        draw_tick_labels_x(self, rect, plot, ticks);
        self.interact_rect(self.current_id(id), rect)
    }

    /// Same as [`Self::plot_bars`], drag paints amplitude (`0..=1`).
    pub fn plot_bars_edit(
        &mut self,
        id: &str,
        size: Vec2,
        values: &mut [f32],
        ticks: &[(f32, &str)],
    ) -> Response {
        let (rect, radius) = plot_rect(self, size);
        self.round_rect(rect, radius, self.theme.plot.bg);
        let axis_h = self.s(13.0);
        let plot = Rect {
            min: rect.min,
            max: Vec2::new(rect.max.x, (rect.max.y - axis_h).max(rect.min.y + 8.0)),
        };
        self.push_clip(plot);
        if !values.is_empty() {
            let n = values.len() as f32;
            let gap = self.s(1.0).min(plot.width() / n * 0.25);
            let bw = ((plot.width() - gap * (n - 1.0)) / n).max(1.0);
            for (i, v) in values.iter().enumerate() {
                let h = plot.height() * v.clamp(0.0, 1.0);
                if h < 0.5 {
                    continue;
                }
                let x = plot.min.x + i as f32 * (bw + gap);
                let bar = Rect {
                    min: Vec2::new(x, plot.max.y - h),
                    max: Vec2::new(x + bw, plot.max.y),
                };
                self.fill_rect(bar, self.theme.plot.line);
            }
        }
        draw_tick_grid_x(self, plot, ticks);
        self.pop_clip();
        draw_tick_labels_x(self, rect, plot, ticks);
        let widget_id = self.current_id(id);
        let mut resp = self.interact_rect(widget_id, plot);
        if self.active_id == Some(widget_id) && self.input.mouse_down && !values.is_empty() {
            self.want_capture = true;
            let n = values.len() as f32;
            let t =
                ((self.input.mouse_pos.x - plot.min.x) / plot.width().max(1.0)).clamp(0.0, 0.999);
            let i = (t * n) as usize;
            let v =
                ((plot.max.y - self.input.mouse_pos.y) / plot.height().max(1.0)).clamp(0.0, 1.0);
            if (values[i] - v).abs() > 1e-4 {
                values[i] = v;
                resp.changed = true;
            }
        }
        resp
    }

    /// Line through `values` in `0..=1`, left = first sample. Same X ticks as `plot_bars`.
    pub fn plot_line(
        &mut self,
        id: &str,
        size: Vec2,
        values: &[f32],
        ticks: &[(f32, &str)],
    ) -> Response {
        let (rect, radius) = plot_rect(self, size);
        self.round_rect(rect, radius, self.theme.plot.bg);
        let axis_h = self.s(13.0);
        let plot = Rect {
            min: rect.min,
            max: Vec2::new(rect.max.x, (rect.max.y - axis_h).max(rect.min.y + 8.0)),
        };
        self.push_clip(plot);
        if values.len() >= 2 {
            let n = (values.len() - 1) as f32;
            let mut pts = Vec::with_capacity(values.len());
            for (i, v) in values.iter().enumerate() {
                let t = i as f32 / n;
                pts.push(Vec2::new(
                    plot.min.x + plot.width() * t,
                    plot.max.y - plot.height() * v.clamp(0.0, 1.0),
                ));
            }
            self.draw_polyline(&pts, self.s(1.5), self.theme.plot.line);
        }
        draw_tick_grid_x(self, plot, ticks);
        self.pop_clip();
        draw_tick_labels_x(self, rect, plot, ticks);
        self.interact_rect(self.current_id(id), rect)
    }

    /// Heatmap: `values.len() == cols * rows`, column-major, row 0 at the bottom.
    /// `ticks` / `notes` are `(0..=1, label)` along the Y axis (bottom → top).
    /// Notes draw in a left column; Hz ticks sit next to the plot.
    pub fn plot_heatmap(
        &mut self,
        id: &str,
        size: Vec2,
        cols: usize,
        rows: usize,
        values: &[f32],
        ticks: &[(f32, &str)],
        notes: &[(f32, &str)],
    ) -> Response {
        let (rect, radius) = plot_rect(self, size);
        self.round_rect(rect, radius, self.theme.plot.bg);
        let note_w = if notes.is_empty() { 0.0 } else { self.s(22.0) };
        let axis_w = note_w + self.s(26.0);
        let plot = Rect {
            min: Vec2::new((rect.min.x + axis_w).min(rect.max.x - 8.0), rect.min.y),
            max: rect.max,
        };
        self.push_clip(plot);
        if cols > 0 && rows > 0 && values.len() >= cols * rows {
            let cw = plot.width() / cols as f32;
            let rh = plot.height() / rows as f32;
            for c in 0..cols {
                for r in 0..rows {
                    let t = values[c * rows + r];
                    if t <= 0.02 {
                        continue;
                    }
                    let cell = Rect {
                        min: Vec2::new(
                            plot.min.x + c as f32 * cw,
                            plot.max.y - (r as f32 + 1.0) * rh,
                        ),
                        max: Vec2::new(
                            plot.min.x + (c as f32 + 1.0) * cw,
                            plot.max.y - r as f32 * rh,
                        ),
                    };
                    self.fill_rect(cell, heat_color(t));
                }
            }
        }
        draw_tick_grid_y(self, plot, ticks);
        self.pop_clip();
        if !notes.is_empty() {
            draw_tick_labels_y(self, rect.min.x + self.s(2.0), rect, plot, notes);
        }
        draw_tick_labels_y(self, rect.min.x + note_w + self.s(2.0), rect, plot, ticks);
        self.interact_rect(self.current_id(id), rect)
    }
}

pub(crate) fn draw_tick_grid_x(ui: &mut Ui, plot: Rect, ticks: &[(f32, &str)]) {
    for &(t, _) in ticks {
        let x = plot.min.x + plot.width() * t.clamp(0.0, 1.0);
        ui.draw_line_segment(
            Vec2::new(x, plot.min.y),
            Vec2::new(x, plot.max.y),
            1.0,
            ui.theme.plot.grid,
        );
    }
}

fn draw_tick_grid_y(ui: &mut Ui, plot: Rect, ticks: &[(f32, &str)]) {
    for &(t, _) in ticks {
        let y = plot.max.y - plot.height() * t.clamp(0.0, 1.0);
        ui.draw_line_segment(
            Vec2::new(plot.min.x, y),
            Vec2::new(plot.max.x, y),
            1.0,
            ui.theme.plot.grid,
        );
    }
}

pub(crate) fn draw_tick_labels_x(ui: &mut Ui, outer: Rect, plot: Rect, ticks: &[(f32, &str)]) {
    let px = ui.s(10.0);
    let mut last = outer.min.x - 8.0;
    for &(t, label) in ticks {
        let x = plot.min.x + plot.width() * t.clamp(0.0, 1.0);
        let tw = ui.text_width_at(label, px);
        let lx = (x - tw * 0.5).clamp(outer.min.x + 1.0, outer.max.x - tw - 1.0);
        if lx < last {
            continue;
        }
        ui.text_sized(
            Vec2::new(lx, plot.max.y + ui.s(1.0)),
            label,
            ui.theme.text.disabled,
            px,
        );
        last = lx + tw + ui.s(4.0);
    }
}

fn draw_tick_labels_y(ui: &mut Ui, x: f32, outer: Rect, plot: Rect, ticks: &[(f32, &str)]) {
    let px = ui.s(10.0);
    let th = ui.text_height_at(px);
    let mut last_y = f32::INFINITY;
    for &(t, label) in ticks {
        let y = plot.max.y - plot.height() * t.clamp(0.0, 1.0);
        let ty = (y - th * 0.5).clamp(outer.min.y + 1.0, outer.max.y - th - 1.0);
        if (last_y - ty).abs() < th {
            continue;
        }
        ui.text_sized(Vec2::new(x, ty), label, ui.theme.text.disabled, px);
        last_y = ty;
    }
}

fn plot_rect(ui: &mut Ui, size: Vec2) -> (Rect, f32) {
    let fill_w = ui.layer().fill_w;
    let filling = size.x <= 0.0 && fill_w > 0.0 && matches!(ui.layer().dir, LayoutDir::Vertical);
    let w = if filling {
        fill_w
    } else {
        ui.s(size.x.max(40.0))
    };
    let h = ui.s(size.y.max(40.0));
    let radius = ui.s(ui.theme.metrics.button_radius);
    let rect = if filling {
        ui.allocate_fill_x(Vec2::new(w, h))
    } else {
        ui.allocate(Vec2::new(w, h))
    };
    (rect, radius)
}

fn heat_color(t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    let (a, b, u) = if t < 0.25 {
        ([0.02, 0.04, 0.14, 1.0], [0.08, 0.18, 0.62, 1.0], t / 0.25)
    } else if t < 0.5 {
        (
            [0.08, 0.18, 0.62, 1.0],
            [0.05, 0.72, 0.78, 1.0],
            (t - 0.25) / 0.25,
        )
    } else if t < 0.75 {
        (
            [0.05, 0.72, 0.78, 1.0],
            [0.95, 0.82, 0.18, 1.0],
            (t - 0.5) / 0.25,
        )
    } else {
        (
            [0.95, 0.82, 0.18, 1.0],
            [1.0, 0.98, 0.92, 1.0],
            (t - 0.75) / 0.25,
        )
    };
    [
        a[0] + (b[0] - a[0]) * u,
        a[1] + (b[1] - a[1]) * u,
        a[2] + (b[2] - a[2]) * u,
        1.0,
    ]
}

fn draw_plot_grid(ui: &mut Ui, rect: Rect, view: &PlotView) {
    for i in 1..4 {
        let t = view.t_min + (view.t_max - view.t_min) * (i as f32 / 4.0);
        let x = view.plot_to_screen(rect, t, view.v_min).x;
        ui.draw_line_segment(
            Vec2::new(x, rect.min.y),
            Vec2::new(x, rect.max.y),
            1.0,
            ui.theme.plot.grid,
        );
    }
    for i in 1..4 {
        let v = view.v_min + (view.v_max - view.v_min) * (i as f32 / 4.0);
        let y = view.plot_to_screen(rect, view.t_min, v).y;
        ui.draw_line_segment(
            Vec2::new(rect.min.x, y),
            Vec2::new(rect.max.x, y),
            1.0,
            ui.theme.plot.grid,
        );
    }
}

fn draw_plot_series(ui: &mut Ui, rect: Rect, view: &PlotView, values: &[f32], color: [f32; 4]) {
    if values.len() < 2 {
        return;
    }
    let n = values.len() - 1;
    let mut pts = Vec::with_capacity(values.len());
    for (i, v) in values.iter().enumerate() {
        let t = i as f32 / n as f32;
        pts.push(view.plot_to_screen(rect, t, *v));
    }
    ui.draw_polyline(&pts, ui.s(2.0), color);
}

fn clip_poly_to_rect(
    poly: Vec<(Vec2, [f32; 2])>,
    clip: Rect,
) -> Vec<(Vec2, [f32; 2])> {
    let edges = [
        (clip.min.x, true, true),
        (clip.max.x, true, false),
        (clip.min.y, false, true),
        (clip.max.y, false, false),
    ];
    let mut current = poly;
    for (bound, is_x, keep_less) in edges {
        if current.is_empty() {
            break;
        }
        let mut next = Vec::with_capacity(current.len() + 1);
        for i in 0..current.len() {
            let a = current[i];
            let b = current[(i + 1) % current.len()];
            let a_in = inside(a.0, bound, is_x, keep_less);
            let b_in = inside(b.0, bound, is_x, keep_less);
            if a_in {
                next.push(a);
            }
            if a_in != b_in {
                next.push(intersect(a, b, bound, is_x));
            }
        }
        current = next;
    }
    current
}

fn inside(p: Vec2, bound: f32, is_x: bool, keep_less: bool) -> bool {
    let v = if is_x { p.x } else { p.y };
    if keep_less {
        v >= bound
    } else {
        v <= bound
    }
}

fn intersect(
    a: (Vec2, [f32; 2]),
    b: (Vec2, [f32; 2]),
    bound: f32,
    is_x: bool,
) -> (Vec2, [f32; 2]) {
    let (av, bv) = if is_x {
        (a.0.x, b.0.x)
    } else {
        (a.0.y, b.0.y)
    };
    let denom = bv - av;
    let t = if denom.abs() < 1e-8 {
        0.0
    } else {
        (bound - av) / denom
    };
    let t = t.clamp(0.0, 1.0);
    let pos = a.0 + (b.0 - a.0) * t;
    let uv = [
        a.1[0] + (b.1[0] - a.1[0]) * t,
        a.1[1] + (b.1[1] - a.1[1]) * t,
    ];
    (pos, uv)
}

fn remap_uv(full: crate::types::Rect, visible: crate::types::Rect, uv_min: [f32; 2], uv_max: [f32; 2]) -> ([f32; 2], [f32; 2]) {
    let rw = full.width().max(1e-4);
    let rh = full.height().max(1e-4);
    let su = uv_max[0] - uv_min[0];
    let sv = uv_max[1] - uv_min[1];
    let u0 = uv_min[0] + su * (visible.min.x - full.min.x) / rw;
    let u1 = uv_min[0] + su * (visible.max.x - full.min.x) / rw;
    let v0 = uv_min[1] + sv * (visible.min.y - full.min.y) / rh;
    let v1 = uv_min[1] + sv * (visible.max.y - full.min.y) / rh;
    ([u0, v0], [u1, v1])
}

fn push_tex_cmd(ui: &mut Ui, rect: crate::types::Rect, slot: u32) {
    let clip = ui.clip();
    let (rect, uv_min, uv_max) = match clip {
        Some(c) => {
            let Some(clipped) = rect.intersect(c) else {
                return;
            };
            let rw = rect.width().max(1.0);
            let rh = rect.height().max(1.0);
            let u0 = (clipped.min.x - rect.min.x) / rw;
            let u1 = (clipped.max.x - rect.min.x) / rw;
            let v0 = (clipped.min.y - rect.min.y) / rh;
            let v1 = (clipped.max.y - rect.min.y) / rh;
            (clipped, [u0, v0], [u1, v1])
        }
        None => (rect, [0.0, 0.0], [1.0, 1.0]),
    };
    ui.draw_list.push(DrawCommand::solid(
        rect,
        uv_min,
        uv_max,
        [1.0, 1.0, 1.0, 1.0],
        1.0,
        slot,
    ));
}
