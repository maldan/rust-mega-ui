use glam::Vec2;

use crate::Ui;
use crate::theme;

#[derive(Clone, Copy)]
pub struct TextStyle {
    pub color: [f32; 4],
    pub size: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: theme::TEXT,
            size: theme::FONT_SIZE,
        }
    }
}

impl Ui {
    pub fn label(&mut self, text: &str) {
        let style = TextStyle {
            color: if self.enabled() {
                theme::TEXT
            } else {
                theme::TEXT_DISABLED
            },
            size: theme::FONT_SIZE,
        };
        self.label_styled(text, style);
    }

    /// `style.size` is in UI points (scaled by `ui.scale()`).
    pub fn label_styled(&mut self, text: &str, style: TextStyle) {
        let px = self.s(style.size);
        let th = self.text_height_at(px);
        // Extra vertical pad only in vertical stacks — horizontal rows (buttons,
        // toolbars) need tight line-height so icons/text share a baseline band.
        let (h, y_off) = if matches!(self.layer().dir, crate::LayoutDir::Vertical) {
            (th + self.s(4.0), self.s(2.0))
        } else {
            (th, 0.0)
        };
        let size = Vec2::new(self.text_width_at(text, px), h);
        let rect = self.allocate(size);
        self.text_sized(rect.min + Vec2::new(0.0, y_off), text, style.color, px);
    }

    /// Left-aligned label in a **fixed** width (UI points). Short/long names share a column.
    pub fn label_fixed(&mut self, width: f32, text: &str) {
        let color = if self.enabled() {
            theme::TEXT
        } else {
            theme::TEXT_DISABLED
        };
        let px = self.s(theme::FONT_SIZE);
        let th = self.text_height_at(px);
        let (h, y_off) = if matches!(self.layer().dir, crate::LayoutDir::Vertical) {
            (th + self.s(4.0), self.s(2.0))
        } else {
            (th, 0.0)
        };
        let rect = self.allocate(Vec2::new(self.s(width.max(1.0)), h));
        self.push_clip(rect);
        self.text_sized(rect.min + Vec2::new(self.s(2.0), y_off), text, color, px);
        self.pop_clip();
    }
}
