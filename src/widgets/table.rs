use glam::Vec2;

use crate::theme;
use crate::types::Rect;
use crate::{new_layer, LayoutDir, Ui};

#[derive(Clone, Copy)]
pub struct TableColumn<'a> {
    pub name: &'a str,
    pub width: f32,
}

pub(crate) struct TableCtx {
    pub widths: Vec<f32>,
    pub col: usize,
    pub row_rect: Rect,
    pub row_i: usize,
}

impl Ui {
    pub fn table(&mut self, _id: &str, cols: &[TableColumn<'_>], add: impl FnOnce(&mut Self)) {
        let mut widths: Vec<f32> = cols.iter().map(|c| c.width.max(1.0)).collect();
        let sum: f32 = widths.iter().sum();
        let avail = self.available_size().x;
        // stretch columns across full available width (widths act as weights)
        if avail > 0.0 && sum > 0.0 {
            let scale = avail / sum;
            for w in &mut widths {
                *w *= scale;
            }
        }
        let total_w = widths.iter().sum::<f32>();
        let row_h = self.s(theme::TABLE_ROW_H);
        let pad = self.s(6.0);

        let header = self.allocate(Vec2::new(total_w, row_h));
        self.round_rect(header, 0.0, theme::TABLE_HEADER);
        {
            let mut x = header.min.x;
            let th = self.text_height();
            for (i, col) in cols.iter().enumerate() {
                let w = widths[i];
                self.text(
                    Vec2::new(x + pad, header.min.y + (row_h - th) * 0.5),
                    col.name,
                    theme::TITLE_TEXT,
                );
                x += w;
            }
        }

        self.table_stack.push(TableCtx {
            widths,
            col: 0,
            row_rect: Rect {
                min: Vec2::ZERO,
                max: Vec2::ZERO,
            },
            row_i: 0,
        });
        add(self);
        self.table_stack.pop();
    }

    pub fn table_row(&mut self, add: impl FnOnce(&mut Self)) {
        let Some(ctx) = self.table_stack.last() else {
            return;
        };
        let widths = ctx.widths.clone();
        let row_i = ctx.row_i;
        let total_w = widths.iter().sum::<f32>();
        let row_h = self.s(theme::TABLE_ROW_H);
        let row = self.allocate(Vec2::new(total_w.max(1.0), row_h));

        let hovered = self.hovered_rect(row);
        let bg = if hovered {
            theme::TABLE_ROW_HOVER
        } else if row_i % 2 == 0 {
            theme::TABLE_ROW
        } else {
            theme::TABLE_ROW_ALT
        };
        self.round_rect(row, 0.0, bg);

        if let Some(ctx) = self.table_stack.last_mut() {
            ctx.col = 0;
            ctx.row_rect = row;
            ctx.row_i += 1;
        }
        add(self);
    }

    pub fn table_cell(&mut self, add: impl FnOnce(&mut Self)) {
        let (cell, fill_w) = {
            let Some(ctx) = self.table_stack.last_mut() else {
                return;
            };
            if ctx.col >= ctx.widths.len() {
                return;
            }
            let w = ctx.widths[ctx.col];
            let x: f32 = ctx.widths[..ctx.col].iter().sum();
            let cell = Rect {
                min: Vec2::new(ctx.row_rect.min.x + x, ctx.row_rect.min.y),
                max: Vec2::new(ctx.row_rect.min.x + x + w, ctx.row_rect.max.y),
            };
            ctx.col += 1;
            (cell, (w - self.s(8.0)).max(0.0))
        };

        self.push_clip(cell);
        let pad = Vec2::new(self.s(4.0), self.s(2.0));
        self.layers.push(new_layer(
            LayoutDir::Horizontal,
            cell.min + pad,
            self.s(4.0),
            fill_w,
            (cell.height() - pad.y * 2.0).max(0.0),
        ));
        add(self);
        self.layers.pop();
        self.pop_clip();
    }
}
