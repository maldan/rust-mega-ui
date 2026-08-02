use glam::Vec2;

use crate::theme;
use crate::Ui;

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
        let size = Vec2::new(self.text_width_at(text, px), th + self.s(4.0));
        let rect = self.allocate(size);
        self.text_sized(
            rect.min + Vec2::new(0.0, self.s(2.0)),
            text,
            style.color,
            px,
        );
    }
}
