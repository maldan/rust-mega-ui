//! HSV color conversion + color edit popup.

use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, DrawCommand, Id, Rect, Response};
use crate::{LayoutDir, Ui};

/// Host texture slot for the color-picker SV field (`kind = 1`).
/// Bind pixels from [`crate::Ui::color_sv_atlas`].
pub const TEX_SLOT_COLOR_SV: u32 = 2;

const SV_TEX_SIZE: u32 = 128;

#[derive(Clone, Copy)]
pub(crate) struct ColorEditState {
    pub open: bool,
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

/// CPU RGBA atlas for the saturation–value field (regenerated when hue changes).
#[derive(Clone)]
pub(crate) struct ColorSvAtlas {
    pub pixels: Vec<u8>,
    pub size: u32,
    hue_key: i32,
    pub dirty: bool,
}

impl Default for ColorSvAtlas {
    fn default() -> Self {
        let mut atlas = Self {
            pixels: Vec::new(),
            size: 0,
            hue_key: -1,
            dirty: false,
        };
        atlas.ensure(0.0);
        atlas
    }
}

impl ColorSvAtlas {
    fn ensure(&mut self, hue: f32) {
        let key = (hue.rem_euclid(1.0) * 4096.0).round() as i32;
        if self.hue_key == key && !self.pixels.is_empty() {
            return;
        }
        let n = SV_TEX_SIZE;
        self.pixels.resize((n * n * 4) as usize, 0);
        for y in 0..n {
            let v = 1.0 - (y as f32 + 0.5) / n as f32;
            for x in 0..n {
                let s = (x as f32 + 0.5) / n as f32;
                let (r, g, b) = hsv_to_rgb(hue, s, v);
                let i = ((y * n + x) * 4) as usize;
                self.pixels[i] = (r * 255.0).round().clamp(0.0, 255.0) as u8;
                self.pixels[i + 1] = (g * 255.0).round().clamp(0.0, 255.0) as u8;
                self.pixels[i + 2] = (b * 255.0).round().clamp(0.0, 255.0) as u8;
                self.pixels[i + 3] = 255;
            }
        }
        self.size = n;
        self.hue_key = key;
        self.dirty = true;
    }
}

pub(crate) fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let v = max;
    let s = if max <= 1e-6 { 0.0 } else { d / max };
    let h = if d <= 1e-6 {
        0.0
    } else if max == r {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h.fract().rem_euclid(1.0), s.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
}

pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor();
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match i as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

fn rgba(r: f32, g: f32, b: f32) -> [f32; 4] {
    [r, g, b, 1.0]
}

impl Ui {
    /// Color swatch + popup editor (HSV square, hue, alpha).
    pub fn color_edit(&mut self, id: &str, color: &mut [f32; 4]) -> Response {
        let widget_id = self.current_id(id);
        let swatch_h = self.s(22.0);
        let swatch_w = self.s(36.0);
        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w.min(self.s(220.0)).max(swatch_w)
        } else {
            swatch_w
        };
        let rect = self.allocate(Vec2::new(width, swatch_h));
        let swatch = Rect::from_min_size(rect.min, Vec2::new(swatch_w.min(width), swatch_h));

        let mut st = self.color_edits.get(&widget_id).copied().unwrap_or_else(|| {
            let (h, s, v) = rgb_to_hsv(color[0], color[1], color[2]);
            ColorEditState {
                open: false,
                h,
                s,
                v,
            }
        });

        let hovered = self.hovered_rect(swatch);
        if hovered {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        // Only the swatch toggles open — not while dragging popup controls.
        let swatch_press = hovered && self.input.mouse_pressed;
        if swatch_press {
            self.active_id = Some(widget_id);
        }
        let swatch_click =
            self.active_id == Some(widget_id) && hovered && self.input.mouse_released;
        if swatch_click {
            st.open = !st.open;
            if st.open {
                let (h, s, v) = rgb_to_hsv(color[0], color[1], color[2]);
                st.h = h;
                st.s = s;
                st.v = v;
            }
        }

        let r = self.s(theme::BTN_RADIUS);
        self.round_rect(swatch, r, theme::BTN_BORDER);
        let inner = swatch.inset(1.0);
        // One swatch — opaque preview of RGB (alpha edited in popup).
        self.round_rect(
            inner,
            (r - 1.0).max(0.0),
            [color[0], color[1], color[2], 1.0],
        );

        let mut changed = false;
        let mut popup_hov = false;
        if st.open {
            let (c, ph) = self.color_popup(widget_id, swatch, &mut st, color);
            changed = c;
            popup_hov = ph;
        }

        self.color_edits.insert(widget_id, st);

        Response {
            hovered: hovered || popup_hov,
            clicked: swatch_click,
            changed,
        }
    }

