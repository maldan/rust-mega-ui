mod dock;
mod draw;
mod font;
mod icon;
mod layout;
mod plot_view;
pub(crate) mod theme;
mod types;
mod widgets;
mod window;

pub use dock::{DockNode, DockState};
pub use layout::{CrossAlign, LayoutOpts, MainAlign};
pub use plot_view::PlotView;
pub use types::{CursorIcon, DrawCommand, Id, Rect, Response, UiInput, UiOutput};
pub use widgets::{
    AnimationCurve, BrowserItem, BrowserResponse, CurveEditorResponse, CurvePoint, CurvePreset,
    ToastKind, apply_preset, ease_in_out, sample_curve,
};
pub use widgets::color_picker::TEX_SLOT_COLOR_SV;
pub use widgets::label::TextStyle;
pub use widgets::scroll::ScrollAxes;
pub use widgets::table::TableColumn;
pub use window::Window;

use std::collections::HashMap;
use std::path::Path;

use glam::Vec2;

use draw::{push_line_segment, push_polyline, push_round_rect};
use font::Font;
use icon::Icons;
use layout::{GridCtx, cross_y};
pub(crate) use layout::new_layer;
use plot_view::PlotView as PlotViewState;
use widgets::curve::CurveEditState;
use widgets::table::TableCtx;

fn font_px_key(px: f32) -> u32 {
    px.round().clamp(1.0, 256.0) as u32
}

pub(crate) struct Layer {
    pub dir: LayoutDir,
    pub cursor: Vec2,
    pub origin: Vec2,
    pub spacing: f32,
    pub used: Vec2,
    pub row_height: f32,
    pub fill_w: f32,
    /// Max content height from origin (0 = unbounded).
    pub fill_h: f32,
    /// Cross-axis alignment for horizontal rows (ignored in vertical).
    pub cross_align: CrossAlign,
    /// Horizontal row: spacer defers trailing items to the right edge.
    pub spacer_at: Option<f32>,
    pub trailing_w: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayoutDir {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy)]
