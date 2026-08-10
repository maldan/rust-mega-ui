use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Response};
use crate::{CrossAlign, new_layer, LayoutDir, Ui};

impl Ui {
    /// Selectable card: click the background to select; children keep their own input.
    ///
    /// Selected state is a blue border only — fill stays neutral.
    ///
    /// ```ignore
    /// if ui.selectable("layer0", active == 0, |ui| {
    ///     ui.label("Layer 1");
    ///     ui.checkbox("vis", &mut visible);
    /// }).clicked() {
    ///     active = 0;
    /// }
    /// ```
    pub fn selectable(
        &mut self,
        id: &str,
        selected: bool,
        add: impl FnOnce(&mut Self),
    ) -> Response {
        let widget_id = self.current_id(id);
        let pad = self.s(10.0);
        let radius = self.s(theme::GROUP_RADIUS);
        let border = self.s(1.0).max(1.0);

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
        let height = prev.y + pad * 2.0;
        let frame = self.allocate(Vec2::new(width, height));

        let hovered = self.hovered_rect(frame);
        if hovered {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        // Claim press first; interactive children drawn below overwrite active_id.
        if hovered && self.input.mouse_pressed {
            self.active_id = Some(widget_id);
        }

        let bg = if hovered && !selected {
            // ~2% lighter than GROUP_BG — barely noticeable lift.
            [0.09, 0.09, 0.09, 1.0]
        } else {
            theme::GROUP_BG
        };
        let border_col = if selected {
            theme::ACCENT
        } else {
            theme::GROUP_BORDER
        };
        self.round_rect(frame, radius, border_col);
        self.round_rect(frame.inset(border), (radius - border).max(0.0), bg);

        let content_origin = Vec2::new(frame.min.x + pad, frame.min.y + pad);
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
        let used = self.layers.pop().unwrap().used;
        self.group_sizes.insert(
            widget_id,
            Vec2::new(used.x.max(1.0), used.y.max(1.0)),
        );
        self.pop_id();

        let clicked =
            self.active_id == Some(widget_id) && hovered && self.input.mouse_released;
        Response {
            hovered,
            clicked,
            changed: clicked,
        }
    }
}
