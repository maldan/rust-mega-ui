//! Hierarchical tree (Blender Outliner-style).
//!
//! - Chevron toggles expand/collapse only.
//! - Click on the row selects (inside [`Ui::tree_scope`]).
//! - Mid / trailing icon buttons via [`TreeRow`].

use glam::Vec2;

use crate::theme;
use crate::types::CursorIcon;
use crate::types::Rect;
use crate::{CrossAlign, LayoutDir, Ui};

/// Result of drawing a tree row for one frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct TreeResponse {
    /// Whether the node is currently expanded (always `false` for leaves).
    pub open: bool,
    /// True if the row body was clicked this frame (not the chevron / icon buttons).
    pub clicked: bool,
}

impl TreeResponse {
    pub fn open(self) -> bool {
        self.open
    }

    pub fn clicked(self) -> bool {
        self.clicked
    }
}

/// Interactable icons on a tree row.
///
/// Mid icons sit after the label (left → right). Call [`TreeRow::spacer`] then
/// trailing icons pin to the right edge (right → left).
pub struct TreeRow<'a> {
    ui: &'a mut Ui,
    mid_x: f32,
    trailing_x: f32,
    y: f32,
    btn: f32,
    trailing_mode: bool,
    pub(crate) icon_hovered: bool,
    pub(crate) trailing_left: f32,
}

impl TreeRow<'_> {
    /// Switch subsequent icons to the right-aligned trailing column.
    pub fn spacer(&mut self) {
        self.trailing_mode = true;
    }

    /// Decorative icon in the current zone (mid or trailing).
    pub fn icon(&mut self, icon: &str) {
        let rect = self.next_rect();
        self.ui
            .draw_icon_at(icon, rect.inset(self.ui.s(2.0)), theme::TEXT_DIM, false);
    }

    /// Clickable icon button. Returns `true` on click.
    pub fn icon_button(&mut self, id: &str, icon: &str, active: bool) -> bool {
        let rect = self.next_rect();
        self.hit_icon_button(id, icon, active, rect)
    }

    fn next_rect(&mut self) -> Rect {
        let gap = self.ui.s(1.0);
        if self.trailing_mode {
            self.trailing_x -= self.btn;
            let r = Rect::from_min_size(Vec2::new(self.trailing_x, self.y), Vec2::splat(self.btn));
            self.trailing_x -= gap;
            self.trailing_left = self.trailing_x;
            r
        } else {
            let r = Rect::from_min_size(Vec2::new(self.mid_x, self.y), Vec2::splat(self.btn));
            self.mid_x += self.btn + gap;
            r
        }
    }

    fn hit_icon_button(&mut self, id: &str, icon: &str, active: bool, rect: Rect) -> bool {
        let enabled = self.ui.enabled();
        let widget_id = self.ui.current_id(id);
        let hovered = enabled && self.ui.hovered_rect(rect);
        if hovered {
            self.icon_hovered = true;
            self.ui.hover_id = Some(widget_id);
            self.ui.want_capture = true;
            self.ui.set_cursor(CursorIcon::Pointer);
        }
        if hovered && self.ui.input.mouse_pressed {
            self.ui.active_id = Some(widget_id);
        }
        let pressed = self.ui.active_id == Some(widget_id) && self.ui.input.mouse_down;
        let clicked = enabled
            && self.ui.active_id == Some(widget_id)
            && hovered
            && self.ui.input.mouse_released;

        if hovered || active {
            self.ui.round_rect(rect, self.ui.s(3.0), theme::BTN_HOVER);
        }
        let color = if !enabled {
            theme::TEXT_DISABLED
        } else if pressed {
            theme::TEXT_BRIGHT
        } else if active {
            theme::ACCENT
        } else if hovered {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        let pad = self.ui.s(3.0);
        self.ui.draw_icon_at(
            icon,
            Rect {
                min: rect.min + Vec2::splat(pad),
                max: rect.max - Vec2::splat(pad),
            },
            color,
            false,
        );
        clicked
    }
}

