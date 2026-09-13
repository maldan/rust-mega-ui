use glam::Vec2;

use crate::Ui;
use crate::theme;
use crate::types::{CursorIcon, Id, Rect, Response};

pub(crate) struct MenuBarCtx {
    pub id: Id,
    pub bar: Rect,
    pub cursor_x: f32,
    /// Currently open top-level menu button id.
    pub open: Option<Id>,
    /// True if pointer is over the bar or any open popup this frame.
    pub pointer_in_menu: bool,
}

pub(crate) struct MenuPopupCtx {
    pub id: Id,
    pub origin: Vec2,
    pub width: f32,
    pub cursor_y: f32,
    pub max_label_w: f32,
    pub popup_rect: Rect,
    pub child_popup: Option<Rect>,
    pub pointer_inside: bool,
}

impl Ui {
    /// Application-style menu bar (File / Edit / …). Fill width of the current layout
    /// (or the viewport when unconstrained).
    pub fn menu_bar(&mut self, add: impl FnOnce(&mut Self)) {
        let bar_h = self.s(theme::MENU_BAR_H);
        let pad = self.s(6.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 {
            fill_w
        } else {
            self.input.viewport.x.max(1.0)
        };

        let bar = self.allocate(Vec2::new(width, bar_h));
        let id = self.current_id("#menubar");
        self.push_id("#menubar");

        self.round_rect(bar, 0.0, theme::MENU_BAR_BG);
        let line = Rect {
            min: Vec2::new(bar.min.x, bar.max.y - 1.0),
            max: bar.max,
        };
        self.round_rect(line, 0.0, theme::WIN_BORDER);

        let mut open = self.menu_bar_open.get(&id).copied().flatten();
        let bar_hovered = self.hovered_overlay(bar) || self.hovered_rect(bar);

        self.menu_bar_stack.push(MenuBarCtx {
            id,
            bar,
            cursor_x: bar.min.x + pad,
            open,
            pointer_in_menu: bar_hovered,
        });

        add(self);

        let ctx = self.menu_bar_stack.pop().unwrap();
        open = ctx.open;

        if self.input.mouse_pressed && !ctx.pointer_in_menu {
            open = None;
            self.menu_sub_open.clear();
        }

        self.menu_bar_open.insert(id, open);
        self.pop_id();
    }

    /// Top-level bar menu, or a nested submenu when called inside another menu.
    pub fn menu(&mut self, label: &str, add: impl FnOnce(&mut Self)) {
        if self.menu_stack.is_empty() {
            self.menu_top_level(label, add);
        } else {
            self.menu_submenu(label, add);
        }
    }

    /// Clickable leaf item inside an open menu.
    pub fn menu_item(&mut self, label: &str) -> Response {
        self.menu_item_enabled(label, self.enabled())
    }

    pub fn menu_item_enabled(&mut self, label: &str, enabled: bool) -> Response {
        self.menu_item_inner(label, None, enabled, true)
    }

    /// Like [`Self::menu_item`], but does not dismiss the menu (for drill-down pages).
    pub fn menu_item_keep_open(&mut self, label: &str) -> Response {
        self.menu_item_inner(label, None, self.enabled(), false)
    }

    /// Submenu-looking row with a chevron; keeps the menu open on click.
    pub fn menu_item_submenu(&mut self, label: &str) -> Response {
        let Some(item) = self.menu_row_with_icon(label, None, self.enabled(), true) else {
            return Response::default();
        };
        let clicked = self.enabled() && item.hovered && self.input.mouse_released;
        Response {
            hovered: item.hovered,
            clicked,
            changed: clicked,
        }
    }

    /// Clickable leaf item with a leading SVG icon.
    pub fn menu_item_icon(&mut self, icon: &str, label: &str) -> Response {
        self.menu_item_icon_enabled(icon, label, self.enabled())
    }

    pub fn menu_item_icon_enabled(&mut self, icon: &str, label: &str, enabled: bool) -> Response {
        self.menu_item_inner(label, Some(icon), enabled, true)
    }

    fn menu_item_inner(
        &mut self,
        label: &str,
        icon: Option<&str>,
        enabled: bool,
        close_on_click: bool,
    ) -> Response {
        let Some(item) = self.menu_row_with_icon(label, icon, enabled, false) else {
            return Response::default();
        };
        let clicked = enabled && item.hovered && self.input.mouse_released;
        if clicked && close_on_click {
            self.close_all_menus();
        }
        Response {
            hovered: item.hovered,
            clicked,
            changed: clicked,
        }
    }

    /// Horizontal rule inside a menu popup. Prefer this (or plain [`Self::separator`])
    /// while a menu is open.
    pub fn menu_separator(&mut self) {
        self.menu_separator_inner();
    }

    /// Non-interactive section header inside a menu (dim label, does not close).
    pub fn menu_section(&mut self, label: &str) {
        let Some(ctx) = self.menu_stack.last() else {
            return;
        };
        let item_h = self.s(theme::MENU_ITEM_H);
        let pad = self.s(10.0);
        let width = ctx.width;
        let y = ctx.cursor_y;
        let origin_x = ctx.origin.x;
        let rect = Rect::from_min_size(
            Vec2::new(origin_x + self.s(3.0), y),
            Vec2::new((width - self.s(6.0)).max(1.0), item_h),
        );
        self.absorb_rect(rect);

        let label_w = self.text_width(label);
        if let Some(ctx) = self.menu_stack.last_mut() {
            ctx.cursor_y += item_h;
            ctx.max_label_w = ctx.max_label_w.max(label_w);
        }

        let text_h = self.text_height();
        self.text_overlay(
            Vec2::new(rect.min.x + pad, rect.min.y + (item_h - text_h) * 0.5),
            label,
            theme::TEXT_DIM,
        );
    }

    /// True while any context menu is open.
    pub fn context_menu_open(&self) -> bool {
        self.context_menu.is_some()
    }
}

impl Ui {
    fn menu_top_level(&mut self, label: &str, add: impl FnOnce(&mut Self)) {
        let Some(bar) = self.menu_bar_stack.last() else {
            // Not inside menu_bar — treat as a one-shot popup button.
            self.menu_orphan(label, add);
            return;
        };

        let bar_id = bar.id;
        let item_id = self.current_id(label);
        let pad_x = self.s(10.0);
        let text_w = self.text_width(label);
        let text_h = self.text_height();
        let btn_w = text_w + pad_x * 2.0;
        let bar_rect = bar.bar;
        let x = bar.cursor_x;
        let btn = Rect::from_min_size(
            Vec2::new(x, bar_rect.min.y),
            Vec2::new(btn_w, bar_rect.height()),
        );

        if let Some(b) = self.menu_bar_stack.last_mut() {
            b.cursor_x += btn_w;
        }

        let hovered = !self.block_input && (self.hovered_overlay(btn) || self.hovered_rect(btn));
        if hovered {
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
            if let Some(b) = self.menu_bar_stack.last_mut() {
                b.pointer_in_menu = true;
            }
        }

        let mut open = self
            .menu_bar_stack
            .last()
            .and_then(|b| b.open)
            .filter(|id| *id == item_id)
            .is_some();

        // Click toggles; while any menu is open, hover switches.
        let bar_open = self.menu_bar_stack.last().and_then(|b| b.open);
        if hovered && self.input.mouse_pressed {
            open = !open || bar_open != Some(item_id);
            if !open {
                self.menu_sub_open.clear();
            }
        } else if hovered && bar_open.is_some() {
            open = true;
        }

        if open {
            if let Some(b) = self.menu_bar_stack.last_mut() {
                b.open = Some(item_id);
            }
        } else if self.menu_bar_stack.last().and_then(|b| b.open) == Some(item_id) {
            if let Some(b) = self.menu_bar_stack.last_mut() {
                b.open = None;
            }
            self.menu_sub_open.clear();
        }

        let active = open || (hovered && self.menu_bar_stack.last().and_then(|b| b.open).is_some());
        if active || hovered {
            self.round_rect(
                btn.inset(self.s(2.0)),
                self.s(3.0),
                if open {
                    theme::MENU_ACTIVE
                } else {
                    theme::MENU_HOVER
                },
            );
        }
        self.text(
            Vec2::new(btn.min.x + pad_x, btn.min.y + (btn.height() - text_h) * 0.5),
            label,
            theme::TEXT,
        );

        if open {
            self.push_id(label);
            let origin = Vec2::new(btn.min.x, bar_rect.max.y);
            self.open_menu_popup(item_id, origin, add);
            self.pop_id();
        }

        let _ = bar_id;
    }

