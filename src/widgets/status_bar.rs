//! Bottom status bar.

use glam::Vec2;

use crate::theme;
use crate::types::Rect;
use crate::{CrossAlign, new_layer, LayoutDir, Ui};

impl Ui {
    /// Full-width status bar pinned to the bottom of the viewport.
    /// Drawn on the overlay layer. Call near the end of the frame.
    pub fn status_bar(&mut self, add: impl FnOnce(&mut Self)) {
        let h = self.s(theme::STATUS_BAR_H);
        let w = self.input.viewport.x.max(1.0);
        let y = (self.input.viewport.y - h).max(0.0);
        let bar = Rect::from_min_size(Vec2::new(0.0, y), Vec2::new(w, h));

        let prev = self.draw_to_overlay;
        self.draw_to_overlay = true;

        self.round_rect(bar, 0.0, theme::STATUS_BAR_BG);
        let line = Rect {
            min: bar.min,
            max: Vec2::new(bar.max.x, bar.min.y + 1.0),
        };
        self.round_rect(line, 0.0, theme::WIN_BORDER);

        let pad = self.s(8.0);
        let origin = Vec2::new(bar.min.x + pad, bar.min.y);
        let spacing = self.s(10.0);
        self.layers.push(new_layer(
            LayoutDir::Horizontal,
            origin,
            spacing,
            (w - pad * 2.0).max(0.0),
            h,
            CrossAlign::Start,
        ));
        self.layer().cursor.y = bar.min.y + (h - self.text_height()) * 0.5;
        add(self);
        self.layers.pop();

        self.draw_to_overlay = prev;
    }
}
