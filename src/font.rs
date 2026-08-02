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
    px: f32,
    ascent: f32,
    line_height: f32,
    atlas: Vec<u8>,
    atlas_w: u32,
    atlas_h: u32,
    pack_x: u32,
    pack_y: u32,
    row_h: u32,
    glyphs: HashMap<(char, u32), Glyph>,
    dirty: bool,
    white_uv: [f32; 2],
}

fn px_key(px: f32) -> u32 {
    (px * 4.0).round().max(1.0) as u32
}

impl Font {
    pub fn load_default(px: f32) -> Self {
        let path = r"C:\Windows\Fonts\segoeui.ttf";
        Self::from_path(path, px).unwrap_or_else(|e| {
            panic!("failed to load default font `{path}`: {e}");
        })
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
        let (ascent, line_height) = match font.horizontal_line_metrics(px) {
            Some(m) => (m.ascent, m.new_line_size),
            None => (px * 0.8, px * 1.2),
        };
        let mut atlas = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE) as usize];
        atlas[0] = 255;
        let white_uv = [0.5 / ATLAS_SIZE as f32, 0.5 / ATLAS_SIZE as f32];
        let mut f = Self {
            font,
            px,
            ascent,
            line_height,
            atlas,
            atlas_w: ATLAS_SIZE,
            atlas_h: ATLAS_SIZE,
            pack_x: 1,
            pack_y: 0,
            row_h: 1,
            glyphs: HashMap::new(),
            dirty: true,
            white_uv,
        };
        for c in ' '..='~' {
            f.glyph(c, px);
        }
        f
    }

    pub fn px(&self) -> f32 {
        self.px
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn line_height_at(&self, px: f32) -> f32 {
        match self.font.horizontal_line_metrics(px) {
            Some(m) => m.new_line_size,
            None => px * 1.2,
        }
    }

    pub fn text_width(&self, text: &str) -> f32 {
        self.text_width_at(text, self.px)
    }

    pub fn text_width_at(&self, text: &str, px: f32) -> f32 {
        text.chars()
            .map(|c| self.font.metrics(c, px).advance_width)
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

    fn ascent_at(&self, px: f32) -> f32 {
        match self.font.horizontal_line_metrics(px) {
            Some(m) => m.ascent,
            None => px * 0.8,
        }
    }

    fn glyph(&mut self, c: char, px: f32) -> Glyph {
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
                self.atlas[dst..dst + gw as usize]
                    .copy_from_slice(&bitmap[src..src + gw as usize]);
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
        for c in text.chars() {
            let g = self.glyph(c, px);
            if g.w > 0.0 && g.h > 0.0 {
                let rect =
                    Rect::from_min_size(Vec2::new(pen + g.x, pos.y + g.y), Vec2::new(g.w, g.h));
                push_textured(out, rect, g.uv_min, g.uv_max, color, clip);
            }
            pen += g.advance;
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
    out.push(DrawCommand {
        rect,
        uv_min,
        uv_max,
        color,
        kind: 0.0,
        tex: 0,
    });
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
