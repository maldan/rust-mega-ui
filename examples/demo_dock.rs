//! Dock layout + UI scale demo.
//!
//! ```text
//! cargo run --example demo_dock
//! ```

#[path = "framework.rs"]
mod framework;

use framework::{Host, Scene};
use glam::{Vec2, Vec3};
use mega_ui::{DockNode, DockState, ScrollAxes, TableColumn, TextStyle, Ui, Window};

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
    quality: usize,
    render_mode: usize,
    position: Vec3,
    rotation: Vec3,
    scale_v: Vec3,
    tint: [f32; 4],
    notes: String,
    log: String,
    last_menu: String,
    progress: f32,
    t: f32,
}

impl Default for DockDemo {
    fn default() -> Self {
        let dock = DockState::new(DockNode::split_h(
            0.62,
            DockNode::leaf(&["Viewport", "Scene"]),
            DockNode::split_v(
                0.55,
                DockNode::leaf(&["Inspector", "Settings"]),
                DockNode::leaf(&["Console", "Assets"]),
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
            quality: 1,
            render_mode: 0,
            position: Vec3::new(0.0, 1.6, 4.0),
            rotation: Vec3::new(-12.0, 180.0, 0.0),
            scale_v: Vec3::ONE,
            tint: [0.35, 0.55, 0.90, 1.0],
            notes: String::from("Scene notes…\n"),
            log: String::from("dock ready\ndrag splitters to resize panes\n"),
            last_menu: String::from("(none)"),
            progress: 0.0,
            t: 0.0,
        }
    }
}

impl Scene for DockDemo {
    fn title() -> &'static str {
        "mega-ui dock demo"
    }

    fn window_size() -> (f64, f64) {
        (1280.0, 800.0)
    }

    fn init(ui: &mut Ui) {
        ui.load_builtin_icons();
    }

    fn build(ui: &mut Ui, state: &mut Self, viewport: Vec2, dt: f32) -> bool {
        state.t += dt;
        state.progress = (0.5 + (state.t * 0.4).sin() * 0.45).clamp(0.0, 1.0);

        ui.set_scale(state.scale);

        ui.menu_bar(|ui| {
            ui.menu("File", |ui| {
                if ui.menu_item_icon("plus", "New Scene").clicked() {
                    state.last_menu = String::from("File / New Scene");
                    state.log.push_str("new scene\n");
                    ui.notify_success("New scene");
                }
                if ui.menu_item_icon("folder", "Open…").clicked() {
                    state.last_menu = String::from("File / Open");
                    state.log.push_str("open…\n");
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
                if ui.menu_item("Duplicate").clicked() {
                    state.log.push_str("duplicate\n");
                }
                if ui.menu_item("Delete").clicked() {
                    state.log.push_str("delete\n");
                }
                ui.separator();
                ui.add_enabled(false, |ui| {
                    let _ = ui.menu_item("Locked action");
                });
            });
            ui.menu("View", |ui| {
                ui.menu("UI Scale", |ui| {
                    if ui.menu_item("100%").clicked() {
                        state.scale = 1.0;
                        state.last_menu = String::from("View / UI Scale / 100%");
                    }
                    if ui.menu_item("125%").clicked() {
                        state.scale = 1.25;
                        state.last_menu = String::from("View / UI Scale / 125%");
                    }
                    if ui.menu_item("150%").clicked() {
                        state.scale = 1.5;
                        state.last_menu = String::from("View / UI Scale / 150%");
                    }
                });
                ui.separator();
                if ui.menu_item("Toggle Wireframe").clicked() {
                    state.wireframe = !state.wireframe;
                    state.last_menu = String::from("View / Toggle Wireframe");
                    state.log.push_str(&format!("wireframe = {}\n", state.wireframe));
                }
                if ui.menu_item("Toggle Shadows").clicked() {
                    state.shadows = !state.shadows;
                    state.log.push_str(&format!("shadows = {}\n", state.shadows));
                }
            });
        });

        ui.set_scale(state.scale);

        let bar_h = 26.0 * state.scale;
        ui.window(
            Window::new("UI Scale")
                .pos(Vec2::new(16.0, bar_h + 12.0))
                .size(Vec2::new(280.0, 180.0)),
            |ui| {
                ui.label(&format!("scale = {:.2}", state.scale));
                ui.label(&format!("menu: {}", state.last_menu));
                if ui.slider("Scale", &mut state.scale, 0.75..=2.0).changed() {
                    state
                        .log
                        .push_str(&format!("scale -> {:.2}\n", state.scale));
                }
                ui.horizontal(|ui| {
                    if ui.button("0.75").clicked() {
                        state.scale = 0.75;
                    }
                    if ui.button("1.0").clicked() {
                        state.scale = 1.0;
                    }
                    if ui.button("1.5").clicked() {
                        state.scale = 1.5;
                    }
                    if ui.button("2.0").clicked() {
                        state.scale = 2.0;
                    }
                });
                ui.separator();
                ui.label("Bake progress");
                ui.progress_bar(state.progress);
                ui.label(&format!("FPS ~ {:.0}", (1.0 / dt.max(1e-4)).min(999.0)));
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
            quality,
            render_mode,
            position,
            rotation,
            scale_v,
            tint,
            notes,
            log,
            scale,
            progress,
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
                });
                ui.separator();
                let tex_size = ui.available_size();
                let zone = ui.surface(tex_size, [0.05, 0.05, 0.06, 1.0]);
                ui.context_menu("viewport_ctx", ui.rect_hovered(zone), |ui| {
                    if ui.menu_item("Frame selected").clicked() {
                        log.push_str("frame selected\n");
                        ui.notify("Frame selected");
                    }
                    if ui.menu_item("Toggle wireframe").clicked() {
                        *wireframe = !*wireframe;
                        ui.notify(&format!("wireframe = {}", *wireframe));
                    }
                    ui.separator();
                    if ui.menu_item_icon("close", "Clear selection").clicked() {
                        ui.notify_warn("Selection cleared");
                    }
                });
            }
            "Scene" => {
                ui.label("Scene hierarchy");
                ui.separator();
                ui.tree_node("world", "World", |ui| {
                    ui.tree_node_icon("camera", "file", "Camera", |ui| {
                        ui.label(name.as_str());
                    });
                    ui.tree_node_icon("lights", "folder", "Lights", |ui| {
                        ui.tree_leaf_icon("sun", "file", "Sun");
                        ui.tree_leaf_icon("fill", "file", "Fill");
                        ui.tree_leaf_icon("rim", "file", "Rim");
                    });
                    ui.tree_node_icon("meshes", "folder", "Meshes", |ui| {
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
                });
            }
            "Inspector" => {
                let size = ui.available_size();
                ui.scroll_area("inspector", size, ScrollAxes::Vertical, |ui| {
                    ui.label("Inspector");
                    ui.separator();
                    ui.text_input("name", name);
                    ui.slider("FOV", fov, 20.0..=120.0);
                    ui.slider("Exposure", exposure, 0.0..=4.0);
                    ui.separator();
                    ui.label("Near / Far (drag_float)");
                    ui.drag_float("near", near_clip, 0.01);
                    ui.drag_float("far", far_clip, 1.0);
                    ui.separator();
                    ui.label("Position");
                    ui.vec3("pos", position, 0.1, Vec3::ZERO);
                    ui.label("Rotation");
                    ui.vec3("rot", rotation, 1.0, Vec3::ZERO);
                    ui.label("Scale");
                    ui.vec3("scale", scale_v, 0.01, Vec3::ONE);
                    ui.separator();
                    ui.checkbox("Wireframe", wireframe);
                    ui.checkbox("Shadows", shadows);
                    ui.select("Quality", quality, &["Low", "Medium", "High", "Ultra"]);
                    ui.separator();
                    ui.label("Tint");
                    ui.color_edit("tint", tint);
                    ui.separator();
                    ui.label(&format!("scale (ui) = {:.2}", *scale));
                    ui.label(&format!(
                        "pos = ({:.1}, {:.1}, {:.1})",
                        position.x, position.y, position.z
                    ));
                });
            }
            "Settings" => {
                ui.tabs("settings_tabs", &["General", "Notes", "About"], |ui, tab| match tab {
                    0 => {
                        ui.label("General");
                        ui.separator();
                        ui.toggle("theme_like", render_mode, &["Shaded", "Wire", "Lit"]);
                        ui.slider("UI Scale", scale, 0.75..=2.0);
                        ui.checkbox("Wireframe", wireframe);
                        ui.checkbox("Shadows", shadows);
                        ui.separator();
                        ui.label("Bake");
                        ui.progress_bar(*progress);
                        ui.separator();
                        ui.label("Drag dock splitters to resize.");
                        ui.label("Tabs switch active leaf content.");
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
                        ui.label("Exercises dock + most widgets.");
                        ui.label("Inspector: floats, vec3, select.");
                        ui.label("Assets: table + scroll.");
                        ui.label("Console: scroll log.");
                    }
                });
            }
            "Console" => {
                ui.horizontal(|ui| {
                    ui.label("Console");
                    if ui.button("Clear").clicked() {
                        log.clear();
                        log.push_str("cleared\n");
                    }
                    if ui.button("Ping").clicked() {
                        log.push_str("ping\n");
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
                ui.label("Assets");
                ui.separator();
                let size = ui.available_size();
                ui.scroll_area("assets", size, ScrollAxes::Vertical, |ui| {
                    ui.table(
                        "asset_table",
                        &[
                            TableColumn {
                                name: "Name",
                                width: 2.5,
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
                            let rows = [
                                ("cube.mesh", "Mesh", "12 KB"),
                                ("grid.mesh", "Mesh", "4 KB"),
                                ("sun.light", "Light", "1 KB"),
                                ("main.mat", "Material", "2 KB"),
                                ("sky.hdr", "Texture", "2.1 MB"),
                                ("char.fbx", "Model", "8.4 MB"),
                                ("footstep.wav", "Audio", "120 KB"),
                                ("ui_atlas.png", "Texture", "512 KB"),
                            ];
                            for (name, kind, size) in rows {
                                let _ = ui.table_row(|ui| {
                                    ui.table_cell(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.icon("file", 14.0);
                                            ui.label(name);
                                        });
                                    });
                                    ui.table_cell(|ui| ui.label(kind));
                                    ui.table_cell(|ui| ui.label(size));
                                });
                            }
                        },
                    );
                });
            }
            other => ui.label(other),
        });

        let fps = (1.0 / dt.max(1e-4)).min(999.0);
        ui.status_bar(|ui| {
            ui.label(&format!("{}", state.last_menu));
            ui.label("·");
            ui.label("RMB in Viewport");
            ui.label("·");
            ui.label(&format!("FPS {:.0}", fps));
        });

        false
    }
}

fn main() {
    Host::run(DockDemo::default());
}
