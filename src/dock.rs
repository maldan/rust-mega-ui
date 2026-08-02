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
    let radius = ui.s(theme::BTN_RADIUS);

    ui.round_rect(rect, 0.0, theme::WIN_BORDER);
    ui.round_rect(rect.inset(1.0), 0.0, theme::WIN_BODY);

    let bar = Rect {
        min: rect.min + Vec2::new(1.0, 1.0),
        max: Vec2::new(rect.max.x - 1.0, rect.min.y + 1.0 + tab_h),
    };
    ui.round_rect(bar, 0.0, theme::DOCK_TAB_BAR);

    let n = tabs.len() as f32;
    let tab_w = (bar.width() / n).max(ui.s(40.0));
    for (i, title) in tabs.iter().enumerate() {
        let x = bar.min.x + i as f32 * tab_w;
        if x >= bar.max.x {
            break;
        }
        let tr = Rect {
            min: Vec2::new(x, bar.min.y),
            max: Vec2::new((x + tab_w).min(bar.max.x), bar.max.y),
        };
        let hovered = ui.hovered_rect(tr);
        if hovered {
            ui.want_capture = true;
            ui.set_cursor(CursorIcon::Pointer);
            ui.hover_id = Some(ui.current_id(&format!("#tab{path}_{i}")));
        }
        if hovered && ui.input.mouse_pressed {
            *active = i;
        }
        let color = if i == *active {
            theme::TAB_ACTIVE
        } else if hovered {
            theme::BTN_HOVER
        } else {
            theme::TAB
        };
        ui.round_rect(tr.inset(1.0), (radius - 1.0).max(0.0), color);
        let tw = ui.text_width(title);
        let th = ui.text_height();
        let tx = tr.min.x + (tr.width() - tw).max(0.0) * 0.5;
        let ty = tr.min.y + (tab_h - th) * 0.5;
        ui.text(Vec2::new(tx, ty), title, theme::TEXT);
    }

    let content = Rect {
        min: Vec2::new(rect.min.x + 1.0 + pad, bar.max.y + pad),
        max: rect.max - Vec2::splat(1.0 + pad),
    };
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
