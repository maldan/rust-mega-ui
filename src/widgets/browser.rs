//! Meta-widget: icon grid for file / asset browsers (Explorer-style).

use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Rect};
use crate::{LayoutDir, Ui};

/// One cell in a [`Ui::browser`] grid.
#[derive(Clone, Copy, Debug)]
pub struct BrowserItem<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub icon: &'a str,
    pub is_folder: bool,
}

/// Result of drawing a browser for one frame.
#[derive(Clone, Debug, Default)]
pub struct BrowserResponse {
    /// Item that was double-clicked (open / activate).
    pub opened: Option<String>,
    /// Selection changed this frame.
    pub changed: bool,
}

impl BrowserResponse {
    pub fn opened(&self) -> Option<&str> {
        self.opened.as_deref()
    }
}

pub(crate) struct BrowserClickState {
    pub last_id: Option<String>,
    pub age: f32,
}

const DBL_CLICK_SEC: f32 = 0.4;

impl Ui {
    /// Icon grid (Explorer / Unity Project-style).
    ///
    /// Each cell: large icon on top, name below.
    /// - Click selects (`selected` updated).
    /// - Double-click opens — id returned in [`BrowserResponse::opened`].
    pub fn browser(
        &mut self,
        id: &str,
        items: &[BrowserItem<'_>],
        selected: &mut Option<String>,
    ) -> BrowserResponse {
        let widget_id = self.current_id(id);
        let mut out = BrowserResponse::default();

        if let Some(st) = self.browser_clicks.get_mut(&widget_id) {
            st.age += self.input.dt;
            if st.age > DBL_CLICK_SEC {
                st.last_id = None;
            }
        }

        let fill_w = self.layer().fill_w;
        let width = if fill_w > 0.0 && matches!(self.layer().dir, LayoutDir::Vertical) {
            fill_w
        } else {
            self.s(200.0)
        };

        let gap = self.s(6.0);
        let cell_w = self.s(76.0);
        let icon_s = self.s(36.0);
        let th = self.text_height();
        let cell_pad = self.s(4.0);
        let cell_h = cell_pad + icon_s + self.s(4.0) + th + cell_pad;
        let cols = ((width + gap) / (cell_w + gap)).floor().max(1.0) as usize;

        self.push_id(id);
        for (row_i, chunk) in items.chunks(cols).enumerate() {
            let row = self.allocate(Vec2::new(width, cell_h));
            for (col, item) in chunk.iter().enumerate() {
                let i = row_i * cols + col;
                let cell = Rect::from_min_size(
                    Vec2::new(row.min.x + col as f32 * (cell_w + gap), row.min.y),
                    Vec2::new(cell_w, cell_h),
                );
                let cell_id = self.current_id(&format!("#item{i}"));
                let hovered = self.hovered_rect(cell);
                let is_sel = selected.as_deref() == Some(item.id);

                if hovered {
                    self.hover_id = Some(cell_id);
                    self.want_capture = true;
                    self.set_cursor(CursorIcon::Pointer);
                }
                if hovered && self.input.mouse_pressed {
                    self.active_id = Some(cell_id);
                }
                let clicked =
                    self.active_id == Some(cell_id) && hovered && self.input.mouse_released;

                if clicked {
                    let is_dbl = self
                        .browser_clicks
                        .get(&widget_id)
                        .map(|st| {
                            st.last_id.as_deref() == Some(item.id) && st.age <= DBL_CLICK_SEC
                        })
                        .unwrap_or(false);

                    if is_dbl {
                        out.opened = Some(item.id.to_string());
                        self.browser_clicks.insert(
                            widget_id,
                            BrowserClickState {
                                last_id: None,
                                age: DBL_CLICK_SEC + 1.0,
                            },
                        );
                    } else {
                        if selected.as_deref() != Some(item.id) {
                            *selected = Some(item.id.to_string());
                            out.changed = true;
                        }
                        self.browser_clicks.insert(
                            widget_id,
                            BrowserClickState {
                                last_id: Some(item.id.to_string()),
                                age: 0.0,
                            },
                        );
                    }
                }

                if is_sel || hovered {
                    let bg = if is_sel {
                        theme::BROWSER_SELECTED
                    } else {
                        theme::TABLE_ROW_HOVER
                    };
                    self.round_rect(cell, self.s(4.0), bg);
                }

                let icon_rect = Rect::from_min_size(
                    Vec2::new(
                        cell.min.x + (cell_w - icon_s) * 0.5,
                        cell.min.y + cell_pad,
                    ),
                    Vec2::splat(icon_s),
                );
                let icon_col = if item.is_folder {
                    theme::DOCK_TAB_TEXT_ACTIVE
                } else {
                    theme::TEXT
                };
                self.draw_icon_at(item.icon, icon_rect, icon_col, false);

                let label_max_w = cell_w - cell_pad * 2.0;
                let label = fit_label(self, item.label, label_max_w);
                let tw = self.text_width(&label);
                let text_col = if is_sel {
                    theme::DOCK_TAB_TEXT_ACTIVE
                } else {
                    theme::TEXT
                };
                let label_y = icon_rect.max.y + self.s(4.0);
                self.text(
                    Vec2::new(cell.min.x + (cell_w - tw) * 0.5, label_y),
                    &label,
                    text_col,
                );
            }
        }
        self.pop_id();

        out
    }
}

fn fit_label(ui: &Ui, text: &str, max_w: f32) -> String {
    if ui.text_width(text) <= max_w {
        return text.to_string();
    }
    let ell = "…";
    let ell_w = ui.text_width(ell);
    if ell_w >= max_w {
        return ell.to_string();
    }
    let mut end = text.len();
    while end > 0 {
        end = text.floor_char_boundary(end.saturating_sub(1));
        let candidate = format!("{}{ell}", &text[..end]);
        if ui.text_width(&candidate) <= max_w {
            return candidate;
        }
    }
    ell.to_string()
}
