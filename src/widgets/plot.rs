use glam::Vec2;

use crate::theme;
use crate::types::DrawCommand;
use crate::{LayoutDir, Ui};

impl Ui {
    /// Host RGBA texture slot (`kind = 1`). `size` in screen pixels.
    /// Slot is bound by the app (0, 1, …).
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

    /// `size` is in UI points (scaled by `ui.scale()`). Use `x <= 0` to fill width.
    pub fn plot(&mut self, size: Vec2, values: &[f32]) {
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

        for i in 1..4 {
            let y = rect.min.y + h * (i as f32 / 4.0);
            self.round_rect(
                crate::Rect {
                    min: Vec2::new(rect.min.x + 1.0, y),
                    max: Vec2::new(rect.max.x - 1.0, y + 1.0),
                },
                0.0,
                theme::PLOT_GRID,
            );
        }

        if values.len() < 2 {
            return;
        }
        let mut ymin = values[0];
        let mut ymax = values[0];
        for v in values {
            ymin = ymin.min(*v);
            ymax = ymax.max(*v);
        }
        if (ymax - ymin).abs() < 1e-5 {
            ymax = ymin + 1.0;
        }
        let pad = self.s(4.0);
        let inner = rect.inset(pad);
        let n = (values.len() - 1) as f32;
        let dot = self.s(2.0);
        for i in 0..values.len() - 1 {
            let t0 = i as f32 / n;
            let t1 = (i + 1) as f32 / n;
            let x0 = inner.min.x + inner.width() * t0;
            let x1 = inner.min.x + inner.width() * t1;
            let y0 = inner.max.y - inner.height() * ((values[i] - ymin) / (ymax - ymin));
            let y1 = inner.max.y - inner.height() * ((values[i + 1] - ymin) / (ymax - ymin));
            let steps = ((x1 - x0).abs() + (y1 - y0).abs()).max(1.0) as i32;
            for s in 0..steps {
                let u = s as f32 / steps as f32;
                let x = x0 + (x1 - x0) * u;
                let y = y0 + (y1 - y0) * u;
                self.round_rect(
                    crate::Rect::from_min_size(
                        Vec2::new(x - dot * 0.5, y - dot * 0.5),
                        Vec2::splat(dot),
                    ),
                    0.0,
                    theme::PLOT_LINE,
                );
            }
        }
    }
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
    ui.draw_list.push(DrawCommand {
        rect,
        uv_min,
        uv_max,
        color: [1.0, 1.0, 1.0, 1.0],
        kind: 1.0,
        tex: slot,
    });
}