impl Ui {
    /// Bind selection for nested `tree_*` calls. Selection is stored in `selected`.
    pub fn tree_scope(&mut self, selected: &mut Option<String>, add: impl FnOnce(&mut Self)) {
        let prev = self.tree_sel.take();
        self.tree_sel = Some(selected.clone());
        add(self);
        if let Some(sel) = self.tree_sel.take() {
            *selected = sel;
        }
        self.tree_sel = prev;
    }

    pub fn tree_node(&mut self, id: &str, label: &str, add: impl FnOnce(&mut Self)) -> TreeResponse {
        self.tree_node_ex(id, None, label, true, |_| {}, add)
    }

    /// Expandable tree node with a leading type icon (e.g. folder).
    pub fn tree_node_icon(
        &mut self,
        id: &str,
        icon: &str,
        label: &str,
        add: impl FnOnce(&mut Self),
    ) -> TreeResponse {
        self.tree_node_ex(id, Some(icon), label, true, |_| {}, add)
    }

    /// Expandable node with mid/trailing icon slots.
    pub fn tree_node_with(
        &mut self,
        id: &str,
        label: &str,
        extras: impl FnOnce(&mut TreeRow),
        add: impl FnOnce(&mut Self),
    ) -> TreeResponse {
        self.tree_node_ex(id, None, label, true, extras, add)
    }

    /// Expandable node with leading icon and mid/trailing slots.
    pub fn tree_node_icon_with(
        &mut self,
        id: &str,
        icon: &str,
        label: &str,
        extras: impl FnOnce(&mut TreeRow),
        add: impl FnOnce(&mut Self),
    ) -> TreeResponse {
        self.tree_node_ex(id, Some(icon), label, true, extras, add)
    }

    /// Non-expandable leaf.
    pub fn tree_leaf(&mut self, id: &str, label: &str) -> TreeResponse {
        self.tree_node_ex(id, None, label, false, |_| {}, |_| {})
    }

    /// Non-expandable leaf with icon (e.g. file).
    pub fn tree_leaf_icon(&mut self, id: &str, icon: &str, label: &str) -> TreeResponse {
        self.tree_node_ex(id, Some(icon), label, false, |_| {}, |_| {})
    }

    /// Leaf with mid/trailing icon slots.
    pub fn tree_leaf_with(
        &mut self,
        id: &str,
        label: &str,
        extras: impl FnOnce(&mut TreeRow),
    ) -> TreeResponse {
        self.tree_node_ex(id, None, label, false, extras, |_| {})
    }

    /// Leaf with leading icon and mid/trailing slots.
    pub fn tree_leaf_icon_with(
        &mut self,
        id: &str,
        icon: &str,
        label: &str,
        extras: impl FnOnce(&mut TreeRow),
    ) -> TreeResponse {
        self.tree_node_ex(id, Some(icon), label, false, extras, |_| {})
    }

