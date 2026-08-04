use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect, Response};
use crate::widgets::edit::{
    byte_at_x, clamp_edit, draw_sel_line, handle_clipboard, handle_typing, has_sel, insert_str,
    move_end, move_home, move_left, move_right, sel_range, EditState,
};
use crate::{LayoutDir, ScrollState, Ui};

struct Line {
    start: usize,
    end: usize,
}

fn wrap_lines(ui: &Ui, text: &str, width: f32) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut line_start = 0usize;
    let mut i = 0usize;
    let mut x = 0.0f32;
    let bytes = text.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            lines.push(Line {
                start: line_start,
                end: i,
            });
            i += 1;
            line_start = i;
            x = 0.0;
            continue;
        }
        let c = text[i..].chars().next().unwrap();
        let cw = ui.text_width(&c.to_string());
        if x + cw > width && i > line_start {
            lines.push(Line {
                start: line_start,
                end: i,
            });
            line_start = i;
            x = 0.0;
            continue;
        }
        x += cw;
        i += c.len_utf8();
    }
    lines.push(Line {
        start: line_start,
        end: text.len(),
    });
    lines
}

fn caret_line(lines: &[Line], caret: usize) -> usize {
    for (i, line) in lines.iter().enumerate() {
        if caret < line.end {
            return i;
        }
        if caret == line.end {
            if i + 1 < lines.len() && lines[i + 1].start == line.end {
                return i + 1;
            }
            return i;
        }
    }
    lines.len().saturating_sub(1)
}

fn clamp_scroll(scroll: usize, lines: usize, view: usize) -> usize {
    scroll.min(lines.saturating_sub(view))
}

