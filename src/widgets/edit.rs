use crate::Ui;
use crate::types::Rect;
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
        ui.theme.text.selection,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(caret: usize, anchor: usize) -> EditState {
        EditState { caret, anchor }
    }

    // -- sel_range / has_sel -------------------------------------------------

    #[test]
    fn sel_range_orders_regardless_of_drag_direction() {
        assert_eq!(sel_range(st(5, 2)), (2, 5));
        assert_eq!(sel_range(st(2, 5)), (2, 5));
    }

    #[test]
    fn has_sel_false_when_caret_equals_anchor() {
        assert!(!has_sel(st(3, 3)));
        assert!(has_sel(st(3, 4)));
    }

    // -- clamp_edit -----------------------------------------------------------

    #[test]
    fn clamp_edit_after_external_text_shrink() {
        let mut s = st(50, 50);
        clamp_edit(&mut s, 3);
        assert_eq!((s.caret, s.anchor), (3, 3));
    }

    #[test]
    fn clamp_edit_leaves_in_range_values_untouched() {
        let mut s = st(1, 2);
        clamp_edit(&mut s, 10);
        assert_eq!((s.caret, s.anchor), (1, 2));
    }

    // -- delete_sel -------------------------------------------------------------

    #[test]
    fn delete_sel_noop_when_no_selection() {
        let mut text = "hello".to_string();
        let mut s = st(2, 2);
        assert!(!delete_sel(&mut text, &mut s));
        assert_eq!(text, "hello");
    }

    #[test]
    fn delete_sel_across_multibyte_boundary() {
        let mut text = "привет мир".to_string();
        let mut s = st(0, "привет".len());
        assert!(delete_sel(&mut text, &mut s));
        assert_eq!(text, " мир");
        assert_eq!(s.caret, 0);
        assert_eq!(s.anchor, 0);
    }

    #[test]
    fn delete_sel_collapses_caret_to_selection_start() {
        let mut text = "aXXXb".to_string();
        let mut s = st(4, 1);
        assert!(delete_sel(&mut text, &mut s));
        assert_eq!(text, "ab");
        assert_eq!(s.caret, 1);
    }

    // -- insert_str -------------------------------------------------------------

    #[test]
    fn insert_str_replaces_selection_first() {
        let mut text = "aXXXb".to_string();
        let mut s = st(1, 4);
        insert_str(&mut text, &mut s, "Y");
        assert_eq!(text, "aYb");
        assert_eq!(s.caret, 2);
        assert_eq!(s.anchor, 2);
    }

    #[test]
    fn insert_str_advances_caret_by_byte_len_of_multibyte_input() {
        let mut text = String::new();
        let mut s = st(0, 0);
        insert_str(&mut text, &mut s, "привет");
        assert_eq!(text, "привет");
        assert_eq!(s.caret, "привет".len());
    }

    // -- move_left / move_right ---------------------------------------------

    #[test]
    fn move_right_steps_over_multibyte_char_not_one_byte() {
        let text = "привет";
        let mut s = st(0, 0);
        move_right(text, &mut s, false);
        // 'п' is 2 bytes in UTF-8 — caret must land on the next char boundary.
        assert_eq!(s.caret, 2);
    }

    #[test]
    fn move_left_steps_back_over_multibyte_char() {
        let text = "a😀b"; // emoji is 4 bytes
        let after_emoji = "a😀".len();
        let mut s = st(after_emoji, after_emoji);
        move_left(text, &mut s, false);
        assert_eq!(s.caret, "a".len());
    }

    #[test]
    fn move_left_without_shift_collapses_to_selection_start() {
        let text = "hello";
        let mut s = st(4, 1);
        move_left(text, &mut s, false);
        assert_eq!(s.caret, 1);
        assert_eq!(s.anchor, 1);
    }

    #[test]
    fn move_right_without_shift_collapses_to_selection_end() {
        let text = "hello";
        let mut s = st(1, 4);
        move_right(text, &mut s, false);
        assert_eq!(s.caret, 4);
        assert_eq!(s.anchor, 4);
    }

    #[test]
    fn move_left_at_start_of_text_is_noop() {
        let text = "hello";
        let mut s = st(0, 0);
        move_left(text, &mut s, false);
        assert_eq!(s.caret, 0);
    }

    #[test]
    fn move_right_at_end_of_text_is_noop() {
        let text = "hello";
        let mut s = st(5, 5);
        move_right(text, &mut s, false);
        assert_eq!(s.caret, 5);
    }

    #[test]
    fn shift_extends_selection_without_collapsing() {
        let text = "hello";
        let mut s = st(0, 0);
        move_right(text, &mut s, true);
        move_right(text, &mut s, true);
        assert_eq!(sel_range(s), (0, 2));
        assert!(has_sel(s));
    }

    // -- move_home / move_end -------------------------------------------------

    #[test]
    fn move_home_resets_anchor_without_shift() {
        let mut s = st(10, 3);
        move_home(&mut s, 2, false);
        assert_eq!(s.caret, 2);
        assert_eq!(s.anchor, 2);
    }

    #[test]
    fn move_home_with_shift_keeps_anchor() {
        let mut s = st(10, 3);
        move_home(&mut s, 2, true);
        assert_eq!(s.caret, 2);
        assert_eq!(s.anchor, 3);
    }

    #[test]
    fn move_end_resets_anchor_without_shift() {
        let mut s = st(0, 3);
        move_end(&mut s, 8, false);
        assert_eq!(s.caret, 8);
        assert_eq!(s.anchor, 8);
    }

    #[test]
    fn move_end_with_shift_keeps_anchor() {
        let mut s = st(0, 3);
        move_end(&mut s, 8, true);
        assert_eq!(s.caret, 8);
        assert_eq!(s.anchor, 3);
    }
}
