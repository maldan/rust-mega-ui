//! Dock layout + UI scale demo.
//!
//! ```text
//! cargo run --example demo_dock
//! ```

#[path = "framework.rs"]
mod framework;

use framework::{Host, Scene};
use glam::{Vec2, Vec3};
use mega_ui::{BrowserItem, DockNode, DockState, ScrollAxes, TableColumn, TextStyle, Ui, Window};

struct DockDemo {
    dock: DockState,
    scale: f32,
    name: String,
    fov: f32,
    exposure: f32,
    near_clip: f32,
    far_clip: f32,
    wireframe: bool,
    shadows: bool,
    bloom: bool,
    enabled: bool,
    quality: usize,
    render_mode: usize,
    cull_mode: usize,
    position: Vec3,
    rotation: Vec3,
    scale_v: Vec3,
    offset: Vec2,
    tint: [f32; 4],
    clear: [f32; 4],
    notes: String,
    search: String,
    log: String,
    last_menu: String,
    progress: f32,
    output: f32,
    drive: f32,
    volume: f32,
    speed: f32,
    t: f32,
    plot: Vec<f32>,
    asset_path: String,
    asset_selected: Option<String>,
    asset_opened: String,
    confirm_open: bool,
}

impl Default for DockDemo {
    fn default() -> Self {
        let dock = DockState::new(DockNode::split_h(
            0.58,
            DockNode::leaf(&["Viewport", "Scene"]),
            DockNode::split_v(
                0.55,
                DockNode::leaf(&["Inspector", "Settings", "Widgets"]),
                DockNode::leaf(&["Console", "Assets", "Plot"]),
            ),
        ));
        Self {
            dock,
            scale: 1.0,
            name: String::from("Main Camera"),
            fov: 60.0,
            exposure: 1.0,
            near_clip: 0.1,
            far_clip: 1000.0,
            wireframe: false,
            shadows: true,
            bloom: true,
            enabled: true,
            quality: 1,
            render_mode: 0,
            cull_mode: 1,
            position: Vec3::new(0.0, 1.6, 4.0),
            rotation: Vec3::new(-12.0, 180.0, 0.0),
            scale_v: Vec3::ONE,
            offset: Vec2::new(0.0, 0.0),
            tint: [0.12, 0.32, 0.72, 1.0],
            clear: [0.05, 0.05, 0.06, 1.0],
            notes: String::from("Scene notes…\nEdit me.\n"),
            search: String::new(),
            log: String::from("dock ready\ndrag splitters to resize panes\n"),
            last_menu: String::from("(none)"),
            progress: 0.0,
            output: 0.72,
            drive: 0.35,
            volume: 0.65,
            speed: 1.0,
            t: 0.0,
            plot: (0..64)
                .map(|i| ((i as f32) * 0.28).sin() * 0.5 + 0.5)
                .collect(),
            asset_path: String::new(),
            asset_selected: None,
            asset_opened: String::from("(none)"),
            confirm_open: false,
        }
    }
}

