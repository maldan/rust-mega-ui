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
use glam::Vec2;
use mega_ui::{ScrollAxes, TableColumn, Ui, Window};

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
    mode: usize,
    theme: usize,
    clicks: u32,
    plot: Vec<f32>,
    show_help: bool,
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
            mode: 0,
            theme: 0,
            clicks: 0,
            plot: (0..48)
                .map(|i| ((i as f32) * 0.35).sin() * 0.5 + 0.5)
                .collect(),
            show_help: true,
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
        (1280.0, 800.0)
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
                .size(Vec2::new(300.0, 420.0))
                .resizable(true)
                .collapsible(true),
            |ui| {
                ui.label("mega-ui demo");
                ui.label(&format!("Last menu: {}", state.last_menu));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.icon("folder", 18.0);
                    ui.icon("file", 18.0);
                    ui.icon("plus", 18.0);
                    ui.icon("close", 18.0);
                    ui.label("builtin icons");
                });
                ui.separator();
                ui.text_input("name", &mut state.name);
                ui.checkbox("Enabled", &mut state.enabled);
                ui.slider("Volume", &mut state.volume, 0.0..=1.0);
                ui.select("Mode", &mut state.mode, &["Edit", "Play", "Inspect"]);
                ui.toggle("Theme", &mut state.theme, &["Dark", "Light"]);
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
                ui.separator();
                ui.label("Plot");
                ui.plot(Vec2::new(0.0, 80.0), &state.plot);
            },
        );

        ui.window(
            Window::new("Help")
                .pos(Vec2::new(340.0, 40.0))
                .size(Vec2::new(260.0, 160.0))
                .open(&mut state.show_help),
            |ui| {
                ui.label("Drag window titles to move.");
                ui.label("File Manager: tree or table.");
                ui.label("Folders open; files do not.");
                ui.separator();
                ui.label(&format!("FPS ~ {:.0}", (1.0 / dt.max(1e-4)).min(999.0)));
            },
        );

        ui.window(
            Window::new("File Manager")
                .pos(Vec2::new(620.0, 40.0))
                .size(Vec2::new(520.0, 520.0))
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
                    });

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
