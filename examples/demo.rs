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

use framework::{Host, Scene};
use glam::{Vec2, Vec3};
use mega_ui::{ScrollAxes, TableColumn, TextStyle, Ui, Window};

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

struct Demo {
    name: String,
    enabled: bool,
    volume: f32,
    speed: f32,
    mode: usize,
    theme: usize,
    quality: usize,
    clicks: u32,
    progress: f32,
    position: Vec2,
    rotation: Vec3,
    scale_v: Vec3,
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
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            name: String::from("mega-ui"),
            enabled: true,
            volume: 0.65,
            speed: 1.25,
            mode: 0,
            theme: 0,
            quality: 1,
            clicks: 0,
            progress: 0.35,
            position: Vec2::new(10.0, 20.0),
            rotation: Vec3::new(0.0, 45.0, 0.0),
            scale_v: Vec3::ONE,
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
        }
    }
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

    fn build(ui: &mut Ui, state: &mut Self, _viewport: Vec2, dt: f32) -> bool {
        ui.menu_bar(|ui| {
            ui.menu("File", |ui| {
                if ui.menu_item_icon("plus", "New").clicked() {
                    state.last_menu = String::from("File / New");
                }
                if ui.menu_item_icon("folder", "Open…").clicked() {
                    state.last_menu = String::from("File / Open");
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
                ui.separator();
                if ui.menu_item_icon("close", "Delete project…").clicked() {
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
                ui.label(&format!("Last menu: {}", state.last_menu));
                ui.separator();

                ui.tabs("widgets_tabs", &["Basics", "Layout", "Plot"], |ui, tab| match tab {
                    0 => {
                        ui.horizontal(|ui| {
                            ui.icon("folder", 18.0);
                            ui.icon("file", 18.0);
                            ui.icon("plus", 18.0);
                            ui.icon("close", 18.0);
                            ui.icon("chevron_left", 18.0);
                            ui.icon("chevron_right", 18.0);
                            ui.icon("chevron_up", 18.0);
                            ui.icon("chevron_down", 18.0);
                            ui.icon("check", 18.0);
                            ui.label("icons");
                        });
                        ui.separator();
                        ui.text_input("name", &mut state.name);
                        ui.checkbox("Enabled", &mut state.enabled);
                        ui.slider("Volume", &mut state.volume, 0.0..=1.0);
                        ui.select("Mode", &mut state.mode, &["Edit", "Play", "Inspect"]);
                        ui.toggle("Theme", &mut state.theme, &["Dark", "Light"]);
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
                            }
                        });
                        if ui.button("Confirm…").clicked() {
                            state.confirm_open = true;
                        }
                        ui.label(&format!("Clicks: {}", state.clicks));
                    }
                    1 => {
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
                        ui.label("Animated plot");
                        ui.plot(Vec2::new(0.0, 120.0), &state.plot);
                        ui.separator();
                        ui.label(&format!("points: {}", state.plot.len()));
                    }
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
                ui.label("drag_float / vec / text_area");
                ui.separator();
                ui.label("Speed");
                ui.drag_float("speed", &mut state.speed, 0.05);
                ui.slider("Speed (slider)", &mut state.speed, 0.0..=5.0);
                ui.separator();
                ui.label("Position (vec2)");
                ui.vec2("pos", &mut state.position, 0.5, Vec2::ZERO);
                ui.label("Rotation (vec3)");
                ui.vec3("rot", &mut state.rotation, 1.0, Vec3::ZERO);
                ui.label("Scale (vec3, default 1)");
                ui.vec3("scale", &mut state.scale_v, 0.01, Vec3::ONE);
                ui.separator();
                ui.label("Notes (text_area)");
                let notes_h = ui.available_size().y.max(80.0);
                ui.text_area("notes", &mut state.notes, Vec2::new(0.0, notes_h));
            },
        );

        ui.window(
            Window::new("Help")
                .pos(Vec2::new(380.0, 480.0))
                .size(Vec2::new(320.0, 160.0))
                .open(&mut state.show_help),
            |ui| {
                ui.label("Drag window titles to move.");
                ui.label("Widgets tab: basics / layout / plot.");
                ui.label("Inputs: drag_float, vec2/3, text_area.");
                ui.label("File Manager: tree or table.");
                ui.separator();
                ui.label(&format!("FPS ~ {:.0}", (1.0 / dt.max(1e-4)).min(999.0)));
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
                    }
                });
            },
        );

        let t = state.started.elapsed().as_secs_f32();
        for (i, v) in state.plot.iter_mut().enumerate() {
            *v = ((i as f32) * 0.35 + t).sin() * 0.45 + 0.5;
        }
        true
    }
}

fn main() {
    Host::run(Demo::default());
}