pub(crate) struct WinState {
    pub pos: Vec2,
    pub size: Vec2,
    pub collapsed: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct ScrollState {
    pub offset: Vec2,
    pub target: Vec2,
    pub content: Vec2,
}

pub struct Ui {
    pub(crate) input: UiInput,
    pub(crate) draw_list: Vec<DrawCommand>,
    pub(crate) id_stack: Vec<Id>,
    pub(crate) layers: Vec<Layer>,
    pub(crate) windows: HashMap<Id, WinState>,
    pub(crate) win_order: Vec<Id>,
    pub(crate) win_rects: HashMap<Id, Rect>,
    pub(crate) window_layers: Vec<(Id, Vec<DrawCommand>)>,
    pub(crate) modal_layer: Vec<DrawCommand>,
    pub(crate) modal_id: Option<Id>,
    /// True if a modal was shown last frame / this frame (blocks background input).
    pub(crate) modal_open: bool,
    pub(crate) modal_request_close: bool,
    pub(crate) hover_window: Option<Id>,
    pub(crate) focus_window: Option<Id>,
    pub(crate) block_input: bool,
    pub(crate) headers: HashMap<Id, bool>,
    pub(crate) trees: HashMap<Id, bool>,
    pub(crate) selects: HashMap<Id, bool>,
    pub(crate) vec_locks: HashMap<Id, bool>,
    pub(crate) color_edits: HashMap<Id, widgets::color_picker::ColorEditState>,
    pub(crate) color_sv: widgets::color_picker::ColorSvAtlas,
    pub(crate) context_menu: Option<(Id, Vec2)>,
    pub(crate) toasts: Vec<widgets::toast::Toast>,
    /// Dock leaf path that last received a click (Unity-style focus strip).
    pub(crate) dock_focus: Option<u32>,
    pub(crate) scrolls: HashMap<Id, ScrollState>,
    pub(crate) edits: HashMap<Id, widgets::edit::EditState>,
    pub(crate) num_bufs: HashMap<Id, String>,
    pub(crate) tabs: HashMap<Id, usize>,
    pub(crate) tab_content_sizes: HashMap<Id, Vec2>,
    pub(crate) browser_clicks: HashMap<Id, widgets::browser::BrowserClickState>,
    pub(crate) table_stack: Vec<TableCtx>,
    pub(crate) clip_stack: Vec<Rect>,
    pub(crate) overlay: Vec<DrawCommand>,
    pub(crate) mouse_absorb: Option<Rect>,
    pub(crate) hover_id: Option<Id>,
    pub(crate) active_id: Option<Id>,
    pub(crate) focus_id: Option<Id>,
    pub(crate) drag_grab: Option<Vec2>,
    pub(crate) want_capture: bool,
    pub(crate) cursor_icon: CursorIcon,
    pub(crate) spacing: f32,
    pub(crate) base_spacing: f32,
    pub(crate) scale: f32,
    pub(crate) enabled_stack: Vec<bool>,
    pub(crate) font: Font,
    pub(crate) icons: Icons,
    pub(crate) scroll_wheel_target: Option<Id>,
    pub(crate) scroll_hover: Option<Id>,
    /// Set by widgets that eat the wheel this frame (knob, nested scroll, …).
    pub(crate) scroll_consumed: bool,
    pub(crate) needs_repaint: bool,
    pub(crate) clipboard_out: Option<String>,
    pub(crate) menu_bar_stack: Vec<widgets::menu::MenuBarCtx>,
    pub(crate) menu_stack: Vec<widgets::menu::MenuPopupCtx>,
    pub(crate) menu_bar_open: HashMap<Id, Option<Id>>,
    pub(crate) menu_sub_open: HashMap<Id, Option<Id>>,
    pub(crate) menu_popup_size: HashMap<Id, Vec2>,
    pub(crate) button_sizes: HashMap<Id, Vec2>,
    pub(crate) group_sizes: HashMap<Id, Vec2>,
    pub(crate) grid_stack: Vec<GridCtx>,
    pub(crate) plot_views: HashMap<Id, PlotViewState>,
    pub(crate) curve_edits: HashMap<Id, CurveEditState>,
    /// Last frame's popup absorb — blocks window focus on press before menus rebuild.
    pub(crate) overlay_block: Option<Rect>,
    /// When true, `text` / `round_rect` paint into the overlay list.
    pub(crate) draw_to_overlay: bool,
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}

impl Ui {
    pub fn new() -> Self {
        Self {
            input: UiInput::default(),
            draw_list: Vec::new(),
            id_stack: Vec::new(),
            layers: Vec::new(),
            windows: HashMap::new(),
            win_order: Vec::new(),
            win_rects: HashMap::new(),
            window_layers: Vec::new(),
            modal_layer: Vec::new(),
            modal_id: None,
            modal_open: false,
            modal_request_close: false,
            hover_window: None,
            focus_window: None,
            block_input: false,
            headers: HashMap::new(),
            trees: HashMap::new(),
            selects: HashMap::new(),
            vec_locks: HashMap::new(),
            color_edits: HashMap::new(),
            color_sv: widgets::color_picker::ColorSvAtlas::default(),
            context_menu: None,
            toasts: Vec::new(),
            dock_focus: None,
            scrolls: HashMap::new(),
            edits: HashMap::new(),
            num_bufs: HashMap::new(),
            tabs: HashMap::new(),
            tab_content_sizes: HashMap::new(),
            browser_clicks: HashMap::new(),
            table_stack: Vec::new(),
            clip_stack: Vec::new(),
            overlay: Vec::new(),
            mouse_absorb: None,
            hover_id: None,
            active_id: None,
            focus_id: None,
            drag_grab: None,
            want_capture: false,
            cursor_icon: CursorIcon::Default,
            spacing: 6.0,
            base_spacing: 6.0,
            scale: 1.0,
            enabled_stack: Vec::new(),
            font: Font::load_default(theme::FONT_SIZE),
            icons: Icons::default(),
            scroll_wheel_target: None,
            scroll_hover: None,
            scroll_consumed: false,
            needs_repaint: false,
            clipboard_out: None,
            menu_bar_stack: Vec::new(),
            menu_stack: Vec::new(),
            menu_bar_open: HashMap::new(),
            menu_sub_open: HashMap::new(),
            menu_popup_size: HashMap::new(),
            button_sizes: HashMap::new(),
            group_sizes: HashMap::new(),
            grid_stack: Vec::new(),
            plot_views: HashMap::new(),
            curve_edits: HashMap::new(),
            overlay_block: None,
            draw_to_overlay: false,
        }
    }

