use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Response};
use crate::widgets::edit::{
    byte_at_x, clamp_edit, draw_sel_line, handle_clipboard, handle_typing, has_sel, move_end,
    move_home, move_left, move_right, sel_range, EditState,
};
use crate::{LayoutDir, Ui};

impl Ui {
    pub fn text_input(&mut self, id: &str, text: &mut String) -> Response {
        let enabled = self.enabled();
        let widget_id = self.current_id(id);
        let height = self.s(28.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            self.s(180.0)
        };
        let rect = self.allocate(Vec2::new(width, height));

        let hovered = enabled && self.hovered_rect(rect);
        if hovered {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Text);
        }
        if hovered && self.input.mouse_pressed {
            self.focus_id = Some(widget_id);
            self.active_id = Some(widget_id);
        }

        let focused = enabled && self.focus_id == Some(widget_id);
        let mut st = self
            .edits
            .get(&widget_id)
            .copied()
            .unwrap_or(EditState {
                caret: text.len(),
                anchor: text.len(),
            });
        clamp_edit(&mut st, text.len());

        let mut changed = false;
        let pad = self.s(8.0);
        let shift = self.input.key_shift;

        if focused {
            self.want_capture = true;
            self.set_cursor(CursorIcon::Text);

            changed |= handle_clipboard(self, text, &mut st);
            clamp_edit(&mut st, text.len());

            if self.input.key_left {
                move_left(text, &mut st, shift);
            }
            if self.input.key_right {
                move_right(text, &mut st, shift);
            }
            if self.input.key_home {
                move_home(&mut st, 0, shift);
            }
            if self.input.key_end {
                move_end(&mut st, text.len(), shift);
            }

            changed |= handle_typing(self, text, &mut st);
            clamp_edit(&mut st, text.len());

            if hovered && self.input.mouse_pressed {
                let local = (self.input.mouse_pos.x - rect.min.x - pad).max(0.0);
                let at = byte_at_x(self, text, local);
                st.caret = at;
                if !shift {
                    st.anchor = at;
                }
            }
            if self.active_id == Some(widget_id) && self.input.mouse_down {
                let local = (self.input.mouse_pos.x - rect.min.x - pad).max(0.0);
                st.caret = byte_at_x(self, text, local);
            }
        }

        self.edits.insert(widget_id, st);

        let border = if focused {
            theme::INPUT_BORDER_FOCUS
        } else {
            theme::INPUT_BORDER
        };
        let bg = if enabled {
            theme::INPUT_BG
        } else {
            theme::INPUT_BG_DISABLED
        };
        self.round_rect(rect, self.s(theme::BTN_RADIUS), border);
        self.round_rect(rect.inset(1.0), self.s(theme::BTN_RADIUS) - 1.0, bg);

        let th = self.text_height();
        let inner_w = width - pad * 2.0;
        let (sel_a, sel_b) = sel_range(st);

        let mut view_start = 0usize;
        if self.text_width(&text[..st.caret]) > inner_w {
            for (i, _) in text.char_indices() {
                if i > st.caret {
                    break;
                }
                if self.text_width(&text[i..st.caret]) <= inner_w * 0.85 {
                    view_start = i;
                    break;
                }
            }
        }

        let mut end = text.len();
        for (i, _) in text[view_start..].char_indices() {
            let abs = view_start + i;
            if self.text_width(&text[view_start..abs]) > inner_w {
                end = abs;
                break;
            }
        }
        let draw = &text[view_start..end];
        let text_pos = Vec2::new(rect.min.x + pad, rect.min.y + (height - th) * 0.5);
        let text_color = if enabled {
            theme::TEXT
        } else {
            theme::TEXT_DISABLED
        };

        if has_sel(st) {
            let a = sel_a.max(view_start).min(end);
            let b = sel_b.max(view_start).min(end);
            if a < b {
                let x0 = text_pos.x + self.text_width(&text[view_start..a]);
                let x1 = text_pos.x + self.text_width(&text[view_start..b]);
                draw_sel_line(self, x0, x1, rect.min.y + self.s(4.0), height - self.s(8.0));
            }
        }

        self.text(text_pos, draw, text_color);

        if focused {
            let caret_x = text_pos.x + self.text_width(&text[view_start..st.caret]) + 1.0;
            self.round_rect(
                crate::Rect::from_min_size(
                    Vec2::new(caret_x, rect.min.y + self.s(6.0)),
                    Vec2::new(1.0, height - self.s(12.0)),
                ),
                0.0,
                theme::TEXT,
            );
        }

        Response {
            hovered,
            clicked: false,
            changed,
        }
    }
}