    fn menu_orphan(&mut self, label: &str, add: impl FnOnce(&mut Self)) {
        // Minimal fallback: a button that opens a popup below itself.
        let id = self.current_id(label);
        let pad_x = self.s(10.0);
        let height = self.s(theme::MENU_BAR_H);
        let width = self.text_width(label) + pad_x * 2.0;
        let btn = self.allocate(Vec2::new(width, height));
        let hovered = self.hovered_rect(btn);
        let mut open = self.menu_bar_open.get(&id).copied().flatten() == Some(id);

        if hovered && self.input.mouse_pressed {
            open = !open;
        }
        if hovered {
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
            self.round_rect(btn.inset(1.0), self.s(3.0), theme::MENU_HOVER);
        }
        self.text(
            Vec2::new(
                btn.min.x + pad_x,
                btn.min.y + (btn.height() - self.text_height()) * 0.5,
            ),
            label,
            theme::TEXT,
        );

        if open {
            self.menu_bar_open.insert(id, Some(id));
            self.push_id(label);
            self.open_menu_popup(id, Vec2::new(btn.min.x, btn.max.y), add);
            self.pop_id();
            if self.input.mouse_pressed && !hovered {
                // close unless popup kept it
                if self.menu_stack.is_empty() {
                    // popup already finished; check absorb
                    if !self.mouse_over_absorb() {
                        self.menu_bar_open.insert(id, None);
                    }
                }
            }
        } else {
            self.menu_bar_open.insert(id, None);
        }
    }