    fn color_popup(
        &mut self,
        id: Id,
        anchor: Rect,
        st: &mut ColorEditState,
        color: &mut [f32; 4],
    ) -> (bool, bool) {
        let pad = self.s(8.0);
        let sv = self.s(160.0);
        let hue_w = self.s(16.0);
        let gap = self.s(6.0);
        let alpha_h = self.s(14.0);
        let popup_w = pad * 2.0 + sv + gap + hue_w;
        let popup_h = pad * 2.0 + sv + gap + alpha_h;

        let mut origin = Vec2::new(anchor.min.x, anchor.max.y + self.s(2.0));
        if origin.x + popup_w > self.input.viewport.x {
            origin.x = (self.input.viewport.x - popup_w).max(0.0);
        }
        if origin.y + popup_h > self.input.viewport.y {
            origin.y = (anchor.min.y - popup_h - self.s(2.0)).max(0.0);
        }
        let popup = Rect::from_min_size(origin, Vec2::new(popup_w, popup_h));
        let radius = self.s(theme::BTN_RADIUS);
        self.round_rect_overlay(popup, radius, theme::BTN_BORDER);
        self.round_rect_overlay(popup.inset(1.0), (radius - 1.0).max(0.0), theme::POPUP_BG);
        self.absorb_rect(popup);

        let popup_hov = popup.contains(self.input.mouse_pos);
        if popup_hov {
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }

        let sv_rect = Rect::from_min_size(origin + Vec2::splat(pad), Vec2::splat(sv));
        let hue_rect = Rect::from_min_size(
            Vec2::new(sv_rect.max.x + gap, sv_rect.min.y),
            Vec2::new(hue_w, sv),
        );
        let alpha_rect = Rect::from_min_size(
            Vec2::new(sv_rect.min.x, sv_rect.max.y + gap),
            Vec2::new(sv + gap + hue_w, alpha_h),
        );

        let mut changed = false;

        self.draw_sv_field(sv_rect, st.h);
        self.stroke_rect_overlay(sv_rect, theme::BTN_BORDER);

        self.draw_hue_bar(hue_rect);
        self.stroke_rect_overlay(hue_rect, theme::BTN_BORDER);

        // Alpha bar
        self.draw_checker_overlay(alpha_rect, 0.0);
        let (cr, cg, cb) = hsv_to_rgb(st.h, st.s, st.v);
        let fill_a = Rect {
            min: alpha_rect.min,
            max: Vec2::new(
                alpha_rect.min.x + alpha_rect.width() * color[3].clamp(0.0, 1.0),
                alpha_rect.max.y,
            ),
        };
        self.round_rect_overlay(fill_a, 0.0, [cr, cg, cb, 1.0]);
        self.stroke_rect_overlay(alpha_rect, theme::BTN_BORDER);

        let sv_id = id.child("sv");
        let hue_id = id.child("hue");
        let alpha_id = id.child("alpha");

        if sv_rect.contains(self.input.mouse_pos) && self.input.mouse_pressed {
            self.active_id = Some(sv_id);
        }
        if hue_rect.contains(self.input.mouse_pos) && self.input.mouse_pressed {
            self.active_id = Some(hue_id);
        }
        if alpha_rect.contains(self.input.mouse_pos) && self.input.mouse_pressed {
            self.active_id = Some(alpha_id);
        }

        let dragging_popup = matches!(
            self.active_id,
            Some(aid) if aid == sv_id || aid == hue_id || aid == alpha_id
        );

        if self.active_id == Some(sv_id) && self.input.mouse_down {
            st.s = ((self.input.mouse_pos.x - sv_rect.min.x) / sv_rect.width()).clamp(0.0, 1.0);
            st.v = (1.0 - (self.input.mouse_pos.y - sv_rect.min.y) / sv_rect.height())
                .clamp(0.0, 1.0);
            changed = true;
            self.want_capture = true;
            self.needs_repaint = true;
        }
        if self.active_id == Some(hue_id) && self.input.mouse_down {
            st.h = ((self.input.mouse_pos.y - hue_rect.min.y) / hue_rect.height()).clamp(0.0, 1.0);
            changed = true;
            self.want_capture = true;
            self.needs_repaint = true;
        }
        if self.active_id == Some(alpha_id) && self.input.mouse_down {
            color[3] =
                ((self.input.mouse_pos.x - alpha_rect.min.x) / alpha_rect.width()).clamp(0.0, 1.0);
            changed = true;
            self.want_capture = true;
            self.needs_repaint = true;
        }

        // Cursor on SV / hue
        let cx = sv_rect.min.x + st.s * sv_rect.width();
        let cy = sv_rect.min.y + (1.0 - st.v) * sv_rect.height();
        let mark = Rect::from_min_size(Vec2::new(cx - 4.0, cy - 4.0), Vec2::splat(8.0));
        self.round_rect_overlay(mark, 3.0, [1.0, 1.0, 1.0, 1.0]);
        self.round_rect_overlay(mark.inset(1.5), 2.0, [0.0, 0.0, 0.0, 1.0]);

        let hy = hue_rect.min.y + st.h * hue_rect.height();
        self.round_rect_overlay(
            Rect {
                min: Vec2::new(hue_rect.min.x - 1.0, hy - 1.0),
                max: Vec2::new(hue_rect.max.x + 1.0, hy + 2.0),
            },
            0.0,
            [1.0, 1.0, 1.0, 1.0],
        );

        if changed {
            let (r, g, b) = hsv_to_rgb(st.h, st.s, st.v);
            color[0] = r;
            color[1] = g;
            color[2] = b;
        }

        // Close only on press strictly outside popup+swatch (not while dragging).
        if self.input.mouse_pressed
            && !popup_hov
            && !anchor.contains(self.input.mouse_pos)
            && !dragging_popup
        {
            st.open = false;
        }

        (changed, popup_hov)
    }