    /// UI scale (1.0 = 100%). Affects sizes, spacing, and font.
    pub fn set_scale(&mut self, scale: f32) {
        let scale = scale.clamp(0.5, 3.0);
        let old_key = font_px_key(theme::FONT_SIZE * self.scale);
        let new_key = font_px_key(theme::FONT_SIZE * scale);
        self.scale = scale;
        self.spacing = self.base_spacing * self.scale;
        // New font pixel size → drop old glyphs/icons so the atlas doesn't fill up
        // with every intermediate slider value (which made text vanish).
        if old_key != new_key {
            self.reset_font_atlas();
        }
    }

    fn reset_font_atlas(&mut self) {
        self.font.reset_atlas();
        self.icons.clear_packed();
        self.font.prewarm(self.font_size());
        self.needs_repaint = true;
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub(crate) fn s(&self, v: f32) -> f32 {
        v * self.scale
    }

    pub(crate) fn font_size(&self) -> f32 {
        theme::FONT_SIZE * self.scale
    }

    pub fn enabled(&self) -> bool {
        self.enabled_stack.last().copied().unwrap_or(true)
    }

    pub fn add_enabled(&mut self, enabled: bool, add: impl FnOnce(&mut Self)) {
        self.enabled_stack.push(self.enabled() && enabled);
        add(self);
        self.enabled_stack.pop();
    }

    pub fn set_font_path(&mut self, path: impl AsRef<Path>, px: f32) -> Result<(), String> {
        self.font = Font::from_path(path, px)?;
        Ok(())
    }

    pub fn set_font_bytes(&mut self, bytes: &[u8], px: f32) -> Result<(), String> {
        self.font = Font::from_bytes(bytes, px)?;
        Ok(())
    }

    pub fn font_atlas(&self) -> (&[u8], u32, u32) {
        let (w, h) = self.font.atlas_size();
        (self.font.atlas_pixels(), w, h)
    }

    pub fn font_atlas_take_dirty(&mut self) -> bool {
        self.font.take_dirty()
    }

    /// RGBA8 atlas for the color-picker SV field (`TEX_SLOT_COLOR_SV`).
    pub fn color_sv_atlas(&self) -> (&[u8], u32, u32) {
        let n = self.color_sv.size.max(1);
        (self.color_sv.pixels.as_slice(), n, n)
    }

    pub fn color_sv_atlas_take_dirty(&mut self) -> bool {
        let d = self.color_sv.dirty;
        self.color_sv.dirty = false;
        d
    }

    pub(crate) fn text_width(&self, text: &str) -> f32 {
        self.font.text_width_at(text, self.font_size())
    }

    pub(crate) fn text_width_at(&self, text: &str, size: f32) -> f32 {
        self.font.text_width_at(text, size)
    }

    pub(crate) fn text_height(&self) -> f32 {
        self.font.line_height_at(self.font_size())
    }

    pub(crate) fn text_height_at(&self, size: f32) -> f32 {
        self.font.line_height_at(size)
    }

    pub fn request_repaint(&mut self) {
        self.needs_repaint = true;
    }

    /// Mark the mouse wheel as used so parent `scroll_area`s ignore it this frame.
    pub(crate) fn consume_scroll(&mut self) {
        self.scroll_consumed = true;
        self.input.scroll_delta = Vec2::ZERO;
    }

    pub fn color_box(&mut self, size: f32, color: [f32; 4]) {
        let gap = self.s(4.0);
        let size = self.s(size);
        let rect = self.allocate(Vec2::new(size + gap, size + gap));
        self.round_rect(
            Rect::from_min_size(rect.min + Vec2::new(0.0, gap * 0.5), Vec2::splat(size)),
            self.s(2.0),
            color,
        );
    }

    /// Allocate and fill a rectangle (hit target / placeholder). Returns the rect.
    pub fn surface(&mut self, size: Vec2, color: [f32; 4]) -> Rect {
        let rect = self.allocate(size);
        self.round_rect(rect, self.s(3.0), color);
        rect
    }
}

impl Ui {
    pub fn begin_frame(&mut self, input: UiInput) {
        // Recover from a previous atlas overflow (glyphs were skipped that frame).
        let atlas_reset = self.font.take_overflow();
        if atlas_reset {
            self.font.reset_atlas();
            self.icons.clear_packed();
            self.font.prewarm(self.font_size());
        }

        self.input = input;
        self.draw_list.clear();
        self.overlay.clear();
        self.window_layers.clear();
        self.modal_layer.clear();
        self.id_stack.clear();
        self.layers.clear();
        self.clip_stack.clear();
        self.table_stack.clear();
        self.grid_stack.clear();
        self.mouse_absorb = None;
        self.hover_id = None;
        self.want_capture = false;
        self.cursor_icon = CursorIcon::Default;
        self.scroll_hover = None;
        self.scroll_consumed = false;
        self.needs_repaint = atlas_reset;
        self.clipboard_out = None;
        self.enabled_stack.clear();
        self.menu_bar_stack.clear();
        self.menu_stack.clear();
        self.draw_to_overlay = false;

        let modal_blocking = self.modal_open;
        self.modal_id = None;
        self.modal_open = false;
        self.modal_request_close = false;
        // Block background until/unless a modal runs and unlocks its own content.
        self.block_input = modal_blocking;

        self.hover_window = self
            .win_order
            .iter()
            .rev()
            .find(|id| {
                self.win_rects
                    .get(id)
                    .map(|r| r.contains(self.input.mouse_pos))
                    .unwrap_or(false)
            })
            .copied();

        if self.input.mouse_pressed {
            let blocked = self
                .overlay_block
                .map(|r| r.contains(self.input.mouse_pos))
                .unwrap_or(false);
            self.focus_id = None;
            if !blocked {
                self.focus_window = self.hover_window;
                if !modal_blocking {
                    if let Some(id) = self.hover_window {
                        self.bring_to_front(id);
                    }
                }
            }
        }

        self.layers
            .push(new_layer(LayoutDir::Vertical, Vec2::ZERO, self.spacing, 0.0, 0.0, CrossAlign::Start));

        self.tick_toasts();

        let vp = self.input.viewport;
        let title_h = self.s(theme::WIN_TITLE_H);
        let min = Vec2::new(self.s(theme::WIN_MIN_W), self.s(theme::WIN_MIN_H));
        for w in self.windows.values_mut() {
            let vis_h = if w.collapsed { title_h } else { w.size.y };
            window::clamp_win(&mut w.pos, &mut w.size, vis_h, vp, min, title_h);
        }
    }

