use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Response};
use crate::{LayoutDir, Ui};

impl Ui {
    pub fn button(&mut self, label: &str) -> Response {
        let enabled = self.enabled();
        let id = self.current_id(label);
        let pad_x = self.s(14.0);
        let text_w = self.text_width(label);
        let text_h = self.text_height();
        let height = self.s(28.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            (text_w + pad_x * 2.0).max(height)
        };
        let size = Vec2::new(width, height);
        let rect = self.allocate(size);

        let hovered = enabled && self.hovered_rect(rect);
        if hovered {
            self.hover_id = Some(id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }

        if hovered && self.input.mouse_pressed {
            self.active_id = Some(id);
        }

        let active = enabled && self.active_id == Some(id);
        let pressed = active && self.input.mouse_down;
        let clicked = active && hovered && self.input.mouse_released;

        let color = if !enabled {
            theme::BTN_DISABLED
        } else if pressed {
            theme::BTN_PRESS
        } else if hovered {
            theme::BTN_HOVER
        } else {
            theme::BTN
        };

        let draw_rect = if pressed {
            crate::Rect {
                min: rect.min + Vec2::new(0.0, 1.0),
                max: rect.max + Vec2::new(0.0, 1.0),
            }
        } else {
            rect
        };

        let radius = self.s(theme::BTN_RADIUS);
        self.round_rect(draw_rect, radius, theme::BTN_BORDER);
        self.round_rect(draw_rect.inset(1.0), (radius - 1.0).max(0.0), color);

        let text_color = if enabled {
            theme::TEXT
        } else {
            theme::TEXT_DISABLED
        };
        let text_pos = Vec2::new(
            draw_rect.min.x + (draw_rect.width() - text_w) * 0.5,
            draw_rect.min.y + (draw_rect.height() - text_h) * 0.5,
        );
        self.text(text_pos, label, text_color);

        Response {
            hovered,
            clicked,
            changed: false,
        }
    }
}
