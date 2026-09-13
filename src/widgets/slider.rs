use glam::Vec2;

use crate::types::{CursorIcon, Rect, Response};
use crate::{LayoutDir, Ui};

fn format_val(v: f32, min: f32, max: f32, step: f32) -> String {
    if step >= 1.0 {
        return format!("{:.0}", v);
    }
    let span = (max - min).abs();
    if span >= 50.0 {
        format!("{:.0}", v)
    } else if span >= 5.0 {
        format!("{:.1}", v)
    } else {
        format!("{:.2}", v)
    }
}

impl Ui {
    pub fn slider(
        &mut self,
        id: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
    ) -> Response {
        self.slider_stepped(id, value, range, 0.0)
    }

    /// Like [`Self::slider`], but drags (and the resulting value) snap to
    /// multiples of `step` — e.g. `step = 1.0` gives an integer-only slider
    /// that still drags smoothly, unlike [`Self::drag_float`]'s text field.
    /// `step <= 0.0` disables snapping (identical to [`Self::slider`]).
    pub fn slider_stepped(
        &mut self,
        id: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        step: f32,
    ) -> Response {
        let widget_id = self.current_id(id);
        let (min, max) = (*range.start(), *range.end());

        let th = self.text_height();
        let gap = self.s(4.0).round().max(2.0);
        let track_row_h = self.s(16.0).round().max(8.0);
        let total_h = th + gap + track_row_h;

        let fill_w = self.layer().fill_w;
        let filling = fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical);
        let width = if filling { fill_w } else { self.s(160.0) };
        let rect = if filling {
            self.allocate_fill_x(Vec2::new(width, total_h)).round_px()
        } else {
            self.allocate(Vec2::new(width, total_h)).round_px()
        };

        let labels_y = rect.min.y.round();
        let track_row = Rect::from_min_size(
            Vec2::new(rect.min.x, (rect.min.y + th + gap).round()),
            Vec2::new(rect.width(), track_row_h),
        );

        let hovered = self.hovered_rect(track_row) || self.hovered_rect(rect);
        if hovered {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        if self.hovered_rect(track_row) && self.input.mouse_pressed {
            self.active_id = Some(widget_id);
        }

        let active = self.active_id == Some(widget_id);
        let mut changed = false;
        if active && self.input.mouse_down {
            let t =
                ((self.input.mouse_pos.x - track_row.min.x) / track_row.width()).clamp(0.0, 1.0);
            let mut new = min + t * (max - min);
            if step > 0.0 {
                new = ((new / step).round() * step).clamp(min.min(max), min.max(max));
            }
            if (new - *value).abs() > f32::EPSILON {
                *value = new;
                changed = true;
            }
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }

        let t = if (max - min).abs() < f32::EPSILON {
            0.0
        } else {
            ((*value - min) / (max - min)).clamp(0.0, 1.0)
        };

        // Labels: min | value | max
        let min_s = format_val(min, min, max, step);
        let val_s = format_val(*value, min, max, step);
        let max_s = format_val(max, min, max, step);
        let val_w = self.text_width(&val_s);
        let max_w = self.text_width(&max_s);

        self.text(
            Vec2::new(rect.min.x.round(), labels_y),
            &min_s,
            self.theme.text.dim,
        );
        self.text(
            Vec2::new(
                (rect.min.x + (rect.width() - val_w) * 0.5).round(),
                labels_y,
            ),
            &val_s,
            self.theme.text.bright,
        );
        self.text(
            Vec2::new((rect.max.x - max_w).round(), labels_y),
            &max_s,
            self.theme.text.dim,
        );

        let track_h = (track_row_h * 0.35).round().max(2.0);
        let track_y = (track_row.min.y + (track_row_h - track_h) * 0.5).round();
        let track = Rect::from_min_size(
            Vec2::new(track_row.min.x, track_y),
            Vec2::new(track_row.width(), track_h),
        );
        let track_r = track_h * 0.5;

        let thumb_w = self.s(14.0).round().max(4.0);
        let thumb_h = (track_row_h - 2.0).round().max(4.0);
        let travel = (track_row.width() - thumb_w).max(0.0);
        let thumb_x = (track_row.min.x + travel * t).round();
        let thumb_y = (track_row.min.y + (track_row_h - thumb_h) * 0.5).round();
        let thumb = Rect::from_min_size(Vec2::new(thumb_x, thumb_y), Vec2::new(thumb_w, thumb_h));
        let thumb_r = thumb_h * 0.5;

        self.round_rect(track, track_r, self.theme.slider.track);
        if t > 0.0 {
            let fill_x1 = (thumb_x + thumb_w * 0.5).clamp(track.min.x, track.max.x);
            let fill = Rect {
                min: track.min,
                max: Vec2::new(fill_x1, track.max.y),
            };
            self.round_rect(fill, track_r, self.theme.slider.fill);
        }

        let thumb_color = if active || hovered {
            self.theme.slider.thumb_hot
        } else {
            self.theme.slider.thumb
        };
        self.round_rect(thumb, thumb_r, self.theme.button.border);
        self.round_rect(
            thumb.inset(1.0).round_px(),
            (thumb_r - 1.0).max(0.0),
            thumb_color,
        );

        Response {
            hovered,
            clicked: false,
            changed,
        }
    }
}
