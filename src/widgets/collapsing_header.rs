use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect};
use crate::{LayoutDir, Ui};

impl Ui {
    pub fn collapsing_header(&mut self, label: &str, add: impl FnOnce(&mut Self)) {
        let id = self.current_id(label);
        let mut open = self.headers.get(&id).copied().unwrap_or(false);

        let height = self.s(26.0);
        let radius = self.s(theme::BTN_RADIUS);
        let indent = self.s(8.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            self.s(200.0)
        };
        let rect = self.allocate(Vec2::new(width, height));

        let hovered = self.hovered_rect(rect);
        if hovered {
            self.hover_id = Some(id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        if hovered && self.input.mouse_pressed {
            self.active_id = Some(id);
        }
        let active = self.active_id == Some(id);
        let pressed = active && self.input.mouse_down;
        let clicked = active && hovered && self.input.mouse_released;
        if clicked {
            open = !open;
        }
        self.headers.insert(id, open);

        let color = if pressed {
            theme::HEADER_PRESS
        } else if hovered {
            theme::HEADER_HOVER
        } else {
            theme::HEADER
        };
        self.round_rect(rect, radius, theme::BTN_BORDER);
        self.round_rect(rect.inset(1.0), (radius - 1.0).max(0.0), color);

        let arrow_s = self.s(12.0);
        let arrow_rect = Rect::from_min_size(
            Vec2::new(
                rect.min.x + indent,
                rect.min.y + (height - arrow_s) * 0.5,
            ),
            Vec2::splat(arrow_s),
        );
        let arrow = if open { "chevron_down" } else { "chevron_right" };
        self.draw_icon_at(arrow, arrow_rect, theme::TEXT_DIM, false);

        let th = self.text_height();
        self.text(
            Vec2::new(rect.min.x + self.s(24.0), rect.min.y + (height - th) * 0.5),
            label,
            theme::TEXT,
        );

        if open {
            self.push_id(label);
            let origin = self.layer().cursor + Vec2::new(indent, 0.0);
            let fill_w = (self.layer().fill_w - indent).max(0.0);
            let fill_h = self.available_size().y;
            let spacing = self.spacing;
            self.layers.push(crate::new_layer(
                LayoutDir::Vertical,
                origin,
                spacing,
                fill_w,
                fill_h,
            ));
            add(self);
            let used = self.layers.pop().unwrap().used;
            if used.x > 0.0 || used.y > 0.0 {
                self.allocate(Vec2::new(used.x + indent, used.y));
            }
            self.pop_id();
        }
    }
}
