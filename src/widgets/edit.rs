use crate::Ui;
use crate::types::Rect;
use glam::Vec2;

#[derive(Clone, Copy, Default)]
pub(crate) struct EditState {
    pub caret: usize,
    pub anchor: usize,
}

// Shared by single-line and multiline fields; hosts need no extra input events.
#[derive(Clone, Copy)]
struct TextClick {
    id: crate::Id,
    pos: Vec2,
    age: f32,
}

#[derive(Default)]
pub(crate) struct TextMouseState {
    last_click: Option<TextClick>,
    candidate: Option<TextClick>,
    word_drag: Option<(crate::Id, usize, usize)>,
}

impl TextMouseState {
    pub(crate) fn begin_frame(&mut self, input: &crate::UiInput, tolerance: f32) {
        if let Some(click) = &mut self.last_click {
            click.age += input.dt.max(0.0);
            if click.age > 0.4
                || (input.mouse_down
                    && !input.mouse_pressed
                    && click.pos.distance(input.mouse_pos) > tolerance)
            {
                self.last_click = None;
            }
        }
        // Any intervening press (including outside text fields) breaks the pair.
        self.candidate = if input.mouse_pressed {
            self.last_click.take()
        } else {
            None
        };
        if input.mouse_pressed || !input.mouse_down {
            self.word_drag = None;
        }
    }

    fn press(&mut self, id: crate::Id, pos: Vec2, tolerance: f32, shift: bool) -> bool {
        let double = !shift
            && self.candidate.take().is_some_and(|click| {
                click.id == id && click.age <= 0.4 && click.pos.distance(pos) <= tolerance
            });
        self.last_click = if double || shift {
            None
        } else {
            Some(TextClick { id, pos, age: 0.0 })
        };
        double
    }
}

pub(crate) fn word_range(text: &str, at: usize) -> (usize, usize) {
    use unicode_segmentation::UnicodeSegmentation;
    if text.is_empty() {
        return (0, 0);
    }
    let at = at.min(text.len().saturating_sub(1));
    text.split_word_bound_indices()
        .find(|(start, part)| at < start + part.len())
        .map(|(start, part)| (start, start + part.len()))
        .unwrap_or((0, 0))
}

// Unlike caret hit-testing, keep the character under the pointer even in its
// right half, so double-clicking the last letter still selects that word.
pub(crate) fn byte_under_x(ui: &Ui, text: &str, x: f32) -> usize {
    let mut end = 0.0;
    for (i, c) in text.char_indices() {
        end += ui.text_width(&c.to_string());
        if x < end {
            return i;
        }
    }
    text.len()
}

pub(crate) fn text_press(
    ui: &mut Ui,
    id: crate::Id,
    text: &str,
    st: &mut EditState,
    at: usize,
    word_at: usize,
) {
    let tolerance = ui.s(4.0);
    if ui
        .text_mouse
        .press(id, ui.input.mouse_pos, tolerance, ui.input.key_shift)
    {
        let (a, b) = word_range(text, word_at);
        st.anchor = a;
        st.caret = b;
        ui.text_mouse.word_drag = Some((id, a, b));
    } else {
        st.caret = at;
        if !ui.input.key_shift {
            st.anchor = at;
        }
    }
}

pub(crate) fn text_drag(
    ui: &Ui,
    id: crate::Id,
    text: &str,
    st: &mut EditState,
    at: usize,
    word_at: usize,
) {
    if let Some((_, a, b)) = ui
        .text_mouse
        .word_drag
        .filter(|(drag_id, _, _)| *drag_id == id)
    {
        let (start, end) = word_range(text, word_at);
        if start < a {
            st.anchor = b.min(text.len());
            st.caret = start;
        } else {
            st.anchor = a.min(text.len());
            st.caret = end.max(b.min(text.len()));
        }
    } else {
        st.caret = at;
    }
}

