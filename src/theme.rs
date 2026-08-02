pub const WIN_BODY: [f32; 4] = [0.16, 0.16, 0.16, 1.0];
pub const WIN_TITLE: [f32; 4] = [0.11, 0.11, 0.11, 1.0];
pub const WIN_TITLE_HOVER: [f32; 4] = [0.15, 0.15, 0.15, 1.0];
pub const WIN_TITLE_PRESS: [f32; 4] = [0.07, 0.07, 0.07, 1.0];
pub const WIN_BORDER: [f32; 4] = [0.05, 0.05, 0.05, 1.0];
pub const WIN_RADIUS: f32 = 5.0;
pub const WIN_TITLE_H: f32 = 24.0;
pub const WIN_MIN_W: f32 = 140.0;
pub const WIN_MIN_H: f32 = 80.0;

pub const BTN: [f32; 4] = [0.25, 0.25, 0.25, 1.0];
pub const BTN_HOVER: [f32; 4] = [0.32, 0.32, 0.32, 1.0];
pub const BTN_PRESS: [f32; 4] = [0.18, 0.18, 0.18, 1.0];
pub const BTN_DISABLED: [f32; 4] = [0.18, 0.18, 0.18, 1.0];
pub const BTN_BORDER: [f32; 4] = [0.08, 0.08, 0.08, 1.0];
pub const BTN_RADIUS: f32 = 5.0;

pub const TEXT: [f32; 4] = [0.72, 0.72, 0.72, 1.0];
pub const TEXT_DISABLED: [f32; 4] = [0.40, 0.40, 0.40, 1.0];
pub const TITLE_TEXT: [f32; 4] = [0.62, 0.62, 0.62, 1.0];
pub const FONT_SIZE: f32 = 14.0;
pub const SELECTION: [f32; 4] = [0.25, 0.40, 0.65, 0.55];

pub const CHECK: [f32; 4] = [0.12, 0.12, 0.12, 1.0];
pub const CHECK_ON: [f32; 4] = [0.63, 0.35, 0.0, 1.0]; // orange
pub const CHECK_BORDER: [f32; 4] = [0.08, 0.08, 0.08, 1.0];

pub const SLIDER_TRACK: [f32; 4] = [0.12, 0.12, 0.12, 1.0];
pub const SLIDER_FILL: [f32; 4] = [0.35, 0.35, 0.35, 1.0];
pub const SLIDER_THUMB: [f32; 4] = [0.55, 0.55, 0.55, 1.0];
pub const SLIDER_THUMB_HOT: [f32; 4] = [0.70, 0.70, 0.70, 1.0];

pub const HEADER: [f32; 4] = [0.16, 0.20, 0.30, 1.0];
pub const HEADER_HOVER: [f32; 4] = [0.20, 0.26, 0.38, 1.0];
pub const HEADER_PRESS: [f32; 4] = [0.12, 0.15, 0.22, 1.0];

pub const INPUT_BG: [f32; 4] = [0.10, 0.10, 0.10, 1.0];
pub const INPUT_BG_DISABLED: [f32; 4] = [0.12, 0.12, 0.12, 1.0];
pub const INPUT_BORDER: [f32; 4] = [0.08, 0.08, 0.08, 1.0];
pub const INPUT_BORDER_FOCUS: [f32; 4] = [0.35, 0.45, 0.65, 1.0];
pub const POPUP_BG: [f32; 4] = [0.14, 0.14, 0.14, 1.0];
pub const POPUP_HOVER: [f32; 4] = [0.22, 0.28, 0.40, 1.0];

pub const SCROLL_BG: [f32; 4] = [0.12, 0.12, 0.12, 1.0];
pub const SCROLL_THUMB: [f32; 4] = [0.40, 0.40, 0.40, 1.0];
pub const SCROLL_THUMB_HOT: [f32; 4] = [0.55, 0.55, 0.55, 1.0];
pub const SCROLL_BAR: f32 = 10.0;
pub const SCROLL_THUMB_MIN: f32 = 18.0;
/// Higher = snappier smooth scroll.
pub const SCROLL_SMOOTH: f32 = 16.0;

pub const TABLE_HEADER: [f32; 4] = [0.12, 0.16, 0.26, 1.0];
pub const TABLE_ROW: [f32; 4] = [0.14, 0.14, 0.14, 1.0];
pub const TABLE_ROW_ALT: [f32; 4] = [0.17, 0.17, 0.17, 1.0];
pub const TABLE_ROW_HOVER: [f32; 4] = [0.20, 0.26, 0.38, 1.0];
pub const TABLE_ROW_H: f32 = 22.0;
pub const TEXT_DIM: [f32; 4] = [0.45, 0.45, 0.45, 1.0];
pub const TEXT_BRIGHT: [f32; 4] = [0.90, 0.90, 0.90, 1.0];

pub const TOGGLE_OFF: [f32; 4] = [0.22, 0.22, 0.22, 1.0];
pub const TOGGLE_ON: [f32; 4] = [0.35, 0.55, 0.85, 1.0];
pub const PROGRESS_BG: [f32; 4] = [0.12, 0.12, 0.12, 1.0];
pub const PROGRESS_FILL: [f32; 4] = [0.40, 0.65, 0.35, 1.0];
pub const TAB: [f32; 4] = [0.18, 0.18, 0.18, 1.0];
pub const TAB_ACTIVE: [f32; 4] = [0.28, 0.34, 0.48, 1.0];
pub const PLOT_BG: [f32; 4] = [0.10, 0.10, 0.10, 1.0];
pub const PLOT_LINE: [f32; 4] = [0.40, 0.75, 0.95, 1.0];
pub const PLOT_GRID: [f32; 4] = [0.20, 0.20, 0.20, 1.0];

pub const DOCK_BG: [f32; 4] = [0.12, 0.12, 0.12, 1.0];
pub const DOCK_TAB_BAR: [f32; 4] = [0.10, 0.10, 0.10, 1.0];
pub const DOCK_SPLIT: f32 = 4.0;
pub const DOCK_SPLIT_COL: [f32; 4] = [0.08, 0.08, 0.08, 1.0];
pub const DOCK_SPLIT_HOT: [f32; 4] = [0.30, 0.45, 0.70, 1.0];
pub const DOCK_TAB_H: f32 = 24.0;
