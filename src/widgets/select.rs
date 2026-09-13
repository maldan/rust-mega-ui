use glam::Vec2;

use crate::types::{CursorIcon, Rect, Response};
use crate::{LayoutDir, Ui};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SelectState {
    pub open: bool,
    pub scroll: f32,
}

impl Ui {
    pub fn select(&mut self, id: &str, selected: &mut usize, options: &[&str]) -> Response {
        let widget_id = self.current_id(id);
        let mut st = self.selects.get(&widget_id).copied().unwrap_or_default();

        let height = self.s(28.0);
        let item_h = self.s(26.0);
        let radius = self.s(self.theme.metrics.button_radius);
        let pad = self.s(10.0);
        let fill_w = self.layer().fill_w;
        let filling = fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical);
        let width = if filling { fill_w } else { self.s(180.0) };

        let header = if filling {
            self.allocate_fill_x(Vec2::new(width, height))
        } else {
            self.allocate(Vec2::new(width, height))
        };
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
            st.open = !st.open;
            if st.open {
                st.scroll = 0.0;
            }
        }

        let list = if st.open && !options.is_empty() {
            Some(Self::select_list_rect(
                header,
                width,
                item_h,
                options.len(),
                self.input.viewport,
                self.s(2.0),
            ))
        } else {
            None
        };

        if let Some(list) = list {
            list_hovered = self.hovered_overlay(list);
            let content_h = item_h * options.len() as f32;
            let scroll_max = (content_h - list.height()).max(0.0);
            if list_hovered && self.input.scroll_delta.y.abs() > 0.0 {
                st.scroll = (st.scroll - self.input.scroll_delta.y).clamp(0.0, scroll_max);
                self.consume_scroll();
                self.want_capture = true;
                self.request_repaint();
            }
            st.scroll = st.scroll.clamp(0.0, scroll_max);

            for (i, _) in options.iter().enumerate() {
                let item = Self::select_item_rect(list, width, item_h, i, st.scroll);
                if !rects_overlap(item, list) {
                    continue;
                }
                if self.hovered_overlay(item) && self.input.mouse_released {
                    if *selected != i {
                        *selected = i;
                        changed = true;
                    }
                    st.open = false;
                }
            }
            if st.open && self.input.mouse_pressed && !header_hovered && !list_hovered {
                st.open = false;
            }
        }

        if changed && header_clicked {
            st.open = false;
        }

        let label = options.get(*selected).copied().unwrap_or("");
        let color = if header_hovered {
            self.theme.button.hover
        } else {
            self.theme.button.bg
        };
        self.round_rect(header, radius, self.theme.button.border);
        self.round_rect(header.inset(1.0), (radius - 1.0).max(0.0), color);

        let th = self.text_height();
        self.text(
            Vec2::new(header.min.x + pad, header.min.y + (height - th) * 0.5),
            label,
            self.theme.text.primary,
        );
        let arrow_s = self.s(12.0);
        let arrow_rect = Rect::from_min_size(
            Vec2::new(
                header.max.x - self.s(18.0),
                header.min.y + (height - arrow_s) * 0.5,
            ),
            Vec2::splat(arrow_s),
        );
        let arrow = if st.open {
            "chevron_up"
        } else {
            "chevron_down"
        };
        self.draw_icon_at(arrow, arrow_rect, self.theme.text.dim, false);

        if st.open
            && let Some(list) = list
        {
            self.absorb_rect(list);
            if list_hovered {
                self.want_capture = true;
                self.set_cursor(CursorIcon::Pointer);
            }

            self.round_rect_overlay(list, radius, self.theme.button.border);
            self.round_rect_overlay(list.inset(1.0), (radius - 1.0).max(0.0), self.theme.popup.bg);

            for (i, opt) in options.iter().enumerate() {
                let item = Self::select_item_rect(list, width, item_h, i, st.scroll);
                if !rects_overlap(item, list) {
                    continue;
                }
                let hot = self.hovered_overlay(item);
                if hot {
                    self.want_capture = true;
                    self.set_cursor(CursorIcon::Pointer);
                    self.round_rect_overlay(item, self.s(3.0), self.theme.popup.hover);
                } else if i == *selected {
                    self.round_rect_overlay(item, self.s(3.0), self.theme.header.bg);
                }

                self.text_overlay(
                    Vec2::new(item.min.x + pad, item.min.y + (item_h - th) * 0.5),
                    opt,
                    self.theme.text.primary,
                );
            }

            // Thin scroll thumb when content overflows.
            let content_h = item_h * options.len() as f32;
            if content_h > list.height() + 0.5 {
                let track = Rect {
                    min: Vec2::new(list.max.x - self.s(5.0), list.min.y + self.s(3.0)),
                    max: Vec2::new(list.max.x - self.s(2.0), list.max.y - self.s(3.0)),
                };
                let thumb_h = (track.height() * (list.height() / content_h))
                    .clamp(self.s(12.0), track.height());
                let t = if content_h > list.height() {
                    st.scroll / (content_h - list.height())
                } else {
                    0.0
                };
                let thumb_y = track.min.y + (track.height() - thumb_h) * t;
                let thumb = Rect::from_min_size(
                    Vec2::new(track.min.x, thumb_y),
                    Vec2::new(track.width(), thumb_h),
                );
                self.round_rect_overlay(thumb, self.s(2.0), [0.45, 0.45, 0.48, 0.85]);
            }
        }

        self.selects.insert(widget_id, st);

        Response {
            hovered: header_hovered || list_hovered,
            clicked: header_clicked,
            changed,
        }
    }

    fn select_list_rect(
        header: Rect,
        width: f32,
        item_h: f32,
        count: usize,
        viewport: Vec2,
        gap: f32,
    ) -> Rect {
        const MAX_VISIBLE: f32 = 10.0;
        let full_h = item_h * count as f32;
        let capped = full_h.min(item_h * MAX_VISIBLE);
        let space_below = (viewport.y - header.max.y - gap).max(0.0);
        let space_above = (header.min.y - gap).max(0.0);

        let open_down = capped <= space_below || space_below >= space_above;
        let list_h = if open_down {
            capped
                .min(space_below)
                .max(item_h.min(space_below.max(item_h)))
        } else {
            capped
                .min(space_above)
                .max(item_h.min(space_above.max(item_h)))
        };

        if open_down {
            Rect::from_min_size(
                Vec2::new(header.min.x, header.max.y + gap),
                Vec2::new(width, list_h),
            )
        } else {
            Rect::from_min_size(
                Vec2::new(header.min.x, header.min.y - gap - list_h),
                Vec2::new(width, list_h),
            )
        }
    }

    fn select_item_rect(list: Rect, width: f32, item_h: f32, index: usize, scroll: f32) -> Rect {
        Rect::from_min_size(
            Vec2::new(
                list.min.x + 1.0,
                list.min.y + 1.0 + index as f32 * item_h - scroll,
            ),
            Vec2::new(width - 2.0, item_h),
        )
    }
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
}