impl Ui {
    /// Multiline text area. `size` — if x/y is 0, uses available width/height.
    pub fn text_area(&mut self, id: &str, text: &mut String, size: Vec2) -> Response {
        let enabled = self.enabled();
        let widget_id = self.current_id(id);
        let avail = self.available_size();
        let fill_w = self.layer().fill_w;
        let width = if size.x > 0.0 {
            size.x
        } else if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else if avail.x > 0.0 {
            avail.x
        } else {
            self.s(220.0)
        };
        let height = if size.y > 0.0 {
            size.y
        } else if avail.y > 0.0 {
            avail.y.max(self.s(80.0))
        } else {
            self.s(100.0)
        };
        let rect = self.allocate(Vec2::new(width, height));

        let pad = self.s(8.0);
        let bar_w = self.s(theme::SCROLL_BAR);
        let line_h = self.text_height();
        let shift = self.input.key_shift;
        let mut changed = false;

        let mut inner = Rect {
            min: rect.min + Vec2::splat(pad),
            max: rect.max - Vec2::splat(pad),
        };
        let mut max_lines = ((inner.height() / line_h).floor() as usize).max(1);
        let mut lines = wrap_lines(self, text, inner.width().max(1.0));
        let mut need_bar = lines.len() > max_lines;
        if need_bar {
            inner.max.x = (rect.max.x - pad - bar_w).max(inner.min.x + 1.0);
            lines = wrap_lines(self, text, inner.width().max(1.0));
            max_lines = ((inner.height() / line_h).floor() as usize).max(1);
            need_bar = lines.len() > max_lines;
        }

        let track = Rect {
            min: Vec2::new(rect.max.x - bar_w - 1.0, inner.min.y),
            max: Vec2::new(rect.max.x - 1.0, inner.max.y),
        };
        let bar_id = widget_id.child("#bar");

        let hovered = enabled && self.hovered_rect(rect);
        let on_bar = need_bar && self.hovered_rect(track);
        if hovered {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(if on_bar {
                CursorIcon::Pointer
            } else {
                CursorIcon::Text
            });
            self.scroll_hover = Some(widget_id);
        }

        let mut st = self
            .edits
            .get(&widget_id)
            .copied()
            .unwrap_or(EditState {
                caret: text.len(),
                anchor: text.len(),
            });
        clamp_edit(&mut st, text.len());

        let mut scroll_st = self.scrolls.get(&widget_id).copied().unwrap_or(ScrollState {
            offset: Vec2::ZERO,
            target: Vec2::ZERO,
            content: Vec2::ZERO,
        });
        let mut scroll = scroll_st.offset.y as usize;

        if self.scroll_wheel_target == Some(widget_id) {
            let dy = self.input.scroll_delta.y;
            if dy.abs() > 0.0 {
                let mut step = (dy / line_h).round() as i32;
                if step == 0 {
                    step = if dy > 0.0 { 1 } else { -1 };
                }
                scroll = (scroll as i32 - step).max(0) as usize;
                self.consume_scroll();
            }
        }

        // scrollbar interact first
        if need_bar && enabled {
            let view_h = track.height();
            let thumb_h = (view_h * max_lines as f32 / lines.len().max(1) as f32)
                .max(self.s(theme::SCROLL_THUMB_MIN))
                .min(view_h);
            let travel = (view_h - thumb_h).max(0.0);
            let max_s = lines.len().saturating_sub(max_lines) as f32;
            let ty = if max_s > 0.0 {
                track.min.y + scroll as f32 / max_s * travel
            } else {
                track.min.y
            };
            let thumb = Rect::from_min_size(Vec2::new(track.min.x, ty), Vec2::new(bar_w, thumb_h));

            if self.hovered_rect(thumb) && self.input.mouse_pressed {
                self.active_id = Some(bar_id);
                self.drag_grab = Some(self.input.mouse_pos - Vec2::new(0.0, ty));
            } else if self.hovered_rect(track) && self.input.mouse_pressed {
                let rel = ((self.input.mouse_pos.y - track.min.y - thumb_h * 0.5)
                    / travel.max(1.0))
                .clamp(0.0, 1.0);
                scroll = (rel * max_s).round() as usize;
                self.active_id = Some(bar_id);
                self.drag_grab = Some(Vec2::new(0.0, thumb_h * 0.5));
            }
            if self.active_id == Some(bar_id) && self.input.mouse_down {
                let grab_y = self.drag_grab.map(|g| g.y).unwrap_or(thumb_h * 0.5);
                let y = self.input.mouse_pos.y - grab_y;
                let rel = ((y - track.min.y) / travel.max(1.0)).clamp(0.0, 1.0);
                scroll = (rel * max_s).round() as usize;
                self.want_capture = true;
                self.set_cursor(CursorIcon::Pointer);
            }
        }

        let focused = enabled && self.focus_id == Some(widget_id);
        let editing = focused && self.active_id != Some(bar_id);

        if hovered && self.input.mouse_pressed && !on_bar {
            self.focus_id = Some(widget_id);
            self.active_id = Some(widget_id);
        }

        if focused {
            self.want_capture = true;
            if !on_bar && self.active_id != Some(bar_id) {
                self.set_cursor(CursorIcon::Text);
            }

            changed |= handle_clipboard(self, text, &mut st);
            if self.input.key_enter {
                insert_str(text, &mut st, "\n");
                changed = true;
            }
            clamp_edit(&mut st, text.len());
            lines = wrap_lines(self, text, inner.width().max(1.0));
            need_bar = lines.len() > max_lines;

            if self.input.key_left {
                move_left(text, &mut st, shift);
            }
            if self.input.key_right {
                move_right(text, &mut st, shift);
            }
            if self.input.key_home {
                let li = caret_line(&lines, st.caret);
                move_home(&mut st, lines[li].start, shift);
            }
            if self.input.key_end {
                let li = caret_line(&lines, st.caret);
                move_end(&mut st, lines[li].end, shift);
            }
            if self.input.key_up || self.input.key_down {
                let li = caret_line(&lines, st.caret);
                let x = self.text_width(&text[lines[li].start..st.caret]);
                let target = if self.input.key_up {
                    li.saturating_sub(1)
                } else {
                    (li + 1).min(lines.len() - 1)
                };
                let line = &lines[target];
                st.caret = line.start + byte_at_x(self, &text[line.start..line.end], x);
                if !shift {
                    st.anchor = st.caret;
                }
            }

            changed |= handle_typing(self, text, &mut st);
            clamp_edit(&mut st, text.len());
            lines = wrap_lines(self, text, inner.width().max(1.0));

            let hit = |ui: &Ui, pos: Vec2, scroll: usize, lines: &[Line]| -> usize {
                let local = pos - inner.min;
                let li = scroll as isize + (local.y / line_h).floor() as isize;
                let li = li.clamp(0, lines.len().saturating_sub(1) as isize) as usize;
                let line = &lines[li];
                line.start + byte_at_x(ui, &text[line.start..line.end], local.x.max(0.0))
            };

            if editing && hovered && self.input.mouse_pressed && !on_bar {
                let at = hit(self, self.input.mouse_pos, scroll, &lines);
                st.caret = at;
                if !shift {
                    st.anchor = at;
                }
            }
            if self.active_id == Some(widget_id) && self.input.mouse_down {
                let local_y = self.input.mouse_pos.y - inner.min.y;
                if local_y < 0.0 {
                    scroll = scroll.saturating_sub(1);
                } else if local_y > inner.height() {
                    scroll += 1;
                }
                scroll = clamp_scroll(scroll, lines.len(), max_lines);
                st.caret = hit(self, self.input.mouse_pos, scroll, &lines);
            }
        }

        let caret_li = caret_line(&lines, st.caret);
        scroll = clamp_scroll(scroll, lines.len(), max_lines);
        if caret_li < scroll {
            scroll = caret_li;
        }
        if caret_li >= scroll + max_lines {
            scroll = caret_li + 1 - max_lines;
        }
        scroll = clamp_scroll(scroll, lines.len(), max_lines);

        scroll_st.offset.y = scroll as f32;
        scroll_st.target.y = scroll as f32;
        scroll_st.content.y = lines.len() as f32;
        self.scrolls.insert(widget_id, scroll_st);
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

        let text_color = if enabled {
            theme::TEXT
        } else {
            theme::TEXT_DISABLED
        };
        let (sel_a, sel_b) = sel_range(st);

        self.push_clip(inner);
        for (row, line) in lines.iter().enumerate().skip(scroll).take(max_lines) {
            let y = inner.min.y + (row - scroll) as f32 * line_h;
            let pos = Vec2::new(inner.min.x, y);

            if has_sel(st) && sel_a < line.end && sel_b > line.start {
                let xa = if sel_a <= line.start {
                    pos.x
                } else {
                    pos.x + self.text_width(&text[line.start..sel_a.min(line.end)])
                };
                let xb = if sel_b >= line.end {
                    if sel_b > line.end {
                        inner.max.x
                    } else {
                        pos.x + self.text_width(&text[line.start..line.end])
                    }
                } else {
                    pos.x + self.text_width(&text[line.start..sel_b.min(line.end)])
                };
                draw_sel_line(self, xa, xb, y, line_h);
            }

            self.text(pos, &text[line.start..line.end], text_color);

            let on_line = st.caret >= line.start
                && (st.caret < line.end
                    || (st.caret == line.end && caret_line(&lines, st.caret) == row));
            if focused && on_line {
                let cx = pos.x + self.text_width(&text[line.start..st.caret]);
                self.round_rect(
                    Rect::from_min_size(Vec2::new(cx, y), Vec2::new(1.0, line_h)),
                    0.0,
                    theme::TEXT,
                );
            }
        }
        self.pop_clip();

        if need_bar {
            let view_h = track.height();
            let thumb_h = (view_h * max_lines as f32 / lines.len().max(1) as f32)
                .max(self.s(theme::SCROLL_THUMB_MIN))
                .min(view_h);
            let travel = (view_h - thumb_h).max(0.0);
            let max_s = lines.len().saturating_sub(max_lines) as f32;
            let ty = if max_s > 0.0 {
                track.min.y + scroll as f32 / max_s * travel
            } else {
                track.min.y
            };
            self.round_rect(track, 0.0, theme::SCROLL_BG);
            let hot = self.active_id == Some(bar_id) || self.hovered_rect(Rect::from_min_size(
                Vec2::new(track.min.x, ty),
                Vec2::new(bar_w, thumb_h),
            ));
            self.round_rect(
                Rect::from_min_size(Vec2::new(track.min.x, ty), Vec2::new(bar_w, thumb_h)),
                0.0,
                if hot {
                    theme::SCROLL_THUMB_HOT
                } else {
                    theme::SCROLL_THUMB
                },
            );
        }

        Response {
            hovered,
            clicked: false,
            changed,
        }
    }
}
