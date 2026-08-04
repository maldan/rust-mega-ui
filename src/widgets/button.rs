use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect, Response};
use crate::{new_layer, CrossAlign, LayoutDir, Ui};

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

        let (hovered, clicked, draw_rect, color) = self.button_interact(id, rect, enabled);

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

    /// Button with arbitrary content (icons, labels, …).
    ///
    /// Size is measured from the previous frame (like menu popups).
    ///
    /// ```ignore
    /// ui.button_with("save", |ui| {
    ///     ui.horizontal(|ui| {
    ///         ui.icon("file", 14.0);
    ///         ui.label("Save");
    ///     });
    /// });
    /// ```
    pub fn button_with(&mut self, id: &str, add: impl FnOnce(&mut Self)) -> Response {
        let enabled = self.enabled();
        let widget_id = self.current_id(id);
        let pad_x = self.s(10.0);
        let pad_y = self.s(5.0);
        let min_h = self.s(28.0);

        let prev = self
            .button_sizes
            .get(&widget_id)
            .copied()
            .unwrap_or(Vec2::new(self.s(40.0), self.s(16.0)));

        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            (prev.x + pad_x * 2.0).max(min_h)
        };
        // Same fixed chrome height as `button`; grow only if content is taller.
        let height = (prev.y + pad_y * 2.0).max(min_h);
        let rect = self.allocate(Vec2::new(width, height));

        let (hovered, clicked, draw_rect, color) = self.button_interact(widget_id, rect, enabled);

        let radius = self.s(theme::BTN_RADIUS);
        self.round_rect(draw_rect, radius, theme::BTN_BORDER);
        self.round_rect(draw_rect.inset(1.0), (radius - 1.0).max(0.0), color);

        let inner = Rect {
            min: draw_rect.min + Vec2::new(pad_x, pad_y),
            max: draw_rect.max - Vec2::new(pad_x, pad_y),
        };
        // Horizontally center the row; each child is cross-centered in `inner`.
        let origin = Vec2::new(
            inner.min.x + (inner.width() - prev.x).max(0.0) * 0.5,
            inner.min.y,
        );

        self.push_id(id);
        let mut layer = new_layer(
            LayoutDir::Horizontal,
            origin,
            self.s(6.0),
            inner.width().max(prev.x),
            inner.height(),
        );
        layer.cross_align = CrossAlign::Center;
        self.layers.push(layer);
        add(self);
        let used = self.layers.pop().unwrap().used;
        self.button_sizes.insert(
            widget_id,
            Vec2::new(used.x.max(1.0), used.y.max(1.0)),
        );
        self.pop_id();

        Response {
            hovered,
            clicked,
            changed: false,
        }
    }

    fn button_interact(
        &mut self,
        id: crate::types::Id,
        rect: Rect,
        enabled: bool,
    ) -> (bool, bool, Rect, [f32; 4]) {
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
            Rect {
                min: rect.min + Vec2::new(0.0, 1.0),
                max: rect.max + Vec2::new(0.0, 1.0),
            }
        } else {
            rect
        };

        (hovered, clicked, draw_rect, color)
    }
}
