use std::f32::consts::PI;

use glam::Vec2;

use crate::draw::{push_arc_cw, push_line, push_round_rect};
use crate::theme;
use crate::types::{CursorIcon, Rect, Response};
use crate::Ui;

/// Math angles: 0 = +x, CCW, y-up. Screen dir = (cos, -sin).
const KNOB_START: f32 = 225.0 * PI / 180.0;
const KNOB_SWEEP: f32 = 270.0 * PI / 180.0;
/// Screen pixels (pre-scale) for a full sweep. Shift = 4× finer.
const DRAG_PX: f32 = 400.0;

fn dir_from_t(t: f32) -> Vec2 {
    let a = KNOB_START - t.clamp(0.0, 1.0) * KNOB_SWEEP;
    Vec2::new(a.cos(), -a.sin())
}

/// `dy` is OS mouse delta (+y is down). Up increases value.
fn t_from_delta(t0: f32, dy: f32, pixels: f32) -> f32 {
    (t0 - dy / pixels.max(1.0)).clamp(0.0, 1.0)
}

impl Ui {
    /// Rotary knob. Label is drawn under the dial. Default orange fill.
    pub fn knob(
        &mut self,
        id: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
    ) -> Response {
        self.knob_colored(id, value, range, theme::KNOB_FILL)
    }

    /// Rotary knob with custom arc fill color.
    pub fn knob_colored(
        &mut self,
        id: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        fill: [f32; 4],
    ) -> Response {
        let enabled = self.enabled();
        let widget_id = self.current_id(id);
        let (min, max) = (*range.start(), *range.end());
        let span = max - min;

        let dial = self.s(theme::KNOB_SIZE);
        let th = self.text_height();
        let gap = self.s(6.0);
        let total = Vec2::new(dial, dial + gap + th);
        let rect = self.allocate(total);

        let center = Vec2::new(rect.min.x + dial * 0.5, rect.min.y + dial * 0.5);
        let r_out = dial * 0.5 - self.s(1.0);
        let arc_w = self.s(3.5);
        let r_in = r_out - arc_w;
        let face_r = r_in - self.s(3.0);

        let hit = Rect::from_min_size(rect.min, Vec2::splat(dial));
        let hovered = enabled && self.hovered_rect(hit);
        if hovered {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        if hovered && self.input.mouse_pressed {
            self.active_id = Some(widget_id);
            self.cursor_anchor = Some(self.input.mouse_pos);
        }

        let active = enabled && self.active_id == Some(widget_id);
        let mut changed = false;

        if active && self.input.mouse_down {
            self.want_capture = true;
            self.hide_cursor = true;
            self.request_repaint();
            if !self.input.mouse_pressed {
                let t0 = if span.abs() < f32::EPSILON {
                    0.0
                } else {
                    ((*value - min) / span).clamp(0.0, 1.0)
                };
                let pixels = self.s(if self.input.key_shift {
                    DRAG_PX * 4.0
                } else {
                    DRAG_PX
                });
                let t = t_from_delta(t0, self.input.mouse_delta.y, pixels);
                let new = min + t * span;
                if (new - *value).abs() > f32::EPSILON {
                    *value = new;
                    changed = true;
                }
            }
        }

        let t = if span.abs() < f32::EPSILON {
            0.0
        } else {
            ((*value - min) / span).clamp(0.0, 1.0)
        };

        let clip = if self.draw_to_overlay {
            None
        } else {
            self.clip()
        };
        let uv = self.font.white_uv();
        let border_w = self.s(2.0);
        let needle_w = self.s(2.0);
        {
            let list = if self.draw_to_overlay {
                &mut self.overlay
            } else {
                &mut self.draw_list
            };

            push_arc_cw(
                list,
                center,
                r_in,
                r_out,
                KNOB_START,
                KNOB_SWEEP,
                0.0,
                1.0,
                theme::KNOB_TRACK,
                uv,
                clip,
            );
            if t > 0.001 {
                push_arc_cw(
                    list,
                    center,
                    r_in,
                    r_out,
                    KNOB_START,
                    KNOB_SWEEP,
                    0.0,
                    t,
                    fill,
                    uv,
                    clip,
                );
            }

            push_round_rect(
                list,
                Rect::from_min_size(
                    center - Vec2::splat(face_r + border_w),
                    Vec2::splat((face_r + border_w) * 2.0),
                ),
                face_r + border_w,
                theme::KNOB_BORDER,
                true,
                true,
                uv,
                clip,
            );
            push_round_rect(
                list,
                Rect::from_min_size(center - Vec2::splat(face_r), Vec2::splat(face_r * 2.0)),
                face_r,
                theme::KNOB_FACE,
                true,
                true,
                uv,
                clip,
            );

            let d = dir_from_t(t);
            let a = center + d * (face_r * 0.22);
            let b = center + d * (face_r * 0.82);
            push_line(list, a, b, needle_w, theme::KNOB_INDICATOR, uv, clip);
        }

        let label_w = self.text_width(id);
        let label_x = rect.min.x + (dial - label_w) * 0.5;
        let label_y = rect.min.y + dial + gap;
        let label_color = if enabled {
            theme::TEXT
        } else {
            theme::TEXT_DISABLED
        };
        self.text(Vec2::new(label_x, label_y), id, label_color);

        Response {
            hovered,
            clicked: false,
            changed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_up_increases_value() {
        let t = t_from_delta(0.5, -50.0, 400.0);
        assert!((t - 0.625).abs() < 1e-5);
    }

    #[test]
    fn drag_clamps() {
        assert_eq!(t_from_delta(0.0, 500.0, 100.0), 0.0);
        assert_eq!(t_from_delta(1.0, -500.0, 100.0), 1.0);
    }

    #[test]
    fn press_does_not_jump() {
        let t = t_from_delta(0.3, 0.0, 400.0);
        assert!((t - 0.3).abs() < 1e-6);
    }
}
