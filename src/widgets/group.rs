use glam::Vec2;

use crate::theme;
use crate::types::Rect;
use crate::{CrossAlign, LayoutDir, Ui, new_layer};

impl Ui {
    /// Group box: border around content with an optional title on the top edge.
    ///
    /// ```ignore
    /// ui.group("audio", "Audio", |ui| {
    ///     ui.knob("output", &mut v, 0.0..=1.0);
    /// });
    /// ui.group("untitled", "", |ui| { /* untitled */ });
    /// ```
    pub fn group(&mut self, id: &str, title: &str, add: impl FnOnce(&mut Self)) {
        let widget_id = self.current_id(id);
        let pad = self.s(10.0);
        let radius = self.s(theme::GROUP_RADIUS);
        let border = self.s(1.0).max(1.0);
        let th = self.text_height();
        let has_title = !title.is_empty();
        let title_half = if has_title { th * 0.5 } else { 0.0 };
        let title_pad_x = self.s(6.0);
        // Title sits on the top edge and hangs into the frame — clear it + extra air.
        let pad_top = if has_title {
            pad + title_half + self.s(4.0)
        } else {
            pad
        };

        let sc = self.scale.max(1e-4);
        let prev_pt = self
            .group_sizes
            .get(&widget_id)
            .copied()
            .unwrap_or(Vec2::new(120.0, 40.0));
        let prev = prev_pt * sc;

        let fill_w = self.layer().fill_w;
        let hug_w = (prev.x + pad * 2.0).max(self.s(80.0));
        // Filled parent (dock leaf, scroll): match fill_w so the group shrinks with the pane.
        // `fill_w.max(hug_w)` ratchets: stretch stores used.x as hug, then shrink never wins.
        // Inside a node, hug content — stretching to fill_w grows forever on zoom-out.
        let filling = fill_w > 0.0
            && matches!(self.layer().dir, LayoutDir::Vertical)
            && self.current_node_id.is_none();
        let width = if filling { fill_w } else { hug_w };
        let height = prev.y + pad_top + pad + title_half;
        let rect = if filling {
            self.allocate_fill_x(Vec2::new(width, height))
        } else {
            self.allocate(Vec2::new(width, height))
        };

        // Frame sits so the top border runs through the title midline.
        let frame = Rect {
            min: Vec2::new(rect.min.x, rect.min.y + title_half),
            max: rect.max,
        };

        // Subtle fill + border (outer stroke via inset).
        self.round_rect(frame, radius, theme::GROUP_BORDER);
        self.round_rect(
            frame.inset(border),
            (radius - border).max(0.0),
            theme::GROUP_BG,
        );

        if has_title {
            let tw = self.text_width(title);
            let label_w = tw + title_pad_x * 2.0;
            let label_x = frame.min.x + self.s(12.0);
            let label_rect = Rect::from_min_size(
                Vec2::new(label_x, frame.min.y - title_half),
                Vec2::new(label_w, th),
            );
            // Cut the border under the title (match window so it doesn't float).
            self.round_rect(label_rect, 0.0, theme::WIN_BODY);
            self.text(
                Vec2::new(label_x + title_pad_x, label_rect.min.y),
                title,
                theme::TITLE_TEXT,
            );
        }

        let content_origin = Vec2::new(frame.min.x + pad, frame.min.y + pad_top);
        let content_w = (frame.width() - pad * 2.0).max(0.0);

        self.push_id(id);
        self.layers.push(new_layer(
            LayoutDir::Vertical,
            content_origin,
            self.spacing,
            content_w,
            0.0,
            CrossAlign::Start,
        ));
        add(self);
        let child = self.layers.pop().unwrap();
        let content = child.hug_x.max(child.used.x);
        self.group_sizes.insert(
            widget_id,
            Vec2::new((content / sc).max(1.0), (child.used.y / sc).max(1.0)),
        );
        if content + pad * 2.0 > width + 1.0 {
            self.request_repaint();
        }
        self.pop_id();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UiInput;
    use crate::{CrossAlign, LayoutDir, Ui, new_layer};

    fn pane_used(ui: &mut Ui, fill_w: f32) -> f32 {
        ui.layers.push(new_layer(
            LayoutDir::Vertical,
            Vec2::ZERO,
            ui.spacing,
            fill_w,
            0.0,
            CrossAlign::Start,
        ));
        let mut v = 0.5;
        ui.group("lens", "Lens", |ui| {
            ui.slider("focal", &mut v, 0.0..=1.0);
        });
        ui.layers.pop().unwrap().used.x
    }

    #[test]
    fn group_in_filled_pane_shrinks_after_stretch() {
        let mut ui = Ui::new();
        let mut input = UiInput::default();
        input.viewport = Vec2::new(800.0, 600.0);

        ui.begin_frame(input.clone());
        let wide = pane_used(&mut ui, 400.0);
        ui.end_frame();
        assert!((wide - 400.0).abs() < 1.0, "wide={wide}");

        ui.begin_frame(input);
        let narrow = pane_used(&mut ui, 180.0);
        ui.end_frame();
        assert!(
            (narrow - 180.0).abs() < 1.0,
            "group kept stretched width: {narrow}"
        );
    }
}
