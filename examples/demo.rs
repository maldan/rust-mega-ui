//! Interactive mega-ui widget showcase.
//!
//! ```text
//! cargo run --example demo
//! ```

#[path = "framework.rs"]
mod framework;

use std::time::Instant;

use framework::{Host, Scene};
use glam::Vec2;
use mega_ui::{Ui, Window};

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
        }
    }
}

impl Scene for Demo {
    fn title() -> &'static str {
        "mega-ui demo"
    }

    fn build(ui: &mut Ui, state: &mut Self, _viewport: Vec2, dt: f32) -> bool {
        ui.menu_bar(|ui| {
            ui.menu("File", |ui| {
                if ui.menu_item("New").clicked() {
                    state.last_menu = String::from("File / New");
                }
                if ui.menu_item("Open…").clicked() {
                    state.last_menu = String::from("File / Open");
                }
                ui.menu("Open Recent", |ui| {
                    if ui.menu_item("project.mega").clicked() {
                        state.last_menu = String::from("File / Open Recent / project.mega");
                    }
                    if ui.menu_item("demo.mega").clicked() {
                        state.last_menu = String::from("File / Open Recent / demo.mega");
                    }
                    ui.separator();
                    ui.add_enabled(false, |ui| {
                        let _ = ui.menu_item("Clear List");
                    });
                });
                ui.separator();
                if ui.menu_item("Exit").clicked() {
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
                .size(Vec2::new(320.0, 420.0))
                .resizable(true)
                .collapsible(true),
            |ui| {
                ui.label("mega-ui demo");
                ui.label(&format!("Last menu: {}", state.last_menu));
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
                ui.label(&format!("Clicks: {}", state.clicks));
                ui.separator();
                ui.label("Plot");
                ui.plot(Vec2::new(0.0, 80.0), &state.plot);
            },
        );

        ui.window(
            Window::new("Help")
                .pos(Vec2::new(370.0, 40.0))
                .size(Vec2::new(280.0, 180.0))
                .open(&mut state.show_help),
            |ui| {
                ui.label("Drag window titles to move.");
                ui.label("Resize from the bottom-right.");
                ui.label("Menu: click File/Edit/View.");
                ui.label("Hover opens submenus.");
                ui.separator();
                ui.label(&format!("FPS ~ {:.0}", (1.0 / dt.max(1e-4)).min(999.0)));
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
