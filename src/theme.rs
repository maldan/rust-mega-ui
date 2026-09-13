//! Runtime theme. [`Theme::default`] is the original product palette;
//! [`Theme::dark`] is near-black. Hosts clone and mutate fields.

pub type Rgba = [f32; 4];

const fn rgb(v: f32) -> Rgba {
    [v, v, v, 1.0]
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub accent: Rgba,
    pub accent_dim: Rgba,
    pub window: WindowTheme,
    pub button: ButtonTheme,
    pub text: TextTheme,
    pub check: CheckTheme,
    pub slider: SliderTheme,
    pub knob: KnobTheme,
    pub group: GroupTheme,
    pub header: HeaderTheme,
    pub input: InputTheme,
    pub popup: PopupTheme,
    pub scroll: ScrollTheme,
    pub table: TableTheme,
    pub axis: AxisTheme,
    pub toggle: ToggleTheme,
    pub progress: ProgressTheme,
    pub plot: PlotTheme,
    pub dock: DockTheme,
    pub menu: MenuTheme,
    pub status: StatusTheme,
    pub toast: ToastTheme,
    pub node: NodeTheme,
    pub metrics: Metrics,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowTheme {
    pub body: Rgba,
    pub title: Rgba,
    pub title_hover: Rgba,
    pub title_press: Rgba,
    pub border: Rgba,
    pub modal_dim: Rgba,
    pub body_bypass: Rgba,
    pub title_bypass: Rgba,
    pub title_bypass_hover: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonTheme {
    pub bg: Rgba,
    pub hover: Rgba,
    pub press: Rgba,
    pub disabled: Rgba,
    pub border: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct TextTheme {
    pub primary: Rgba,
    pub disabled: Rgba,
    pub title: Rgba,
    pub dim: Rgba,
    pub bright: Rgba,
    pub selection: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct CheckTheme {
    pub bg: Rgba,
    pub on: Rgba,
    pub border: Rgba,
    pub mark: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct SliderTheme {
    pub track: Rgba,
    pub fill: Rgba,
    pub thumb: Rgba,
    pub thumb_hot: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct KnobTheme {
    pub track: Rgba,
    pub fill: Rgba,
    pub face: Rgba,
    pub border: Rgba,
    pub indicator: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct GroupTheme {
    pub bg: Rgba,
    pub border: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct HeaderTheme {
    pub bg: Rgba,
    pub hover: Rgba,
    pub press: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct InputTheme {
    pub bg: Rgba,
    pub bg_disabled: Rgba,
    pub border: Rgba,
    pub border_focus: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct PopupTheme {
    pub bg: Rgba,
    pub hover: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollTheme {
    pub bg: Rgba,
    pub thumb: Rgba,
    pub thumb_hot: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct TableTheme {
    pub header: Rgba,
    pub row: Rgba,
    pub row_alt: Rgba,
    pub row_hover: Rgba,
    pub selected: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct AxisTheme {
    pub x: Rgba,
    pub y: Rgba,
    pub z: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct ToggleTheme {
    pub off: Rgba,
    pub on: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct ProgressTheme {
    pub bg: Rgba,
    pub fill: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct PlotTheme {
    pub bg: Rgba,
    pub line: Rgba,
    pub grid: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct DockTheme {
    pub bg: Rgba,
    pub tab_bar: Rgba,
    pub tab: Rgba,
    pub tab_active: Rgba,
    pub tab_hover: Rgba,
    pub tab_text: Rgba,
    pub tab_text_active: Rgba,
    pub focus: Rgba,
    pub split: Rgba,
    pub split_hot: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct MenuTheme {
    pub bar_bg: Rgba,
    pub hover: Rgba,
    pub active: Rgba,
    pub sep: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct StatusTheme {
    pub bg: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct ToastTheme {
    pub bg: Rgba,
    pub info: Rgba,
    pub success: Rgba,
    pub warn: Rgba,
    pub error: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct NodeTheme {
    pub running: Rgba,
    pub bypass: Rgba,
    pub frame: Rgba,
    pub frame_sel: Rgba,
    pub frame_border: Rgba,
    pub frame_border_sel: Rgba,
}

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub window_radius: f32,
    pub window_title_h: f32,
    pub window_min_w: f32,
    pub window_min_h: f32,
    pub button_radius: f32,
    pub font_size: f32,
    pub knob_size: f32,
    pub group_radius: f32,
    pub scroll_bar: f32,
    pub scroll_gap: f32,
    pub scroll_thumb_min: f32,
    pub scroll_smooth: f32,
    pub table_row_h: f32,
    pub dock_split: f32,
    pub dock_tab_h: f32,
    pub dock_tab_radius: f32,
    pub menu_bar_h: f32,
    pub menu_item_h: f32,
    pub menu_sep_h: f32,
    pub menu_min_w: f32,
    pub status_bar_h: f32,
    pub toast_lifetime: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::product()
    }
}

impl Theme {
    fn default_metrics() -> Metrics {
        Metrics {
            window_radius: 6.0,
            window_title_h: 26.0,
            window_min_w: 140.0,
            window_min_h: 80.0,
            button_radius: 5.0,
            font_size: 14.0,
            knob_size: 52.0,
            group_radius: 4.0,
            scroll_bar: 10.0,
            scroll_gap: 6.0,
            scroll_thumb_min: 18.0,
            scroll_smooth: 16.0,
            table_row_h: 22.0,
            dock_split: 2.0,
            dock_tab_h: 22.0,
            dock_tab_radius: 3.0,
            menu_bar_h: 26.0,
            menu_item_h: 24.0,
            menu_sep_h: 8.0,
            menu_min_w: 140.0,
            status_bar_h: 24.0,
            toast_lifetime: 3.2,
        }
    }

    /// Original product palette (cool deep-dark, blue accent).
    fn product() -> Self {
        let accent = [0.12, 0.32, 0.72, 1.0];
        let accent_dim = [0.10, 0.26, 0.58, 1.0];
        let body = rgb(0.09);
        let border = rgb(0.02);
        Self {
            accent,
            accent_dim,
            window: WindowTheme {
                body,
                title: rgb(0.06),
                title_hover: rgb(0.10),
                title_press: rgb(0.04),
                border,
                modal_dim: [0.0, 0.0, 0.0, 0.62],
                body_bypass: [0.11, 0.10, 0.08, 1.0],
                title_bypass: [0.20, 0.16, 0.08, 1.0],
                title_bypass_hover: [0.26, 0.20, 0.10, 1.0],
            },
            button: ButtonTheme {
                bg: rgb(0.16),
                hover: rgb(0.22),
                press: rgb(0.12),
                disabled: rgb(0.11),
                border: rgb(0.03),
            },
            text: TextTheme {
                primary: rgb(0.78),
                disabled: rgb(0.35),
                title: rgb(0.58),
                dim: rgb(0.40),
                bright: rgb(0.90),
                selection: [0.12, 0.28, 0.58, 0.42],
            },
            check: CheckTheme {
                bg: rgb(0.06),
                on: accent,
                border: rgb(0.03),
                mark: [0.95, 0.97, 1.0, 1.0],
            },
            slider: SliderTheme {
                track: rgb(0.05),
                fill: rgb(0.28),
                thumb: rgb(0.55),
                thumb_hot: rgb(0.72),
            },
            knob: KnobTheme {
                track: rgb(0.22),
                fill: [0.95, 0.52, 0.14, 1.0],
                face: rgb(0.36),
                border: rgb(0.08),
                indicator: rgb(0.96),
            },
            group: GroupTheme {
                bg: rgb(0.07),
                border: rgb(0.03),
            },
            header: HeaderTheme {
                bg: rgb(0.13),
                hover: rgb(0.18),
                press: rgb(0.10),
            },
            input: InputTheme {
                bg: rgb(0.05),
                bg_disabled: rgb(0.07),
                border: rgb(0.03),
                border_focus: accent_dim,
            },
            popup: PopupTheme {
                bg: rgb(0.10),
                hover: rgb(0.18),
            },
            scroll: ScrollTheme {
                bg: rgb(0.05),
                thumb: rgb(0.28),
                thumb_hot: rgb(0.40),
            },
            table: TableTheme {
                header: rgb(0.07),
                row: body,
                row_alt: rgb(0.11),
                row_hover: rgb(0.16),
                selected: [0.14, 0.24, 0.42, 1.0],
            },
            axis: AxisTheme {
                x: [0.82, 0.28, 0.28, 1.0],
                y: [0.32, 0.72, 0.36, 1.0],
                z: [0.32, 0.52, 0.88, 1.0],
            },
            toggle: ToggleTheme {
                off: rgb(0.14),
                on: accent_dim,
            },
            progress: ProgressTheme {
                bg: rgb(0.05),
                fill: [0.36, 0.58, 0.34, 1.0],
            },
            plot: PlotTheme {
                bg: rgb(0.05),
                line: accent,
                grid: rgb(0.14),
            },
            dock: DockTheme {
                bg: rgb(0.07),
                tab_bar: rgb(0.045),
                tab: rgb(0.045),
                tab_active: body,
                tab_hover: rgb(0.07),
                tab_text: rgb(0.50),
                tab_text_active: rgb(0.90),
                focus: [0.23, 0.47, 1.0, 1.0],
                split: rgb(0.05),
                split_hot: accent_dim,
            },
            menu: MenuTheme {
                bar_bg: rgb(0.07),
                hover: rgb(0.18),
                active: rgb(0.22),
                sep: border,
            },
            status: StatusTheme { bg: rgb(0.06) },
            toast: ToastTheme {
                bg: [0.12, 0.12, 0.12, 0.96],
                info: [0.40, 0.60, 0.90, 1.0],
                success: [0.40, 0.72, 0.42, 1.0],
                warn: [0.85, 0.65, 0.25, 1.0],
                error: [0.85, 0.35, 0.32, 1.0],
            },
            node: NodeTheme {
                running: [0.18, 0.68, 0.32, 1.0],
                bypass: [0.55, 0.42, 0.12, 1.0],
                frame: [0.14, 0.22, 0.36, 0.10],
                frame_sel: [0.16, 0.32, 0.58, 0.10],
                frame_border: [0.32, 0.48, 0.72, 0.55],
                frame_border_sel: [0.40, 0.62, 0.95, 0.85],
            },
            metrics: Self::default_metrics(),
        }
    }

    /// Near-black / OLED-leaning surfaces.
    pub fn dark() -> Self {
        let mut t = Self::default();
        let body = rgb(0.035);
        t.window.body = body;
        t.window.title = rgb(0.018);
        t.window.title_hover = rgb(0.055);
        t.window.title_press = rgb(0.01);
        t.window.border = rgb(0.0);
        t.window.modal_dim = [0.0, 0.0, 0.0, 0.72];
        t.window.body_bypass = [0.06, 0.055, 0.04, 1.0];
        t.window.title_bypass = [0.14, 0.11, 0.05, 1.0];
        t.window.title_bypass_hover = [0.18, 0.14, 0.06, 1.0];
        t.button.bg = rgb(0.11);
        t.button.hover = rgb(0.16);
        t.button.press = rgb(0.07);
        t.button.disabled = rgb(0.06);
        t.button.border = rgb(0.0);
        t.text.primary = rgb(0.82);
        t.text.disabled = rgb(0.32);
        t.text.title = rgb(0.52);
        t.check.bg = rgb(0.02);
        t.check.border = rgb(0.0);
        t.slider.track = rgb(0.02);
        t.slider.fill = rgb(0.22);
        t.group.bg = rgb(0.025);
        t.group.border = rgb(0.0);
        t.header.bg = rgb(0.08);
        t.header.hover = rgb(0.12);
        t.header.press = rgb(0.05);
        t.input.bg = rgb(0.02);
        t.input.bg_disabled = rgb(0.03);
        t.input.border = rgb(0.0);
        t.popup.bg = rgb(0.05);
        t.popup.hover = rgb(0.12);
        t.scroll.bg = rgb(0.02);
        t.scroll.thumb = rgb(0.22);
        t.scroll.thumb_hot = rgb(0.34);
        t.table.header = rgb(0.025);
        t.table.row = body;
        t.table.row_alt = rgb(0.05);
        t.table.row_hover = rgb(0.09);
        t.toggle.off = rgb(0.08);
        t.progress.bg = rgb(0.02);
        t.plot.bg = rgb(0.02);
        t.plot.grid = rgb(0.08);
        t.dock.bg = rgb(0.025);
        t.dock.tab_bar = rgb(0.015);
        t.dock.tab = rgb(0.015);
        t.dock.tab_active = body;
        t.dock.tab_hover = rgb(0.04);
        t.dock.split = rgb(0.0);
        t.menu.bar_bg = rgb(0.025);
        t.menu.sep = rgb(0.0);
        t.status.bg = rgb(0.018);
        t.toast.bg = [0.06, 0.06, 0.06, 0.96];
        t
    }
}
