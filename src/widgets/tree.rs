use glam::Vec2;

use crate::theme;
use crate::types::CursorIcon;
use crate::{LayoutDir, Ui};

impl Ui {
    pub fn tree_node(&mut self, id: &str, label: &str, add: impl FnOnce(&mut Self)) -> bool {
        let widget_id = self.current_id(id);
        let mut open = self.trees.get(&widget_id).copied().unwrap_or(false);

        let height = self.s(22.0);
        let indent = self.s(14.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            self.s(180.0)
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
        let clicked =
            self.active_id == Some(widget_id) && hovered && self.input.mouse_released;
        if clicked {
            open = !open;
        }
        self.trees.insert(widget_id, open);

        if hovered {
            self.round_rect(rect, self.s(3.0), theme::HEADER_HOVER);
        }

        let arrow = if open { "v" } else { ">" };
        let th = self.text_height();
        self.text(
            Vec2::new(rect.min.x + self.s(4.0), rect.min.y + (height - th) * 0.5),
            arrow,
            theme::TEXT,
        );
        self.text(
            Vec2::new(rect.min.x + self.s(18.0), rect.min.y + (height - th) * 0.5),
            label,
            theme::TEXT,
        );

        if open {
            self.push_id(id);
            let origin = self.layer().cursor + Vec2::new(indent, 0.0);
            let fill = (self.layer().fill_w - indent).max(0.0);
            let fill_h = self.available_size().y;
            let spacing = self.spacing;
            self.layers.push(crate::new_layer(
                LayoutDir::Vertical,
                origin,
                spacing,
                fill,
                fill_h,
            ));
            add(self);
            let used = self.layers.pop().unwrap().used;
            if used.x > 0.0 || used.y > 0.0 {
                self.allocate(Vec2::new(used.x + indent, used.y));
            }
            self.pop_id();
        }
        open
    }
}