    fn menu_submenu(&mut self, label: &str, add: impl FnOnce(&mut Self)) {
        let Some(item) = self.menu_row_with_icon(label, None, self.enabled(), true) else {
            return;
        };
        let parent_id = self.menu_stack.last().map(|m| m.id).unwrap();
        let item_id = item.id;

        if item.hovered {
            self.menu_sub_open.insert(parent_id, Some(item_id));
        }

        let open = self.menu_sub_open.get(&parent_id).copied().flatten() == Some(item_id);
        if !open {
            return;
        }

        self.push_id(label);
        let parent_popup = self.menu_stack.last().unwrap().popup_rect;
        let est_w = self
            .menu_popup_size
            .get(&item_id)
            .map(|s| s.x)
            .unwrap_or(self.s(theme::MENU_MIN_W));
        let mut origin = Vec2::new(parent_popup.max.x - self.s(4.0), item.rect.min.y);
        if origin.x + est_w > self.input.viewport.x - self.s(4.0) {
            origin.x = (parent_popup.min.x - est_w + self.s(4.0)).max(0.0);
        }
        self.open_menu_popup(item_id, origin, add);
        self.pop_id();
    }

    fn open_menu_popup(&mut self, id: Id, origin: Vec2, add: impl FnOnce(&mut Self)) {
        let min_w = self.s(theme::MENU_MIN_W);
        let pad = self.s(4.0);
        let prev = self.menu_popup_size.get(&id).copied().unwrap_or(Vec2::new(
            min_w,
            self.s(theme::MENU_ITEM_H) * 12.0 + pad * 2.0,
        ));

        let mut popup = Rect::from_min_size(origin, prev);
        let overflow_y = popup.max.y - self.input.viewport.y;
        if overflow_y > 0.0 {
            popup.min.y = (popup.min.y - overflow_y).max(0.0);
            popup.max.y = popup.min.y + prev.y;
        }
        let overflow_x = popup.max.x - self.input.viewport.x;
        if overflow_x > 0.0 {
            popup.min.x = (popup.min.x - overflow_x).max(0.0);
            popup.max.x = popup.min.x + prev.x;
        }

        let radius = self.s(theme::BTN_RADIUS);
        self.round_rect_overlay(popup, radius, theme::BTN_BORDER);
        self.round_rect_overlay(popup.inset(1.0), (radius - 1.0).max(0.0), theme::POPUP_BG);

        let pointer_inside = self.hovered_overlay(popup);
        if pointer_inside {
            self.want_capture = true;
            if let Some(b) = self.menu_bar_stack.last_mut() {
                b.pointer_in_menu = true;
            }
        }
        self.absorb_rect(popup);

        self.menu_stack.push(MenuPopupCtx {
            id,
            origin: popup.min,
            width: prev.x,
            cursor_y: popup.min.y + pad,
            max_label_w: 0.0,
            popup_rect: popup,
            child_popup: None,
            pointer_inside,
        });

        add(self);

        let ctx = self.menu_stack.pop().unwrap();
        let content_h = (ctx.cursor_y - popup.min.y + pad).max(self.s(theme::MENU_ITEM_H));
        let chevron_room = self.s(18.0);
        let content_w = (ctx.max_label_w + self.s(20.0) + chevron_room + pad * 2.0).max(min_w);
        self.menu_popup_size
            .insert(id, Vec2::new(content_w, content_h));

        let tight = Rect::from_min_size(popup.min, Vec2::new(content_w, content_h));
        self.absorb_rect(tight);

        if let Some(parent) = self.menu_stack.last_mut() {
            parent.child_popup = Some(tight);
            if ctx.pointer_inside || tight.contains(self.input.mouse_pos) {
                parent.pointer_inside = true;
            }
        }
        if (ctx.pointer_inside || tight.contains(self.input.mouse_pos))
            && let Some(b) = self.menu_bar_stack.last_mut()
        {
            b.pointer_in_menu = true;
        }
    }

