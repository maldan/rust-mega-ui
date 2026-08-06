use glam::Vec2;

use crate::plot_view::PlotView;
use crate::theme;
use crate::types::{DrawCommand, Rect, Response};
use crate::{LayoutDir, Ui};

impl Ui {
    /// Host RGBA texture slot (`kind = 1`). `size` in screen pixels.
    pub fn texture(&mut self, slot: u32, size: Vec2) {
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
        let w = if size.x <= 0.0 && fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical)
        {
            fill_w
        } else {
            self.s(size.x.max(40.0))
        };
        let h = self.s(size.y.max(40.0));
        let radius = self.s(theme::BTN_RADIUS);
        let rect = self.allocate(Vec2::new(w, h));
        self.round_rect(rect, radius, theme::PLOT_BG);
        draw_plot_grid(self, rect, view);
        draw_plot_series(self, rect, view, values, theme::PLOT_LINE);
        self.interact_rect(self.current_id(id), rect)
    }

    /// Interactive plot: wheel zoom, alt+drag pan.
    pub fn plot_interactive(
        &mut self,
        id: &str,
        size: Vec2,
        values: &[f32],
    ) -> Response {
        let widget_id = self.current_id(id);
        let fill_w = self.layer().fill_w;
        let w = if size.x <= 0.0 && fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical)
        {
            fill_w
        } else {
            self.s(size.x.max(40.0))
        };
        let h = self.s(size.y.max(40.0));
        let radius = self.s(theme::BTN_RADIUS);
        let rect = self.allocate(Vec2::new(w, h));

        let mut view = self
            .plot_views
            .get(&widget_id)
            .copied()
            .unwrap_or_else(|| PlotView::fit_values(values));

        if values.len() >= 2 {
            view.t_min = 0.0;
            view.t_max = 1.0;
        }

        self.round_rect(rect, radius, theme::PLOT_BG);
        draw_plot_grid(self, rect, &view);
        draw_plot_series(self, rect, &view, values, theme::PLOT_LINE);

        let hovered = self.hovered_rect(rect);
        if hovered {
            self.want_capture = true;
            if self.input.scroll_delta.y.abs() > 0.0 {
                let mp = self.input.mouse_pos;
                let plot = view.screen_to_plot(rect, mp);
                let factor = if self.input.scroll_delta.y > 0.0 { 0.9 } else { 1.1 };
                view.zoom_uniform(plot.x, plot.y, factor);
                self.consume_scroll();
                self.request_repaint();
            }
        }

        let active_id = widget_id.child("pan");
        if hovered && self.input.mouse_pressed && self.input.key_ctrl {
            self.active_id = Some(active_id);
        }
        let panning = self.active_id == Some(active_id) && self.input.mouse_down;
        if panning {
            if let Some(grab) = self.drag_grab {
                let delta = self.input.mouse_pos - grab;
                let tw = (view.t_max - view.t_min).max(1e-5);
                let vh = (view.v_max - view.v_min).max(1e-5);
                view.pan(-delta.x / rect.width() * tw, delta.y / rect.height() * vh);
                self.drag_grab = Some(self.input.mouse_pos);
                self.request_repaint();
            }
        }
        if self.input.mouse_pressed && self.active_id == Some(active_id) {
            self.drag_grab = Some(self.input.mouse_pos);
        }

        let resp = self.interact_rect(widget_id, rect);
        self.plot_views.insert(widget_id, view);
        resp
    }
}

fn draw_plot_grid(ui: &mut Ui, rect: Rect, view: &PlotView) {
    for i in 1..4 {
        let t = view.t_min + (view.t_max - view.t_min) * (i as f32 / 4.0);
        let x = view.plot_to_screen(rect, t, view.v_min).x;
        ui.draw_line_segment(
            Vec2::new(x, rect.min.y),
            Vec2::new(x, rect.max.y),
            1.0,
            theme::PLOT_GRID,
        );
    }
    for i in 1..4 {
        let v = view.v_min + (view.v_max - view.v_min) * (i as f32 / 4.0);
        let y = view.plot_to_screen(rect, view.t_min, v).y;
        ui.draw_line_segment(
            Vec2::new(rect.min.x, y),
            Vec2::new(rect.max.x, y),
            1.0,
            theme::PLOT_GRID,
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
