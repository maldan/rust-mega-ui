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
    /// 2 = SDF rounded rect (`params` = width, height, radius, corner mode).
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
    pub viewport: Vec2,
    /// Wheel delta in pixels (x = horizontal, y = vertical; +y = scroll up / content down).
    pub scroll_delta: Vec2,
    /// Frame delta time in seconds.
    pub dt: f32,
    /// Characters typed this frame.
    pub text: String,
    pub key_backspace: bool,
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
    /// Clipboard text for paste (filled by host when key_paste).
    pub clipboard: String,
}

#[derive(Clone, Debug, Default)]
pub struct UiOutput {
    pub draw_list: Vec<DrawCommand>,
    pub want_capture_mouse: bool,
    pub want_capture_keyboard: bool,
    pub cursor: CursorIcon,
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
