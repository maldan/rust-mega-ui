use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect, Response};
use crate::widgets::edit::{
    byte_at_x, clamp_edit, draw_sel_line, handle_clipboard, has_sel, insert_str, move_end,
    move_home, move_left, move_right, sel_range, EditState,
};
use crate::{LayoutDir, Ui};

pub(crate) fn format_float(v: f32, step: f32) -> String {
    let v = snap_to_step(v, step);
    let d = step_decimals(step);
    let s = format!("{v:.prec$}", prec = d);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() || s == "-" || s == "-0" {
        "0".into()
    } else {
        s
    }
}

fn step_decimals(step: f32) -> usize {
    if step <= 0.0 || step >= 1.0 {
        return 0;
    }
    (-(step as f64).log10()).round().clamp(0.0, 6.0) as usize
}

fn snap_to_step(v: f32, step: f32) -> f32 {
    if step <= 0.0 || !v.is_finite() {
        return v;
    }
    let v = v as f64;
    let step = step as f64;
    ((v / step).round() * step) as f32
}

fn filter_float(s: &str) -> String {
    s.chars()
        .filter(|c| matches!(c, '0'..='9' | '.' | '-' | '+' | 'e' | 'E'))
        .collect()
}

impl Ui {
    /// Numeric field: type floats, Up/Down = step, left grip = drag value.
    pub fn drag_float(&mut self, id: &str, value: &mut f32, step: f32) -> Response {
        self.drag_float_grip(id, value, step, None)
    }

