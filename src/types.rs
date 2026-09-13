use glam::Vec2;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Id(pub(crate) u64);

impl Id {
    pub fn new(v: impl std::hash::Hash) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        let mut h = DefaultHasher::new();
        std::hash::Hash::hash(&v, &mut h);
        Self(h.finish())
    }

    pub fn child(self, v: impl std::hash::Hash) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        let mut h = DefaultHasher::new();
        std::hash::Hash::hash(&self.0, &mut h);
        std::hash::Hash::hash(&v, &mut h);
        Self(h.finish())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self {
            min,
            max: min + size,
        }
    }

    pub fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn inset(self, v: f32) -> Self {
        Self {
            min: self.min + Vec2::splat(v),
            max: self.max - Vec2::splat(v),
        }
    }

    /// Snap edges to the pixel grid (crisper fills / borders).
    pub fn round_px(self) -> Self {
        Self {
            min: Vec2::new(self.min.x.round(), self.min.y.round()),
            max: Vec2::new(self.max.x.round(), self.max.y.round()),
        }
    }

    pub fn contains(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.y >= self.min.y && p.x < self.max.x && p.y < self.max.y
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        let min = Vec2::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y));
        let max = Vec2::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y));
        if min.x < max.x && min.y < max.y {
            Some(Self { min, max })
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DrawCommand {
    pub rect: Rect,
    /// Atlas UVs (`kind` 0/1). For SDF round (`kind` 2): local UV in content space
    /// where (0,0)=top-left of the unpadded rect and (1,1)=bottom-right.
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    /// Per-corner RGBA (0..1): top-left, top-right, bottom-right, bottom-left.
    /// Solid fills use the same color in all four corners.
    pub colors: [[f32; 4]; 4],
    /// 0 = font atlas (alpha in .r), 1 = host RGBA texture (`tex` = slot),
    /// 2 = SDF rounded rect (`params` = width, height, radius, corner mode),
    /// 3 = SDF line segment (`params` = ax, ay, bx, by; thickness in `uv_min.x`).
    pub kind: f32,
    /// Host texture slot when `kind == 1` (bound by the app).
    pub tex: u32,
    /// Extra per-quad data. SDF round: `[w, h, radius, corners]` where corners
    /// is 0=all, 1=top only, 2=bottom only.
    pub params: [f32; 4],
}

impl DrawCommand {
    /// Uniform-color quad (all corners the same).
    pub fn solid(
        rect: Rect,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        color: [f32; 4],
        kind: f32,
        tex: u32,
    ) -> Self {
        Self {
            rect,
            uv_min,
            uv_max,
            colors: [color; 4],
            kind,
            tex,
            params: [0.0; 4],
        }
    }

    /// Gradient quad with independent corner colors (TL, TR, BR, BL).
    pub fn gradient(
        rect: Rect,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        colors: [[f32; 4]; 4],
        kind: f32,
        tex: u32,
    ) -> Self {
        Self {
            rect,
            uv_min,
            uv_max,
            colors,
            kind,
            tex,
            params: [0.0; 4],
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct UiInput {
    pub mouse_pos: Vec2,
    pub mouse_down: bool,
    pub mouse_pressed: bool,
    pub mouse_released: bool,
    /// Right mouse button (context menus).
    pub mouse_right_down: bool,
    pub mouse_right_pressed: bool,
    pub mouse_right_released: bool,
    /// Middle mouse button (node-space pan, etc.).
    pub mouse_middle_down: bool,
    pub mouse_middle_pressed: bool,
    pub mouse_middle_released: bool,
    pub viewport: Vec2,
    /// Wheel delta in pixels (x = horizontal, y = vertical; +y = scroll up / content down).
    pub scroll_delta: Vec2,
    /// Relative mouse motion this frame (pixels; +y is down). Used by knobs.
    pub mouse_delta: Vec2,
    /// Frame delta time in seconds.
    pub dt: f32,
    /// Characters typed this frame.
    pub text: String,
    pub key_backspace: bool,
    pub key_delete: bool,
    pub key_enter: bool,
    pub key_left: bool,
    pub key_right: bool,
    pub key_up: bool,
    pub key_down: bool,
    pub key_home: bool,
    pub key_end: bool,
    pub key_shift: bool,
    pub key_ctrl: bool,
    pub key_copy: bool,
    pub key_paste: bool,
    pub key_cut: bool,
    pub key_select_all: bool,
    /// Ctrl/Cmd+D — duplicate (e.g. selected nodes).
    pub key_duplicate: bool,
    /// Clipboard text for paste (filled by host when key_paste).
    pub clipboard: String,
}

#[derive(Clone, Debug, Default)]
pub struct UiOutput {
    pub draw_list: Vec<DrawCommand>,
    pub want_capture_mouse: bool,
    pub want_capture_keyboard: bool,
    pub cursor: CursorIcon,
    /// Hide the OS cursor (knob vertical-drag, FL-style).
    pub hide_cursor: bool,
    /// Screen position to restore when the hidden cursor is shown again.
    pub cursor_anchor: Option<Vec2>,
    /// Keep redrawing (e.g. smooth scroll animation).
    pub needs_repaint: bool,
    /// Set clipboard to this text (copy/cut).
    pub clipboard: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorIcon {
    #[default]
    Default,
    Pointer,
    Move,
    ResizeNwse,
    ResizeEw,
    ResizeNs,
    Text,
}

impl CursorIcon {
    pub(crate) fn priority(self) -> u8 {
        match self {
            Self::Default => 0,
            Self::Pointer => 1,
            Self::Text => 1,
            Self::Move => 2,
            Self::ResizeNwse | Self::ResizeEw | Self::ResizeNs => 3,
        }
    }
}

/// Snapshot of window/input routing state (for debugging stuck drags).
#[derive(Clone, Copy, Debug, Default)]
pub struct UiInputDebug {
    pub hover_window: Option<Id>,
    pub focus_window: Option<Id>,
    pub block_input: bool,
    pub active_id: Option<Id>,
    pub modal_open: bool,
    pub overlay_block: bool,
    pub mouse_absorb: bool,
    pub win_rects: usize,
    /// `hover_window` is set but that window was not built this frame.
    pub ghost_hover: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Response {
    pub hovered: bool,
    pub clicked: bool,
    pub changed: bool,
}

impl Response {
    pub fn clicked(self) -> bool {
        self.clicked
    }

    pub fn changed(self) -> bool {
        self.changed
    }
}

/// Allocated hit region for custom widgets (piano roll, canvases).
#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub rect: Rect,
    pub hovered: bool,
    /// Left or right button pressed on this area; stays true until release.
    pub active: bool,
}

/// Pointer snapshot for custom widgets. `input` itself is crate-private.
#[derive(Clone, Copy, Debug)]
pub struct Pointer {
    pub pos: Vec2,
    pub down: bool,
    pub pressed: bool,
    pub released: bool,
    pub right_down: bool,
    pub right_pressed: bool,
    pub scroll: Vec2,
    pub ctrl: bool,
    pub delete: bool,
    pub duplicate: bool,
    pub copy: bool,
    pub cut: bool,
    pub paste: bool,
    pub select_all: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Rect -----------------------------------------------------------------

    #[test]
    fn rect_from_min_size_computes_max() {
        let r = Rect::from_min_size(Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0));
        assert_eq!(r.min, Vec2::new(1.0, 2.0));
        assert_eq!(r.max, Vec2::new(4.0, 6.0));
        assert_eq!(r.width(), 3.0);
        assert_eq!(r.height(), 4.0);
    }

    #[test]
    fn rect_contains_is_half_open_at_max_edge() {
        let r = Rect::from_min_size(Vec2::ZERO, Vec2::splat(10.0));
        assert!(r.contains(Vec2::new(0.0, 0.0)));
        assert!(r.contains(Vec2::new(9.99, 9.99)));
        assert!(!r.contains(Vec2::new(10.0, 5.0)));
        assert!(!r.contains(Vec2::new(5.0, 10.0)));
    }

    #[test]
    fn rect_contains_rejects_points_before_min() {
        let r = Rect::from_min_size(Vec2::new(5.0, 5.0), Vec2::splat(10.0));
        assert!(!r.contains(Vec2::new(4.99, 6.0)));
        assert!(!r.contains(Vec2::new(6.0, 4.99)));
    }

    #[test]
    fn rect_intersect_overlapping_returns_common_area() {
        let a = Rect::from_min_size(Vec2::ZERO, Vec2::splat(10.0));
        let b = Rect::from_min_size(Vec2::new(5.0, 5.0), Vec2::splat(10.0));
        let i = a.intersect(b).expect("rects overlap");
        assert_eq!(i.min, Vec2::new(5.0, 5.0));
        assert_eq!(i.max, Vec2::new(10.0, 10.0));
    }

    #[test]
    fn rect_intersect_disjoint_is_none() {
        let a = Rect::from_min_size(Vec2::ZERO, Vec2::splat(5.0));
        let b = Rect::from_min_size(Vec2::new(10.0, 10.0), Vec2::splat(5.0));
        assert!(a.intersect(b).is_none());
    }

    #[test]
    fn rect_intersect_touching_edges_is_none() {
        // Adjacent (non-overlapping) rects must not produce a zero-area "phantom" clip.
        let a = Rect::from_min_size(Vec2::ZERO, Vec2::splat(5.0));
        let b = Rect::from_min_size(Vec2::new(5.0, 0.0), Vec2::splat(5.0));
        assert!(a.intersect(b).is_none());
    }

    #[test]
    fn rect_inset_shrinks_symmetrically() {
        let r = Rect::from_min_size(Vec2::ZERO, Vec2::splat(10.0));
        let inset = r.inset(2.0);
        assert_eq!(inset.min, Vec2::splat(2.0));
        assert_eq!(inset.max, Vec2::splat(8.0));
    }

    #[test]
    fn rect_round_px_rounds_both_corners() {
        let r = Rect {
            min: Vec2::new(1.2, 1.6),
            max: Vec2::new(4.4, 4.5),
        };
        let rounded = r.round_px();
        assert_eq!(rounded.min, Vec2::new(1.0, 2.0));
        assert_eq!(rounded.max, Vec2::new(4.0, 5.0));
    }

    // -- Id ---------------------------------------------------------------------

    #[test]
    fn id_same_input_is_stable() {
        assert_eq!(Id::new("button_1"), Id::new("button_1"));
    }

    #[test]
    fn id_different_input_differs() {
        assert_ne!(Id::new("button_1"), Id::new("button_2"));
    }

    #[test]
    fn id_child_differs_from_parent_and_from_siblings() {
        let root = Id::new("window");
        let a = root.child("tab_a");
        let b = root.child("tab_b");
        assert_ne!(a, root);
        assert_ne!(b, root);
        assert_ne!(a, b);
    }

    #[test]
    fn id_child_is_deterministic() {
        let root = Id::new("window");
        assert_eq!(root.child("tab"), root.child("tab"));
    }

    #[test]
    fn id_nested_scopes_disambiguate_same_local_label() {
        // Mirrors `Ui::id_scope`: two siblings using the same local id ("label")
        // under different parent scopes must not collide.
        let row0 = Id::new("row_0").child("label");
        let row1 = Id::new("row_1").child("label");
        assert_ne!(row0, row1);
    }

    // -- CursorIcon priority ------------------------------------------------

    #[test]
    fn cursor_icon_priority_resize_beats_pointer_and_default() {
        assert!(CursorIcon::ResizeEw.priority() > CursorIcon::Pointer.priority());
        assert!(CursorIcon::Pointer.priority() > CursorIcon::Default.priority());
    }
}
