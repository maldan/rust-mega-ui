use glam::Vec2;

use crate::Ui;
use crate::types::{CursorIcon, Rect, Response};

impl Ui {
    pub fn checkbox(&mut self, id: &str, label: &str, checked: &mut bool) -> Response {
        let id = self.current_id(id);
        let box_s: f32 = self.s(16.0);
        let gap = self.s(8.0);
        let text_w = self.text_width(label);
        let text_h = self.text_height();
        let height = box_s.max(text_h + self.s(4.0));
        let width = box_s + gap + text_w;
        let rect = self.allocate(Vec2::new(width, height));

        let box_rect = Rect::from_min_size(
            Vec2::new(rect.min.x, rect.min.y + (height - box_s) * 0.5),
            Vec2::splat(box_s),
        );
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
        let clicked = active && hovered && self.input.mouse_released;
        let mut changed = false;
        if clicked {
            *checked = !*checked;
            changed = true;
        }

        self.round_rect(box_rect, self.s(3.0), self.theme.check.border);
        let fill = if *checked {
            self.theme.check.on
        } else if hovered {
            self.theme.button.hover
        } else {
            self.theme.check.bg
        };
        self.round_rect(box_rect.inset(1.0), self.s(2.0), fill);
        if *checked {
            let mark = box_rect.inset(self.s(2.0));
            self.draw_icon_at("check", mark, self.theme.check.mark, false);
        }

        self.text(
            Vec2::new(box_rect.max.x + gap, rect.min.y + (height - text_h) * 0.5),
            label,
            self.theme.text.primary,
        );

        Response {
            hovered,
            clicked,
            changed,
        }
    }
}
