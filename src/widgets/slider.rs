use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect, Response};
use crate::{LayoutDir, Ui};

impl Ui {
    pub fn slider(&mut self, id: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> Response {
        let widget_id = self.current_id(id);
        let height = self.s(18.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            self.s(160.0)
        };
        let rect = self.allocate(Vec2::new(width, height));

        let hovered = self.hovered_rect(rect);
        if hovered {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        if hovered && self.input.mouse_pressed {
            self.active_id = Some(widget_id);
        }

        let active = self.active_id == Some(widget_id);
        let mut changed = false;
        if active && self.input.mouse_down {
            let t = ((self.input.mouse_pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
            let (min, max) = (*range.start(), *range.end());
            let new = min + t * (max - min);
            if (new - *value).abs() > f32::EPSILON {
                *value = new;
                changed = true;
            }
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }

        let (min, max) = (*range.start(), *range.end());
        let t = if (max - min).abs() < f32::EPSILON {
            0.0
        } else {
            ((*value - min) / (max - min)).clamp(0.0, 1.0)
        };

        let track = rect;
        let radius = self.s(theme::BTN_RADIUS);
        let pad = self.s(2.0);
        let thumb_w = self.s(12.0);
        let thumb_h = height - pad * 2.0;
        let travel = (rect.width() - thumb_w).max(0.0);

        self.round_rect(track, radius, theme::SLIDER_TRACK);
        if t > 0.0 {
            let fill = Rect {
                min: track.min,
                max: Vec2::new(track.min.x + travel * t + thumb_w * 0.5, track.max.y),
            };
            self.round_rect(fill, radius, theme::SLIDER_FILL);
        }

        let thumb = Rect::from_min_size(
            Vec2::new(rect.min.x + travel * t, rect.min.y + pad),
            Vec2::new(thumb_w, thumb_h),
        );
        let thumb_color = if active || hovered {
            theme::SLIDER_THUMB_HOT
        } else {
            theme::SLIDER_THUMB
        };
        self.round_rect(thumb, radius, thumb_color);

        let label = format!("{:.1}", *value);
        let tw = self.text_width(&label);
        let th = self.text_height();
        if tw + 4.0 < rect.width() * 0.45 {
            self.text(
                Vec2::new(rect.max.x - tw, rect.min.y + (height - th) * 0.5),
                &label,
                theme::TITLE_TEXT,
            );
        }

        Response {
            hovered,
            clicked: false,
            changed,
        }
    }
}