    fn menu_row_with_icon(
        &mut self,
        label: &str,
        icon: Option<&str>,
        enabled: bool,
        submenu: bool,
    ) -> Option<MenuRow> {
        let ctx = self.menu_stack.last()?;
        let id = self.current_id(label);
        let item_h = self.s(theme::MENU_ITEM_H);
        let pad = self.s(10.0);
        let icon_s = self.s(14.0);
        let icon_gap = if icon.is_some() {
            icon_s + self.s(8.0)
        } else {
            0.0
        };
        let width = ctx.width;
        let y = ctx.cursor_y;
        let origin_x = ctx.origin.x;
        let rect = Rect::from_min_size(
            Vec2::new(origin_x + self.s(3.0), y),
            Vec2::new((width - self.s(6.0)).max(1.0), item_h),
        );
        // Always absorb the row — popup bg size can lag one frame behind content,
        // and lower items (submenus) would otherwise sit outside the hit target.
        self.absorb_rect(rect);

        let label_w = self.text_width(label) + icon_gap;
        if let Some(ctx) = self.menu_stack.last_mut() {
            ctx.cursor_y += item_h;
            ctx.max_label_w = ctx.max_label_w.max(label_w);
        }

        let hovered = enabled && self.hovered_overlay(rect);
        if hovered {
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
            self.round_rect_overlay(rect, self.s(3.0), theme::POPUP_HOVER);
            if let Some(b) = self.menu_bar_stack.last_mut() {
                b.pointer_in_menu = true;
            }
            if let Some(ctx) = self.menu_stack.last_mut() {
                ctx.pointer_inside = true;
            }
            if !submenu && let Some(ctx) = self.menu_stack.last() {
                let parent = ctx.id;
                self.menu_sub_open.insert(parent, None);
            }
        }

        let text_h = self.text_height();
        let color = if enabled {
            theme::TEXT
        } else {
            theme::TEXT_DISABLED
        };

        let mut text_x = rect.min.x + pad;
        if let Some(icon_id) = icon {
            let icon_rect = Rect::from_min_size(
                Vec2::new(rect.min.x + pad, rect.min.y + (item_h - icon_s) * 0.5),
                Vec2::splat(icon_s),
            );
            self.draw_icon_at(icon_id, icon_rect, color, true);
            text_x += icon_gap;
        }

        self.text_overlay(
            Vec2::new(text_x, rect.min.y + (item_h - text_h) * 0.5),
            label,
            color,
        );

        if submenu {
            let arrow_s = self.s(12.0);
            let arrow_rect = Rect::from_min_size(
                Vec2::new(
                    rect.max.x - self.s(16.0),
                    rect.min.y + (item_h - arrow_s) * 0.5,
                ),
                Vec2::splat(arrow_s),
            );
            self.draw_icon_at("chevron_right", arrow_rect, theme::TEXT_DIM, true);
        }

        Some(MenuRow { id, rect, hovered })
    }

