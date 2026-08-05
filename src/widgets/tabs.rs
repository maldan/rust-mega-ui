use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect};
use crate::{LayoutDir, Ui};

impl Ui {
    pub fn tabs(&mut self, id: &str, titles: &[&str], add: impl FnOnce(&mut Self, usize)) {
        let widget_id = self.current_id(id);
        let mut selected = self.tabs.get(&widget_id).copied().unwrap_or(0);
        if !titles.is_empty() {
            selected = selected.min(titles.len() - 1);
        }

        let tab_h = self.s(theme::DOCK_TAB_H);
        let tab_pad_x = self.s(10.0);
        let tab_gap = self.s(1.0);
        let radius = self.s(theme::DOCK_TAB_RADIUS);
        let pad = self.s(8.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 {
            fill_w
        } else {
            self.s(200.0)
        };

        // One chrome: tab strip + content, no layout gap between them.
        let content_id = widget_id.child("__content");
        let prev = self
            .tab_content_sizes
            .get(&content_id)
            .copied()
            .unwrap_or(Vec2::new(width, self.s(48.0)));
        let content_h = (prev.y + pad * 2.0).max(self.s(32.0));
        let outer = self.allocate(Vec2::new(width, tab_h + content_h));

        // Outer frame.
        self.round_rect(outer, radius, theme::WIN_BORDER);
        self.round_rect(
            outer.inset(1.0),
            (radius - 1.0).max(0.0),
            theme::WIN_BODY,
        );

        let bar = Rect {
            min: outer.min + Vec2::splat(1.0),
            max: Vec2::new(outer.max.x - 1.0, outer.min.y + 1.0 + tab_h),
        };
        self.round_rect(bar, 0.0, theme::DOCK_TAB_BAR);

        let mut x = bar.min.x + self.s(2.0);
        for (i, title) in titles.iter().enumerate() {
            let tw = self.text_width(title);
            let tab_w = (tw + tab_pad_x * 2.0).max(self.s(36.0));
            let is_active = i == selected;
            // Active tab flush into content (square bottom); inactive only top-rounded.
            let tr = Rect {
                min: Vec2::new(
                    x,
                    if is_active {
                        bar.min.y
                    } else {
                        bar.min.y + self.s(2.0)
                    },
                ),
                max: Vec2::new(
                    x + tab_w,
                    if is_active {
                        bar.max.y + 1.0
                    } else {
                        bar.max.y
                    },
                ),
            };

            let hovered = self.hovered_rect(tr);
            if hovered {
                self.want_capture = true;
                self.set_cursor(CursorIcon::Pointer);
            }
            if hovered && self.input.mouse_pressed {
                selected = i;
            }

            let color = if is_active {
                theme::DOCK_TAB_ACTIVE
            } else if hovered {
                theme::DOCK_TAB_HOVER
            } else {
                theme::DOCK_TAB
            };
            self.round_rect_corners(tr, radius, color, true, false);

            let th = self.text_height();
            let text_col = if is_active {
                theme::DOCK_TAB_TEXT_ACTIVE
            } else {
                theme::DOCK_TAB_TEXT
            };
            self.text(
                Vec2::new(
                    tr.min.x + tab_pad_x,
                    tr.min.y + (tr.height() - th) * 0.5,
                ),
                title,
                text_col,
            );

            x = tr.max.x + tab_gap;
        }

        self.tabs.insert(widget_id, selected);

        // Content sits directly under the bar (same fill as active tab).
        let content = Rect {
            min: Vec2::new(outer.min.x + 1.0, bar.max.y),
            max: outer.max - Vec2::splat(1.0),
        };

        self.push_id(id);
        let origin = content.min + Vec2::splat(pad);
        let inner_w = (content.width() - pad * 2.0).max(0.0);
        let inner_h = (content.height() - pad * 2.0).max(0.0);
        self.layers.push(crate::new_layer(
            LayoutDir::Vertical,
            origin,
            self.spacing,
            inner_w,
            inner_h,
        ));
        add(self, selected);
        let used = self.layers.pop().unwrap().used;
        self.tab_content_sizes.insert(
            content_id,
            Vec2::new(used.x.max(inner_w * 0.5), used.y.max(self.s(24.0))),
        );
        self.pop_id();
    }
}