    fn tree_node_ex(
        &mut self,
        id: &str,
        icon: Option<&str>,
        label: &str,
        expandable: bool,
        extras: impl FnOnce(&mut TreeRow),
        add: impl FnOnce(&mut Self),
    ) -> TreeResponse {
        let widget_id = self.current_id(id);
        let arrow_id = widget_id.child("#arrow");
        let sel_id = widget_id.child("#sel");
        let mut open = expandable && self.trees.get(&widget_id).copied().unwrap_or(false);

        let height = self.s(24.0);
        let indent = self.s(14.0);
        let icon_s = self.s(14.0);
        let arrow_draw = self.s(16.0);
        let arrow_slot = self.s(20.0);
        let btn = self.s(20.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            self.s(180.0)
        };
        let rect = self.allocate(Vec2::new(width, height));

        let arrow_rect = Rect::from_min_size(
            Vec2::new(rect.min.x + self.s(2.0), rect.min.y),
            Vec2::new(arrow_slot, height),
        );
        let arrow_hovered = expandable && self.hovered_rect(arrow_rect);
        let row_hovered = self.hovered_rect(rect);
        let is_selected = self.tree_sel.as_ref().and_then(|s| s.as_deref()) == Some(id);

        // Background first so icons/text paint on top.
        if is_selected || row_hovered {
            let bg = if is_selected {
                theme::BROWSER_SELECTED
            } else {
                theme::HEADER_HOVER
            };
            self.round_rect(rect, self.s(3.0), bg);
        }

        // Chevron — expand/collapse only.
        if expandable {
            if arrow_hovered {
                self.hover_id = Some(arrow_id);
                self.want_capture = true;
                self.set_cursor(CursorIcon::Pointer);
            }
            if arrow_hovered && self.input.mouse_pressed {
                self.active_id = Some(arrow_id);
            }
            let arrow_clicked =
                self.active_id == Some(arrow_id) && arrow_hovered && self.input.mouse_released;
            if arrow_clicked {
                open = !open;
            }
            self.trees.insert(widget_id, open);

            let draw_rect = Rect::from_min_size(
                Vec2::new(
                    arrow_rect.min.x + (arrow_slot - arrow_draw) * 0.5,
                    rect.min.y + (height - arrow_draw) * 0.5,
                ),
                Vec2::splat(arrow_draw),
            );
            let arrow = if open { "chevron_down" } else { "chevron_right" };
            self.draw_icon_at(arrow, draw_rect, theme::TEXT_DIM, false);
        }

        let mut x = rect.min.x + self.s(2.0) + arrow_slot;
        if let Some(icon_id) = icon {
            let icon_rect = Rect::from_min_size(
                Vec2::new(x, rect.min.y + (height - icon_s) * 0.5),
                Vec2::splat(icon_s),
            );
            self.draw_icon_at(icon_id, icon_rect, theme::TEXT, false);
            x += icon_s + self.s(4.0);
        }

        let th = self.text_height();
        self.text(
            Vec2::new(x, rect.min.y + (height - th) * 0.5),
            label,
            theme::TEXT,
        );
        let mid_x = x + self.text_width(label) + self.s(4.0);

        self.push_id(id);
        let trailing_edge = rect.max.x - self.s(2.0);
        let row_y = rect.min.y + (height - btn) * 0.5;
        let mut row = TreeRow {
            ui: self,
            mid_x,
            trailing_x: trailing_edge,
            y: row_y,
            btn,
            trailing_mode: false,
            icon_hovered: false,
            trailing_left: trailing_edge,
        };
        extras(&mut row);
        let icon_hovered = row.icon_hovered;
        let trailing_left = row.trailing_left;

        let select_rect = Rect {
            min: Vec2::new(arrow_rect.max.x, rect.min.y),
            max: Vec2::new(trailing_left.max(arrow_rect.max.x), rect.max.y),
        };
        let select_hovered = self.hovered_rect(select_rect) && !icon_hovered && !arrow_hovered;

        if select_hovered {
            self.hover_id = Some(sel_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        if select_hovered && self.input.mouse_pressed {
            self.active_id = Some(sel_id);
        }
        let clicked =
            self.active_id == Some(sel_id) && select_hovered && self.input.mouse_released;
        if clicked {
            if let Some(sel) = self.tree_sel.as_mut() {
                *sel = Some(id.to_string());
            }
        }

        if expandable && open {
            let origin = self.layer().cursor + Vec2::new(indent, 0.0);
            let fill = (self.layer().fill_w - indent).max(0.0);
            let fill_h = self.available_size().y;
            let spacing = self.spacing;
            self.layers.push(crate::new_layer(
                LayoutDir::Vertical,
                origin,
                spacing,
                fill,
                fill_h,
                CrossAlign::Start,
            ));
            add(self);
            let used = self.layers.pop().unwrap().used;
            if used.x > 0.0 || used.y > 0.0 {
                self.allocate(Vec2::new(used.x + indent, used.y));
            }
        }
        self.pop_id();

        TreeResponse { open, clicked }
    }
}
