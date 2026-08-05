use glam::Vec2;

use crate::theme;
use crate::types::Rect;
use crate::{new_layer, LayoutDir, Ui};

impl Ui {
    /// Group box: border around content with an optional title on the top edge.
    ///
    /// ```ignore
    /// ui.group("Audio", |ui| {
    ///     ui.knob("Output", &mut v, 0.0..=1.0);
    /// });
    /// ui.group("", |ui| { /* untitled */ });
    /// ```
    pub fn group(&mut self, title: &str, add: impl FnOnce(&mut Self)) {
        let id_key = if title.is_empty() { "__group" } else { title };
        let widget_id = self.current_id(id_key);
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

        let prev = self
            .group_sizes
            .get(&widget_id)
            .copied()
            .unwrap_or(Vec2::new(self.s(120.0), self.s(40.0)));

        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            (prev.x + pad * 2.0).max(self.s(80.0))
        };
        let height = prev.y + pad_top + pad + title_half;
        let rect = self.allocate(Vec2::new(width, height));

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

        self.push_id(id_key);
        self.layers.push(new_layer(
            LayoutDir::Vertical,
            content_origin,
            self.spacing,
            content_w,
            0.0,
        ));
        add(self);
        let used = self.layers.pop().unwrap().used;
        self.group_sizes.insert(
            widget_id,
            Vec2::new(used.x.max(1.0), used.y.max(1.0)),
        );
        self.pop_id();
    }
}