    /// Like [`Self::drag_float`], with an optional colored drag grip.
    pub fn drag_float_grip(
        &mut self,
        id: &str,
        value: &mut f32,
        step: f32,
        grip_color: Option<[f32; 4]>,
    ) -> Response {
        let enabled = self.enabled();
        let widget_id = self.current_id(id);
        let grip_id = widget_id.child("__grip");
        let height = self.s(28.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else if matches!(self.layer().dir, LayoutDir::Horizontal) {
            self.s(72.0)
        } else {
            self.s(120.0)
        };
        let rect = self.allocate(Vec2::new(width, height));
        let grip_w = if grip_color.is_some() {
            self.s(8.0)
        } else {
            self.s(6.0)
        }
        .min(width * 0.25);
        let grip = Rect::from_min_size(rect.min, Vec2::new(grip_w, height));
        let text_rect = Rect {
            min: Vec2::new(rect.min.x + grip_w, rect.min.y),
            max: rect.max,
        };

        let grip_hov = enabled && self.hovered_rect(grip);
        let text_hov = enabled && self.hovered_rect(text_rect);
        let hovered = grip_hov || text_hov;
        let dragging = self.active_id == Some(grip_id);

        if grip_hov || dragging {
            self.hover_id = Some(grip_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::ResizeEw);
        } else if text_hov {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Text);
        }

        if enabled && grip_hov && self.input.mouse_pressed {
            self.active_id = Some(grip_id);
            self.focus_id = None;
            // x = last mouse, y = leftover pixels toward next step
            self.drag_grab = Some(Vec2::new(self.input.mouse_pos.x, 0.0));
            // commit any open edit buffer
            if let Some(buf) = self.num_bufs.remove(&widget_id) {
                if let Ok(v) = buf.parse::<f32>() {
                    *value = snap_to_step(v, step);
                }
            }
        }
        if enabled && text_hov && self.input.mouse_pressed {
            self.focus_id = Some(widget_id);
            self.active_id = Some(widget_id);
        }

        let mut changed = false;

        // mouse drag on grip — only whole steps
        if dragging && self.input.mouse_down {
            let grab = self.drag_grab.unwrap_or(self.input.mouse_pos);
            let dx = self.input.mouse_pos.x - grab.x;
            let mut acc = grab.y + dx;
            // pixels per one step; Shift = finer (more px per step)
            let sens = if self.input.key_shift { 40.0 } else { 8.0 };
            let steps = (acc / sens).trunc();
            if steps != 0.0 {
                *value = snap_to_step(*value + steps * step, step);
                acc -= steps * sens;
                changed = true;
                if let Some(buf) = self.num_bufs.get_mut(&widget_id) {
                    *buf = format_float(*value, step);
                }
            }
            self.drag_grab = Some(Vec2::new(self.input.mouse_pos.x, acc));
            self.want_capture = true;
            self.set_cursor(CursorIcon::ResizeEw);
            self.needs_repaint = true;
        }

        let focused = enabled && self.focus_id == Some(widget_id);

        if !focused && !dragging {
            if let Some(buf) = self.num_bufs.remove(&widget_id) {
                if let Ok(v) = buf.parse::<f32>() {
                    let v = snap_to_step(v, step);
                    if *value != v {
                        *value = v;
                        changed = true;
                    }
                }
            } else {
                // keep stored value on the step grid
                let v = snap_to_step(*value, step);
                if (*value - v).abs() > f32::EPSILON {
                    *value = v;
                }
            }
        } else if focused && !self.num_bufs.contains_key(&widget_id) {
            let s = format_float(*value, step);
            let len = s.len();
            self.num_bufs.insert(widget_id, s);
            self.edits.insert(
                widget_id,
                EditState {
                    caret: len,
                    anchor: len,
                },
            );
        }

        let display = if focused {
            self.num_bufs
                .get(&widget_id)
                .cloned()
                .unwrap_or_else(|| format_float(*value, step))
        } else {
            format_float(*value, step)
        };
        let mut text = display;

        let mut st = self.edits.get(&widget_id).copied().unwrap_or(EditState {
            caret: text.len(),
            anchor: text.len(),
        });
        clamp_edit(&mut st, text.len());

        let pad = self.s(6.0);
        let shift = self.input.key_shift;

        if focused && !dragging {
            self.want_capture = true;
            if !grip_hov {
                self.set_cursor(CursorIcon::Text);
            }

            let before = text.clone();
            changed |= handle_clipboard(self, &mut text, &mut st);
            if text != before {
                let filtered = filter_float(&text);
                if filtered != text {
                    text = filtered;
                    clamp_edit(&mut st, text.len());
                }
            }

            if self.input.key_left {
                move_left(&text, &mut st, shift);
            }
            if self.input.key_right {
                move_right(&text, &mut st, shift);
            }
            if self.input.key_home {
                move_home(&mut st, 0, shift);
            }
            if self.input.key_end {
                move_end(&mut st, text.len(), shift);
            }

            if self.input.key_up || self.input.key_down {
                let cur = snap_to_step(text.parse::<f32>().unwrap_or(*value), step);
                let next = snap_to_step(
                    if self.input.key_up {
                        cur + step
                    } else {
                        cur - step
                    },
                    step,
                );
                *value = next;
                text = format_float(next, step);
                st.caret = text.len();
                st.anchor = st.caret;
                changed = true;
            } else {
                if self.input.key_backspace {
                    use crate::widgets::edit::delete_sel;
                    if delete_sel(&mut text, &mut st) {
                        changed = true;
                    } else if st.caret > 0 {
                        let prev = text[..st.caret]
                            .char_indices()
                            .next_back()
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        text.replace_range(prev..st.caret, "");
                        st.caret = prev;
                        st.anchor = prev;
                        changed = true;
                    }
                }
                if !self.input.key_ctrl && !self.input.text.is_empty() {
                    let filtered = filter_float(&self.input.text);
                    if !filtered.is_empty() {
                        insert_str(&mut text, &mut st, &filtered);
                        changed = true;
                    }
                }
                if let Ok(v) = text.parse::<f32>() {
                    // while typing keep raw parse; snap only on valid complete numbers
                    if *value != v {
                        *value = v;
                        changed = true;
                    }
                }
            }

            if text_hov && self.input.mouse_pressed {
                let local = (self.input.mouse_pos.x - text_rect.min.x - pad).max(0.0);
                let at = byte_at_x(self, &text, local);
                st.caret = at;
                if !shift {
                    st.anchor = at;
                }
            }
            if self.active_id == Some(widget_id) && self.input.mouse_down {
                let local = (self.input.mouse_pos.x - text_rect.min.x - pad).max(0.0);
                st.caret = byte_at_x(self, &text, local);
            }

            self.num_bufs.insert(widget_id, text.clone());
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
        let r = self.s(theme::BTN_RADIUS);
        self.round_rect(rect, r, border);
        self.round_rect(rect.inset(1.0), (r - 1.0).max(0.0), bg);

        // grip strip
        let grip_col = match grip_color {
            Some(base) => {
                if dragging {
                    brighten(base, 0.25)
                } else if grip_hov {
                    brighten(base, 0.12)
                } else {
                    base
                }
            }
            None => {
                if dragging {
                    theme::SLIDER_THUMB_HOT
                } else if grip_hov {
                    theme::SLIDER_THUMB
                } else {
                    theme::SLIDER_FILL
                }
            }
        };
        let grip_inner = Rect {
            min: Vec2::new(rect.min.x + 1.0, rect.min.y + 1.0),
            max: Vec2::new(rect.min.x + grip_w, rect.max.y - 1.0),
        };
        self.round_rect(grip_inner, (r - 1.0).max(0.0), grip_col);

        let th = self.text_height();
        let text_w = text_rect.width() - pad * 2.0;
        let (sel_a, sel_b) = sel_range(st);

        let mut view_start = 0usize;
        if self.text_width(&text[..st.caret.min(text.len())]) > text_w {
            for (i, _) in text.char_indices() {
                if i > st.caret {
                    break;
                }
                if self.text_width(&text[i..st.caret.min(text.len())]) <= text_w * 0.85 {
                    view_start = i;
                    break;
                }
            }
        }

        let mut end = text.len();
        for (i, _) in text[view_start..].char_indices() {
            let abs = view_start + i;
            if self.text_width(&text[view_start..abs]) > text_w {
                end = abs;
                break;
            }
        }
        let draw = &text[view_start..end];
        let text_pos = Vec2::new(text_rect.min.x + pad, rect.min.y + (height - th) * 0.5);
        let text_color = if enabled {
            theme::TEXT
        } else {
            theme::TEXT_DISABLED
        };

        if focused && has_sel(st) {
            let a = sel_a.max(view_start).min(end);
            let b = sel_b.max(view_start).min(end);
            if a < b {
                let x0 = text_pos.x + self.text_width(&text[view_start..a]);
                let x1 = text_pos.x + self.text_width(&text[view_start..b]);
                draw_sel_line(self, x0, x1, rect.min.y + self.s(4.0), height - self.s(8.0));
            }
        }

        self.text(text_pos, draw, text_color);

        if focused && !dragging {
            let caret_x =
                text_pos.x + self.text_width(&text[view_start..st.caret.min(text.len())]) + 1.0;
            self.round_rect(
                Rect::from_min_size(
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

fn brighten(c: [f32; 4], amount: f32) -> [f32; 4] {
    [
        (c[0] + amount).min(1.0),
        (c[1] + amount).min(1.0),
        (c[2] + amount).min(1.0),
        c[3],
    ]
}
