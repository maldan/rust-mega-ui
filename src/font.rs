use std::collections::HashMap;
use std::path::Path;

use glam::Vec2;

use super::types::{DrawCommand, Rect};

const ATLAS_SIZE: u32 = 1024;

#[derive(Clone, Copy)]
struct Glyph {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    advance: f32,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
}

pub struct Font {
    font: fontdue::Font,
    atlas: Vec<u8>,
    atlas_w: u32,
    atlas_h: u32,
    pack_x: u32,
    pack_y: u32,
    row_h: u32,
    glyphs: HashMap<(char, u32), Glyph>,
    dirty: bool,
    /// Atlas ran out of space last pack attempt — caller should reset next frame.
    overflow: bool,
    white_uv: [f32; 2],
}

fn px_key(px: f32) -> u32 {
    // Rasterization and measurement must use this same canonical size.
    // 1px buckets avoid packing near-identical sizes while dragging scale.
    px.round().clamp(1.0, 256.0) as u32
}

impl Font {
    pub fn load_default(px: f32) -> Self {
        let candidates: &[&str] = if cfg!(target_os = "windows") {
            &[
                r"C:\Windows\Fonts\segoeui.ttf",
                r"C:\Windows\Fonts\arial.ttf",
                r"C:\Windows\Fonts\calibri.ttf",
            ]
        } else if cfg!(target_os = "macos") {
            &[
                "/System/Library/Fonts/Supplemental/Arial.ttf",
                "/System/Library/Fonts/Supplemental/Helvetica.ttf",
                "/Library/Fonts/Arial.ttf",
            ]
        } else {
            &[
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
                "/usr/share/fonts/TTF/DejaVuSans.ttf",
            ]
        };

        let mut last_err = String::new();
        for path in candidates {
            match Self::from_path(path, px) {
                Ok(font) => return font,
                Err(e) => last_err = format!("{path}: {e}"),
            }
        }
        panic!("failed to load a default system font (last: {last_err})");
    }

    pub fn from_path(path: impl AsRef<Path>, px: f32) -> Result<Self, String> {
        let bytes = std::fs::read(path.as_ref()).map_err(|e| e.to_string())?;
        Self::from_bytes(&bytes, px)
    }

