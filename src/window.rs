use glam::Vec2;

use super::draw::push_round_rect;
use super::theme;
use super::types::{CursorIcon, Id, Rect};
use super::{new_layer, LayoutDir, Ui, WinState};

pub(crate) fn clamp_win(
    pos: &mut Vec2,
    size: &mut Vec2,
    vis_h: f32,
    viewport: Vec2,
    min: Vec2,
    title_h: f32,
) {
    let vp = Vec2::new(viewport.x.max(min.x), viewport.y.max(min.y));
    size.x = size.x.clamp(min.x, vp.x);
    size.y = size.y.clamp(min.y, vp.y);
    let h = vis_h.clamp(title_h, vp.y);
    pos.x = pos.x.clamp(0.0, (vp.x - size.x).max(0.0));
    pos.y = pos.y.clamp(0.0, (vp.y - h).max(0.0));
}

/// Optional window features via builder.
pub struct Window<'a> {
    pub(crate) title: &'a str,
    pub(crate) default_pos: Vec2,
    pub(crate) default_size: Vec2,
    pub(crate) resizable: bool,
    pub(crate) collapsible: bool,
    pub(crate) open: Option<&'a mut bool>,
}

impl<'a> Window<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            default_pos: Vec2::new(40.0, 40.0),
            default_size: Vec2::new(280.0, 200.0),
            resizable: false,
            collapsible: false,
            open: None,
        }
    }

    pub fn pos(mut self, pos: Vec2) -> Self {
        self.default_pos = pos;
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.default_size = size;
        self
    }

    pub fn resizable(mut self, v: bool) -> Self {
        self.resizable = v;
        self
    }

    pub fn collapsible(mut self, v: bool) -> Self {
        self.collapsible = v;
        self
    }

    pub fn open(mut self, open: &'a mut bool) -> Self {
        self.open = Some(open);
        self
    }
}

impl Ui {
    fn draw_title_btn(&mut self, rect: Rect, label: &str, hovered: bool, id: Id) {
        let pressed = self.active_id == Some(id) && self.input.mouse_down;
        let color = if pressed {
            theme::BTN_PRESS
        } else if hovered {
            theme::BTN_HOVER
        } else {
            [0.20, 0.20, 0.20, 1.0]
        };
        self.round_rect(rect, 3.0, color);
        let tw = self.text_width(label);
        let th = self.text_height();
        self.text(
            Vec2::new(
                rect.min.x + (rect.width() - tw) * 0.5,
                rect.min.y + (rect.height() - th) * 0.5,
            ),
            label,
            theme::TITLE_TEXT,
        );
    }

    fn title_btn_rect(pos: Vec2, size_x: f32, title_h: f32, index_from_right: i32, s: f32) -> Rect {
        let btn = 16.0 * s;
        let pad = 4.0 * s;
        let x = pos.x + size_x - pad - (index_from_right as f32 + 1.0) * (btn + pad) + pad;
        Rect::from_min_size(
            Vec2::new(x, pos.y + (title_h - btn) * 0.5),
            Vec2::splat(btn),
        )
    }