pub(crate) fn input_view_start(ui: &Ui, text: &str, caret: usize, width: f32) -> usize {
    if ui.text_width(&text[..caret]) > width {
        for (i, _) in text.char_indices() {
            if i > caret {
                break;
            }
            if ui.text_width(&text[i..caret]) <= width * 0.85 {
                return i;
            }
        }
    }
    0
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
    if ui.input.key_delete {
        if delete_sel(text, st) {
            changed = true;
        } else if st.caret < text.len() {
            let next = st.caret + text[st.caret..].chars().next().unwrap().len_utf8();
            text.replace_range(st.caret..next, "");
            st.anchor = st.caret;
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

#[cfg(test)]
mod text_mouse_tests {
    use super::*;
    use crate::UiInput;

    fn frame(ui: &mut Ui, text: &mut String, multiline: bool, input: UiInput) -> (usize, usize) {
        ui.begin_frame(UiInput {
            viewport: Vec2::new(800.0, 600.0),
            ..input
        });
        if multiline {
            ui.text_area("field", text, Vec2::new(180.0, 100.0));
        } else {
            ui.text_input("field", text);
        }
        let range = sel_range(ui.edits[&ui.current_id("field")]);
        ui.end_frame();
        range
    }

    fn press(pos: Vec2) -> UiInput {
        UiInput {
            mouse_pos: pos,
            mouse_pressed: true,
            mouse_down: true,
            dt: 0.05,
            ..Default::default()
        }
    }

    fn release(pos: Vec2) -> UiInput {
        UiInput {
            mouse_pos: pos,
            mouse_released: true,
            dt: 0.05,
            ..Default::default()
        }
    }

    #[test]
    fn unicode_words_spaces_punctuation_and_empty_text() {
        for (text, at, expected) in [
            ("hello world", 2, "hello"),
            ("привет мир", 5, "привет"),
            ("cafe\u{301}!", 4, "cafe\u{301}"),
            ("hello, world", 5, ","),
            ("one   two", 4, "   "),
            ("a😀b", 2, "😀"),
            ("last", 4, "last"),
            ("", 0, ""),
        ] {
            let (a, b) = word_range(text, at);
            assert_eq!(&text[a..b], expected);
        }
    }

    #[test]
    fn double_click_selects_word_survives_hold_release_and_replaces_selection() {
        for multiline in [false, true] {
            let mut ui = Ui::new();
            let mut text = "привет мир".to_owned();
            let pos = Vec2::new(8.0 + ui.text_width("при"), 14.0);
            let one = frame(&mut ui, &mut text, multiline, press(pos));
            assert_eq!(one.0, one.1);
            frame(&mut ui, &mut text, multiline, release(pos));
            assert_eq!(
                frame(&mut ui, &mut text, multiline, press(pos)),
                (0, "привет".len())
            );
            assert_eq!(
                frame(
                    &mut ui,
                    &mut text,
                    multiline,
                    UiInput {
                        mouse_pos: pos,
                        mouse_down: true,
                        dt: 0.02,
                        ..Default::default()
                    }
                ),
                (0, "привет".len())
            );
            assert_eq!(
                frame(&mut ui, &mut text, multiline, release(pos)),
                (0, "привет".len())
            );
            frame(
                &mut ui,
                &mut text,
                multiline,
                UiInput {
                    text: "hi".into(),
                    ..Default::default()
                },
            );
            assert_eq!(text, "hi мир");
        }
    }

    #[test]
    fn last_letter_right_half_selects_the_word_and_word_drag_extends_both_ways() {
        for multiline in [false, true] {
            let mut ui = Ui::new();
            let mut text = "one two end".to_owned();
            let pos = Vec2::new(
                8.0 + ui.text_width("one tw") + ui.text_width("o") * 0.8,
                14.0,
            );
            frame(&mut ui, &mut text, multiline, press(pos));
            frame(&mut ui, &mut text, multiline, release(pos));
            assert_eq!(frame(&mut ui, &mut text, multiline, press(pos)), (4, 7));
            let right = Vec2::new(8.0 + ui.text_width("one two e"), 14.0);
            assert_eq!(
                frame(
                    &mut ui,
                    &mut text,
                    multiline,
                    UiInput {
                        mouse_pos: right,
                        mouse_down: true,
                        ..Default::default()
                    }
                ),
                (4, 11)
            );
            assert_eq!(
                frame(
                    &mut ui,
                    &mut text,
                    multiline,
                    UiInput {
                        mouse_pos: Vec2::new(10.0, 14.0),
                        mouse_down: true,
                        ..Default::default()
                    }
                ),
                (0, 7)
            );
        }
    }

    #[test]
    fn slow_distant_shift_and_intervening_clicks_do_not_select_words() {
        for multiline in [false, true] {
            for kind in 0..4 {
                let mut ui = Ui::new();
                let mut text = "hello world".to_owned();
                let pos = Vec2::new(12.0, 14.0);
                frame(&mut ui, &mut text, multiline, press(pos));
                frame(&mut ui, &mut text, multiline, release(pos));
                let mut next = press(pos);
                match kind {
                    0 => next.dt = 0.6,
                    1 => next.mouse_pos.x += 20.0,
                    2 => next.key_shift = true,
                    _ => {
                        let outside = Vec2::new(500.0, 400.0);
                        frame(&mut ui, &mut text, multiline, press(outside));
                        frame(&mut ui, &mut text, multiline, release(outside));
                    }
                }
                let range = frame(&mut ui, &mut text, multiline, next);
                assert_ne!(range, (0, 5), "kind={kind}, multiline={multiline}");
            }
        }
    }

    #[test]
    fn ordinary_drag_remains_character_based_and_breaks_double_click_pair() {
        for multiline in [false, true] {
            let mut ui = Ui::new();
            let mut text = "hello world".to_owned();
            let pos = Vec2::new(8.0, 14.0);
            frame(&mut ui, &mut text, multiline, press(pos));
            let end = Vec2::new(8.0 + ui.text_width("hel"), 14.0);
            assert_eq!(
                frame(
                    &mut ui,
                    &mut text,
                    multiline,
                    UiInput {
                        mouse_pos: end,
                        mouse_down: true,
                        ..Default::default()
                    }
                ),
                (0, 3)
            );
            frame(&mut ui, &mut text, multiline, release(end));
            let range = frame(&mut ui, &mut text, multiline, press(pos));
            assert_eq!(range, (0, 0));
        }
    }

    #[test]
    fn scrolled_input_hit_testing_uses_visible_text() {
        let mut ui = Ui::new();
        let mut text = "first second third fourth fifth sixth seventh".to_owned();
        let start = input_view_start(&ui, &text, text.len(), ui.s(164.0));
        assert!(start > 0);
        let pos = Vec2::new(8.0 + ui.text_width(&text[start..start + 1]) * 0.2, 14.0);
        assert_eq!(frame(&mut ui, &mut text, false, press(pos)), (start, start));
    }

    #[test]
    fn click_pair_requires_same_widget() {
        let mut mouse = TextMouseState::default();
        let ui = Ui::new();
        let a = ui.current_id("a");
        let b = ui.current_id("b");
        let pos = Vec2::ZERO;
        mouse.begin_frame(&press(pos), 4.0);
        assert!(!mouse.press(a, pos, 4.0, false));
        mouse.begin_frame(&release(pos), 4.0);
        mouse.begin_frame(&press(pos), 4.0);
        assert!(!mouse.press(b, pos, 4.0, false));
    }

    #[test]
    fn multiline_double_click_handles_soft_wrap_and_vertical_scroll() {
        let mut ui = Ui::new();
        let mut text = "extraordinarilylongwordwithoutspaces".to_owned();
        // The same logical word spans several visual lines.
        let pos = Vec2::new(12.0, 8.0 + ui.text_height() * 1.5);
        frame(&mut ui, &mut text, true, press(pos));
        frame(&mut ui, &mut text, true, release(pos));
        let len = text.len();
        assert_eq!(frame(&mut ui, &mut text, true, press(pos)), (0, len));

        let mut text = "zero\none\ntwo\nthree\nfour\nfive\nsix\nseven\neight".to_owned();
        frame(&mut ui, &mut text, true, UiInput::default());
        let id = ui.current_id("field");
        ui.edits.insert(
            id,
            EditState {
                caret: 0,
                anchor: 0,
            },
        );
        let offset = ui.text_height() * 3.0;
        let scroll = ui.scrolls.get_mut(&id).unwrap();
        scroll.offset.y = offset;
        scroll.target.y = offset;
        let pos = Vec2::new(12.0, 8.0 + ui.text_height() * 0.5);
        // Break any pair left by the previous example.
        frame(
            &mut ui,
            &mut text,
            true,
            UiInput {
                dt: 0.6,
                ..Default::default()
            },
        );
        frame(&mut ui, &mut text, true, press(pos));
        frame(&mut ui, &mut text, true, release(pos));
        let (a, b) = frame(&mut ui, &mut text, true, press(pos));
        assert_eq!(&text[a..b], "three");
    }
}