    pub fn from_bytes(bytes: &[u8], px: f32) -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| e.to_string())?;
        Ok(Self::from_fontdue(font, px))
    }

    fn from_fontdue(font: fontdue::Font, px: f32) -> Self {
        let mut atlas = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize];
        atlas[0] = 255;
        let white_uv = [0.5 / ATLAS_SIZE as f32, 0.5 / ATLAS_SIZE as f32];
        let mut f = Self {
            font,
            atlas,
            atlas_w: ATLAS_SIZE,
            atlas_h: ATLAS_SIZE,
            pack_x: 1,
            pack_y: 0,
            row_h: 1,
            glyphs: HashMap::new(),
            dirty: true,
            overflow: false,
            white_uv,
        };
        for c in ' '..='~' {
            f.glyph(c, px);
        }
        f
    }

    /// Wipe packed glyphs/icons slot and restart the packer (keeps the font face).
    pub fn reset_atlas(&mut self) {
        self.atlas.fill(0);
        self.atlas[0] = 255;
        self.pack_x = 1;
        self.pack_y = 0;
        self.row_h = 1;
        self.glyphs.clear();
        self.dirty = true;
        self.overflow = false;
    }

    /// Rasterize basic ASCII at `px` so the first frame after a reset has text ready.
    pub fn prewarm(&mut self, px: f32) {
        for c in ' '..='~' {
            let _ = self.glyph(c, px);
        }
    }

    pub fn take_overflow(&mut self) -> bool {
        let o = self.overflow;
        self.overflow = false;
        o
    }

    pub fn line_height_at(&self, px: f32) -> f32 {
        let raster_px = px_key(px) as f32;
        let ratio = px.clamp(1.0, 256.0) / raster_px;
        match self.font.horizontal_line_metrics(raster_px) {
            Some(m) => m.new_line_size * ratio,
            None => raster_px * 1.2 * ratio,
        }
    }

    pub fn text_width_at(&self, text: &str, px: f32) -> f32 {
        let raster_px = px_key(px) as f32;
        let ratio = px.clamp(1.0, 256.0) / raster_px;
        text.chars()
            .map(|c| self.font.metrics(c, raster_px).advance_width * ratio)
            .sum()
    }

    pub fn white_uv(&self) -> [f32; 2] {
        self.white_uv
    }

    pub fn atlas_size(&self) -> (u32, u32) {
        (self.atlas_w, self.atlas_h)
    }

    pub fn atlas_pixels(&self) -> &[u8] {
        &self.atlas
    }

    pub fn take_dirty(&mut self) -> bool {
        let d = self.dirty;
        self.dirty = false;
        d
    }

    /// Pack an R8 alpha bitmap into the atlas. Returns UVs or `None` if full.
    pub(crate) fn pack_alpha(
        &mut self,
        pixels: &[u8],
        gw: u32,
        gh: u32,
    ) -> Option<([f32; 2], [f32; 2])> {
        if gw == 0 || gh == 0 || pixels.len() < (gw * gh) as usize {
            return Some((self.white_uv, self.white_uv));
        }
        let pad = 1u32;
        if self.pack_x + gw + pad > self.atlas_w {
            self.pack_x = 1;
            self.pack_y += self.row_h + pad;
            self.row_h = 0;
        }
        if self.pack_y + gh + pad > self.atlas_h {
            self.overflow = true;
            return None;
        }
        let x = self.pack_x;
        let y = self.pack_y;
        for row in 0..gh {
            let src = (row * gw) as usize;
            let dst = ((y + row) * self.atlas_w + x) as usize;
            self.atlas[dst..dst + gw as usize].copy_from_slice(&pixels[src..src + gw as usize]);
        }
        self.pack_x += gw + pad;
        self.row_h = self.row_h.max(gh);
        self.dirty = true;
        let aw = self.atlas_w as f32;
        let ah = self.atlas_h as f32;
        Some((
            [x as f32 / aw, y as f32 / ah],
            [(x + gw) as f32 / aw, (y + gh) as f32 / ah],
        ))
    }

    fn ascent_at(&self, px: f32) -> f32 {
        match self.font.horizontal_line_metrics(px) {
            Some(m) => m.ascent,
            None => px * 0.8,
        }
    }

    fn glyph(&mut self, c: char, px: f32) -> Glyph {
        let px = px_key(px) as f32;
        let key = (c, px_key(px));
        if let Some(g) = self.glyphs.get(&key) {
            return *g;
        }
        let (metrics, bitmap) = self.font.rasterize(c, px);
        let gw = metrics.width as u32;
        let gh = metrics.height as u32;
        let ascent = self.ascent_at(px);

        let (uv_min, uv_max) = if gw == 0 || gh == 0 {
            (self.white_uv, self.white_uv)
        } else {
            let pad = 1u32;
            if self.pack_x + gw + pad > self.atlas_w {
                self.pack_x = 1;
                self.pack_y += self.row_h + pad;
                self.row_h = 0;
            }
            if self.pack_y + gh + pad > self.atlas_h {
                // Don't cache empties — next frame will reset the atlas.
                self.overflow = true;
                return Glyph {
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                    advance: metrics.advance_width,
                    uv_min: self.white_uv,
                    uv_max: self.white_uv,
                };
            }
            let x = self.pack_x;
            let y = self.pack_y;
            for row in 0..gh {
                let src = row as usize * metrics.width;
                let dst = ((y + row) * self.atlas_w + x) as usize;
                self.atlas[dst..dst + gw as usize].copy_from_slice(&bitmap[src..src + gw as usize]);
            }
            self.pack_x += gw + pad;
            self.row_h = self.row_h.max(gh);
            self.dirty = true;

            let aw = self.atlas_w as f32;
            let ah = self.atlas_h as f32;
            (
                [x as f32 / aw, y as f32 / ah],
                [(x + gw) as f32 / aw, (y + gh) as f32 / ah],
            )
        };

        let g = Glyph {
            x: metrics.xmin as f32,
            y: ascent - metrics.ymin as f32 - metrics.height as f32,
            w: gw as f32,
            h: gh as f32,
            advance: metrics.advance_width,
            uv_min,
            uv_max,
        };
        self.glyphs.insert(key, g);
        g
    }

    pub fn draw_text(
        &mut self,
        out: &mut Vec<DrawCommand>,
        pos: Vec2,
        text: &str,
        color: [f32; 4],
        px: f32,
        clip: Option<Rect>,
    ) {
        let mut pen = pos.x;
        let ratio = px.clamp(1.0, 256.0) / px_key(px) as f32;
        for c in text.chars() {
            let g = self.glyph(c, px);
            if g.w > 0.0 && g.h > 0.0 {
                let rect =
                    Rect::from_min_size(Vec2::new(pen + g.x * ratio, pos.y + g.y * ratio), Vec2::new(g.w * ratio, g.h * ratio));
                push_textured(out, rect, g.uv_min, g.uv_max, color, clip);
            }
            pen += g.advance * ratio;
        }
    }
}

