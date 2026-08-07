use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect, Response};
use crate::widgets::edit::{
    byte_at_x, clamp_edit, draw_sel_line, handle_clipboard, handle_typing, has_sel, insert_str,
    move_end, move_home, move_left, move_right, sel_range, EditState,
};
use crate::widgets::scroll::{
    draw_vertical_scroll_bar, interact_vertical_scroll_bar, vertical_scroll_track,
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

/// Prefer the visual line where the caret sits after a soft wrap.
fn caret_line(lines: &[Line], caret: usize) -> usize {
    for (i, line) in lines.iter().enumerate() {
        if caret < line.end {
            return i;
        }
        if caret == line.end {
            // Soft wrap: caret belongs on the next visual line.
            if i + 1 < lines.len() && lines[i + 1].start == line.end {
                return i + 1;
            }
            return i;
        }
    }
    lines.len().saturating_sub(1)
}

fn scroll_to_caret(offset_y: &mut f32, caret_li: usize, line_h: f32, view_h: f32, max_scroll: f32) {
    let caret_top = caret_li as f32 * line_h;
    let caret_bot = caret_top + line_h;
    if caret_top < *offset_y {
        *offset_y = caret_top;
    } else if caret_bot > *offset_y + view_h {
        *offset_y = caret_bot - view_h;
    }
    *offset_y = offset_y.clamp(0.0, max_scroll);
}

fn layout_text_area(
    ui: &Ui,
    text: &str,
    rect: Rect,
    pad: f32,
    bar_w: f32,
    gap: f32,
    line_h: f32,
) -> (Rect, Vec<Line>, bool, f32, f32) {
    let mut inner = Rect {
        min: rect.min + Vec2::splat(pad),
        max: rect.max - Vec2::splat(pad),
    };
    let view_h = inner.height().max(1.0);
    let mut lines = wrap_lines(ui, text, inner.width().max(1.0));
    let mut content_h = lines.len() as f32 * line_h;
    let mut need_bar = content_h > view_h + 0.5;
    if need_bar {
        inner.max.x = (inner.max.x - bar_w - gap).max(inner.min.x + 1.0);
        lines = wrap_lines(ui, text, inner.width().max(1.0));
        content_h = lines.len() as f32 * line_h;
        need_bar = content_h > view_h + 0.5;
        if !need_bar {
            // Bar not needed after rewrap — restore full width.
            inner.max.x = rect.max.x - pad;
            lines = wrap_lines(ui, text, inner.width().max(1.0));
            content_h = lines.len() as f32 * line_h;
        }
    }
    (inner, lines, need_bar, content_h, view_h)
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
        let gap = self.s(theme::SCROLL_GAP);
        let line_h = self.text_height().max(1.0);
        let shift = self.input.key_shift;
        let mut changed = false;
        let mut caret_moved = false;

        let bar_id = widget_id.child("#bar");

        let mut st = self
            .edits
            .get(&widget_id)
            .copied()
            .unwrap_or(EditState {
                caret: text.len(),
                anchor: text.len(),
            });
        clamp_edit(&mut st, text.len());

        let (mut inner, mut lines, mut need_bar, mut content_h, view_h) =
            layout_text_area(self, text, rect, pad, bar_w, gap, line_h);
        let mut max_scroll = (content_h - view_h).max(0.0);

        let mut scroll_st = self.scrolls.get(&widget_id).copied().unwrap_or(ScrollState {
            offset: Vec2::ZERO,
            target: Vec2::ZERO,
            content: Vec2::ZERO,
        });
        scroll_st.offset.y = scroll_st.offset.y.clamp(0.0, max_scroll);
        scroll_st.target.y = scroll_st.target.y.clamp(0.0, max_scroll);

        let track = if need_bar {
            Some(vertical_scroll_track(inner, bar_w, gap))
        } else {
            None
        };
        let on_bar = need_bar
            && track.is_some_and(|t| self.hovered_rect(t) || self.active_id == Some(bar_id));

        let hovered = enabled && (self.hovered_rect(rect) || on_bar);
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

        if self.scroll_wheel_target == Some(widget_id) && !self.scroll_consumed {
            let dy = self.input.scroll_delta.y;
            if dy.abs() > 0.0 && need_bar {
                scroll_st.offset.y = (scroll_st.offset.y - dy).clamp(0.0, max_scroll);
                scroll_st.target.y = scroll_st.offset.y;
                self.consume_scroll();
            }
        }

        let bar_dragging = need_bar
            && enabled
            && interact_vertical_scroll_bar(
                self,
                bar_id,
                inner,
                content_h,
                &mut scroll_st.offset.y,
                bar_w,
                gap,
            );
        if bar_dragging {
            scroll_st.target.y = scroll_st.offset.y;
        }

        let focused = enabled && self.focus_id == Some(widget_id);
        let editing = focused && self.active_id != Some(bar_id);

        if hovered && self.input.mouse_pressed && !on_bar {
            self.focus_id = Some(widget_id);
            self.active_id = Some(widget_id);
        }
        if on_bar && self.input.mouse_pressed && enabled {
            self.focus_id = Some(widget_id);
        }

        if focused {
            self.want_capture = true;
            if !on_bar && self.active_id != Some(bar_id) {
                self.set_cursor(CursorIcon::Text);
            }

            let clip_changed = handle_clipboard(self, text, &mut st);
            changed |= clip_changed;
            caret_moved |= clip_changed || self.input.key_select_all;

            if self.input.key_enter {
                insert_str(text, &mut st, "\n");
                changed = true;
                caret_moved = true;
            }

            if self.input.key_left {
                move_left(text, &mut st, shift);
                caret_moved = true;
            }
            if self.input.key_right {
                move_right(text, &mut st, shift);
                caret_moved = true;
            }

            // Relayout after mutations that change wrap before vertical nav.
            if changed {
                clamp_edit(&mut st, text.len());
                let laid = layout_text_area(self, text, rect, pad, bar_w, gap, line_h);
                inner = laid.0;
                lines = laid.1;
                need_bar = laid.2;
                content_h = laid.3;
                max_scroll = (content_h - view_h).max(0.0);
                scroll_st.offset.y = scroll_st.offset.y.clamp(0.0, max_scroll);
            } else {
                clamp_edit(&mut st, text.len());
                lines = wrap_lines(self, text, inner.width().max(1.0));
            }

            if self.input.key_home {
                let li = caret_line(&lines, st.caret);
                move_home(&mut st, lines[li].start, shift);
                caret_moved = true;
            }
            if self.input.key_end {
                let li = caret_line(&lines, st.caret);
                move_end(&mut st, lines[li].end, shift);
                caret_moved = true;
            }
            if self.input.key_up || self.input.key_down {
                let li = caret_line(&lines, st.caret);
                let x = self.text_width(&text[lines[li].start..st.caret.min(lines[li].end)]);
                let target = if self.input.key_up {
                    li.saturating_sub(1)
                } else {
                    (li + 1).min(lines.len().saturating_sub(1))
                };
                let line = &lines[target];
                st.caret = line.start + byte_at_x(self, &text[line.start..line.end], x);
                if !shift {
                    st.anchor = st.caret;
                }
                caret_moved = true;
            }

            let typed = handle_typing(self, text, &mut st);
            changed |= typed;
            caret_moved |= typed;
            clamp_edit(&mut st, text.len());

            if typed || self.input.key_enter {
                let laid = layout_text_area(self, text, rect, pad, bar_w, gap, line_h);
                inner = laid.0;
                lines = laid.1;
                need_bar = laid.2;
                content_h = laid.3;
                max_scroll = (content_h - view_h).max(0.0);
                scroll_st.offset.y = scroll_st.offset.y.clamp(0.0, max_scroll);
            } else {
                lines = wrap_lines(self, text, inner.width().max(1.0));
                content_h = lines.len() as f32 * line_h;
                max_scroll = (content_h - view_h).max(0.0);
            }

            let hit = |ui: &Ui, pos: Vec2, offset_y: f32, lines: &[Line]| -> usize {
                let local = pos - inner.min;
                let li = ((offset_y + local.y.max(0.0)) / line_h).floor() as isize;
                let li = li.clamp(0, lines.len().saturating_sub(1) as isize) as usize;
                let line = &lines[li];
                line.start + byte_at_x(ui, &text[line.start..line.end], local.x.max(0.0))
            };

            if editing && hovered && self.input.mouse_pressed && !on_bar {
                let at = hit(self, self.input.mouse_pos, scroll_st.offset.y, &lines);
                st.caret = at;
                if !shift {
                    st.anchor = at;
                }
                caret_moved = true;
            }
            if self.active_id == Some(widget_id) && self.input.mouse_down {
                let local_y = self.input.mouse_pos.y - inner.min.y;
                if local_y < 0.0 {
                    scroll_st.offset.y = (scroll_st.offset.y - line_h).max(0.0);
                    scroll_st.target.y = scroll_st.offset.y;
                } else if local_y > inner.height() {
                    scroll_st.offset.y = (scroll_st.offset.y + line_h).min(max_scroll);
                    scroll_st.target.y = scroll_st.offset.y;
                }
                st.caret = hit(self, self.input.mouse_pos, scroll_st.offset.y, &lines);
                caret_moved = true;
            }
        }

        content_h = lines.len() as f32 * line_h;
        max_scroll = (content_h - view_h).max(0.0);
        scroll_st.offset.y = scroll_st.offset.y.clamp(0.0, max_scroll);
        scroll_st.target.y = scroll_st.target.y.clamp(0.0, max_scroll);

        if caret_moved {
            let caret_li = caret_line(&lines, st.caret);
            scroll_to_caret(
                &mut scroll_st.offset.y,
                caret_li,
                line_h,
                view_h,
                max_scroll,
            );
            scroll_st.target.y = scroll_st.offset.y;
        }

        scroll_st.content.y = content_h;
        let offset_y = scroll_st.offset.y;
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

        // Pixel scroll: draw every line that intersects the viewport.
        let first = (offset_y / line_h).floor().max(0.0) as usize;
        let last = ((offset_y + view_h) / line_h).ceil() as usize;
        let last = last.min(lines.len());

        self.push_clip(inner);
        for row in first..last {
            let line = &lines[row];
            let y = inner.min.y + row as f32 * line_h - offset_y;
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
        }

        if focused {
            let caret_li = caret_line(&lines, st.caret);
            let line = &lines[caret_li];
            let y = inner.min.y + caret_li as f32 * line_h - offset_y;
            let cx = inner.min.x
                + self.text_width(&text[line.start..st.caret.clamp(line.start, line.end)]);
            // Keep caret visible even at the exact right edge / empty last line.
            let caret_x = cx.min(inner.max.x - 1.0).max(inner.min.x);
            self.round_rect(
                Rect::from_min_size(Vec2::new(caret_x, y), Vec2::new(1.0, line_h)),
                0.0,
                theme::TEXT,
            );
        }
        self.pop_clip();

        if need_bar {
            draw_vertical_scroll_bar(self, bar_id, inner, content_h, offset_y, bar_w, gap);
        }

        Response {
            hovered,
            clicked: false,
            changed,
        }
    }
}