    pub fn end_frame(&mut self) -> UiOutput {
        if self.input.mouse_released {
            self.active_id = None;
            self.drag_grab = None;
            self.focus_window = None;
        }
        self.scroll_wheel_target = self.scroll_hover;

        // background → windows (skip modal id) → modal dim+window → overlays
        let modal_id = self.modal_id;
        let mut composed = std::mem::take(&mut self.draw_list);
        for id in &self.win_order {
            if Some(*id) == modal_id {
                continue;
            }
            if let Some((_, cmds)) = self.window_layers.iter().find(|(i, _)| i == id) {
                composed.extend_from_slice(cmds);
            }
        }
        for (id, cmds) in &self.window_layers {
            if Some(*id) == modal_id {
                continue;
            }
            if !self.win_order.contains(id) {
                composed.extend_from_slice(cmds);
            }
        }
        composed.append(&mut self.modal_layer);
        // Modal window draw cmds live in window_layers too — append after dim.
        if let Some(mid) = modal_id {
            if let Some((_, cmds)) = self.window_layers.iter().find(|(i, _)| *i == mid) {
                composed.extend_from_slice(cmds);
            }
        }
        // Toasts draw into overlay.
        self.draw_toasts();
        composed.append(&mut self.overlay);
        self.draw_list = composed;

        // Popups + full-screen modal dim both block the next press under them.
        self.overlay_block = if self.modal_open {
            Some(Rect::from_min_size(Vec2::ZERO, self.input.viewport))
        } else {
            self.mouse_absorb
        };

        UiOutput {
            draw_list: std::mem::take(&mut self.draw_list),
            want_capture_mouse: self.want_capture || self.modal_open,
            want_capture_keyboard: self.focus_id.is_some() || self.modal_open,
            cursor: self.cursor_icon,
            needs_repaint: self.needs_repaint,
            clipboard: self.clipboard_out.take(),
        }
    }

    pub(crate) fn bring_to_front(&mut self, id: Id) {
        self.win_order.retain(|x| *x != id);
        self.win_order.push(id);
    }

    pub(crate) fn window_input_ok(&self, id: Id) -> bool {
        if self.modal_open {
            return self.modal_id == Some(id);
        }
        self.hover_window == Some(id) || self.focus_window == Some(id)
    }

    pub(crate) fn set_cursor(&mut self, icon: CursorIcon) {
        if icon.priority() >= self.cursor_icon.priority() {
            self.cursor_icon = icon;
        }
    }