    pub fn window(&mut self, mut cfg: Window<'_>, add: impl FnOnce(&mut Self)) {
        if cfg.open.as_ref().is_some_and(|o| !**o) {
            return;
        }

        let draw_start = self.draw_list.len();
        self.push_id(cfg.title);
        let window_id = *self.id_stack.last().unwrap();
        if !self.win_order.contains(&window_id) {
            self.win_order.push(window_id);
        }

        let input_ok = self.window_input_ok(window_id);
        let prev_block = self.block_input;
        if !input_ok {
            self.block_input = true;
        }

        let title_id = window_id.child("#title");
        let resize_id = window_id.child("#resize");
        let collapse_id = window_id.child("#collapse");
        let close_id = window_id.child("#close");

        let entry = self.windows.entry(window_id).or_insert(WinState {
            pos: cfg.default_pos,
            size: cfg.default_size,
            collapsed: false,
        });
        let mut pos = entry.pos;
        let mut size = entry.size;
        let mut collapsed = entry.collapsed;

        let title_h = self.s(theme::WIN_TITLE_H);
        let closable = cfg.open.is_some();
        let sc = self.scale;

        let mut btn_i = 0;
        let mut on_chrome = false;

        let close_r = if closable {
            let r = Self::title_btn_rect(pos, size.x, title_h, btn_i, sc);
            btn_i += 1;
            let resp = self.interact_rect(close_id, r);
            on_chrome |= resp.hovered || self.active_id == Some(close_id);
            if resp.clicked() {
                if let Some(open) = cfg.open.as_mut() {
                    **open = false;
                }
                self.windows
                    .insert(window_id, WinState { pos, size, collapsed });
                self.block_input = prev_block;
                let cmds = self.draw_list.split_off(draw_start);
                self.window_layers.push((window_id, cmds));
                self.pop_id();
                return;
            }
            Some((r, resp.hovered))
        } else {
            None
        };

        let collapse_r = if cfg.collapsible {
            let r = Self::title_btn_rect(pos, size.x, title_h, btn_i, sc);
            let resp = self.interact_rect(collapse_id, r);
            on_chrome |= resp.hovered || self.active_id == Some(collapse_id);
            if resp.clicked() {
                collapsed = !collapsed;
            }
            Some((r, resp.hovered))
        } else {
            None
        };

        let title_bar = Rect::from_min_size(pos, Vec2::new(size.x, title_h));
        let title_hover = !self.block_input && title_bar.contains(self.input.mouse_pos) && !on_chrome;

        if title_hover && self.input.mouse_pressed {
            self.active_id = Some(title_id);
            self.drag_grab = Some(self.input.mouse_pos - pos);
        }
        if title_hover {
            self.set_cursor(CursorIcon::Move);
        }
        if self.active_id == Some(title_id) {
            if let Some(grab) = self.drag_grab {
                pos = self.input.mouse_pos - grab;
            }
            self.want_capture = true;
            self.set_cursor(CursorIcon::Move);
        }

        let handle = self.s(14.0);
        if cfg.resizable && !collapsed {
            let resize_rect = Rect::from_min_size(
                pos + Vec2::new(size.x - handle, size.y - handle),
                Vec2::splat(handle),
            );
            let hover = !self.block_input && resize_rect.contains(self.input.mouse_pos);
            if hover {
                self.want_capture = true;
                self.set_cursor(CursorIcon::ResizeNwse);
            }
            if hover && self.input.mouse_pressed {
                self.active_id = Some(resize_id);
                self.drag_grab = Some(self.input.mouse_pos - (pos + size));
            }
            if self.active_id == Some(resize_id) {
                if let Some(grab) = self.drag_grab {
                    size = self.input.mouse_pos - pos - grab;
                }
                self.want_capture = true;
                self.set_cursor(CursorIcon::ResizeNwse);
            }
        }

        let min = Vec2::new(self.s(theme::WIN_MIN_W), self.s(theme::WIN_MIN_H));
        let vis_h = if collapsed { title_h } else { size.y };
        clamp_win(&mut pos, &mut size, vis_h, self.input.viewport, min, title_h);
        let vis_h = if collapsed { title_h } else { size.y };
        clamp_win(&mut pos, &mut size, vis_h, self.input.viewport, min, title_h);

        self.windows
            .insert(window_id, WinState { pos, size, collapsed });

        let rect = Rect::from_min_size(pos, Vec2::new(size.x, vis_h));
        self.win_rects.insert(window_id, rect);
        if rect.contains(self.input.mouse_pos) && input_ok {
            self.want_capture = true;
        }

        let title_pressed = self.active_id == Some(title_id) && self.input.mouse_down;
        let title_color = if title_pressed {
            theme::WIN_TITLE_PRESS
        } else if title_hover {
            theme::WIN_TITLE_HOVER
        } else {
            theme::WIN_TITLE
        };

        self.round_rect(rect, self.s(theme::WIN_RADIUS), theme::WIN_BORDER);
        self.round_rect(rect.inset(1.0), self.s(theme::WIN_RADIUS) - 1.0, theme::WIN_BODY);
        let uv = self.font.white_uv();
        let title_r = self.s(theme::WIN_RADIUS) - 1.0;
        push_round_rect(
            &mut self.draw_list,
            Rect {
                min: rect.min + Vec2::new(1.0, 1.0),
                max: Vec2::new(rect.max.x - 1.0, pos.y + title_h),
            },
            title_r,
            title_color,
            true,
            collapsed,
            uv,
            None,
        );
        let th = self.text_height();
        self.text(
            pos + Vec2::new(self.s(10.0), (title_h - th) * 0.5),
            cfg.title,
            theme::TITLE_TEXT,
        );

        let mut btn_i = 0;
        if closable {
            let r = Self::title_btn_rect(pos, size.x, title_h, btn_i, sc);
            btn_i += 1;
            let hovered =
                close_r.map(|(_, h)| h).unwrap_or(false) || r.contains(self.input.mouse_pos);
            self.draw_title_btn(r, "x", hovered && !self.block_input, close_id);
        }
        if cfg.collapsible {
            let r = Self::title_btn_rect(pos, size.x, title_h, btn_i, sc);
            let hovered =
                collapse_r.map(|(_, h)| h).unwrap_or(false) || r.contains(self.input.mouse_pos);
            let mark = if collapsed { "+" } else { "-" };
            self.draw_title_btn(r, mark, hovered && !self.block_input, collapse_id);
        }

        if !collapsed {
            let pad = self.s(12.0);
            let top = title_h + self.s(10.0);
            let content_w = (size.x - pad * 2.0).max(0.0);
            let content_h = (size.y - top - pad).max(0.0);
            let content_clip = Rect {
                min: Vec2::new(pos.x + 1.0, pos.y + title_h),
                max: Vec2::new(pos.x + size.x - 1.0, pos.y + size.y - 1.0),
            };
            self.push_clip(content_clip);
            self.layers.push(new_layer(
                LayoutDir::Vertical,
                pos + Vec2::new(pad, top),
                self.spacing,
                content_w,
                content_h,
            ));
            add(self);
            self.layers.pop();
            self.pop_clip();
        }

        self.block_input = prev_block;
        let cmds = self.draw_list.split_off(draw_start);
        self.window_layers.push((window_id, cmds));
        self.pop_id();
    }
}
