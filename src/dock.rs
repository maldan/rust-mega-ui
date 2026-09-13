use glam::Vec2;

use crate::types::{CursorIcon, Rect};
use crate::{CrossAlign, LayoutDir, Ui, new_layer};

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
        self.round_rect(rect, 0.0, self.theme.dock.bg);
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
            let gap = ui.s(ui.theme.metrics.dock_split);
            let min = ui.s(MIN_PANE);
            let split_id = ui.current_id(&format!("#split{path}"));

            // Hit-test against last frame's split strip (approx from current ratio).
            let preview = split_rects(rect, *axis, *ratio, gap, min);
            let hovered = !ui.block_input
                && !ui.mouse_over_absorb()
                && preview.1.contains(ui.input.mouse_pos);
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
                ui.theme.dock.split_hot
            } else {
                ui.theme.dock.split
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
            // Not enough room for two `min`-sized panes — split evenly instead of
            // clamping (clamp's lower bound would exceed its upper bound and panic).
            let w1 = if avail < min * 2.0 {
                avail * 0.5
            } else {
                (avail * ratio).clamp(min, avail - min)
            };
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
            let h1 = if avail < min * 2.0 {
                avail * 0.5
            } else {
                (avail * ratio).clamp(min, avail - min)
            };
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

    let tab_h = ui.s(ui.theme.metrics.dock_tab_h);
    let pad = ui.s(6.0);
    let tab_pad_x = ui.s(10.0);
    let tab_gap = ui.s(1.0);
    let radius = ui.s(ui.theme.metrics.dock_tab_radius);
    let more_s = ui.s(16.0);

    // Pane chrome
    ui.round_rect(rect, 0.0, ui.theme.window.border);
    ui.round_rect(rect.inset(1.0), 0.0, ui.theme.window.body);

    let bar = Rect {
        min: rect.min + Vec2::new(1.0, 1.0),
        max: Vec2::new(rect.max.x - 1.0, rect.min.y + 1.0 + tab_h),
    };
    ui.round_rect(bar, 0.0, ui.theme.dock.tab_bar);

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
            ui.theme.dock.tab_active
        } else if hovered {
            ui.theme.dock.tab_hover
        } else {
            ui.theme.dock.tab
        };
        ui.round_rect_corners(tr, radius, color, true, false);

        // Thin accent on the active tab of the pane that last received a click —
        // helps orient which pane will receive keyboard shortcuts / paste.
        if is_active && ui.dock_focus == Some(path) {
            let accent = Rect {
                min: tr.min,
                max: Vec2::new(tr.max.x, tr.min.y + ui.s(2.0)),
            };
            ui.round_rect(accent, 0.0, ui.theme.dock.focus);
        }

        let th = ui.text_height();
        let text_col = if is_active {
            ui.theme.dock.tab_text_active
        } else {
            ui.theme.dock.tab_text
        };
        ui.text(
            Vec2::new(tr.min.x + tab_pad_x, tr.min.y + (tr.height() - th) * 0.5),
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
        ui.round_rect(more_rect, ui.s(3.0), ui.theme.dock.tab_hover);
    }
    let icon_r = Rect::from_min_size(more_rect.min + Vec2::splat(ui.s(2.0)), Vec2::splat(more_s));
    ui.draw_icon_at(
        "more_vert",
        icon_r,
        if more_hov {
            ui.theme.dock.tab_text_active
        } else {
            ui.theme.dock.tab_text
        },
        false,
    );
    ui.context_menu(&format!("#dock_more{path}"), more_hov, |ui| {
        if tabs.len() > 1 && ui.menu_item("close_tab", "Close Tab").clicked() {
            // Caller owns tab list; just signal via notify for now.
            ui.notify("Close tab");
        }
        if ui.menu_item("close_others", "Close Others").clicked() {
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
        CrossAlign::Start,
    ));
    add(ui, panel);
    ui.layers.pop();
    ui.pop_id();
    ui.pop_clip();
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- DockNode constructors ------------------------------------------------

    #[test]
    fn split_h_clamps_extreme_ratios_into_safe_range() {
        let n = DockNode::split_h(1.5, DockNode::leaf(&["a"]), DockNode::leaf(&["b"]));
        let DockNode::Split { ratio, axis, .. } = n else {
            panic!("expected Split");
        };
        assert_eq!(axis, DockAxis::Horizontal);
        assert!((0.05..=0.95).contains(&ratio));
        assert_eq!(ratio, 0.95);
    }

    #[test]
    fn split_v_clamps_negative_ratio_to_minimum() {
        let n = DockNode::split_v(-1.0, DockNode::leaf(&["a"]), DockNode::leaf(&["b"]));
        let DockNode::Split { ratio, axis, .. } = n else {
            panic!("expected Split");
        };
        assert_eq!(axis, DockAxis::Vertical);
        assert_eq!(ratio, 0.05);
    }

    #[test]
    fn leaf_starts_on_first_tab() {
        let n = DockNode::leaf(&["a", "b", "c"]);
        let DockNode::Leaf { tabs, active } = n else {
            panic!("expected Leaf");
        };
        assert_eq!(tabs, vec!["a", "b", "c"]);
        assert_eq!(active, 0);
    }

    // -- split_rects ------------------------------------------------------------

    fn full_rect(w: f32, h: f32) -> Rect {
        Rect::from_min_size(Vec2::ZERO, Vec2::new(w, h))
    }

    #[test]
    fn split_rects_horizontal_covers_rect_with_no_gaps_or_overlap() {
        let rect = full_rect(400.0, 200.0);
        let (a, s, b) = split_rects(rect, DockAxis::Horizontal, 0.5, 4.0, 64.0);
        // Panes + splitter tile the rect exactly, left to right.
        assert_eq!(a.min.x, rect.min.x);
        assert_eq!(a.max.x, s.min.x);
        assert_eq!(s.max.x, b.min.x);
        assert_eq!(b.max.x, rect.max.x);
        // Full height everywhere.
        for r in [a, s, b] {
            assert_eq!(r.min.y, rect.min.y);
            assert_eq!(r.max.y, rect.max.y);
        }
    }

    #[test]
    fn split_rects_vertical_covers_rect_with_no_gaps_or_overlap() {
        let rect = full_rect(200.0, 400.0);
        let (a, s, b) = split_rects(rect, DockAxis::Vertical, 0.5, 4.0, 64.0);
        assert_eq!(a.min.y, rect.min.y);
        assert_eq!(a.max.y, s.min.y);
        assert_eq!(s.max.y, b.min.y);
        assert_eq!(b.max.y, rect.max.y);
        for r in [a, s, b] {
            assert_eq!(r.min.x, rect.min.x);
            assert_eq!(r.max.x, rect.max.x);
        }
    }

    #[test]
    fn split_rects_respects_min_pane_width_at_extreme_ratio() {
        let rect = full_rect(400.0, 200.0);
        let min = 64.0;
        // Ratio pushed to the edge — first pane must not shrink below `min`.
        let (a, _s, b) = split_rects(rect, DockAxis::Horizontal, 0.0, 4.0, min);
        assert!(
            a.width() >= min - 1e-3,
            "pane a shrank below min: {}",
            a.width()
        );
        assert!(
            b.width() >= min - 1e-3,
            "pane b shrank below min: {}",
            b.width()
        );
    }

    #[test]
    fn split_rects_degenerates_to_half_when_too_small_for_two_min_panes() {
        // Available space smaller than 2×min — falls back to an even 50/50 split
        // instead of leaving one pane negative-width or overlapping.
        let rect = full_rect(100.0, 200.0);
        let min = 64.0;
        let (a, s, b) = split_rects(rect, DockAxis::Horizontal, 0.9, 4.0, min);
        let avail = (rect.width() - 4.0).max(0.0);
        assert!((a.width() - avail * 0.5).abs() < 1e-3);
        assert!(a.width() > 0.0);
        assert!(b.width() > 0.0);
        assert!(s.width() >= 0.0);
    }

    #[test]
    fn split_rects_never_produces_negative_width_panes() {
        for ratio in [-5.0, 0.0, 0.5, 1.0, 5.0] {
            let (a, _s, b) = split_rects(
                full_rect(300.0, 150.0),
                DockAxis::Horizontal,
                ratio,
                4.0,
                64.0,
            );
            assert!(a.width() >= 0.0, "ratio {ratio}: a width {}", a.width());
            assert!(b.width() >= 0.0, "ratio {ratio}: b width {}", b.width());
        }
    }
}