    pub(crate) fn current_id(&self, local: &str) -> Id {
        match self.id_stack.last() {
            Some(parent) => parent.child(local),
            None => Id::new(local),
        }
    }

    pub(crate) fn push_id(&mut self, local: &str) {
        let id = self.current_id(local);
        self.id_stack.push(id);
    }

    pub(crate) fn pop_id(&mut self) {
        self.id_stack.pop();
    }

    /// Scope widget ids so identical labels (e.g. "X") stay unique across siblings.
    pub fn id_scope(&mut self, id: &str, add: impl FnOnce(&mut Self)) {
        self.push_id(id);
        add(self);
        self.pop_id();
    }

    pub(crate) fn layer(&mut self) -> &mut Layer {
        self.layers.last_mut().unwrap()
    }

    /// Remaining space in the current layout (width = fill_w, height = to bottom of fill_h).
    pub fn available_size(&self) -> Vec2 {
        let layer = self.layers.last().unwrap();
        let w = layer.fill_w.max(0.0);
        let h = if layer.fill_h > 0.0 {
            (layer.origin.y + layer.fill_h - layer.cursor.y).max(0.0)
        } else {
            0.0
        };
        Vec2::new(w, h)
    }

    pub(crate) fn allocate(&mut self, size: Vec2) -> Rect {
        let layer = self.layer();
        let rect = match layer.dir {
            LayoutDir::Vertical => {
                let rect = Rect::from_min_size(layer.cursor, size);
                layer.cursor.y += size.y + layer.spacing;
                layer.used.x = layer.used.x.max(size.x);
                layer.used.y = layer.cursor.y - layer.origin.y - layer.spacing;
                rect
            }
            LayoutDir::Horizontal => {
                if layer.spacer_at.is_some() && layer.fill_w > 0.0 {
                    layer.trailing_w += size.x;
                    let item_x = layer.origin.x + layer.fill_w - layer.trailing_w;
                    let y = cross_y(layer, size.y);
                    let rect = Rect::from_min_size(Vec2::new(item_x, y), size);
                    layer.trailing_w += layer.spacing;
                    layer.row_height = layer.row_height.max(size.y);
                    layer.used.x = layer.fill_w;
                    layer.used.y = layer.row_height;
                    return rect;
                }
                let y = cross_y(layer, size.y);
                let rect = Rect::from_min_size(Vec2::new(layer.cursor.x, y), size);
                layer.cursor.x += size.x + layer.spacing;
                layer.row_height = layer.row_height.max(size.y);
                layer.used.x = layer.cursor.x - layer.origin.x - layer.spacing;
                layer.used.y = layer.row_height;
                rect
            }
        };
        rect
    }

    pub fn vertical(&mut self, add: impl FnOnce(&mut Self)) {
        self.layout_with(LayoutDir::Vertical, LayoutOpts::default(), add);
    }

    pub fn horizontal(&mut self, add: impl FnOnce(&mut Self)) {
        self.layout_with(LayoutDir::Horizontal, LayoutOpts::default(), add);
    }

    /// Screen-space line segment (GPU SDF).
    pub fn line(&mut self, a: Vec2, b: Vec2, thickness: f32, color: [f32; 4]) {
        self.draw_line_segment(a, b, thickness, color);
    }

    pub(crate) fn draw_line_segment(&mut self, a: Vec2, b: Vec2, thickness: f32, color: [f32; 4]) {
        let clip = if self.draw_to_overlay {
            None
        } else {
            self.clip()
        };
        let out = if self.draw_to_overlay {
            &mut self.overlay
        } else {
            &mut self.draw_list
        };
        push_line_segment(out, a, b, thickness, color, clip);
    }

    pub(crate) fn draw_polyline(&mut self, points: &[Vec2], thickness: f32, color: [f32; 4]) {
        let clip = if self.draw_to_overlay {
            None
        } else {
            self.clip()
        };
        let out = if self.draw_to_overlay {
            &mut self.overlay
        } else {
            &mut self.draw_list
        };
        push_polyline(out, points, thickness, color, clip);
    }

    pub(crate) fn layout_with(
        &mut self,
        dir: LayoutDir,
        opts: LayoutOpts,
        add: impl FnOnce(&mut Self),
    ) {
        self.layout_impl(dir, opts, add);
    }

