use glam::Vec2;

use crate::{CrossAlign, LayoutDir, Rect, Ui, new_layer};

fn placement(anchor: Rect, size: Vec2, viewport: Vec2, gap: f32) -> Rect {
    let size = size.max(Vec2::ONE).min(viewport.max(Vec2::ONE));
    let below = (viewport.y - anchor.max.y - gap).max(0.0);
    let above = (anchor.min.y - gap).max(0.0);
    let y = if size.y <= below || below >= above {
        anchor.max.y + gap
    } else {
        anchor.min.y - gap - size.y
    };
    let origin = Vec2::new(anchor.max.x - size.x, y)
        .max(Vec2::ZERO)
        .min((viewport - size).max(Vec2::ZERO));
    Rect::from_min_size(origin, size)
}

impl Ui {
    /// A regular button opening a popover with arbitrary widgets.
    /// `size` is in UI points. Returns the content result while open.
    pub fn popover_button<R>(
        &mut self,
        id: &str,
        label: &str,
        open: &mut bool,
        size: Vec2,
        add: impl FnOnce(&mut Self) -> R,
    ) -> Option<R> {
        // The trigger remains interactive while the popover blocks its parent.
        self.popover_depth += 1;
        let arrow = if *open { "chevron_up" } else { "chevron_down" };
        let (response, anchor) = self.button_with_rect(id, label, Some(arrow));
        self.popover_depth -= 1;
        if response.clicked() {
            *open = !*open;
            self.request_repaint();
        }
        self.popover_at(id, anchor, open, size, add)
    }

    /// Anchored overlay with normal layout, input, clipping and arbitrary content.
    /// Anchor coordinates are screen pixels; `size` is in UI points.
    /// Opens above when needed, stays inside the viewport, closes on Escape or
    /// an outside press, and consumes the dismissing event.
    pub fn popover_at<R>(
        &mut self,
        id: &str,
        anchor: Rect,
        open: &mut bool,
        size: Vec2,
        add: impl FnOnce(&mut Self) -> R,
    ) -> Option<R> {
        let widget_id = self.current_id(id);
        if !self.enabled() {
            *open = false;
        }
        let rect = placement(
            anchor,
            size * self.scale(),
            self.input.viewport,
            self.s(4.0),
        );
        let dismiss = *open
            && (self.input.key_escape
                || (self.input.mouse_pressed
                    && !rect.contains(self.input.mouse_pos)
                    && !anchor.contains(self.input.mouse_pos)));
        if dismiss {
            *open = false;
            self.input.key_escape = false;
            self.input.mouse_pressed = false;
            self.input.mouse_released = false;
            self.focus_id = None;
            self.want_capture = true;
        }
        if !*open {
            self.popover_regions.remove(&widget_id);
            return None;
        }
        self.popover_regions.insert(widget_id, (rect, self.frame));
        self.want_capture = true;

        // Paint normal widgets into a detached draw list. This preserves their
        // scroll clips while composing the resulting panel above its parent.
        let parent_draw = std::mem::take(&mut self.draw_list);
        let parent_clip = std::mem::take(&mut self.clip_stack);
        let parent_absorb = std::mem::take(&mut self.mouse_absorb);
        let parent_overlay = std::mem::replace(&mut self.draw_to_overlay, false);
        let parent_block = std::mem::replace(&mut self.block_input, false);
        self.popover_depth += 1;
        self.push_clip(rect);
        self.round_rect(rect, self.s(3.0), self.theme.button.border);
        self.round_rect(rect.inset(1.0), self.s(2.0), self.theme.popup.bg);
        let pad = self.s(8.0);
        self.push_id(id);
        self.layers.push(new_layer(
            LayoutDir::Vertical,
            rect.min + Vec2::splat(pad),
            self.spacing,
            (rect.width() - pad * 2.0).max(1.0),
            (rect.height() - pad * 2.0).max(1.0),
            CrossAlign::Start,
        ));
        let result = add(self);
        self.layers.pop();
        self.pop_id();
        self.popover_depth -= 1;
        self.overlay.append(&mut self.draw_list);
        self.draw_list = parent_draw;
        self.clip_stack = parent_clip;
        self.mouse_absorb.extend(parent_absorb);
        self.absorb_rect(rect);
        self.draw_to_overlay = parent_overlay;
        self.block_input = parent_block;
        Some(result)
    }

