//! Dock layout + UI scale demo.
//!
//! ```text
//! cargo run --example demo_dock
//! ```

#[path = "framework.rs"]
mod framework;

use framework::{Host, Scene};
use glam::Vec2;
use mega_ui::{DockNode, DockState, ScrollAxes, Ui, Window};

struct DockDemo {
    dock: DockState,
    scale: f32,
    name: String,
    fov: f32,
    wireframe: bool,
    log: String,
    last_menu: String,
}

impl Default for DockDemo {
    fn default() -> Self {
        let dock = DockState::new(DockNode::split_h(
            0.68,
            DockNode::leaf(&["Viewport", "Scene"]),
            DockNode::split_v(
                0.58,
                DockNode::leaf(&["Inspector", "Settings"]),
                DockNode::leaf(&["Console"]),
            ),
        ));
        Self {
            dock,
            scale: 1.0,
            name: String::from("Main Camera"),
            fov: 60.0,
            wireframe: false,
            log: String::from("dock ready\ndrag splitters to resize panes\n"),
            last_menu: String::from("(none)"),
        }
    }
}

impl Scene for DockDemo {
    fn title() -> &'static str {
        "mega-ui dock demo"
    }

    fn window_size() -> (f64, f64) {
        (1100.0, 720.0)
    }

    fn init(ui: &mut Ui) {
        ui.load_builtin_icons();
    }

    fn build(ui: &mut Ui, state: &mut Self, viewport: Vec2, dt: f32) -> bool {
        ui.set_scale(state.scale);

        ui.menu_bar(|ui| {
            ui.menu("File", |ui| {
                if ui.menu_item_icon("plus", "New Scene").clicked() {
                    state.last_menu = String::from("File / New Scene");
                    state.log.push_str("new scene\n");
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
                }
            });
        });

        ui.set_scale(state.scale);

        let bar_h = 26.0 * state.scale;
        ui.window(
            Window::new("UI Scale")
                .pos(Vec2::new(16.0, bar_h + 12.0))
                .size(Vec2::new(260.0, 130.0)),
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
                ui.label(&format!("FPS ~ {:.0}", (1.0 / dt.max(1e-4)).min(999.0)));
            },
        );

        ui.set_scale(state.scale);

        let DockDemo {
            dock,
            name,
            fov,
            wireframe,
            log,
            scale,
            ..
        } = state;

        let dock_size = Vec2::new(viewport.x, (viewport.y - 26.0 * *scale).max(1.0));
        ui.dock_space("main", dock_size, dock, |ui, tab| match tab {
            "Viewport" => {
                ui.label("Viewport");
                ui.separator();
                ui.texture(0, ui.available_size());
            }
            "Scene" => {
                ui.label("Scene hierarchy");
                ui.separator();
                ui.tree_node("world", "World", |ui| {
                    ui.tree_node("camera", "Camera", |ui| {
                        ui.label(name.as_str());
                    });
                    ui.tree_node("lights", "Lights", |ui| {
                        ui.label("Sun");
                        ui.label("Fill");
                    });
                    ui.tree_node("meshes", "Meshes", |ui| {
                        ui.label("Cube");
                        ui.label("Plane");
                    });
                });
            }
            "Inspector" => {
                ui.label("Inspector");
                ui.separator();
                ui.text_input("name", name);
                ui.slider("FOV", fov, 20.0..=120.0);
                ui.checkbox("Wireframe", wireframe);
                ui.separator();
                ui.label(&format!("scale (ui) = {:.2}", *scale));
            }
            "Settings" => {
                ui.label("Settings");
                ui.separator();
                ui.label("Drag dock splitters to resize.");
                ui.label("Tabs switch active leaf content.");
                ui.label("Menu bar + UI Scale window set scale.");
            }
            "Console" => {
                ui.label("Console");
                ui.separator();
                let size = ui.available_size();
                ui.scroll_area("log", size, ScrollAxes::Vertical, |ui| {
                    for line in log.lines() {
                        ui.label(line);
                    }
                });
            }
            other => ui.label(other),
        });

        false
    }
}

fn main() {
    Host::run(DockDemo::default());
}
