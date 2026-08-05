use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect};
use crate::{new_layer, LayoutDir, Ui};

const MIN_PANE: f32 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockAxis {
    /// Left | Right
    Horizontal,
    /// Top / Bottom
    Vertical,
}

#[derive(Clone, Debug)]
pub enum DockNode {
    Leaf {
        tabs: Vec<String>,
        active: usize,
    },
    Split {
        axis: DockAxis,
        /// Share of the first child (0..1).
        ratio: f32,
        first: Box<DockNode>,
        second: Box<DockNode>,
    },
}

impl DockNode {
    pub fn leaf(tabs: &[&str]) -> Self {
        Self::Leaf {
            tabs: tabs.iter().map(|s| (*s).to_string()).collect(),
            active: 0,
        }
    }

    pub fn split_h(ratio: f32, left: Self, right: Self) -> Self {
        Self::Split {
            axis: DockAxis::Horizontal,
            ratio: ratio.clamp(0.05, 0.95),
            first: Box::new(left),
            second: Box::new(right),
        }
    }

    pub fn split_v(ratio: f32, top: Self, bottom: Self) -> Self {
        Self::Split {
            axis: DockAxis::Vertical,
            ratio: ratio.clamp(0.05, 0.95),
            first: Box::new(top),
            second: Box::new(bottom),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DockState {
    pub root: DockNode,
}

impl DockState {
    pub fn new(root: DockNode) -> Self {
        Self { root }
    }
}

impl Ui {
    /// Fill `size` (or full viewport if zero) with a dock tree. `add` is called for the
    /// active tab id of every leaf.
    pub fn dock_space(
        &mut self,
        id: &str,
        size: Vec2,
        state: &mut DockState,
        mut add: impl FnMut(&mut Self, &str),
    ) {
        let size = if size.x > 0.0 && size.y > 0.0 {
            size
        } else {
            self.input.viewport
        };
        let rect = self.allocate(size);
        self.push_id(id);
        self.round_rect(rect, 0.0, theme::DOCK_BG);
        layout_node(self, &mut state.root, rect, 0, &mut add);
        self.pop_id();
    }
}

fn layout_node(
    ui: &mut Ui,
    node: &mut DockNode,
    rect: Rect,
    path: u32,
    add: &mut impl FnMut(&mut Ui, &str),
) {
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return;
    }

    match node {
        DockNode::Leaf { tabs, active } => {
            draw_leaf(ui, tabs, active, rect, path, add);
        }
        DockNode::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let gap = ui.s(theme::DOCK_SPLIT);
            let min = ui.s(MIN_PANE);
            let split_id = ui.current_id(&format!("#split{path}"));

            // Hit-test against last frame's split strip (approx from current ratio).
            let preview = split_rects(rect, *axis, *ratio, gap, min);
            let hovered =
                !ui.block_input && !ui.mouse_over_absorb() && preview.1.contains(ui.input.mouse_pos);
            if hovered {
                ui.want_capture = true;
                ui.set_cursor(match *axis {
                    DockAxis::Horizontal => CursorIcon::ResizeEw,
                    DockAxis::Vertical => CursorIcon::ResizeNs,
                });
            }
            if hovered && ui.input.mouse_pressed {
                ui.active_id = Some(split_id);
            }
            if ui.active_id == Some(split_id) {
                ui.want_capture = true;
                ui.set_cursor(match *axis {
                    DockAxis::Horizontal => CursorIcon::ResizeEw,
                    DockAxis::Vertical => CursorIcon::ResizeNs,
                });
                *ratio = match *axis {
                    DockAxis::Horizontal => {
                        ((ui.input.mouse_pos.x - rect.min.x) / rect.width()).clamp(0.08, 0.92)
                    }
                    DockAxis::Vertical => {
                        ((ui.input.mouse_pos.y - rect.min.y) / rect.height()).clamp(0.08, 0.92)
                    }
                };
            }

            let (a_rect, split_rect, b_rect) = split_rects(rect, *axis, *ratio, gap, min);
            let split_color = if ui.active_id == Some(split_id) || hovered {
                theme::DOCK_SPLIT_HOT
            } else {
                theme::DOCK_SPLIT_COL
            };
            ui.round_rect(split_rect, 0.0, split_color);

            layout_node(ui, first, a_rect, path * 2 + 1, add);
            layout_node(ui, second, b_rect, path * 2 + 2, add);
        }
    }
}

fn split_rects(rect: Rect, axis: DockAxis, ratio: f32, gap: f32, min: f32) -> (Rect, Rect, Rect) {
    match axis {
        DockAxis::Horizontal => {
            let avail = (rect.width() - gap).max(0.0);
            let mut w1 = (avail * ratio).clamp(min.min(avail * 0.5), (avail - min).max(0.0));
            if avail < min * 2.0 {
                w1 = avail * 0.5;
            }
            let a = Rect::from_min_size(rect.min, Vec2::new(w1, rect.height()));
            let s = Rect::from_min_size(
                Vec2::new(rect.min.x + w1, rect.min.y),
                Vec2::new(gap, rect.height()),
            );
            let b = Rect {
                min: Vec2::new(s.max.x, rect.min.y),
                max: rect.max,
            };
            (a, s, b)
        }
        DockAxis::Vertical => {
            let avail = (rect.height() - gap).max(0.0);
            let mut h1 = (avail * ratio).clamp(min.min(avail * 0.5), (avail - min).max(0.0));
            if avail < min * 2.0 {
                h1 = avail * 0.5;
            }
            let a = Rect::from_min_size(rect.min, Vec2::new(rect.width(), h1));
            let s = Rect::from_min_size(
                Vec2::new(rect.min.x, rect.min.y + h1),
                Vec2::new(rect.width(), gap),
            );
            let b = Rect {
                min: Vec2::new(rect.min.x, s.max.y),
                max: rect.max,
            };
            (a, s, b)
        }
    }
}

fn draw_leaf(
    ui: &mut Ui,
    tabs: &[String],
    active: &mut usize,
    rect: Rect,
    path: u32,
    add: &mut impl FnMut(&mut Ui, &str),
) {
    if tabs.is_empty() {
        return;
    }
    *active = (*active).min(tabs.len() - 1);

    let tab_h = ui.s(theme::DOCK_TAB_H);
    let pad = ui.s(6.0);
    let tab_pad_x = ui.s(10.0);
    let tab_gap = ui.s(1.0);
    let radius = ui.s(theme::DOCK_TAB_RADIUS);
    let more_s = ui.s(16.0);

    // Pane chrome
    ui.round_rect(rect, 0.0, theme::WIN_BORDER);
    ui.round_rect(rect.inset(1.0), 0.0, theme::WIN_BODY);

    let bar = Rect {
        min: rect.min + Vec2::new(1.0, 1.0),
        max: Vec2::new(rect.max.x - 1.0, rect.min.y + 1.0 + tab_h),
    };
    ui.round_rect(bar, 0.0, theme::DOCK_TAB_BAR);

    // Content-sized tabs, left-aligned (not stretched).
    let more_w = more_s + ui.s(8.0);
    let tabs_right = (bar.max.x - more_w).max(bar.min.x);
    let mut x = bar.min.x + ui.s(2.0);
    for (i, title) in tabs.iter().enumerate() {
        let tw = ui.text_width(title);
        let tab_w = (tw + tab_pad_x * 2.0).max(ui.s(36.0));
        if x + tab_w > tabs_right {
            break;
        }
        let is_active = i == *active;
        // Active tab flush with content; inactive slightly inset.
        let tr = Rect {
            min: Vec2::new(
                x,
                if is_active {
                    bar.min.y
                } else {
                    bar.min.y + ui.s(2.0)
                },
            ),
            max: Vec2::new(x + tab_w, bar.max.y),
        };

        let hovered = ui.hovered_rect(tr);
        if hovered {
            ui.want_capture = true;
            ui.set_cursor(CursorIcon::Pointer);
            ui.hover_id = Some(ui.current_id(&format!("#tab{path}_{i}")));
        }
        if hovered && ui.input.mouse_pressed {
            *active = i;
            ui.dock_focus = Some(path);
        }

        let color = if is_active {
            theme::DOCK_TAB_ACTIVE
        } else if hovered {
            theme::DOCK_TAB_HOVER
        } else {
            theme::DOCK_TAB
        };
        ui.round_rect_corners(tr, radius, color, true, false);

        let th = ui.text_height();
        let text_col = if is_active {
            theme::DOCK_TAB_TEXT_ACTIVE
        } else {
            theme::DOCK_TAB_TEXT
        };
        ui.text(
            Vec2::new(
                tr.min.x + tab_pad_x,
                tr.min.y + (tr.height() - th) * 0.5,
            ),
            title,
            text_col,
        );

        x = tr.max.x + tab_gap;
    }

    // ⋮ menu on the right
    let more_rect = Rect::from_min_size(
        Vec2::new(bar.max.x - more_w, bar.min.y + (tab_h - more_s) * 0.5),
        Vec2::splat(more_s + ui.s(4.0)),
    );
    let more_hov = ui.hovered_rect(more_rect);
    if more_hov {
        ui.want_capture = true;
        ui.set_cursor(CursorIcon::Pointer);
        ui.round_rect(more_rect, ui.s(3.0), theme::DOCK_TAB_HOVER);
    }
    let icon_r = Rect::from_min_size(
        more_rect.min + Vec2::splat(ui.s(2.0)),
        Vec2::splat(more_s),
    );
    ui.draw_icon_at(
        "more_vert",
        icon_r,
        if more_hov {
            theme::DOCK_TAB_TEXT_ACTIVE
        } else {
            theme::DOCK_TAB_TEXT
        },
        false,
    );
    ui.context_menu(&format!("#dock_more{path}"), more_hov, |ui| {
        if tabs.len() > 1 {
            if ui.menu_item("Close Tab").clicked() {
                // Caller owns tab list; just signal via notify for now.
                ui.notify("Close tab");
            }
        }
        if ui.menu_item("Close Others").clicked() {
            ui.notify("Close others");
        }
    });

    // Clicking content focuses the pane.
    let content = Rect {
        min: Vec2::new(rect.min.x + 1.0 + pad, bar.max.y + pad),
        max: rect.max - Vec2::splat(1.0 + pad),
    };
    if ui.hovered_rect(content) && ui.input.mouse_pressed {
        ui.dock_focus = Some(path);
    }

    if content.width() < 1.0 || content.height() < 1.0 {
        return;
    }

    let panel = tabs[*active].as_str();
    ui.push_clip(content);
    ui.push_id(panel);
    ui.layers.push(new_layer(
        LayoutDir::Vertical,
        content.min,
        ui.spacing,
        content.width(),
        content.height(),
    ));
    add(ui, panel);
    ui.layers.pop();
    ui.pop_id();
    ui.pop_clip();
}