    /// Focus a widget in the current ID scope, for example a popover's search.
    pub fn focus_widget(&mut self, id: &str) {
        self.focus_id = Some(self.current_id(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UiInput;

    #[test]
    fn opens_above_and_clamps_to_viewport() {
        let rect = placement(
            Rect::from_min_size(Vec2::new(290.0, 260.0), Vec2::new(80.0, 28.0)),
            Vec2::new(200.0, 160.0),
            Vec2::new(400.0, 300.0),
            4.0,
        );
        assert_eq!(rect.min, Vec2::new(170.0, 96.0));
        assert!(rect.max.x <= 400.0 && rect.max.y <= 300.0);
    }

    #[test]
    fn arbitrary_content_is_above_parent_and_escape_does_not_close_parent() {
        let mut ui = Ui::new();
        let mut open = true;
        ui.begin_frame(UiInput {
            viewport: Vec2::new(500.0, 400.0),
            ..Default::default()
        });
        let anchor = Rect::from_min_size(Vec2::new(30.0, 20.0), Vec2::new(80.0, 28.0));
        let mut text = String::new();
        ui.popover_at("test", anchor, &mut open, Vec2::new(220.0, 160.0), |ui| {
            ui.focus_widget("search");
            ui.text_input("search", &mut text);
            ui.button("inside", "Choose");
        });
        assert!(!ui.overlay.is_empty());
        assert!(ui.draw_list.is_empty());
        assert!(!ui.rect_hovered(Rect::from_min_size(Vec2::ZERO, Vec2::splat(500.0))));
        ui.end_frame();
        ui.begin_frame(UiInput {
            viewport: Vec2::new(500.0, 400.0),
            key_escape: true,
            ..Default::default()
        });
        assert!(
            ui.popover_at(
                "test",
                anchor,
                &mut open,
                Vec2::new(220.0, 160.0),
                |_| panic!("closed popup built")
            )
            .is_none()
        );
        assert!(!open && !ui.input.key_escape);
    }

    #[test]
    fn search_accepts_typing_and_outside_click_cannot_activate_parent() {
        let mut ui = Ui::new();
        let mut open = true;
        let mut search = String::new();
        let anchor = Rect::from_min_size(Vec2::new(100.0, 10.0), Vec2::new(80.0, 28.0));
        let viewport = Vec2::new(500.0, 400.0);
        ui.begin_frame(UiInput {
            viewport,
            ..Default::default()
        });
        ui.popover_at(
            "search_popup",
            anchor,
            &mut open,
            Vec2::new(220.0, 160.0),
            |ui| {
                ui.focus_widget("query");
                ui.text_input("query", &mut search);
            },
        );
        ui.end_frame();
        ui.begin_frame(UiInput {
            viewport,
            text: "moon".into(),
            ..Default::default()
        });
        ui.popover_at(
            "search_popup",
            anchor,
            &mut open,
            Vec2::new(220.0, 160.0),
            |ui| {
                ui.text_input("query", &mut search);
            },
        );
        ui.end_frame();
        assert_eq!(search, "moon");
        ui.begin_frame(UiInput {
            viewport,
            mouse_pos: Vec2::new(20.0, 12.0),
            mouse_pressed: true,
            mouse_down: true,
            ..Default::default()
        });
        assert!(!ui.button("parent", "Run").clicked());
        ui.popover_at(
            "search_popup",
            anchor,
            &mut open,
            Vec2::new(220.0, 160.0),
            |_| (),
        );
        assert!(!open);
        ui.end_frame();
        ui.begin_frame(UiInput {
            viewport,
            mouse_pos: Vec2::new(20.0, 12.0),
            mouse_released: true,
            ..Default::default()
        });
        assert!(!ui.button("parent", "Run").clicked());
    }
}
