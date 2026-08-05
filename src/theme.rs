//! Deep dark theme — near-black surfaces, dark borders, cool blue accent.

/// App / desktop clear color (match host framebuffer clear).
pub const DESKTOP: [f32; 4] = [0.04, 0.04, 0.04, 1.0];

pub const WIN_BODY: [f32; 4] = [0.09, 0.09, 0.09, 1.0];
pub const WIN_TITLE: [f32; 4] = [0.06, 0.06, 0.06, 1.0];
pub const WIN_TITLE_HOVER: [f32; 4] = [0.10, 0.10, 0.10, 1.0];
pub const WIN_TITLE_PRESS: [f32; 4] = [0.04, 0.04, 0.04, 1.0];
pub const WIN_BORDER: [f32; 4] = [0.02, 0.02, 0.02, 1.0];
pub const WIN_RADIUS: f32 = 6.0;
pub const WIN_TITLE_H: f32 = 26.0;
pub const WIN_MIN_W: f32 = 140.0;
pub const WIN_MIN_H: f32 = 80.0;
pub const MODAL_DIM: [f32; 4] = [0.0, 0.0, 0.0, 0.62];

/// Cool blue accent (interactive only — surfaces stay neutral).
pub const ACCENT: [f32; 4] = [0.12, 0.32, 0.72, 1.0];
pub const ACCENT_DIM: [f32; 4] = [0.10, 0.26, 0.58, 1.0];
pub const ACCENT_SOFT: [f32; 4] = [0.06, 0.12, 0.24, 1.0];

pub const BTN: [f32; 4] = [0.16, 0.16, 0.16, 1.0];
pub const BTN_HOVER: [f32; 4] = [0.22, 0.22, 0.22, 1.0];
pub const BTN_PRESS: [f32; 4] = [0.12, 0.12, 0.12, 1.0];
pub const BTN_DISABLED: [f32; 4] = [0.11, 0.11, 0.11, 1.0];
pub const BTN_BORDER: [f32; 4] = [0.03, 0.03, 0.03, 1.0];
pub const BTN_RADIUS: f32 = 5.0;

pub const TEXT: [f32; 4] = [0.78, 0.78, 0.78, 1.0];
pub const TEXT_DISABLED: [f32; 4] = [0.35, 0.35, 0.35, 1.0];
pub const TITLE_TEXT: [f32; 4] = [0.58, 0.58, 0.58, 1.0];
pub const FONT_SIZE: f32 = 14.0;
pub const SELECTION: [f32; 4] = [0.12, 0.28, 0.58, 0.42];

pub const CHECK: [f32; 4] = [0.06, 0.06, 0.06, 1.0];
pub const CHECK_ON: [f32; 4] = ACCENT;
pub const CHECK_BORDER: [f32; 4] = [0.03, 0.03, 0.03, 1.0];
pub const CHECK_MARK: [f32; 4] = [0.95, 0.97, 1.0, 1.0];

pub const SLIDER_TRACK: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub const SLIDER_FILL: [f32; 4] = [0.28, 0.28, 0.28, 1.0];
pub const SLIDER_THUMB: [f32; 4] = [0.55, 0.55, 0.55, 1.0];
pub const SLIDER_THUMB_HOT: [f32; 4] = [0.72, 0.72, 0.72, 1.0];

/// Rotary knob (arc track + face).
pub const KNOB_TRACK: [f32; 4] = [0.22, 0.22, 0.22, 1.0];
pub const KNOB_FILL: [f32; 4] = [0.95, 0.52, 0.14, 1.0];
pub const KNOB_FACE: [f32; 4] = [0.36, 0.36, 0.36, 1.0];
pub const KNOB_BORDER: [f32; 4] = [0.08, 0.08, 0.08, 1.0];
pub const KNOB_INDICATOR: [f32; 4] = [0.96, 0.96, 0.96, 1.0];
pub const KNOB_SIZE: f32 = 52.0;

/// Group box (fieldset): darker than window, lighter than input.
pub const GROUP_BG: [f32; 4] = [0.07, 0.07, 0.07, 1.0];
pub const GROUP_BORDER: [f32; 4] = INPUT_BORDER;
pub const GROUP_RADIUS: f32 = 4.0;

pub const HEADER: [f32; 4] = [0.13, 0.13, 0.13, 1.0];
pub const HEADER_HOVER: [f32; 4] = [0.18, 0.18, 0.18, 1.0];
pub const HEADER_PRESS: [f32; 4] = [0.10, 0.10, 0.10, 1.0];

pub const INPUT_BG: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub const INPUT_BG_DISABLED: [f32; 4] = [0.07, 0.07, 0.07, 1.0];
pub const INPUT_BORDER: [f32; 4] = [0.03, 0.03, 0.03, 1.0];
pub const INPUT_BORDER_FOCUS: [f32; 4] = ACCENT_DIM;
pub const POPUP_BG: [f32; 4] = [0.10, 0.10, 0.10, 1.0];
pub const POPUP_HOVER: [f32; 4] = [0.18, 0.18, 0.18, 1.0];

