use glam::Vec2;

use crate::types::{CursorIcon, Rect};
use crate::{CrossAlign, LayoutDir, Ui};

impl Ui {
    pub fn tabs(&mut self, id: &str, titles: &[&str], add: impl FnOnce(&mut Self, usize)) {
        let widget_id = self.current_id(id);
        let mut selected = self.tabs.get(&widget_id).copied().unwrap_or(0);
        if !titles.is_empty() {
            selected = selected.min(titles.len() - 1);
        }

        let tab_h = self.s(self.theme.metrics.dock_tab_h);
        let tab_pad_x = self.s(10.0);
        let tab_gap = self.s(1.0);
        let radius = self.s(self.theme.metrics.dock_tab_radius);
        let pad = self.s(8.0);
        let avail = self.available_size();
        let width = if avail.x > 0.0 {
            avail.x
        } else {
            self.s(200.0)
        };

        // In a filled parent (dock leaf / window body) expand to remaining height.
        // Otherwise shrink-wrap to last frame's content measure.
        let content_id = widget_id.child("__content");
        // `child` does not go through `current_id`, so mark this map key live or
        // `gc_retained_state` drops the measure every frame and shrink-wrap
        // stays at the 48px default (clips plots inside nodes).
        self.keep(content_id);
        let prev = self
            .tab_content_sizes
            .get(&content_id)
            .copied()
            .unwrap_or(Vec2::new(width, self.s(48.0)));
        let total_h = if self.layer().fill_h > 0.0 && avail.y > 0.0 {
            avail.y
        } else {
            tab_h + (prev.y + pad * 2.0).max(self.s(32.0))
        };
        let outer = self.allocate(Vec2::new(width, total_h));

        // Outer frame.
        self.round_rect(outer, radius, self.theme.window.border);
        self.round_rect(outer.inset(1.0), (radius - 1.0).max(0.0), self.theme.window.body);

        let bar = Rect {
            min: outer.min + Vec2::splat(1.0),
            max: Vec2::new(outer.max.x - 1.0, outer.min.y + 1.0 + tab_h),
        };
        self.round_rect(bar, 0.0, self.theme.dock.tab_bar);

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
                self.theme.dock.tab_active
            } else if hovered {
                self.theme.dock.tab_hover
            } else {
                self.theme.dock.tab
            };
            self.round_rect_corners(tr, radius, color, true, false);

            let th = self.text_height();
            let text_col = if is_active {
                self.theme.dock.tab_text_active
            } else {
                self.theme.dock.tab_text
            };
            self.text(
                Vec2::new(tr.min.x + tab_pad_x, tr.min.y + (tr.height() - th) * 0.5),
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
            CrossAlign::Start,
        ));
        add(self, selected);
        let used = self.layers.pop().unwrap().used;
        let measured = Vec2::new(used.x.max(inner_w * 0.5), used.y.max(self.s(24.0)));
        if (measured.y - prev.y).abs() > 1.0 {
            self.request_repaint();
        }
        self.tab_content_sizes.insert(content_id, measured);
        self.pop_id();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use crate::types::UiInput;
    use crate::{LayoutDir, Ui, new_layer};

    fn tab_h(ui: &mut Ui) -> f32 {
        ui.layers.push(new_layer(
            LayoutDir::Vertical,
            Vec2::ZERO,
            ui.spacing,
            280.0,
            0.0,
            CrossAlign::Start,
        ));
        ui.tabs("env", &["Volume", "Pitch"], |ui, _| {
            let _ = ui.area("plot", Vec2::new(260.0, 120.0));
        });
        ui.layers.pop().unwrap().used.y
    }

    #[test]
    fn shrink_wrap_tabs_keep_content_height_across_frames() {
        let mut ui = Ui::new();
        let mut input = UiInput::default();
        input.viewport = Vec2::new(800.0, 600.0);

        ui.begin_frame(input.clone());
        let first = tab_h(&mut ui);
        ui.end_frame();

        ui.begin_frame(input.clone());
        let second = tab_h(&mut ui);
        ui.end_frame();

        ui.begin_frame(input);
        let third = tab_h(&mut ui);
        ui.end_frame();

        assert!(
            second > first + 40.0,
            "second frame should grow from measured plot, first={first} second={second}"
        );
        assert!(
            (third - second).abs() < 2.0,
            "height must persist after GC, second={second} third={third}"
        );
    }
}
