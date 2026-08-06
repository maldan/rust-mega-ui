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
        self.push_clip(rect);
        draw_plot_grid(self, rect, view);
        draw_plot_series(self, rect, view, values, theme::PLOT_LINE);
        self.pop_clip();
        self.interact_rect(self.current_id(id), rect)
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