impl Scene for DockDemo {
    fn title() -> &'static str {
        "mega-ui dock demo"
    }

    fn window_size() -> (f64, f64) {
        (1360.0, 860.0)
    }

    fn init(ui: &mut Ui) {
        ui.load_builtin_icons();
    }

    fn build(ui: &mut Ui, state: &mut Self, viewport: Vec2, dt: f32) -> bool {
        state.t += dt;
        state.progress = (0.5 + (state.t * 0.4).sin() * 0.45).clamp(0.0, 1.0);
        for (i, v) in state.plot.iter_mut().enumerate() {
            *v = ((i as f32) * 0.28 + state.t * 2.0).sin() * 0.45 + 0.5;
        }

        ui.set_scale(state.scale);

        ui.menu_bar(|ui| {
            ui.menu("File", |ui| {
                if ui.menu_item_icon("plus", "New Scene").clicked() {
                    state.last_menu = String::from("File / New Scene");
                    state.log.push_str("new scene\n");
                    ui.notify_success("New scene");
                }
                if ui.menu_item_icon("folder_open", "Open…").clicked() {
                    state.last_menu = String::from("File / Open");
                    state.log.push_str("open…\n");
                }
                if ui.menu_item_icon("save", "Save").clicked() {
                    state.last_menu = String::from("File / Save");
                    state.log.push_str("save\n");
                    ui.notify_success("Saved");
                }
                ui.menu("Open Recent", |ui| {
                    if ui.menu_item_icon("file", "level_01.mega").clicked() {
                        state.last_menu = String::from("File / Open Recent / level_01.mega");
                        state.log.push_str("open level_01.mega\n");
                    }
                    if ui.menu_item_icon("file", "sandbox.mega").clicked() {
                        state.last_menu = String::from("File / Open Recent / sandbox.mega");
                        state.log.push_str("open sandbox.mega\n");
                    }
                });
                ui.separator();
                if ui.menu_item_icon("close", "Exit").clicked() {
                    state.last_menu = String::from("File / Exit");
                    state.log.push_str("exit\n");
                }
            });
            ui.menu("Edit", |ui| {
                if ui.menu_item_icon("undo", "Undo").clicked() {
                    state.log.push_str("undo\n");
                }
                if ui.menu_item_icon("redo", "Redo").clicked() {
                    state.log.push_str("redo\n");
                }
                ui.separator();
                if ui.menu_item_icon("copy", "Duplicate").clicked() {
                    state.log.push_str("duplicate\n");
                }
                if ui.menu_item_icon("edit", "Rename").clicked() {
                    state.log.push_str("rename\n");
                }
                if ui.menu_item_icon("delete", "Delete").clicked() {
                    state.confirm_open = true;
                    state.last_menu = String::from("Edit / Delete");
                }
                ui.separator();
                ui.add_enabled(false, |ui| {
                    let _ = ui.menu_item_icon("lock", "Locked action");
                });
            });
            ui.menu("View", |ui| {
                ui.menu("UI Scale", |ui| {
                    for (label, v) in [("100%", 1.0), ("125%", 1.25), ("150%", 1.5), ("200%", 2.0)] {
                        if ui.menu_item(label).clicked() {
                            state.scale = v;
                            state.last_menu = format!("View / UI Scale / {label}");
                        }
                    }
                });
                ui.separator();
                if ui.menu_item_icon("settings", "Settings panel").clicked() {
                    state.last_menu = String::from("View / Settings");
                }
                if ui.menu_item("Toggle Wireframe").clicked() {
                    state.wireframe = !state.wireframe;
                    state.log.push_str(&format!("wireframe = {}\n", state.wireframe));
                }
                if ui.menu_item("Toggle Shadows").clicked() {
                    state.shadows = !state.shadows;
                    state.log.push_str(&format!("shadows = {}\n", state.shadows));
                }
            });
            ui.menu("Help", |ui| {
                if ui.menu_item_icon("info", "About").clicked() {
                    ui.notify("mega-ui dock demo");
                }
                if ui.menu_item_icon("warning", "Report issue").clicked() {
                    ui.notify_warn("Issue tracker…");
                }
            });
        });

        ui.set_scale(state.scale);

        let bar_h = 26.0 * state.scale;
        ui.window(
            Window::new("UI Scale")
                .pos(Vec2::new(16.0, bar_h + 12.0))
                .size(Vec2::new(300.0, 320.0))
                .resizable(true)
                .collapsible(true),
            |ui| {
                ui.label(&format!("scale = {:.2}", state.scale));
                ui.label(&format!("menu: {}", state.last_menu));
                if ui.slider("Scale", &mut state.scale, 0.75..=2.0).changed() {
                    state
                        .log
                        .push_str(&format!("scale -> {:.2}\n", state.scale));
                }
                ui.horizontal(|ui| {
                    for (label, v) in [("0.75", 0.75), ("1.0", 1.0), ("1.5", 1.5), ("2.0", 2.0)] {
                        if ui.button(label).clicked() {
                            state.scale = v;
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button_with("add_scene", |ui| {
                            ui.icon("plus", 14.0);
                            ui.label("Add");
                        })
                        .clicked()
                    {
                        state.log.push_str("add from scale window\n");
                        ui.notify("Add");
                    }
                    if ui
                        .button_with("del_scene", |ui| {
                            ui.icon("delete", 14.0);
                            ui.label("Delete");
                        })
                        .clicked()
                    {
                        state.confirm_open = true;
                    }
                    if ui
                        .button_with("save_scene", |ui| {
                            ui.icon("save", 14.0);
                            ui.label("Save");
                        })
                        .clicked()
                    {
                        ui.notify_success("Saved");
                    }
                });
                ui.separator();
                ui.label("Bake progress");
                ui.progress_bar(state.progress);
                ui.label(&format!("FPS ~ {:.0}", (1.0 / dt.max(1e-4)).min(999.0)));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.icon_colored("warning", 16.0, [0.95, 0.72, 0.18, 1.0]);
                    ui.icon_colored("info", 16.0, [0.35, 0.65, 0.95, 1.0]);
                    ui.icon_colored("check", 16.0, [0.35, 0.78, 0.42, 1.0]);
                    ui.label("tints");
                });
            },
        );

        ui.set_scale(state.scale);

        let DockDemo {
            dock,
            name,
            fov,
            exposure,
            near_clip,
            far_clip,
            wireframe,
            shadows,
            bloom,
            enabled,
            quality,
            render_mode,
            cull_mode,
            position,
            rotation,
            scale_v,
            offset,
            tint,
            clear,
            notes,
            search,
            log,
            scale,
            progress,
            output,
            drive,
            volume,
            speed,
            plot,
            asset_path,
            asset_selected,
            asset_opened,
            confirm_open,
            ..
        } = state;

        let status_h = 24.0 * *scale;
        let dock_size =
            Vec2::new(viewport.x, (viewport.y - 26.0 * *scale - status_h).max(1.0));
        ui.dock_space("main", dock_size, dock, |ui, tab| match tab {
            "Viewport" => {
                ui.label_styled(
                    "Viewport",
                    TextStyle {
                        color: [0.85, 0.85, 0.85, 1.0],
                        size: 16.0,
                    },
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.toggle("mode", render_mode, &["Shaded", "Wire", "Lit"]);
                    ui.select("cull", cull_mode, &["None", "Back", "Front"]);
                });
                ui.separator();
                let tex_size = ui.available_size();
                let zone = ui.surface(tex_size, *clear);
                ui.context_menu("viewport_ctx", ui.rect_hovered(zone), |ui| {
                    if ui.menu_item_icon("search", "Frame selected").clicked() {
                        log.push_str("frame selected\n");
                        ui.notify("Frame selected");
                    }
                    if ui.menu_item("Toggle wireframe").clicked() {
                        *wireframe = !*wireframe;
                        ui.notify(&format!("wireframe = {}", *wireframe));
                    }
                    if ui.menu_item_icon("copy", "Copy view").clicked() {
                        log.push_str("copy view\n");
                    }
                    ui.separator();
                    if ui.menu_item_icon("delete", "Clear selection").clicked() {
                        ui.notify_warn("Selection cleared");
                    }
                });
            }
            "Scene" => {
                ui.horizontal(|ui| {
                    ui.label("Scene");
                    ui.icon("search", 14.0);
                });
                ui.text_input("scene_search", search);
                ui.separator();
                let size = ui.available_size();
                ui.scroll_area("scene_tree", size, ScrollAxes::Vertical, |ui| {
                    ui.tree_node("world", "World", |ui| {
                        ui.tree_node_icon("camera", "file", "Camera", |ui| {
                            ui.label(name.as_str());
                        });
                        ui.tree_node_icon("lights", "folder", "Lights", |ui| {
                            ui.tree_leaf_icon("sun", "file", "Sun");
                            ui.tree_leaf_icon("fill", "file", "Fill");
                            ui.tree_leaf_icon("rim", "file", "Rim");
                        });
                        ui.tree_node_icon("meshes", "folder_open", "Meshes", |ui| {
                            ui.collapsing_header("Primitives", |ui| {
                                ui.tree_leaf_icon("cube", "file", "Cube");
                                ui.tree_leaf_icon("plane", "file", "Plane");
                                ui.tree_leaf_icon("sphere", "file", "Sphere");
                            });
                            ui.collapsing_header("Imported", |ui| {
                                ui.tree_leaf_icon("char", "file", "Character");
                                ui.tree_leaf_icon("prop", "file", "Prop_A");
                            });
                        });
                        ui.tree_node_icon("ui", "folder", "UI", |ui| {
                            ui.tree_leaf_icon("hud", "file", "HUD");
                            ui.tree_leaf_icon("menu", "file", "MainMenu");
                        });
                    });
                });
            }
            "Inspector" => {
                let size = ui.available_size();
                ui.scroll_area("inspector", size, ScrollAxes::Vertical, |ui| {
                    ui.horizontal(|ui| {
                        ui.icon_colored("file", 16.0, [0.7, 0.85, 1.0, 1.0]);
                        ui.label("Inspector");
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.icon("save", 16.0);
                        ui.icon("search", 16.0);
                        ui.icon("settings", 16.0);
                        ui.icon("edit", 16.0);
                        ui.icon("copy", 16.0);
                        ui.icon("lock", 16.0);
                        ui.icon_colored("warning", 16.0, [0.95, 0.72, 0.18, 1.0]);
                    });
                    ui.separator();
                    ui.checkbox("Enabled", enabled);
                    ui.add_enabled(*enabled, |ui| {
                        ui.text_input("name", name);
                        ui.group("Lens", |ui| {
                            ui.slider("FOV", fov, 20.0..=120.0);
                            ui.slider("Exposure", exposure, 0.0..=4.0);
                            ui.label("Near / Far (drag_float)");
                            ui.drag_float("near", near_clip, 0.01);
                            ui.drag_float("far", far_clip, 1.0);
                        });
                        ui.group("Transform", |ui| {
                            ui.label("Position");
                            ui.vec3("pos", position, 0.1, Vec3::ZERO);
                            ui.label("Rotation");
                            ui.vec3("rot", rotation, 1.0, Vec3::ZERO);
                            ui.label("Scale");
                            ui.vec3("scl", scale_v, 0.01, Vec3::ONE);
                            ui.label("Offset (vec2)");
                            ui.vec2("off", offset, 0.1, Vec2::ZERO);
                        });
                        ui.group("Mix", |ui| {
                            ui.horizontal(|ui| {
                                ui.knob("Output", output, 0.0..=1.0);
                                ui.knob_colored(
                                    "Drive",
                                    drive,
                                    0.0..=1.0,
                                    [0.35, 0.72, 0.85, 1.0],
                                );
                            });
                            ui.slider("Volume", volume, 0.0..=1.0);
                            ui.drag_float("Speed", speed, 0.05);
                        });
                        ui.group("Flags", |ui| {
                            ui.checkbox("Wireframe", wireframe);
                            ui.checkbox("Shadows", shadows);
                            ui.checkbox("Bloom", bloom);
                            ui.select("Quality", quality, &["Low", "Medium", "High", "Ultra"]);
                        });
                        ui.separator();
                        ui.label("Tint");
                        ui.color_edit("tint", tint);
                        ui.label("Clear color");
                        ui.color_edit("clear", clear);
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui
                            .button_with("insp_add", |ui| {
                                ui.icon("plus", 14.0);
                                ui.label("Add");
                            })
                            .clicked()
                        {
                            log.push_str("inspector add\n");
                            ui.notify("Add component");
                        }
                        if ui
                            .button_with("insp_del", |ui| {
                                ui.icon("delete", 14.0);
                                ui.label("Remove");
                            })
                            .clicked()
                        {
                            *confirm_open = true;
                        }
                        if ui
                            .button_with("insp_refresh", |ui| {
                                ui.icon("refresh", 14.0);
                                ui.label("Reload");
                            })
                            .clicked()
                        {
                            ui.notify("Reloaded");
                        }
                    });
                    ui.separator();
                    ui.label(&format!("scale (ui) = {:.2}", *scale));
                    ui.label(&format!(
                        "pos = ({:.1}, {:.1}, {:.1})",
                        position.x, position.y, position.z
                    ));
                });
            }
            "Settings" => {
                ui.tabs(
                    "settings_tabs",
                    &["General", "Notes", "About"],
                    |ui, tab| match tab {
                        0 => {
                            let size = ui.available_size();
                            ui.scroll_area("settings_gen", size, ScrollAxes::Vertical, |ui| {
                                ui.label("General");
                                ui.separator();
                                ui.group("Display", |ui| {
                                    ui.toggle(
                                        "theme_like",
                                        render_mode,
                                        &["Shaded", "Wire", "Lit"],
                                    );
                                    ui.slider("UI Scale", scale, 0.75..=2.0);
                                    ui.checkbox("Wireframe", wireframe);
                                    ui.checkbox("Shadows", shadows);
                                    ui.checkbox("Bloom", bloom);
                                });
                                ui.group("Bake", |ui| {
                                    ui.progress_bar(*progress);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .button_with("bake_run", |ui| {
                                                ui.icon("plus", 14.0);
                                                ui.label("Bake");
                                            })
                                            .clicked()
                                        {
                                            log.push_str("bake started\n");
                                            ui.notify_success("Bake started");
                                        }
                                        if ui
                                            .button_with("bake_cancel", |ui| {
                                                ui.icon("close", 14.0);
                                                ui.label("Cancel");
                                            })
                                            .clicked()
                                        {
                                            ui.notify_warn("Bake cancelled");
                                        }
                                    });
                                });
                                ui.separator();
                                ui.label("Drag dock splitters to resize.");
                                ui.label("Tabs switch active leaf content.");
                            });
                        }
                        1 => {
                            ui.label("Scene notes (text_area)");
                            ui.separator();
                            let h = ui.available_size().y.max(100.0);
                            ui.text_area("notes", notes, Vec2::new(0.0, h));
                        }
                        _ => {
                            ui.label_styled(
                                "mega-ui dock demo",
                                TextStyle {
                                    color: [0.85, 0.75, 0.35, 1.0],
                                    size: 18.0,
                                },
                            );
                            ui.separator();
                            ui.label("Dock + floating window + modal.");
                            ui.label("Inspector: vectors, knobs, color, enabled.");
                            ui.label("Widgets: table, toggles, icon tints.");
                            ui.label("Assets: browser grid. Plot: live samples.");
                        }
                    },
                );
            }
            "Widgets" => {
                let size = ui.available_size();
                ui.scroll_area("widgets_pane", size, ScrollAxes::Vertical, |ui| {
                    ui.label("Widget playground");
                    ui.separator();
                    ui.group("Inputs", |ui| {
                        ui.toggle("w_mode", render_mode, &["A", "B", "C"]);
                        ui.select("w_quality", quality, &["Low", "Medium", "High", "Ultra"]);
                        ui.drag_float("w_speed", speed, 0.1);
                        ui.slider("w_volume", volume, 0.0..=1.0);
                    });
                    ui.group("Icons", |ui| {
                        ui.horizontal(|ui| {
                            for id in ["folder", "folder_open", "file", "save", "search", "grid"] {
                                ui.icon(id, 18.0);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.icon_colored("warning", 18.0, [0.95, 0.72, 0.18, 1.0]);
                            ui.icon_colored("info", 18.0, [0.35, 0.65, 0.95, 1.0]);
                            ui.icon_colored("delete", 18.0, [0.85, 0.28, 0.28, 1.0]);
                            ui.icon_colored("check", 18.0, [0.35, 0.78, 0.42, 1.0]);
                            ui.icon_colored("folder", 18.0, [0.95, 0.78, 0.28, 1.0]);
                        });
                    });
                    ui.group("Swatches", |ui| {
                        ui.horizontal(|ui| {
                            ui.color_box(22.0, *tint);
                            ui.color_box(22.0, *clear);
                            ui.color_box(22.0, [0.85, 0.28, 0.28, 1.0]);
                            ui.color_box(22.0, [0.35, 0.78, 0.42, 1.0]);
                            ui.color_box(22.0, [0.95, 0.72, 0.18, 1.0]);
                        });
                    });
                    ui.group("Table", |ui| {
                        ui.table(
                            "demo_table",
                            &[
                                TableColumn {
                                    name: "Name",
                                    width: 2.0,
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
                                for (name, kind, size) in [
                                    ("main.scene", "Scene", "12 KB"),
                                    ("sky.hdr", "Texture", "2.1 MB"),
                                    ("cube.mesh", "Mesh", "8 KB"),
                                    ("footstep.wav", "Audio", "120 KB"),
                                ] {
                                    if ui
                                        .table_row(|ui| {
                                            ui.table_cell(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.icon("file", 14.0);
                                                    ui.label(name);
                                                });
                                            });
                                            ui.table_cell(|ui| ui.label(kind));
                                            ui.table_cell(|ui| ui.label(size));
                                        })
                                        .clicked()
                                    {
                                        log.push_str(&format!("table click {name}\n"));
                                        ui.notify(name);
                                    }
                                }
                            },
                        );
                    });
                    ui.group("Disabled", |ui| {
                        ui.add_enabled(false, |ui| {
                            let _ = ui.button("Disabled button");
                            ui.checkbox("Can't toggle", wireframe);
                        });
                    });
                });
            }
            "Console" => {
                ui.horizontal(|ui| {
                    ui.label("Console");
                    if ui
                        .button_with("log_clear", |ui| {
                            ui.icon("delete", 14.0);
                            ui.label("Clear");
                        })
                        .clicked()
                    {
                        log.clear();
                        log.push_str("cleared\n");
                    }
                    if ui
                        .button_with("log_ping", |ui| {
                            ui.icon("plus", 14.0);
                            ui.label("Ping");
                        })
                        .clicked()
                    {
                        log.push_str("ping\n");
                    }
                    if ui
                        .button_with("log_info", |ui| {
                            ui.icon("info", 14.0);
                            ui.label("Info");
                        })
                        .clicked()
                    {
                        ui.notify("Hello from console");
                    }
                    if ui
                        .button_with("log_warn", |ui| {
                            ui.icon("warning", 14.0);
                            ui.label("Warn");
                        })
                        .clicked()
                    {
                        ui.notify_warn("Something fishy");
                    }
                });
                ui.separator();
                let size = ui.available_size();
                ui.scroll_area("log", size, ScrollAxes::Vertical, |ui| {
                    for line in log.lines() {
                        ui.label(line);
                    }
                });
            }
            "Assets" => {
                ui.horizontal(|ui| {
                    ui.icon("folder_open", 16.0);
                    let crumb = if asset_path.is_empty() {
                        "Assets"
                    } else {
                        asset_path.as_str()
                    };
                    ui.label(crumb);
                });
                ui.label(&format!("opened: {asset_opened}"));
                ui.separator();

                let entries: Vec<(&str, &str, &str, bool)> = match asset_path.as_str() {
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
                };
                let items: Vec<BrowserItem<'_>> = entries
                    .iter()
                    .map(|(id, label, icon, is_folder)| BrowserItem {
                        id,
                        label,
                        icon,
                        is_folder: *is_folder,
                    })
                    .collect();

                let size = ui.available_size();
                let mut nav: Option<String> = None;
                let mut opened: Option<String> = None;
                ui.scroll_area("assets", size, ScrollAxes::Vertical, |ui| {
                    let resp = ui.browser("dock_assets", &items, asset_selected);
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
                });
                if let Some(folder) = nav {
                    if folder == ".." {
                        asset_path.clear();
                    } else {
                        *asset_path = folder;
                    }
                    *asset_selected = None;
                }
                if let Some(res) = opened {
                    *asset_opened = res.clone();
                    log.push_str(&format!("open asset {res}\n"));
                    ui.notify(&format!("Open asset: {res}"));
                }
            }
            "Plot" => {
                ui.label("Live plot");
                ui.separator();
                ui.label(&format!("samples = {}", plot.len()));
                ui.slider("Speed", speed, 0.1..=3.0);
                ui.separator();
                let size = ui.available_size();
                ui.plot(Vec2::new(size.x.max(80.0), size.y.max(80.0)), plot);
            }
            other => ui.label(other),
        });

        ui.modal(
            Window::new("Confirm")
                .size(Vec2::new(320.0, 150.0))
                .open(confirm_open),
            |ui| {
                ui.horizontal(|ui| {
                    ui.icon_colored("warning", 18.0, [0.95, 0.72, 0.18, 1.0]);
                    ui.label("Delete selected object?");
                });
                ui.label("This cannot be undone.");
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                    if ui
                        .button_with("confirm_del", |ui| {
                            ui.icon("delete", 14.0);
                            ui.label("Delete");
                        })
                        .clicked()
                    {
                        ui.close_modal();
                        log.push_str("confirmed delete\n");
                        ui.notify_error("Deleted");
                    }
                });
            },
        );

        let fps = (1.0 / dt.max(1e-4)).min(999.0);
        ui.status_bar(|ui| {
            ui.label(&format!("{}", state.last_menu));
            ui.label("·");
            ui.label("RMB in Viewport");
            ui.label("·");
            ui.label(&format!("opened: {asset_opened}"));
            ui.label("·");
            ui.label(&format!("FPS {:.0}", fps));
        });

        // Keep redrawing: progress / plot / toasts are time-driven.
        true
    }
}

fn main() {
    Host::run(DockDemo::default());
}