    /// SV field: one textured quad from the CPU HSV atlas.
    fn draw_sv_field(&mut self, rect: Rect, hue: f32) {
        self.color_sv.ensure(hue);
        self.overlay.push(DrawCommand::solid(
            rect,
            [0.0, 0.0],
            [1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            1.0,
            TEX_SLOT_COLOR_SV,
        ));
    }

    /// Hue strip: 6 gradient segments (linear RGB is correct within each 60° sector).
    fn draw_hue_bar(&mut self, rect: Rect) {
        const STOPS: [f32; 7] = [0.0, 1.0 / 6.0, 2.0 / 6.0, 3.0 / 6.0, 4.0 / 6.0, 5.0 / 6.0, 1.0];
        let h = rect.height().max(1.0);
        for i in 0..6 {
            let y0 = rect.min.y + STOPS[i] * h;
            let y1 = rect.min.y + STOPS[i + 1] * h;
            let (r0, g0, b0) = hsv_to_rgb(STOPS[i], 1.0, 1.0);
            let (r1, g1, b1) = hsv_to_rgb(STOPS[i + 1], 1.0, 1.0);
            let top = rgba(r0, g0, b0);
            let bot = rgba(r1, g1, b1);
            self.gradient_overlay(
                Rect {
                    min: Vec2::new(rect.min.x, y0),
                    max: Vec2::new(rect.max.x, y1),
                },
                [top, top, bot, bot],
            );
        }
    }

    fn draw_checker_overlay(&mut self, rect: Rect, radius: f32) {
        self.round_rect_overlay(rect, radius, [0.22, 0.22, 0.22, 1.0]);
        let cell = self.s(6.0).max(4.0);
        let mut y = rect.min.y;
        let mut row = 0i32;
        while y < rect.max.y {
            let y1 = (y + cell).min(rect.max.y);
            let mut x = rect.min.x;
            let mut col = 0i32;
            while x < rect.max.x {
                let x1 = (x + cell).min(rect.max.x);
                if (row + col) % 2 == 0 {
                    self.round_rect_overlay(
                        Rect {
                            min: Vec2::new(x, y),
                            max: Vec2::new(x1, y1),
                        },
                        0.0,
                        [0.32, 0.32, 0.32, 1.0],
                    );
                }
                x = x1;
                col += 1;
            }
            y = y1;
            row += 1;
        }
    }

    fn stroke_rect_overlay(&mut self, rect: Rect, color: [f32; 4]) {
        let t = 1.0;
        self.round_rect_overlay(
            Rect {
                min: rect.min,
                max: Vec2::new(rect.max.x, rect.min.y + t),
            },
            0.0,
            color,
        );
        self.round_rect_overlay(
            Rect {
                min: Vec2::new(rect.min.x, rect.max.y - t),
                max: rect.max,
            },
            0.0,
            color,
        );
        self.round_rect_overlay(
            Rect {
                min: rect.min,
                max: Vec2::new(rect.min.x + t, rect.max.y),
            },
            0.0,
            color,
        );
        self.round_rect_overlay(
            Rect {
                min: Vec2::new(rect.max.x - t, rect.min.y),
                max: rect.max,
            },
            0.0,
            color,
        );
    }
}
