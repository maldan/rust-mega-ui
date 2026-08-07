use crate::theme;
use crate::types::Rect;
use crate::Ui;
use glam::Vec2;

#[derive(Clone, Copy, Default)]
pub(crate) struct EditState {
    pub caret: usize,
    pub anchor: usize,
}

pub(crate) fn sel_range(st: EditState) -> (usize, usize) {
    (st.caret.min(st.anchor), st.caret.max(st.anchor))
}

pub(crate) fn has_sel(st: EditState) -> bool {
    st.caret != st.anchor
}

pub(crate) fn clamp_edit(st: &mut EditState, len: usize) {
    st.caret = st.caret.min(len);
    st.anchor = st.anchor.min(len);
}

pub(crate) fn delete_sel(text: &mut String, st: &mut EditState) -> bool {
    let (a, b) = sel_range(*st);
    if a == b {
        return false;
    }
    text.replace_range(a..b, "");
    st.caret = a;
    st.anchor = a;
    true
}

pub(crate) fn insert_str(text: &mut String, st: &mut EditState, s: &str) {
    delete_sel(text, st);
    text.insert_str(st.caret, s);
    st.caret += s.len();
    st.anchor = st.caret;
}

pub(crate) fn move_left(text: &str, st: &mut EditState, shift: bool) {
    if !shift && has_sel(*st) {
        let (a, _) = sel_range(*st);
        st.caret = a;
        st.anchor = a;
        return;
    }
    if st.caret > 0 {
        st.caret = text[..st.caret]
            .char_indices()
            .next_back()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }
    if !shift {
        st.anchor = st.caret;
    }
}

pub(crate) fn move_right(text: &str, st: &mut EditState, shift: bool) {
    if !shift && has_sel(*st) {
        let (_, b) = sel_range(*st);
        st.caret = b;
        st.anchor = b;
        return;
    }
    if st.caret < text.len() {
        st.caret += text[st.caret..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
    }
    if !shift {
        st.anchor = st.caret;
    }
}

pub(crate) fn move_home(st: &mut EditState, line_start: usize, shift: bool) {
    st.caret = line_start;
    if !shift {
        st.anchor = st.caret;
    }
}

pub(crate) fn move_end(st: &mut EditState, line_end: usize, shift: bool) {
    st.caret = line_end;
    if !shift {
        st.anchor = st.caret;
    }
}

pub(crate) fn byte_at_x(ui: &Ui, text: &str, x: f32) -> usize {
    let mut acc = 0.0;
    let mut at = 0usize;
    for (i, c) in text.char_indices() {
        let cw = ui.text_width(&c.to_string());
        if acc + cw * 0.5 >= x {
            return i;
        }
        acc += cw;
        at = i + c.len_utf8();
    }
    at
}

pub(crate) fn handle_clipboard(ui: &mut Ui, text: &mut String, st: &mut EditState) -> bool {
    let mut changed = false;
    if ui.input.key_select_all {
        st.anchor = 0;
        st.caret = text.len();
    }
    if ui.input.key_copy || ui.input.key_cut {
        let (a, b) = sel_range(*st);
        if a < b {
            let clip = text[a..b].to_string();
            ui.clipboard_buf = clip.clone();
            ui.clipboard_out = Some(clip);
        }
        if ui.input.key_cut && a < b {
            text.replace_range(a..b, "");
            st.caret = a;
            st.anchor = a;
            changed = true;
        }
    }
    if ui.input.key_paste {
        let paste = if !ui.input.clipboard.is_empty() {
            ui.input.clipboard.clone()
        } else if !ui.clipboard_buf.is_empty() {
            ui.clipboard_buf.clone()
        } else {
            String::new()
        };
        if !paste.is_empty() {
            insert_str(text, st, &paste);
            changed = true;
        }
    }
    changed
}

pub(crate) fn handle_typing(ui: &Ui, text: &mut String, st: &mut EditState) -> bool {
    let mut changed = false;
    if ui.input.key_backspace {
        if delete_sel(text, st) {
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
    if !ui.input.key_ctrl && !ui.input.text.is_empty() {
        insert_str(text, st, &ui.input.text);
        changed = true;
    }
    changed
}

pub(crate) fn draw_sel_line(ui: &mut Ui, x0: f32, x1: f32, y: f32, h: f32) {
    if (x1 - x0).abs() < 0.5 {
        return;
    }
    let (min_x, max_x) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
    ui.round_rect(
        Rect::from_min_size(Vec2::new(min_x, y), Vec2::new(max_x - min_x, h)),
        0.0,
        theme::SELECTION,
    );
}