pub fn push_textured(
    out: &mut Vec<DrawCommand>,
    rect: Rect,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    color: [f32; 4],
    clip: Option<Rect>,
) {
    let (rect, uv_min, uv_max) = match clip {
        Some(c) => {
            let Some(clipped) = rect.intersect(c) else {
                return;
            };
            let w = rect.width();
            let h = rect.height();
            if w <= 0.0 || h <= 0.0 {
                return;
            }
            let u0 = (clipped.min.x - rect.min.x) / w;
            let u1 = (clipped.max.x - rect.min.x) / w;
            let v0 = (clipped.min.y - rect.min.y) / h;
            let v1 = (clipped.max.y - rect.min.y) / h;
            let du = uv_max[0] - uv_min[0];
            let dv = uv_max[1] - uv_min[1];
            (
                clipped,
                [uv_min[0] + du * u0, uv_min[1] + dv * v0],
                [uv_min[0] + du * u1, uv_min[1] + dv * v1],
            )
        }
        None => (rect, uv_min, uv_max),
    };
    out.push(DrawCommand::solid(rect, uv_min, uv_max, color, 0.0, 0));
}

pub fn push_solid(
    out: &mut Vec<DrawCommand>,
    rect: Rect,
    color: [f32; 4],
    white_uv: [f32; 2],
    clip: Option<Rect>,
) {
    push_textured(out, rect, white_uv, white_uv, color, clip);
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn corner_color_at(colors: [[f32; 4]; 4], u: f32, v: f32) -> [f32; 4] {
    // colors: TL, TR, BR, BL
    let top = lerp4(colors[0], colors[1], u);
    let bot = lerp4(colors[3], colors[2], u);
    lerp4(top, bot, v)
}

/// Solid white-texel quad with per-corner colors (TL, TR, BR, BL).
/// When `clip` is set, the rect is intersected and corner colors are remapped.
pub fn push_gradient(
    out: &mut Vec<DrawCommand>,
    rect: Rect,
    colors: [[f32; 4]; 4],
    white_uv: [f32; 2],
    clip: Option<Rect>,
) {
    let w = rect.width();
    let h = rect.height();
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let (draw, cols) = match clip {
        Some(c) => {
            let Some(clipped) = rect.intersect(c) else {
                return;
            };
            let u0 = (clipped.min.x - rect.min.x) / w;
            let u1 = (clipped.max.x - rect.min.x) / w;
            let v0 = (clipped.min.y - rect.min.y) / h;
            let v1 = (clipped.max.y - rect.min.y) / h;
            let cols = [
                corner_color_at(colors, u0, v0),
                corner_color_at(colors, u1, v0),
                corner_color_at(colors, u1, v1),
                corner_color_at(colors, u0, v1),
            ];
            (clipped, cols)
        }
        None => (rect, colors),
    };
    out.push(DrawCommand::gradient(
        draw, white_uv, white_uv, cols, 0.0, 0,
    ));
}

#[cfg(test)]
mod scale_metrics_tests {
    use super::*;

    #[test]
    fn measurements_scale_smoothly_across_raster_size_boundaries() {
        let font = Font::load_default(14.0);
        let text = "Проверка wrapping and cursor alignment";
        let reference_width = font.text_width_at(text, 14.0) / 14.0;
        let reference_height = font.line_height_at(14.0) / 14.0;
        for px in [8.49, 8.51, 14.49, 14.51, 18.49, 18.51, 29.49, 29.51] {
            assert!((font.text_width_at(text, px) / px - reference_width).abs() < 0.001, "width at {px}");
            assert!((font.line_height_at(px) / px - reference_height).abs() < 0.001, "height at {px}");
        }
    }

    #[test]
    fn caret_measurement_matches_drawn_text_after_fractional_scale_changes() {
        let mut font = Font::load_default(14.0);
        let origin = Vec2::new(8.0, 8.0);
        for px in [14.0, 14.14, 14.42, 14.56, 14.84, 15.12, 14.28, 18.2] {
            for text in [
                "Redraw with high quality",
                "one   two  ",
                "привет мир",
                "e\u{301} 😀",
            ] {
                let expected = origin.x + font.text_width_at(text, px);
                let marker = font.glyph('|', px);
                let mut commands = Vec::new();
                font.draw_text(
                    &mut commands,
                    origin,
                    &format!("{text}|"),
                    [1.0; 4],
                    px,
                    None,
                );
                let actual = commands.last().unwrap().rect.min.x - marker.x * (px.clamp(1.0, 256.0) / px_key(px) as f32);
                assert!(
                    (actual - expected).abs() < 0.001,
                    "px={px}, text={text:?}: caret={expected}, drawn end={actual}"
                );
            }
        }
    }

    #[test]
    fn glyph_metrics_do_not_depend_on_first_fractional_size_in_cache_bucket() {
        let mut font = Font::load_default(14.14);
        for px in [14.42, 14.14, 18.2, 18.4] {
            for c in ['a', ' ', 'я', '😀'] {
                let expected = font.font.metrics(c, px_key(px) as f32).advance_width;
                let actual = font.glyph(c, px).advance;
                assert!(
                    (actual - expected).abs() < 0.001,
                    "px={px}, char={c:?}: expected={expected}, actual={actual}"
                );
            }
        }
    }
}
