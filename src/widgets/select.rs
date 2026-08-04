use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect, Response};
use crate::{LayoutDir, Ui};

impl Ui {
    pub fn select(&mut self, id: &str, selected: &mut usize, options: &[&str]) -> Response {
        let widget_id = self.current_id(id);
        let mut open = self.selects.get(&widget_id).copied().unwrap_or(false);

        let height = self.s(28.0);
        let item_h = self.s(26.0);
        let radius = self.s(theme::BTN_RADIUS);
        let pad = self.s(10.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            self.s(180.0)
        };

        let header = self.allocate(Vec2::new(width, height));
        let header_hovered = self.hovered_rect(header);

        if header_hovered {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        if header_hovered && self.input.mouse_pressed {
            self.active_id = Some(widget_id);
        }

        let mut changed = false;
        let mut list_hovered = false;

        let header_clicked =
            self.active_id == Some(widget_id) && header_hovered && self.input.mouse_released;
        if header_clicked {
            open = !open;
        }

        let list = if open && !options.is_empty() {
            let list_h = item_h * options.len() as f32;
            Some(Rect::from_min_size(
                Vec2::new(header.min.x, header.max.y + self.s(2.0)),
                Vec2::new(width, list_h),
            ))
        } else {
            None
        };

        if let Some(list) = list {
            list_hovered = self.hovered_overlay(list);
            for (i, _) in options.iter().enumerate() {
                let item = Rect::from_min_size(
                    Vec2::new(list.min.x + 1.0, list.min.y + 1.0 + i as f32 * item_h),
                    Vec2::new(width - 2.0, item_h),
                );
                if self.hovered_overlay(item) && self.input.mouse_released {
                    if *selected != i {
                        *selected = i;
                        changed = true;
                    }
                    open = false;
                }
            }
            if open && self.input.mouse_pressed && !header_hovered && !list_hovered {
                open = false;
            }
        }

        if changed && header_clicked {
            open = false;
        }

        let label = options.get(*selected).copied().unwrap_or("");
        let color = if header_hovered {
            theme::BTN_HOVER
        } else {
            theme::BTN
        };
        self.round_rect(header, radius, theme::BTN_BORDER);
        self.round_rect(header.inset(1.0), (radius - 1.0).max(0.0), color);

        let th = self.text_height();
        self.text(
            Vec2::new(header.min.x + pad, header.min.y + (height - th) * 0.5),
            label,
            theme::TEXT,
        );
        let arrow_s = self.s(12.0);
        let arrow_rect = Rect::from_min_size(
            Vec2::new(
                header.max.x - self.s(18.0),
                header.min.y + (height - arrow_s) * 0.5,
            ),
            Vec2::splat(arrow_s),
        );
        let arrow = if open { "chevron_up" } else { "chevron_down" };
        self.draw_icon_at(arrow, arrow_rect, theme::TEXT_DIM, false);

        if open {
            if let Some(list) = list {
                self.mouse_absorb = Some(list);
                if list_hovered {
                    self.want_capture = true;
                    self.set_cursor(CursorIcon::Pointer);
                }

                self.round_rect_overlay(list, radius, theme::BTN_BORDER);
                self.round_rect_overlay(list.inset(1.0), (radius - 1.0).max(0.0), theme::POPUP_BG);

                for (i, opt) in options.iter().enumerate() {
                    let item = Rect::from_min_size(
                        Vec2::new(list.min.x + 1.0, list.min.y + 1.0 + i as f32 * item_h),
                        Vec2::new(width - 2.0, item_h),
                    );
                    let hot = self.hovered_overlay(item);
                    if hot {
                        self.want_capture = true;
                        self.set_cursor(CursorIcon::Pointer);
                        self.round_rect_overlay(item, self.s(3.0), theme::POPUP_HOVER);
                    } else if i == *selected {
                        self.round_rect_overlay(item, self.s(3.0), theme::HEADER);
                    }

                    self.text_overlay(
                        Vec2::new(item.min.x + pad, item.min.y + (item_h - th) * 0.5),
                        opt,
                        theme::TEXT,
                    );
                }
            }
        }

        self.selects.insert(widget_id, open);

        Response {
            hovered: header_hovered || list_hovered,
            clicked: header_clicked,
            changed,
        }
    }
}
