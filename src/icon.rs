//! SVG icons rasterized into the font atlas (R8 alpha, same pipeline as glyphs).

use std::collections::HashMap;

use glam::Vec2;

use crate::font::{push_textured, Font};
use crate::theme;
use crate::types::{CursorIcon, Rect};
use crate::Ui;

#[derive(Clone, Copy)]
pub(crate) struct PackedIcon {
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

pub(crate) struct Icons {
    svg: HashMap<String, Vec<u8>>,
    packed: HashMap<(String, u32), PackedIcon>,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            svg: HashMap::new(),
            packed: HashMap::new(),
        }
    }
}

fn px_key(px: f32) -> u32 {
    px.round().clamp(8.0, 128.0) as u32
}

fn rasterize_svg(svg: &[u8], px: u32) -> Result<(u32, u32, Vec<u8>), String> {
    let opts = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg, &opts).map_err(|e| e.to_string())?;

    let size = tree.size();
    let scale = px as f32 / size.width().max(size.height()).max(1.0);
    let w = (size.width() * scale).round().max(1.0) as u32;
    let h = (size.height() * scale).round().max(1.0) as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h)
        .ok_or_else(|| "icon pixmap alloc failed".to_string())?;
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let mut alpha = Vec::with_capacity((w * h) as usize);
    for pix in pixmap.data().chunks_exact(4) {
        let r = pix[0] as u16;
        let g = pix[1] as u16;
        let b = pix[2] as u16;
        let a = pix[3] as u16;
        let lum = (r * 54 + g * 183 + b * 19) / 256;
        alpha.push(((lum * a) / 255) as u8);
    }
    Ok((w, h, alpha))
}

impl Icons {
    pub fn load(&mut self, id: impl Into<String>, svg: &[u8]) {
        self.svg.insert(id.into(), svg.to_vec());
    }

    pub fn load_many<'a>(&mut self, icons: impl IntoIterator<Item = (&'a str, &'a [u8])>) {
        for (id, svg) in icons {
            self.load(id, svg);
        }
    }

    pub fn resolve(&mut self, font: &mut Font, id: &str, size_px: f32) -> Option<PackedIcon> {
        let px = px_key(size_px);
        let key = (id.to_string(), px);
        if let Some(p) = self.packed.get(&key) {
            return Some(*p);
        }
        let svg = self.svg.get(id)?.clone();
        let (w, h, alpha) = rasterize_svg(&svg, px).ok()?;
        let (uv_min, uv_max) = font.pack_alpha(&alpha, w, h)?;
        let packed = PackedIcon { uv_min, uv_max };
        self.packed.insert(key, packed);
        Some(packed)
    }
}

impl Ui {
    /// Register SVG icons (bytes). Call once at startup.
    pub fn load_icons<'a>(&mut self, icons: impl IntoIterator<Item = (&'a str, &'a [u8])>) {
        self.icons.load_many(icons);
    }

    pub fn load_icon(&mut self, id: impl Into<String>, svg: &[u8]) {
        self.icons.load(id, svg);
    }

    /// Built-in set: folder, file, close, delete, plus, chevrons, check, …
    pub fn load_builtin_icons(&mut self) {
        self.load_icons([
            ("folder", include_bytes!("../icons/folder.svg").as_slice()),
            ("file", include_bytes!("../icons/file.svg").as_slice()),
            ("close", include_bytes!("../icons/close.svg").as_slice()),
            ("delete", include_bytes!("../icons/delete.svg").as_slice()),
            ("plus", include_bytes!("../icons/plus.svg").as_slice()),
            (
                "chevron_right",
                include_bytes!("../icons/chevron_right.svg").as_slice(),
            ),
            (
                "chevron_left",
                include_bytes!("../icons/chevron_left.svg").as_slice(),
            ),
            (
                "chevron_up",
                include_bytes!("../icons/chevron_up.svg").as_slice(),
            ),
            (
                "chevron_down",
                include_bytes!("../icons/chevron_down.svg").as_slice(),
            ),
            ("check", include_bytes!("../icons/check.svg").as_slice()),
            ("lock", include_bytes!("../icons/lock.svg").as_slice()),
            ("unlock", include_bytes!("../icons/unlock.svg").as_slice()),
            ("reset", include_bytes!("../icons/reset.svg").as_slice()),
            ("more_vert", include_bytes!("../icons/more_vert.svg").as_slice()),
        ]);
    }

    /// Clickable icon button. Returns `true` on click.
    pub(crate) fn icon_button(&mut self, id: &str, icon: &str, active: bool) -> bool {
        let enabled = self.enabled();
        let widget_id = self.current_id(id);
        let size = self.s(22.0);
        let rect = self.allocate(Vec2::splat(size));
        let hovered = enabled && self.hovered_rect(rect);
        if hovered {
            self.hover_id = Some(widget_id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        if hovered && self.input.mouse_pressed {
            self.active_id = Some(widget_id);
        }
        let pressed = self.active_id == Some(widget_id) && self.input.mouse_down;
        let clicked = enabled
            && self.active_id == Some(widget_id)
            && hovered
            && self.input.mouse_released;

        if hovered || active {
            self.round_rect(rect, self.s(3.0), theme::BTN_HOVER);
        }
        let color = if !enabled {
            theme::TEXT_DISABLED
        } else if pressed {
            theme::TEXT_BRIGHT
        } else if active {
            theme::ACCENT
        } else if hovered {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        let pad = self.s(3.0);
        self.draw_icon_at(
            icon,
            Rect {
                min: rect.min + Vec2::splat(pad),
                max: rect.max - Vec2::splat(pad),
            },
            color,
            false,
        );
        clicked
    }

    /// Layout an icon (`size` in UI points). Tinted with theme text color.
    pub fn icon(&mut self, id: &str, size: f32) {
        self.icon_colored(id, size, theme::TEXT);
    }

    pub fn icon_colored(&mut self, id: &str, size: f32, color: [f32; 4]) {
        let size = self.s(size.max(1.0));
        let rect = self.allocate(Vec2::splat(size));
        self.draw_icon_at(id, rect, color, false);
    }

    /// Draw icon inside `rect` without allocating layout.
    pub(crate) fn draw_icon_at(&mut self, id: &str, rect: Rect, color: [f32; 4], overlay: bool) {
        let px = rect.height().max(rect.width());
        let Some(packed) = self.icons.resolve(&mut self.font, id, px) else {
            return;
        };
        if overlay {
            push_textured(
                &mut self.overlay,
                rect,
                packed.uv_min,
                packed.uv_max,
                color,
                None,
            );
        } else {
            let clip = self.clip();
            push_textured(
                &mut self.draw_list,
                rect,
                packed.uv_min,
                packed.uv_max,
                color,
                clip,
            );
        }
    }
}
