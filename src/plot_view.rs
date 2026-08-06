use glam::Vec2;

use crate::types::Rect;

/// Pan/zoom mapping for plot and curve widgets.
#[derive(Clone, Copy, Debug)]
pub struct PlotView {
    pub t_min: f32,
    pub t_max: f32,
    pub v_min: f32,
    pub v_max: f32,
}

impl Default for PlotView {
    fn default() -> Self {
        Self {
            t_min: 0.0,
            t_max: 1.0,
            v_min: 0.0,
            v_max: 1.0,
        }
    }
}

impl PlotView {
    pub fn new(t_min: f32, t_max: f32, v_min: f32, v_max: f32) -> Self {
        Self {
            t_min,
            t_max,
            v_min,
            v_max,
        }
    }

    pub fn fit_values(values: &[f32]) -> Self {
        if values.is_empty() {
            return Self::default();
        }
        let mut vmin = values[0];
        let mut vmax = values[0];
        for v in values {
            vmin = vmin.min(*v);
            vmax = vmax.max(*v);
        }
        if (vmax - vmin).abs() < 1e-5 {
            vmax = vmin + 1.0;
        }
        let pad = (vmax - vmin) * 0.08;
        Self {
            t_min: 0.0,
            t_max: 1.0,
            v_min: vmin - pad,
            v_max: vmax + pad,
        }
    }

    pub fn plot_to_screen(&self, rect: Rect, t: f32, v: f32) -> Vec2 {
        let tw = (self.t_max - self.t_min).max(1e-5);
        let vh = (self.v_max - self.v_min).max(1e-5);
        let u = (t - self.t_min) / tw;
        let y = (v - self.v_min) / vh;
        Vec2::new(
            rect.min.x + u * rect.width(),
            rect.max.y - y * rect.height(),
        )
    }

    pub fn screen_to_plot(&self, rect: Rect, p: Vec2) -> Vec2 {
        let tw = (self.t_max - self.t_min).max(1e-5);
        let vh = (self.v_max - self.v_min).max(1e-5);
        let u = ((p.x - rect.min.x) / rect.width().max(1e-3)).clamp(0.0, 1.0);
        let y = 1.0 - ((p.y - rect.min.y) / rect.height().max(1e-3)).clamp(0.0, 1.0);
        Vec2::new(self.t_min + u * tw, self.v_min + y * vh)
    }

    pub fn pan(&mut self, dt: f32, dv: f32) {
        self.t_min -= dt;
        self.t_max -= dt;
        self.v_min -= dv;
        self.v_max -= dv;
    }

    pub fn zoom_uniform(&mut self, center_t: f32, center_v: f32, factor: f32) {
        let f = factor.clamp(0.1, 10.0);
        let tw = self.t_max - self.t_min;
        let vh = self.v_max - self.v_min;
        let new_tw = tw * f;
        let new_vh = vh * f;
        let ut = if tw > 1e-5 {
            (center_t - self.t_min) / tw
        } else {
            0.5
        };
        let uv = if vh > 1e-5 {
            (center_v - self.v_min) / vh
        } else {
            0.5
        };
        self.t_min = center_t - ut * new_tw;
        self.t_max = self.t_min + new_tw;
        self.v_min = center_v - uv * new_vh;
        self.v_max = self.v_min + new_vh;
    }
}