    pub(crate) fn push_clip(&mut self, rect: Rect) {
        let next = match self.clip_stack.last().copied() {
            Some(prev) => prev.intersect(rect).unwrap_or(Rect {
                min: rect.min,
                max: rect.min,
            }),
            None => rect,
        };
        self.clip_stack.push(next);
    }

    pub(crate) fn pop_clip(&mut self) {
        self.clip_stack.pop();
    }

    pub(crate) fn clip(&self) -> Option<Rect> {
        self.clip_stack.last().copied()
    }

    /// Hit-test with current clip. Popup absorb blocks widgets under overlays.
    pub fn rect_hovered(&self, rect: Rect) -> bool {
        self.hovered_rect(rect)
    }

    /// Hit-test with current clip. Popup absorb blocks widgets under overlays.
    pub(crate) fn hovered_rect(&self, rect: Rect) -> bool {
        if self.block_input {
            return false;
        }
        if self.mouse_over_absorb() {
            return false;
        }
        if !rect.contains(self.input.mouse_pos) {
            return false;
        }
        if let Some(c) = self.clip() {
            if !c.contains(self.input.mouse_pos) {
                return false;
            }
        }
        true
    }

    /// True when the pointer is over an overlay popup (menu / select / …).
    pub(crate) fn mouse_over_absorb(&self) -> bool {
        self.mouse_absorb
            .map(|r| r.contains(self.input.mouse_pos))
            .unwrap_or(false)
    }

    /// Hit-test for overlay popups (ignores window clip + mouse_absorb).
    pub(crate) fn hovered_overlay(&self, rect: Rect) -> bool {
        rect.contains(self.input.mouse_pos)
    }

    pub(crate) fn round_rect(&mut self, rect: Rect, radius: f32, color: [f32; 4]) {
        self.round_rect_corners(rect, radius, color, true, true);
    }

    /// Rounded rect with independent top / bottom corner rounding.
    pub(crate) fn round_rect_corners(
        &mut self,
        rect: Rect,
        radius: f32,
        color: [f32; 4],
        round_top: bool,
        round_bot: bool,
    ) {
        if self.draw_to_overlay {
            let uv = self.font.white_uv();
            push_round_rect(
                &mut self.overlay,
                rect,
                radius,
                color,
                round_top,
                round_bot,
                uv,
                None,
            );
            return;
        }
        let clip = self.clip();
        let uv = self.font.white_uv();
        push_round_rect(
            &mut self.draw_list,
            rect,
            radius,
            color,
            round_top,
            round_bot,
            uv,
            clip,
        );
    }

    pub(crate) fn round_rect_overlay(&mut self, rect: Rect, radius: f32, color: [f32; 4]) {
        // Overlays (popups, menus, toasts) must ignore window/scroll clips.
        let uv = self.font.white_uv();
        push_round_rect(&mut self.overlay, rect, radius, color, true, true, uv, None);
    }

    /// Overlay quad with per-corner colors (TL, TR, BR, BL). Sharp corners only.
    pub(crate) fn gradient_overlay(&mut self, rect: Rect, colors: [[f32; 4]; 4]) {
        let uv = self.font.white_uv();
        crate::font::push_gradient(&mut self.overlay, rect, colors, uv);
    }

    pub(crate) fn text(&mut self, pos: Vec2, text: &str, color: [f32; 4]) {
        if self.draw_to_overlay {
            self.text_overlay(pos, text, color);
            return;
        }
        self.text_sized(pos, text, color, self.font_size());
    }

    pub(crate) fn text_sized(&mut self, pos: Vec2, text: &str, color: [f32; 4], size: f32) {
        if self.draw_to_overlay {
            self.font
                .draw_text(&mut self.overlay, pos, text, color, size, None);
            return;
        }
        let clip = self.clip();
        self.font
            .draw_text(&mut self.draw_list, pos, text, color, size, clip);
    }

    pub(crate) fn text_overlay(&mut self, pos: Vec2, text: &str, color: [f32; 4]) {
        let size = self.font_size();
        self.font
            .draw_text(&mut self.overlay, pos, text, color, size, None);
    }

    pub(crate) fn interact_rect(&mut self, id: Id, rect: Rect) -> Response {
        let hovered = self.hovered_rect(rect);
        if hovered {
            self.hover_id = Some(id);
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }
        if hovered && self.input.mouse_pressed {
            self.active_id = Some(id);
        }
        let active = self.active_id == Some(id);
        let clicked = active && hovered && self.input.mouse_released;
        Response {
            hovered,
            clicked,
            changed: false,
        }
    }
}
