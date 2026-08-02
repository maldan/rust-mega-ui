use glam::Vec2;

use crate::theme;
use crate::types::CursorIcon;
use crate::{LayoutDir, Ui};

impl Ui {
    pub fn tabs(&mut self, id: &str, titles: &[&str], add: impl FnOnce(&mut Self, usize)) {
        let widget_id = self.current_id(id);
        let mut selected = self.tabs.get(&widget_id).copied().unwrap_or(0);
        if !titles.is_empty() {
            selected = selected.min(titles.len() - 1);
        }

        let height = self.s(26.0);
        let radius = self.s(theme::BTN_RADIUS);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 {
            fill_w
        } else {
            self.s(200.0)
        };
        let bar = self.allocate(Vec2::new(width, height));
        self.round_rect(bar, radius, theme::BTN_BORDER);

        let n = titles.len().max(1) as f32;
        let tab_w = bar.width() / n;
        for (i, title) in titles.iter().enumerate() {
            let r = crate::Rect::from_min_size(
                Vec2::new(bar.min.x + i as f32 * tab_w, bar.min.y),
                Vec2::new(tab_w, height),
            );
            let hovered = self.hovered_rect(r);
            if hovered {
                self.want_capture = true;
                self.set_cursor(CursorIcon::Pointer);
            }
            if hovered && self.input.mouse_pressed {
                selected = i;
            }
            let color = if i == selected {
                theme::TAB_ACTIVE
            } else if hovered {
                theme::BTN_HOVER
            } else {
                theme::TAB
            };
            self.round_rect(r.inset(1.0), (radius - 1.0).max(0.0), color);
            let tw = self.text_width(title);
            let th = self.text_height();
            self.text(
                Vec2::new(r.min.x + (tab_w - tw) * 0.5, r.min.y + (height - th) * 0.5),
                title,
                theme::TEXT,
            );
        }

        self.tabs.insert(widget_id, selected);

        self.push_id(id);
        let avail = self.available_size();
        let origin = self.layer().cursor;
        self.layers.push(crate::new_layer(
            LayoutDir::Vertical,
            origin,
            self.spacing,
            avail.x,
            avail.y,
        ));
        add(self, selected);
        let used = self.layers.pop().unwrap().used;
        if used.x > 0.0 || used.y > 0.0 {
            self.allocate(used);
        }
        self.pop_id();
    }
}
