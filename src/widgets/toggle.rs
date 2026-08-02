use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Response};
use crate::{LayoutDir, Ui};

impl Ui {
    /// One-of-many choice (segmented / radio group).
    pub fn toggle(&mut self, id: &str, selected: &mut usize, options: &[&str]) -> Response {
        let widget_id = self.current_id(id);
        if options.is_empty() {
            return Response::default();
        }
        if *selected >= options.len() {
            *selected = 0;
        }

        let height = self.s(26.0);
        let radius = self.s(theme::BTN_RADIUS);
        let pad = self.s(24.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            options
                .iter()
                .map(|o| self.text_width(o) + pad)
                .sum::<f32>()
                .max(self.s(120.0))
        };
        let rect = self.allocate(Vec2::new(width, height));
        self.round_rect(rect, radius, theme::BTN_BORDER);
        self.round_rect(rect.inset(1.0), (radius - 1.0).max(0.0), theme::TOGGLE_OFF);

        let n = options.len() as f32;
        let seg_w = rect.width() / n;
        let mut changed = false;
        let mut any_hover = false;

        for (i, label) in options.iter().enumerate() {
            let seg = crate::Rect::from_min_size(
                Vec2::new(rect.min.x + i as f32 * seg_w, rect.min.y),
                Vec2::new(seg_w, height),
            );
            let hovered = self.hovered_rect(seg);
            if hovered {
                any_hover = true;
                self.want_capture = true;
                self.set_cursor(CursorIcon::Pointer);
                self.hover_id = Some(widget_id.child(i));
            }
            if hovered && self.input.mouse_pressed {
                self.active_id = Some(widget_id.child(i));
            }
            let clicked = self.active_id == Some(widget_id.child(i))
                && hovered
                && self.input.mouse_released;
            if clicked && *selected != i {
                *selected = i;
                changed = true;
            }

            let active = *selected == i;
            let color = if active {
                theme::TOGGLE_ON
            } else if hovered {
                theme::BTN_HOVER
            } else {
                theme::TOGGLE_OFF
            };
            if active || hovered {
                self.round_rect(seg.inset(self.s(2.0)), (radius - 1.0).max(0.0), color);
            }

            let tw = self.text_width(label);
            let th = self.text_height();
            self.text(
                Vec2::new(
                    seg.min.x + (seg_w - tw) * 0.5,
                    seg.min.y + (height - th) * 0.5,
                ),
                label,
                if active {
                    theme::TEXT_BRIGHT
                } else {
                    theme::TEXT
                },
            );
        }

        Response {
            hovered: any_hover,
            clicked: changed,
            changed,
        }
    }

    pub fn progress_bar(&mut self, value: f32) {
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            self.s(160.0)
        };
        let height = self.s(14.0);
        let radius = self.s(theme::BTN_RADIUS);
        let rect = self.allocate(Vec2::new(width, height));
        let t = value.clamp(0.0, 1.0);
        self.round_rect(rect, radius, theme::PROGRESS_BG);
        if t > 0.0 {
            let fill = crate::Rect {
                min: rect.min,
                max: Vec2::new(rect.min.x + rect.width() * t, rect.max.y),
            };
            self.round_rect(fill, radius, theme::PROGRESS_FILL);
        }
        let label = format!("{:.0}%", t * 100.0);
        let tw = self.text_width(&label);
        let th = self.text_height();
        if tw + self.s(4.0) < width {
            self.text(
                Vec2::new(rect.min.x + (width - tw) * 0.5, rect.min.y + (height - th) * 0.5),
                &label,
                theme::TEXT,
            );
        }
    }
}