pub const SCROLL_BG: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub const SCROLL_THUMB: [f32; 4] = [0.28, 0.28, 0.28, 1.0];
pub const SCROLL_THUMB_HOT: [f32; 4] = [0.40, 0.40, 0.40, 1.0];
pub const SCROLL_BAR: f32 = 10.0;
/// Gap between content and scrollbar track.
pub const SCROLL_GAP: f32 = 6.0;
pub const SCROLL_THUMB_MIN: f32 = 18.0;
/// Higher = snappier smooth scroll.
pub const SCROLL_SMOOTH: f32 = 16.0;

pub const TABLE_HEADER: [f32; 4] = [0.07, 0.07, 0.07, 1.0];
pub const TABLE_ROW: [f32; 4] = [0.09, 0.09, 0.09, 1.0];
pub const TABLE_ROW_ALT: [f32; 4] = [0.11, 0.11, 0.11, 1.0];
pub const TABLE_ROW_HOVER: [f32; 4] = [0.16, 0.16, 0.16, 1.0];
pub const TABLE_ROW_H: f32 = 22.0;
pub const TEXT_DIM: [f32; 4] = [0.40, 0.40, 0.40, 1.0];
pub const TEXT_BRIGHT: [f32; 4] = [0.90, 0.90, 0.90, 1.0];

/// Axis colors for vector drag grips.
pub const AXIS_X: [f32; 4] = [0.82, 0.28, 0.28, 1.0];
pub const AXIS_Y: [f32; 4] = [0.32, 0.72, 0.36, 1.0];
pub const AXIS_Z: [f32; 4] = [0.32, 0.52, 0.88, 1.0];

pub const TOGGLE_OFF: [f32; 4] = [0.14, 0.14, 0.14, 1.0];
pub const TOGGLE_ON: [f32; 4] = ACCENT_DIM;
pub const PROGRESS_BG: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub const PROGRESS_FILL: [f32; 4] = [0.36, 0.58, 0.34, 1.0];
pub const PLOT_BG: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub const PLOT_LINE: [f32; 4] = ACCENT;
pub const PLOT_GRID: [f32; 4] = [0.14, 0.14, 0.14, 1.0];

pub const DOCK_BG: [f32; 4] = [0.07, 0.07, 0.07, 1.0];
/// Dark strip behind / beside tabs.
pub const DOCK_TAB_BAR: [f32; 4] = [0.045, 0.045, 0.045, 1.0];
/// Inactive tab — same dark as the bar.
pub const DOCK_TAB: [f32; 4] = [0.045, 0.045, 0.045, 1.0];
/// Active tab = window body so it merges with content.
pub const DOCK_TAB_ACTIVE: [f32; 4] = WIN_BODY;
pub const DOCK_TAB_HOVER: [f32; 4] = [0.07, 0.07, 0.07, 1.0];
pub const DOCK_TAB_TEXT: [f32; 4] = [0.50, 0.50, 0.50, 1.0];
pub const DOCK_TAB_TEXT_ACTIVE: [f32; 4] = [0.90, 0.90, 0.90, 1.0];
/// In-window tabs share dock colors (active light / inactive dark).
pub const TAB: [f32; 4] = DOCK_TAB;
pub const TAB_ACTIVE: [f32; 4] = DOCK_TAB_ACTIVE;
/// Selected row in file / asset browser lists.
pub const BROWSER_SELECTED: [f32; 4] = [0.14, 0.24, 0.42, 1.0];
/// Focus accent on the active tab only.
pub const DOCK_FOCUS: [f32; 4] = [0.23, 0.47, 1.0, 1.0];
pub const DOCK_SPLIT: f32 = 2.0;
pub const DOCK_SPLIT_COL: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub const DOCK_SPLIT_HOT: [f32; 4] = ACCENT_DIM;
pub const DOCK_TAB_H: f32 = 22.0;
pub const DOCK_TAB_RADIUS: f32 = 3.0;

pub const MENU_BAR_BG: [f32; 4] = [0.07, 0.07, 0.07, 1.0];
pub const MENU_BAR_H: f32 = 26.0;
pub const MENU_ITEM_H: f32 = 24.0;
pub const MENU_SEP_H: f32 = 8.0;
pub const MENU_MIN_W: f32 = 140.0;
pub const MENU_HOVER: [f32; 4] = [0.18, 0.18, 0.18, 1.0];
pub const MENU_ACTIVE: [f32; 4] = [0.22, 0.22, 0.22, 1.0];
pub const MENU_SEP: [f32; 4] = [0.02, 0.02, 0.02, 1.0];

pub const STATUS_BAR_BG: [f32; 4] = [0.06, 0.06, 0.06, 1.0];
pub const STATUS_BAR_H: f32 = 24.0;

pub const TOAST_BG: [f32; 4] = [0.12, 0.12, 0.12, 0.96];
pub const TOAST_INFO: [f32; 4] = [0.40, 0.60, 0.90, 1.0];
pub const TOAST_SUCCESS: [f32; 4] = [0.40, 0.72, 0.42, 1.0];
pub const TOAST_WARN: [f32; 4] = [0.85, 0.65, 0.25, 1.0];
pub const TOAST_ERROR: [f32; 4] = [0.85, 0.35, 0.32, 1.0];
pub const TOAST_LIFETIME: f32 = 3.2;
