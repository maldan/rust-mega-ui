//! Interactive mega-ui widget showcase.
//!
//! ```text
//! cargo run --example demo
//! ```

#[path = "framework.rs"]
mod framework;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use framework::{DrawStats, Host, Scene};
use glam::{Vec2, Vec3};
use mega_ui::{BrowserItem, ScrollAxes, TableColumn, TextStyle, Ui, Window};
use mega_ui::{AnimationCurve, ease_in_out, sample_curve};

struct FsEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

fn list_dir(path: &Path) -> Vec<FsEntry> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(path) else {
        return out;
    };
    for e in rd.flatten() {
        let path = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let meta = e.metadata().ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = meta.map(|m| m.len()).unwrap_or(0);
        out.push(FsEntry {
            name,
            path,
            is_dir,
            size,
        });
    }
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    out
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn default_root() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        if fs::read_dir(&cwd).is_ok() {
            return cwd;
        }
    }
    if let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        if home.is_dir() {
            return home;
        }
    }
    PathBuf::from("C:\\")
}

/// Virtual asset tree for the Asset Manager demo.
fn asset_entries(path: &str) -> Vec<(&'static str, &'static str, &'static str, bool)> {
    match path {
        "" => vec![
            ("Scenes", "Scenes", "folder", true),
            ("Textures", "Textures", "folder", true),
            ("Materials", "Materials", "folder", true),
            ("Meshes", "Meshes", "folder", true),
            ("Audio", "Audio", "folder", true),
        ],
        "Scenes" => vec![
            ("..", "..", "folder", true),
            ("main.scene", "main.scene", "file", false),
            ("menu.scene", "menu.scene", "file", false),
            ("loading.scene", "loading.scene", "file", false),
        ],
        "Textures" => vec![
            ("..", "..", "folder", true),
            ("sky.hdr", "sky.hdr", "file", false),
            ("ui_atlas.png", "ui_atlas.png", "file", false),
            ("noise.png", "noise.png", "file", false),
        ],
        "Materials" => vec![
            ("..", "..", "folder", true),
            ("main.mat", "main.mat", "file", false),
            ("glass.mat", "glass.mat", "file", false),
        ],
        "Meshes" => vec![
            ("..", "..", "folder", true),
            ("cube.mesh", "cube.mesh", "file", false),
            ("grid.mesh", "grid.mesh", "file", false),
            ("char.fbx", "char.fbx", "file", false),
        ],
        "Audio" => vec![
            ("..", "..", "folder", true),
            ("footstep.wav", "footstep.wav", "file", false),
            ("music.ogg", "music.ogg", "file", false),
        ],
        _ => vec![("..", "..", "folder", true)],
    }
}

struct CalcState {
    display: String,
    acc: f64,
    pending: Option<char>,
    fresh: bool,
}

impl Default for CalcState {
    fn default() -> Self {
        Self {
            display: String::from("0"),
            acc: 0.0,
            pending: None,
            fresh: true,
        }
    }
}

struct Demo {
    name: String,
    enabled: bool,
    volume: f32,
    output: f32,
    drive: f32,
    speed: f32,
    mode: usize,
    theme: usize,
    quality: usize,
    clicks: u32,
    progress: f32,
    position: Vec2,
    rotation: Vec3,
    scale_v: Vec3,
    tint: [f32; 4],
    notes: String,
    plot: Vec<f32>,
    show_help: bool,
    show_inputs: bool,
    started: Instant,
    last_menu: String,
    confirm_open: bool,
    /// 0 = tree, 1 = table
    fm_view: usize,
    fm_path: PathBuf,
    show_fm: bool,
    /// New meta-widget file explorer (keep old FM too).
    show_explorer: bool,
    explorer_path: PathBuf,
    explorer_selected: Option<String>,
    explorer_opened: String,
    /// Abstract asset browser.
    show_assets: bool,
    asset_path: String,
    asset_selected: Option<String>,
    asset_opened: String,
    ui_scale: f32,
    anim_curve: AnimationCurve,
    curve_scrub: f32,
    curve_drive: f32,
    show_calc: bool,
    calc: CalcState,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            name: String::from("mega-ui"),
            enabled: true,
            volume: 0.65,
            output: 0.72,
            drive: 0.35,
            speed: 1.25,
            mode: 0,
            theme: 0,
            quality: 1,
            clicks: 0,
            progress: 0.35,
            position: Vec2::new(10.0, 20.0),
            rotation: Vec3::new(0.0, 45.0, 0.0),
            scale_v: Vec3::ONE,
            tint: [0.12, 0.32, 0.72, 1.0],
            notes: String::from("Multiline notes.\nEdit me — text_area test.\n"),
            plot: (0..48)
                .map(|i| ((i as f32) * 0.35).sin() * 0.5 + 0.5)
                .collect(),
            show_help: true,
            show_inputs: true,
            started: Instant::now(),
            last_menu: String::from("(none)"),
            confirm_open: false,
            fm_view: 0,
            fm_path: default_root(),
            show_fm: true,
            show_explorer: true,
            explorer_path: default_root(),
            explorer_selected: None,
            explorer_opened: String::from("(none)"),
            show_assets: true,
            asset_path: String::new(),
            asset_selected: None,
            asset_opened: String::from("(none)"),
            ui_scale: 1.0,
            anim_curve: ease_in_out(),
            curve_scrub: 0.35,
            curve_drive: 0.0,
            show_calc: true,
            calc: CalcState::default(),
        }
    }
}