    fn menu_separator_inner(&mut self) {
        let Some(ctx) = self.menu_stack.last() else {
            return;
        };
        let sep_h = self.s(theme::MENU_SEP_H);
        let y = ctx.cursor_y;
        let x0 = ctx.origin.x + self.s(8.0);
        let x1 = ctx.origin.x + ctx.width - self.s(8.0);
        let line = Rect {
            min: Vec2::new(x0, y + sep_h * 0.5),
            max: Vec2::new(x1, y + sep_h * 0.5 + 1.0),
        };
        if let Some(ctx) = self.menu_stack.last_mut() {
            ctx.cursor_y += sep_h;
        }
        self.round_rect_overlay(line, 0.0, theme::MENU_SEP);
    }

    fn close_all_menus(&mut self) {
        if let Some(b) = self.menu_bar_stack.last_mut() {
            b.open = None;
        }
        for open in self.menu_bar_open.values_mut() {
            *open = None;
        }
        self.menu_sub_open.clear();
        self.context_menu = None;
    }

    /// Right-click context menu. Opens when `hovered` and RMB pressed; items via
    /// [`Self::menu_item`] / [`Self::menu_item_icon`] inside `add`.
    pub fn context_menu(&mut self, id: &str, hovered: bool, add: impl FnOnce(&mut Self)) {
        let widget_id = self.current_id(id);

        if hovered && self.input.mouse_right_pressed {
            for o in self.menu_bar_open.values_mut() {
                *o = None;
            }
            self.menu_sub_open.clear();
            self.context_menu = Some((widget_id, self.input.mouse_pos));
        }

        let open = self
            .context_menu
            .filter(|(i, _)| *i == widget_id)
            .map(|(_, p)| p);

        if let Some(pos) = open {
            self.push_id(id);
            self.open_menu_popup(widget_id, pos, add);
            // Close only on press outside the menu tree (parent + submenus).
            if self.input.mouse_pressed && !self.mouse_over_absorb() {
                self.context_menu = None;
                self.menu_sub_open.clear();
            }
            self.pop_id();
        }
    }

    pub(crate) fn absorb_rect(&mut self, rect: Rect) {
        self.mouse_absorb.push(rect);
    }
}

struct MenuRow {
    id: Id,
    rect: Rect,
    hovered: bool,
}
