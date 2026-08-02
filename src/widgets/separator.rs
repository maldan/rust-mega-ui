use glam::Vec2;

use crate::theme;
use crate::types::Rect;
use crate::{LayoutDir, Ui};

impl Ui {
    pub fn separator(&mut self) {
        if !self.menu_stack.is_empty() {
            self.menu_separator();
            return;
        }
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            self.s(100.0)
        };
        let height = self.s(9.0);
        let rect = self.allocate(Vec2::new(width, height));
        let y = rect.min.y + height * 0.5;
        let line = Rect {
            min: Vec2::new(rect.min.x, y),
            max: Vec2::new(rect.max.x, y + 1.0),
        };
        self.round_rect(line, 0.0, theme::WIN_BORDER);
    }
}