fn parse_calc_display(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

fn format_calc_display(v: f64) -> String {
    if !v.is_finite() {
        return String::from("Error");
    }
    let rounded = (v * 1e10).round() / 1e10;
    if (rounded - rounded.round()).abs() < 1e-9 {
        format!("{}", rounded.round() as i64)
    } else {
        let s = format!("{:.8}", rounded);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn calc_apply(acc: f64, rhs: f64, op: char) -> f64 {
    match op {
        '+' => acc + rhs,
        '-' => acc - rhs,
        '*' => acc * rhs,
        '/' => if rhs.abs() < 1e-12 { f64::NAN } else { acc / rhs },
        _ => rhs,
    }
}

fn calc_digit(calc: &mut CalcState, d: char) {
    if calc.display == "Error" {
        calc.display = String::from("0");
        calc.acc = 0.0;
        calc.pending = None;
    }
    if calc.fresh {
        calc.display.clear();
        calc.fresh = false;
    }
    if d == '.' {
        if calc.display.contains('.') {
            return;
        }
        if calc.display.is_empty() {
            calc.display.push('0');
        }
        calc.display.push('.');
        return;
    }
    if calc.display == "0" {
        calc.display = d.to_string();
    } else {
        calc.display.push(d);
    }
}

fn calc_op(calc: &mut CalcState, op: char) {
    if calc.display == "Error" {
        calc.display = String::from("0");
        calc.acc = 0.0;
        calc.pending = None;
    }
    let current = parse_calc_display(&calc.display);
    if let Some(prev) = calc.pending {
        calc.acc = calc_apply(calc.acc, current, prev);
    } else {
        calc.acc = current;
    }
    calc.display = format_calc_display(calc.acc);
    calc.pending = Some(op);
    calc.fresh = true;
}

fn calc_equals(calc: &mut CalcState) {
    if calc.display == "Error" {
        return;
    }
    let current = parse_calc_display(&calc.display);
    if let Some(op) = calc.pending {
        calc.acc = calc_apply(calc.acc, current, op);
        calc.display = format_calc_display(calc.acc);
        calc.pending = None;
        calc.fresh = true;
    }
}

fn calc_clear(calc: &mut CalcState) {
    calc.display = String::from("0");
    calc.acc = 0.0;
    calc.pending = None;
    calc.fresh = true;
}

fn draw_calc_screen(ui: &mut Ui, text: &str) {
    ui.group("", |ui| {
        ui.row(|ui| {
            ui.spacer();
            ui.label_styled(
                text,
                TextStyle {
                    color: [0.92, 0.92, 0.92, 1.0],
                    size: 22.0,
                },
            );
        });
    });
}

fn calc_key(ui: &mut Ui, label: &str, calc: &mut CalcState, action: char) {
    ui.id_scope(label, |ui| {
        if ui.button(label).clicked() {
            match action {
                '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' => {
                    calc_digit(calc, action);
                }
                '.' => calc_digit(calc, '.'),
                'C' => calc_clear(calc),
                '=' => calc_equals(calc),
                '+' | '-' | '*' | '/' => calc_op(calc, action),
                _ => {}
            }
        }
    });
}

fn draw_fs_tree(ui: &mut Ui, path: &Path, depth: u32) {
    if depth > 8 {
        ui.label("…");
        return;
    }
    for entry in list_dir(path) {
        let id = entry.path.to_string_lossy();
        if entry.is_dir {
            ui.tree_node_icon(&id, "folder", &entry.name, |ui| {
                draw_fs_tree(ui, &entry.path, depth + 1);
            });
        } else {
            ui.tree_leaf_icon(&id, "file", &entry.name);
        }
    }
}

impl Scene for Demo {
    fn title() -> &'static str {
        "mega-ui demo"
    }

    fn window_size() -> (f64, f64) {
        (1400.0, 860.0)
    }

    fn init(ui: &mut Ui) {
        ui.load_builtin_icons();
    }

    fn build(ui: &mut Ui, state: &mut Self, _viewport: Vec2, dt: f32, stats: DrawStats) -> bool {
        ui.set_scale(state.ui_scale);

        ui.menu_bar(|ui| {
            ui.menu("File", |ui| {
                if ui.menu_item_icon("plus", "New").clicked() {
                    state.last_menu = String::from("File / New");
                    ui.notify_success("Created new project");
                }
                if ui.menu_item_icon("folder", "Open…").clicked() {
                    state.last_menu = String::from("File / Open");
                    ui.notify("Open dialog…");
                }
                ui.menu("Open Recent", |ui| {
                    if ui.menu_item_icon("file", "project.mega").clicked() {
                        state.last_menu = String::from("File / Open Recent / project.mega");
                    }
                    if ui.menu_item_icon("file", "demo.mega").clicked() {
                        state.last_menu = String::from("File / Open Recent / demo.mega");
                    }
                    ui.separator();
                    ui.add_enabled(false, |ui| {
                        let _ = ui.menu_item("Clear List");
                    });
                });
                ui.separator();
                if ui.menu_item_icon("folder", "File Manager").clicked() {
                    state.show_fm = true;
                    state.last_menu = String::from("File / File Manager");
                }
                if ui.menu_item_icon("folder", "Explorer").clicked() {
                    state.show_explorer = true;
                    state.last_menu = String::from("File / Explorer");
                }
                if ui.menu_item_icon("file", "Asset Manager").clicked() {
                    state.show_assets = true;
                    state.last_menu = String::from("File / Asset Manager");
                }
                ui.separator();
                if ui.menu_item_icon("delete", "Delete project…").clicked() {
                    state.confirm_open = true;
                    state.last_menu = String::from("File / Delete project");
                }
                ui.separator();
                if ui.menu_item_icon("close", "Exit").clicked() {
                    state.last_menu = String::from("File / Exit");
                }
            });
            ui.menu("Edit", |ui| {
                if ui.menu_item("Undo").clicked() {
                    state.last_menu = String::from("Edit / Undo");
                }
                if ui.menu_item("Redo").clicked() {
                    state.last_menu = String::from("Edit / Redo");
                }
                ui.separator();
                if ui.menu_item("Cut").clicked() {
                    state.last_menu = String::from("Edit / Cut");
                }
                if ui.menu_item("Copy").clicked() {
                    state.last_menu = String::from("Edit / Copy");
                }
                if ui.menu_item("Paste").clicked() {
                    state.last_menu = String::from("Edit / Paste");
                }
            });
            ui.menu("View", |ui| {
                if ui.menu_item("Toggle Help").clicked() {
                    state.show_help = !state.show_help;
                    state.last_menu = String::from("View / Toggle Help");
                }
                if ui.menu_item("Toggle Inputs").clicked() {
                    state.show_inputs = !state.show_inputs;
                }
                if ui.menu_item("Toggle File Manager").clicked() {
                    state.show_fm = !state.show_fm;
                }
                if ui.menu_item("Calculator").clicked() {
                    state.show_calc = !state.show_calc;
                    state.last_menu = String::from("View / Calculator");
                }
                ui.menu("UI Scale", |ui| {
                    if ui.menu_item("75%").clicked() {
                        state.ui_scale = 0.75;
                        state.last_menu = String::from("View / UI Scale / 75%");
                    }
                    if ui.menu_item("100%").clicked() {
                        state.ui_scale = 1.0;
                        state.last_menu = String::from("View / UI Scale / 100%");
                    }
                    if ui.menu_item("125%").clicked() {
                        state.ui_scale = 1.25;
                        state.last_menu = String::from("View / UI Scale / 125%");
                    }
                    if ui.menu_item("150%").clicked() {
                        state.ui_scale = 1.5;
                        state.last_menu = String::from("View / UI Scale / 150%");
                    }
                    if ui.menu_item("200%").clicked() {
                        state.ui_scale = 2.0;
                        state.last_menu = String::from("View / UI Scale / 200%");
                    }
                });
                ui.menu("Theme", |ui| {
                    if ui.menu_item("Dark").clicked() {
                        state.theme = 0;
                        state.last_menu = String::from("View / Theme / Dark");
                    }
                    if ui.menu_item("Light").clicked() {
                        state.theme = 1;
                        state.last_menu = String::from("View / Theme / Light");
                    }
                });
            });
        });

        ui.window(
            Window::new("Widgets")
                .pos(Vec2::new(24.0, 40.0))
                .size(Vec2::new(340.0, 520.0))
                .resizable(true)
                .collapsible(true),
            |ui| {
                let size = ui.available_size();
                ui.scroll_area("widgets_scroll", size, ScrollAxes::Vertical, |ui| {
                ui.label(&format!("Last menu: {}", state.last_menu));
                ui.separator();

                ui.tabs("widgets_tabs", &["Basics", "Layout", "Plot"], |ui, tab| match tab {
                    0 => {
                        ui.label("icons");
                        let icon_rows: &[&[&str]] = &[
                            &[
                                "folder",
                                "folder_open",
                                "file",
                                "save",
                                "search",
                                "settings",
                                "edit",
                                "copy",
                            ],
                            &[
                                "undo",
                                "redo",
                                "refresh",
                                "reset",
                                "grid",
                                "plus",
                                "close",
                                "check",
                                "delete",
                                "lock",
                                "unlock",
                                "more_vert",
                            ],
                            &[
                                "chevron_left",
                                "chevron_right",
                                "chevron_up",
                                "chevron_down",
                                "warning",
                                "info",
                            ],
                        ];
                        for row in icon_rows {
                            ui.horizontal(|ui| {
                                for id in *row {
                                    ui.icon(id, 18.0);
                                }
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.icon_colored("warning", 18.0, [0.95, 0.72, 0.18, 1.0]);
                            ui.icon_colored("info", 18.0, [0.35, 0.65, 0.95, 1.0]);
                            ui.icon_colored("delete", 18.0, [0.85, 0.28, 0.28, 1.0]);
                            ui.icon_colored("check", 18.0, [0.35, 0.78, 0.42, 1.0]);
                            ui.icon_colored("folder", 18.0, [0.95, 0.78, 0.28, 1.0]);
                            ui.label("icon_colored");
                        });
                        ui.separator();
                        ui.text_input("name", &mut state.name);
                        ui.checkbox("Enabled", &mut state.enabled);
                        ui.slider("Volume", &mut state.volume, 0.0..=1.0);
                        ui.separator();
                        ui.group("Mix", |ui| {
                            ui.horizontal(|ui| {
                                ui.knob("Output", &mut state.output, 0.0..=1.0);
                                ui.knob_colored(
                                    "Drive",
                                    &mut state.drive,
                                    0.0..=1.0,
                                    [0.35, 0.72, 0.85, 1.0],
                                );
                            });
                        });
                        ui.group("Mode", |ui| {
                            ui.select("Mode", &mut state.mode, &["Edit", "Play", "Inspect"]);
                            ui.toggle("Theme", &mut state.theme, &["Dark", "Light"]);
                        });
                        ui.separator();
                        ui.label("Tint (color_edit)");
                        ui.color_edit("tint", &mut state.tint);
                        ui.separator();
                        ui.label("Progress");
                        ui.progress_bar(state.progress);
                        ui.horizontal(|ui| {
                            if ui.button("-0.1").clicked() {
                                state.progress = (state.progress - 0.1).clamp(0.0, 1.0);
                            }
                            if ui.button("+0.1").clicked() {
                                state.progress = (state.progress + 0.1).clamp(0.0, 1.0);
                            }
                            if ui.button("Fill").clicked() {
                                state.progress = 1.0;
                            }
                        });
                        ui.separator();
                        ui.add_enabled(state.enabled, |ui| {
                            if ui.button("Click me").clicked() {
                                state.clicks += 1;
                                ui.notify(&format!("Clicks: {}", state.clicks));
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui
                                .button_with("add_btn", |ui| {
                                    ui.icon("plus", 14.0);
                                    ui.label("Add");
                                })
                                .clicked()
                            {
                                state.clicks += 1;
                                ui.notify("Add clicked");
                            }
                            if ui
                                .button_with("folder_btn", |ui| {
                                    ui.icon("folder", 14.0);
                                    ui.label("Open");
                                })
                                .clicked()
                            {
                                ui.notify("Open clicked");
                            }
                            if ui
                                .button_with("del_btn", |ui| {
                                    ui.icon("delete", 14.0);
                                    ui.label("Delete");
                                })
                                .clicked()
                            {
                                ui.notify_warn("Delete clicked");
                            }
                            if ui
                                .button_with("close_btn", |ui| {
                                    ui.icon("close", 14.0);
                                    ui.label("Close");
                                })
                                .clicked()
                            {
                                ui.notify("Close clicked");
                            }
                        });
                        if ui.button("Confirm…").clicked() {
                            state.confirm_open = true;
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Toast OK").clicked() {
                                ui.notify_success("All good");
                            }
                            if ui.button("Warn").clicked() {
                                ui.notify_warn("Careful…");
                            }
                            if ui.button("Error").clicked() {
                                ui.notify_error("Something failed");
                            }
                        });
                        ui.label(&format!("Clicks: {}", state.clicks));
                        ui.label("Right-click the zone:");
                        let zone_w = ui.available_size().x.max(80.0);
                        let zone = ui.surface(Vec2::new(zone_w, 48.0), [0.08, 0.08, 0.08, 1.0]);
                        ui.context_menu("widgets_ctx", ui.rect_hovered(zone), |ui| {
                            if ui.menu_item_icon("plus", "Add item").clicked() {
                                state.clicks += 1;
                                ui.notify("Context: Add item");
                            }
                            if ui.menu_item_icon("file", "Duplicate").clicked() {
                                ui.notify("Context: Duplicate");
                            }
                            ui.separator();
                            if ui.menu_item_icon("delete", "Delete").clicked() {
                                ui.notify_error("Context: Delete");
                            }
                        });
                    }
                    1 => {
                        ui.label("Layout: row / flex / property / grid");
                        ui.separator();
                        ui.row(|ui| {
                            ui.label("Left");
                            ui.spacer();
                            ui.label("Right");
                        });
                        ui.property("Volume", 0.35, |ui| {
                            ui.slider("pv", &mut state.volume, 0.0..=1.0);
                        });
                        ui.property("Name", 0.35, |ui| {
                            ui.text_input("pv_name", &mut state.name);
                        });
                        ui.separator();
                        ui.label("Knob grid (3 cols)");
                        ui.grid(3, |ui| {
                            ui.grid_cell(|ui| {
                                ui.knob("g_out", &mut state.output, 0.0..=1.0);
                            });
                            ui.grid_cell(|ui| {
                                ui.knob_colored(
                                    "g_drv",
                                    &mut state.drive,
                                    0.0..=1.0,
                                    [0.35, 0.72, 0.85, 1.0],
                                );
                            });
                            ui.grid_cell(|ui| {
                                ui.knob("g_spd", &mut state.speed, 0.0..=5.0);
                            });
                        });
                        ui.separator();
                        ui.collapsing_header("Collapsing A", |ui| {
                            ui.label("Nested content inside header.");
                            ui.checkbox("Nested flag", &mut state.enabled);
                            ui.button("Nested button");
                        });
                        ui.collapsing_header("Collapsing B", |ui| {
                            ui.label("Another section.");
                            ui.select("Quality", &mut state.quality, &["Low", "Med", "High"]);
                        });
                        ui.separator();
                        ui.label_styled(
                            "Styled label (larger)",
                            TextStyle {
                                color: [0.85, 0.75, 0.35, 1.0],
                                size: 18.0,
                            },
                        );
                        ui.label_styled(
                            "Dim small text",
                            TextStyle {
                                color: [0.45, 0.45, 0.45, 1.0],
                                size: 12.0,
                            },
                        );
                        ui.separator();
                        ui.tree_node("demo_tree", "Demo tree", |ui| {
                            ui.tree_node("child_a", "Child A", |ui| {
                                ui.tree_leaf_icon("leaf1", "file", "leaf one");
                                ui.tree_leaf_icon("leaf2", "file", "leaf two");
                            });
                            ui.tree_node_icon("child_b", "folder", "Child B", |ui| {
                                ui.label("folder contents");
                            });
                        });
                    }
                    _ => {
                        ui.label("Live plot");
                        ui.plot(Vec2::new(0.0, 100.0), &state.plot);
                        ui.separator();
                        ui.label("Curve editor (click add point, Shift+click delete)");
                        let curve_resp = ui.curve_editor(
                            "anim_curve",
                            &mut state.anim_curve,
                            Vec2::new(0.0, 160.0),
                        );
                        ui.slider("scrub", &mut state.curve_scrub, 0.0..=1.0);
                        ui.curve_preview_time("anim_curve", state.curve_scrub);
                        state.curve_drive = sample_curve(&state.anim_curve, state.curve_scrub);
                        ui.property("Sampled", 0.35, |ui| {
                            ui.label(&format!("{:.3}", state.curve_drive));
                        });
                        ui.knob_colored(
                            "curve_knob",
                            &mut state.curve_drive,
                            0.0..=1.0,
                            [0.95, 0.52, 0.14, 1.0],
                        );
                        if curve_resp.changed {
                            ui.notify("Curve changed");
                        }
                        ui.separator();
                        ui.label(&format!("plot points: {}", state.plot.len()));
                    }
                });
                });
            },
        );

        ui.window(
            Window::new("Inputs")
                .pos(Vec2::new(380.0, 40.0))
                .size(Vec2::new(320.0, 420.0))
                .resizable(true)
                .open(&mut state.show_inputs),
            |ui| {
                let size = ui.available_size();
                ui.scroll_area("inputs_scroll", size, ScrollAxes::Vertical, |ui| {
                ui.property("Speed", 0.35, |ui| {
                    ui.drag_float("speed", &mut state.speed, 0.05);
                });
                ui.property("Speed slider", 0.35, |ui| {
                    ui.slider("speed_s", &mut state.speed, 0.0..=5.0);
                });
                ui.separator();
                ui.label("Position (vec2)");
                ui.vec2("pos", &mut state.position, 0.5, Vec2::ZERO);
                ui.label("Rotation (vec3)");
                ui.vec3("rot", &mut state.rotation, 1.0, Vec3::ZERO);
                ui.label("Scale (vec3, default 1)");
                ui.vec3("scale", &mut state.scale_v, 0.01, Vec3::ONE);
                ui.separator();
                ui.label("Notes (text_area)");
                ui.text_area("notes", &mut state.notes, Vec2::new(0.0, 120.0));
                });
            },
        );

        ui.window(
            Window::new("Help")
                .pos(Vec2::new(380.0, 480.0))
                .size(Vec2::new(320.0, 200.0))
                .open(&mut state.show_help),
            |ui| {
                let size = ui.available_size();
                ui.scroll_area("help_scroll", size, ScrollAxes::Vertical, |ui| {
                ui.label("Drag window titles to move.");
                ui.label("Widgets tab: basics / layout / plot.");
                ui.label("Inputs: drag_float, vec2/3, text_area.");
                ui.label("File Manager: tree or table.");
                ui.label("Explorer / Assets: browser meta-widget.");
                ui.separator();
                ui.label(&format!("UI scale = {:.0}%", state.ui_scale * 100.0));
                ui.slider("UI Scale", &mut state.ui_scale, 0.75..=2.0);
                ui.horizontal(|ui| {
                    if ui.button("75%").clicked() {
                        state.ui_scale = 0.75;
                    }
                    if ui.button("100%").clicked() {
                        state.ui_scale = 1.0;
                    }
                    if ui.button("150%").clicked() {
                        state.ui_scale = 1.5;
                    }
                    if ui.button("200%").clicked() {
                        state.ui_scale = 2.0;
                    }
                });
                ui.separator();
                ui.label(&format!("FPS ~ {:.0}", (1.0 / dt.max(1e-4)).min(999.0)));
                });
            },
        );

        ui.window(
            Window::new("File Manager")
                .pos(Vec2::new(720.0, 40.0))
                .size(Vec2::new(560.0, 560.0))
                .resizable(true)
                .open(&mut state.show_fm),
            |ui| {
                ui.toggle("View", &mut state.fm_view, &["Tree", "Table"]);
                ui.separator();
                let path_str = state.fm_path.to_string_lossy().into_owned();
                ui.label(&path_str);
                ui.separator();

                let avail = ui.available_size();
                let list_w = avail.x.max(100.0);
                let list_h = (avail.y - 4.0).max(80.0);

                if state.fm_view == 0 {
                    ui.scroll_area(
                        "fm_tree",
                        Vec2::new(list_w, list_h),
                        ScrollAxes::Vertical,
                        |ui| {
                            let entries = list_dir(&state.fm_path);
                            if entries.is_empty() {
                                ui.label("(empty or unreadable)");
                            } else {
                                draw_fs_tree(ui, &state.fm_path, 0);
                            }
                        },
                    );
                } else {
                    let entries = list_dir(&state.fm_path);
                    let mut nav: Option<PathBuf> = None;

                    ui.scroll_area(
                        "fm_table",
                        Vec2::new(list_w, list_h),
                        ScrollAxes::Vertical,
                        |ui| {
                            if entries.is_empty() {
                                ui.label("(empty or unreadable)");
                            }
                            ui.table(
                                "files",
                                &[
                                    TableColumn {
                                        name: "Name",
                                        width: 3.0,
                                    },
                                    TableColumn {
                                        name: "Type",
                                        width: 1.0,
                                    },
                                    TableColumn {
                                        name: "Size",
                                        width: 1.0,
                                    },
                                ],
                                |ui| {
                                    if state.fm_path.parent().is_some() {
                                        if ui
                                            .table_row(|ui| {
                                                ui.table_cell(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.icon("folder", 14.0);
                                                        ui.label("..");
                                                    });
                                                });
                                                ui.table_cell(|ui| ui.label("Up"));
                                                ui.table_cell(|ui| ui.label(""));
                                            })
                                            .clicked()
                                        {
                                            nav = state.fm_path.parent().map(|p| p.to_path_buf());
                                        }
                                    }

                                    for entry in &entries {
                                        let kind = if entry.is_dir { "Folder" } else { "File" };
                                        let size = if entry.is_dir {
                                            String::from("—")
                                        } else {
                                            format_size(entry.size)
                                        };
                                        let icon = if entry.is_dir { "folder" } else { "file" };
                                        let name = entry.name.clone();
                                        let is_dir = entry.is_dir;
                                        let path = entry.path.clone();

                                        if ui
                                            .table_row(|ui| {
                                                ui.table_cell(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.icon(icon, 14.0);
                                                        ui.label(&name);
                                                    });
                                                });
                                                ui.table_cell(|ui| ui.label(kind));
                                                ui.table_cell(|ui| ui.label(&size));
                                            })
                                            .clicked()
                                            && is_dir
                                        {
                                            nav = Some(path);
                                        }
                                    }
                                },
                            );
                        },
                    );

                    if let Some(p) = nav {
                        state.fm_path = p;
                    }
                }
            },
        );

        // Meta-widget: Explorer (file browser) — old File Manager kept above.
        ui.window(
            Window::new("Explorer")
                .pos(Vec2::new(40.0, 580.0))
                .size(Vec2::new(360.0, 320.0))
                .resizable(true)
                .open(&mut state.show_explorer),
            |ui| {
                let path_str = state.explorer_path.to_string_lossy().into_owned();
                ui.label(&path_str);
                ui.label(&format!("opened: {}", state.explorer_opened));
                ui.separator();

                let mut items: Vec<(String, String, bool)> = Vec::new();
                if state.explorer_path.parent().is_some() {
                    items.push(("..".into(), "..".into(), true));
                }
                for e in list_dir(&state.explorer_path) {
                    items.push((
                        e.path.to_string_lossy().into_owned(),
                        e.name,
                        e.is_dir,
                    ));
                }
                let browser_items: Vec<BrowserItem<'_>> = items
                    .iter()
                    .map(|(id, name, is_dir)| BrowserItem {
                        id: id.as_str(),
                        label: name.as_str(),
                        icon: if *is_dir { "folder" } else { "file" },
                        is_folder: *is_dir,
                    })
                    .collect();

                let avail = ui.available_size();
                let mut open_nav: Option<PathBuf> = None;
                let mut open_file: Option<String> = None;
                ui.scroll_area(
                    "explorer_list",
                    Vec2::new(avail.x.max(80.0), (avail.y - 4.0).max(60.0)),
                    ScrollAxes::Vertical,
                    |ui| {
                        let resp = ui.browser(
                            "explorer",
                            &browser_items,
                            &mut state.explorer_selected,
                        );
                        if let Some(id) = resp.opened() {
                            if id == ".." {
                                open_nav = state.explorer_path.parent().map(|p| p.to_path_buf());
                            } else {
                                let p = PathBuf::from(id);
                                if p.is_dir() {
                                    open_nav = Some(p);
                                } else {
                                    open_file = Some(id.to_string());
                                }
                            }
                        }
                    },
                );
                if let Some(p) = open_nav {
                    state.explorer_path = p;
                    state.explorer_selected = None;
                }
                if let Some(f) = open_file {
                    state.explorer_opened = f.clone();
                    ui.notify(&format!("Open file: {f}"));
                }
            },
        );

        // Meta-widget: Asset Manager (abstract resources).
        ui.window(
            Window::new("Asset Manager")
                .pos(Vec2::new(420.0, 580.0))
                .size(Vec2::new(340.0, 320.0))
                .resizable(true)
                .open(&mut state.show_assets),
            |ui| {
                let crumb = if state.asset_path.is_empty() {
                    "Assets"
                } else {
                    state.asset_path.as_str()
                };
                ui.label(crumb);
                ui.label(&format!("opened: {}", state.asset_opened));
                ui.separator();

                let entries = asset_entries(&state.asset_path);
                let browser_items: Vec<BrowserItem<'_>> = entries
                    .iter()
                    .map(|(id, label, icon, is_folder)| BrowserItem {
                        id,
                        label,
                        icon,
                        is_folder: *is_folder,
                    })
                    .collect();

                let avail = ui.available_size();
                let mut nav: Option<String> = None;
                let mut opened: Option<String> = None;
                ui.scroll_area(
                    "asset_list",
                    Vec2::new(avail.x.max(80.0), (avail.y - 4.0).max(60.0)),
                    ScrollAxes::Vertical,
                    |ui| {
                        let resp =
                            ui.browser("assets", &browser_items, &mut state.asset_selected);
                        if let Some(id) = resp.opened() {
                            if let Some((_, _, _, is_folder)) =
                                entries.iter().find(|(eid, _, _, _)| *eid == id)
                            {
                                if *is_folder {
                                    nav = Some(id.to_string());
                                } else {
                                    opened = Some(id.to_string());
                                }
                            }
                        }
                    },
                );
                if let Some(folder) = nav {
                    if folder == ".." {
                        state.asset_path.clear();
                    } else {
                        state.asset_path = folder;
                    }
                    state.asset_selected = None;
                }
                if let Some(res) = opened {
                    state.asset_opened = res.clone();
                    ui.notify(&format!("Open asset: {res}"));
                }
            },
        );

        ui.window(
            Window::new("Calculator")
                .pos(Vec2::new(720.0, 480.0))
                .size(Vec2::new(248.0, 320.0))
                .resizable(true)
                .open(&mut state.show_calc),
            |ui| {
                ui.label("grid layout demo");
                draw_calc_screen(ui, &state.calc.display);
                ui.space(6.0);
                ui.grid_with(4, Some(6.0), |ui| {
                    for (label, action) in [
                        ("7", '7'),
                        ("8", '8'),
                        ("9", '9'),
                        ("/", '/'),
                        ("4", '4'),
                        ("5", '5'),
                        ("6", '6'),
                        ("*", '*'),
                        ("1", '1'),
                        ("2", '2'),
                        ("3", '3'),
                        ("-", '-'),
                        ("C", 'C'),
                        ("0", '0'),
                        (".", '.'),
                        ("+", '+'),
                    ] {
                        ui.grid_cell(|ui| {
                            calc_key(ui, label, &mut state.calc, action);
                        });
                    }
                });
                ui.space(4.0);
                if ui.button("=").clicked() {
                    calc_equals(&mut state.calc);
                }
            },
        );

        ui.modal(
            Window::new("Confirm")
                .size(Vec2::new(320.0, 160.0))
                .open(&mut state.confirm_open),
            |ui| {
                ui.label("Delete this project?");
                ui.label("This cannot be undone.");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                        state.last_menu = String::from("Confirm / Cancel");
                    }
                    if ui.button("Delete").clicked() {
                        ui.close_modal();
                        state.clicks = 0;
                        state.last_menu = String::from("Confirm / Delete");
                        ui.notify_warn("Project deleted");
                    }
                });
            },
        );

        let t = state.started.elapsed().as_secs_f32();
        for (i, v) in state.plot.iter_mut().enumerate() {
            *v = ((i as f32) * 0.35 + t).sin() * 0.45 + 0.5;
        }

        let fps = (1.0 / dt.max(1e-4)).min(999.0);
        ui.status_bar(|ui| {
            ui.label(&format!("menu: {}", state.last_menu));
            ui.label("·");
            ui.label("RMB = context menu");
            ui.label("·");
            ui.label(&format!(
                "cmds {} · batches {} · quads {}",
                stats.commands, stats.batches, stats.quads
            ));
            ui.label("·");
            ui.label(&format!("FPS {:.0}", fps));
        });

        true
    }
}

fn main() {
    Host::run(Demo::default());
}
