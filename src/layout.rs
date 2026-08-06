use glam::Vec2;

use crate::types::Rect;
use crate::{LayoutDir, Layer, Ui};

/// Row/column layout options.
#[derive(Clone, Copy, Debug)]
pub struct LayoutOpts {
    pub spacing: Option<f32>,
    pub cross_align: CrossAlign,
    pub main_align: MainAlign,
}

impl Default for LayoutOpts {
    fn default() -> Self {
        Self {
            spacing: None,
            cross_align: CrossAlign::Start,
            main_align: MainAlign::Start,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossAlign {
    Start,
    Center,
    End,
}

impl Default for CrossAlign {
    fn default() -> Self {
        Self::Start
    }
}

impl Default for MainAlign {
    fn default() -> Self {
        Self::Start
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainAlign {
    Start,
    Center,
    End,
    SpaceBetween,
}

pub(crate) struct GridCtx {
    pub cols: usize,
    pub col: usize,
    pub cell_w: f32,
    pub gap: f32,
    pub row_y: f32,
    pub row_h: f32,
    pub origin_x: f32,
    pub total_w: f32,
}

impl Ui {
    /// Fixed gap along the current layout axis.
    pub fn space(&mut self, pixels: f32) {
        let s = self.s(pixels);
        let layer = self.layer();
        match layer.dir {
            LayoutDir::Vertical => {
                layer.cursor.y += s;
                layer.used.y += s;
            }
            LayoutDir::Horizontal => {
                layer.cursor.x += s;
                layer.used.x += s;
            }
        }
    }

    /// Push trailing row items to the right edge (use before right-aligned widgets).
    pub fn spacer(&mut self) {
        let layer = self.layer();
        if matches!(layer.dir, LayoutDir::Horizontal) && layer.fill_w > 0.0 {
            layer.spacer_at = Some(layer.cursor.x);
            layer.trailing_w = 0.0;
            return;
        }
        if matches!(layer.dir, LayoutDir::Vertical) && layer.fill_h > 0.0 {
            let remaining = layer.fill_h - (layer.cursor.y - layer.origin.y);
            if remaining > 0.0 {
                layer.cursor.y += remaining;
                layer.used.y = layer.fill_h;
            }
        }
    }

    /// Horizontal row with optional layout options.
    pub fn row(&mut self, add: impl FnOnce(&mut Self)) {
        self.row_with(LayoutOpts::default(), add);
    }

    pub fn row_with(&mut self, opts: LayoutOpts, add: impl FnOnce(&mut Self)) {
        self.layout_with(LayoutDir::Horizontal, opts, add);
    }

    /// Vertical column with optional layout options.
    pub fn column(&mut self, add: impl FnOnce(&mut Self)) {
        self.column_with(LayoutOpts::default(), add);
    }

    pub fn column_with(&mut self, opts: LayoutOpts, add: impl FnOnce(&mut Self)) {
        self.layout_with(LayoutDir::Vertical, opts, add);
    }

    /// Alias for [`Ui::row`].
    pub fn same_line(&mut self, add: impl FnOnce(&mut Self)) {
        self.row(add);
    }

    pub fn with_layout(&mut self, opts: LayoutOpts, add: impl FnOnce(&mut Self)) {
        let dir = self.layer().dir;
        self.layout_impl(dir, opts, add);
    }

    /// Child region taking a fraction of remaining horizontal space (weight 1.0 = all remaining).
    pub fn flex(&mut self, fraction: f32, add: impl FnOnce(&mut Self)) {
        let parent = self.layers.len() - 1;
        let layer = &self.layers[parent];
        let fraction = fraction.clamp(0.0, 1.0);

        match layer.dir {
            LayoutDir::Horizontal => {
                let remaining_w =
                    (layer.fill_w - (layer.cursor.x - layer.origin.x)).max(0.0);
                let w = remaining_w * fraction;
                let origin = layer.cursor;
                let spacing = layer.spacing;
                let fill_h = layer.fill_h;
                let cross = layer.cross_align;

                self.layers.push(new_layer(
                    LayoutDir::Vertical,
                    origin,
                    spacing,
                    w,
                    fill_h,
                    cross,
                ));
                add(self);
                let child = self.layers.pop().unwrap();
                let h = child.used.y.max(child.row_height);
                self.finish_flex_child(parent, Vec2::new(w.max(child.used.x), h));
            }
            LayoutDir::Vertical => {
                let remaining_h =
                    (layer.fill_h - (layer.cursor.y - layer.origin.y)).max(0.0);
                let h = remaining_h * fraction;
                let origin = layer.cursor;
                let spacing = layer.spacing;
                let fill_w = layer.fill_w;

                self.layers.push(new_layer(
                    LayoutDir::Vertical,
                    origin,
                    spacing,
                    fill_w,
                    h,
                    CrossAlign::Start,
                ));
                add(self);
                let child = self.layers.pop().unwrap();
                let w = child.used.x;
                self.finish_flex_child(parent, Vec2::new(w, h.max(child.used.y)));
            }
        }
    }

    fn finish_flex_child(&mut self, parent_idx: usize, size: Vec2) {
        let layer = &mut self.layers[parent_idx];
        if size.x <= 0.0 && size.y <= 0.0 {
            return;
        }
        match layer.dir {
            LayoutDir::Horizontal => {
                let y = cross_y(layer, size.y);
                let rect = Rect::from_min_size(Vec2::new(layer.cursor.x, y), size);
                layer.cursor.x += size.x + layer.spacing;
                layer.row_height = layer.row_height.max(size.y);
                layer.used.x = layer.cursor.x - layer.origin.x - layer.spacing;
                layer.used.y = layer.row_height;
                let _ = rect;
            }
            LayoutDir::Vertical => {
                let rect = Rect::from_min_size(layer.cursor, size);
                layer.cursor.y += size.y + layer.spacing;
                layer.used.x = layer.used.x.max(size.x);
                layer.used.y = layer.cursor.y - layer.origin.y - layer.spacing;
                let _ = rect;
            }
        }
    }

    /// Inspector row: label + widget (fixed column widths from row start).
    pub fn property(&mut self, label: &str, label_frac: f32, add: impl FnOnce(&mut Self)) {
        let frac = label_frac.clamp(0.1, 0.9);
        self.row(|ui| {
            let fill_w = ui.layer().fill_w;
            let spacing = ui.spacing;
            let total = (fill_w - spacing).max(0.0);
            let w_label = total * frac;
            let w_widget = total - w_label;
            ui.flex_width(w_label, |ui| {
                ui.label(label);
            });
            ui.flex_width(w_widget, add);
        });
    }

    /// Child region with an exact width (horizontal row).
    pub fn flex_width(&mut self, width: f32, add: impl FnOnce(&mut Self)) {
        let parent = self.layers.len() - 1;
        let layer = &self.layers[parent];
        let w = width.max(0.0);

        if layer.dir == LayoutDir::Horizontal {
            let origin = layer.cursor;
            let spacing = layer.spacing;
            let fill_h = layer.fill_h;
            let cross = layer.cross_align;

            self.layers.push(new_layer(
                LayoutDir::Vertical,
                origin,
                spacing,
                w,
                fill_h,
                cross,
            ));
            add(self);
            let child = self.layers.pop().unwrap();
            let h = child.used.y.max(child.row_height);
            self.finish_flex_child(parent, Vec2::new(w.max(child.used.x), h));
        } else {
            self.flex(1.0, add);
        }
    }

    /// Fixed-column grid. Call [`Ui::grid_cell`] inside the closure for each item.
    pub fn grid(&mut self, cols: usize, add: impl FnOnce(&mut Self)) {
        self.grid_with(cols, None, add);
    }

    pub fn grid_with(
        &mut self,
        cols: usize,
        gap: Option<f32>,
        add: impl FnOnce(&mut Self),
    ) {
        let cols = cols.max(1);
        let gap = gap.map(|g| self.s(g)).unwrap_or(self.spacing);
        let avail = self.available_size();
        let total_w = avail.x;
        let cell_w = if total_w > 0.0 {
            (total_w - gap * (cols as f32 - 1.0)) / cols as f32
        } else {
            self.s(80.0)
        };
        let origin = self.layer().cursor;

        self.grid_stack.push(GridCtx {
            cols,
            col: 0,
            cell_w,
            gap,
            row_y: origin.y,
            row_h: 0.0,
            origin_x: origin.x,
            total_w,
        });
        add(self);
        let ctx = self.grid_stack.pop().unwrap();
        if ctx.row_h > 0.0 || ctx.col > 0 {
            let rows = if ctx.col > 0 {
                (ctx.col + cols - 1) / cols
            } else {
                0
            };
            let used_h = if rows > 0 {
                rows as f32 * ctx.row_h + (rows as f32 - 1.0) * gap
            } else {
                0.0
            };
            if used_h > 0.0 {
                self.allocate(Vec2::new(total_w.max(ctx.total_w), used_h));
            }
        }
    }

    /// One cell inside an active [`Ui::grid`].
    pub fn grid_cell(&mut self, add: impl FnOnce(&mut Self)) {
        if self.grid_stack.is_empty() {
            add(self);
            return;
        }
        let (cols, col_i, cell_w, gap, row_y, row_h, origin_x) = {
            let ctx = self.grid_stack.last().unwrap();
            (
                ctx.cols,
                ctx.col,
                ctx.cell_w,
                ctx.gap,
                ctx.row_y,
                ctx.row_h,
                ctx.origin_x,
            )
        };
        let col = col_i % cols;
        let (origin_y, row_h_start) = if col == 0 && col_i > 0 {
            (row_y + row_h + gap, 0.0)
        } else {
            (row_y, row_h)
        };
        let x = origin_x + col as f32 * (cell_w + gap);
        let origin = Vec2::new(x, origin_y);

        self.layers.push(new_layer(
            LayoutDir::Vertical,
            origin,
            self.spacing,
            cell_w,
            0.0,
            CrossAlign::Start,
        ));
        add(self);
        let child = self.layers.pop().unwrap();
        let new_row_h = row_h_start.max(child.used.y);

        let ctx = self.grid_stack.last_mut().unwrap();
        if col == 0 && col_i > 0 {
            ctx.row_y = origin_y;
            ctx.row_h = new_row_h;
        } else {
            ctx.row_h = ctx.row_h.max(new_row_h);
        }
        ctx.col += 1;
    }

    /// Width available for fill widgets in a vertical parent layer.
    pub(crate) fn child_fill_width(&self) -> f32 {
        let layer = self.layers.last().unwrap();
        if matches!(layer.dir, LayoutDir::Vertical) && layer.fill_w > 0.0 {
            layer.fill_w
        } else {
            0.0
        }
    }

    pub(crate) fn layout_impl(
        &mut self,
        dir: LayoutDir,
        opts: LayoutOpts,
        add: impl FnOnce(&mut Self),
    ) {
        let origin = self.layer().cursor;
        let spacing = opts.spacing.unwrap_or(self.spacing);
        let avail = self.available_size();
        let (fill_w, fill_h) = match dir {
            LayoutDir::Vertical => (avail.x, avail.y),
            LayoutDir::Horizontal => (avail.x, avail.y),
        };

        self.layers.push(new_layer(
            dir,
            origin,
            spacing,
            fill_w,
            fill_h,
            opts.cross_align,
        ));
        add(self);
        let used = self.layers.pop().unwrap().used;

        if opts.main_align == MainAlign::SpaceBetween && dir == LayoutDir::Horizontal {
            let layer_fill = fill_w;
            if layer_fill > used.x && used.x > 0.0 {
                // Space is consumed via spacer semantics — already laid out.
            }
        }

        if used.x > 0.0 || used.y > 0.0 {
            self.allocate(used);
        }
    }
}

pub(crate) fn cross_y(layer: &Layer, size_y: f32) -> f32 {
    match layer.cross_align {
        CrossAlign::Start => layer.cursor.y,
        CrossAlign::Center if layer.fill_h > 0.0 => {
            layer.origin.y + (layer.fill_h - size_y) * 0.5
        }
        CrossAlign::End if layer.fill_h > 0.0 => layer.origin.y + layer.fill_h - size_y,
        _ => layer.cursor.y,
    }
}

pub(crate) fn new_layer(
    dir: LayoutDir,
    origin: Vec2,
    spacing: f32,
    fill_w: f32,
    fill_h: f32,
    cross_align: CrossAlign,
) -> Layer {
    Layer {
        dir,
        cursor: origin,
        origin,
        spacing,
        used: Vec2::ZERO,
        row_height: 0.0,
        fill_w,
        fill_h,
        cross_align,
        spacer_at: None,
        trailing_w: 0.0,
    }
}
