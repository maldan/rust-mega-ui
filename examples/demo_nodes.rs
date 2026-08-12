//! Node graph workspace demo (dock layout).
//!
//! ```text
//! cargo run --example demo_nodes --features wgpu
//! ```
//!
//! Controls:
//! - MMB drag (or LMB on empty) — pan
//! - Wheel — zoom
//! - Drag node title — move (snapped, default 10)
//! - Ctrl+click — multi-select; drag title moves all; Delete removes all
//! - Drag ports — connect
//! - Click wire → select, click again / Delete / Shift+click — delete wire
//! - Node ✕ or Delete — delete node(s)
//! - RMB empty canvas — spawn menu

#[path = "framework.rs"]
mod framework;

use framework::{DrawStats, Host, Scene};
use glam::Vec2;
use mega_ui::{
    port_type, DockNode, DockState, NodePortSide, NodeSpace, ScrollAxes, TextStyle, Ui,
};

#[derive(Clone, Debug)]
enum DemoKind {
    Number,
    Add,
    Print,
}

#[derive(Clone, Debug)]
struct DemoNode {
    id: String,
    kind: DemoKind,
    title: String,
    pos: Vec2,
    value: f32,
    text: String,
}

struct NodesDemo {
    dock: DockState,
    space: NodeSpace,
    nodes: Vec<DemoNode>,
    next_id: u64,
    status: String,
    log: String,
}

impl Default for NodesDemo {
    fn default() -> Self {
        let dock = DockState::new(DockNode::split_h(
            0.72,
            DockNode::leaf(&["Graph"]),
            DockNode::split_v(
                0.58,
                DockNode::leaf(&["Inspector", "Nodes"]),
                DockNode::leaf(&["Log"]),
            ),
        ));

        let mut space = NodeSpace::new();
        space.pan = Vec2::new(40.0, 40.0);

        let nodes = vec![
            DemoNode {
                id: "n1".into(),
                kind: DemoKind::Number,
                title: "Number A".into(),
                pos: Vec2::new(40.0, 80.0),
                value: 2.0,
                text: String::new(),
            },
            DemoNode {
                id: "n2".into(),
                kind: DemoKind::Number,
                title: "Number B".into(),
                pos: Vec2::new(40.0, 260.0),
                value: 3.0,
                text: String::new(),
            },
            DemoNode {
                id: "n3".into(),
                kind: DemoKind::Add,
                title: "Add".into(),
                pos: Vec2::new(320.0, 160.0),
                value: 0.0,
                text: String::new(),
            },
            DemoNode {
                id: "n4".into(),
                kind: DemoKind::Print,
                title: "Print".into(),
                pos: Vec2::new(560.0, 160.0),
                value: 0.0,
                text: String::new(),
            },
        ];

        space.links.push(mega_ui::NodeLink {
            id: 1,
            from_node: "n1".into(),
            from_port: "value".into(),
            to_node: "n3".into(),
            to_port: "a".into(),
            ty: port_type::FLOAT,
        });
        space.links.push(mega_ui::NodeLink {
            id: 2,
            from_node: "n2".into(),
            from_port: "value".into(),
            to_node: "n3".into(),
            to_port: "b".into(),
            ty: port_type::FLOAT,
        });
        space.links.push(mega_ui::NodeLink {
            id: 3,
            from_node: "n3".into(),
            from_port: "sum".into(),
            to_node: "n4".into(),
            to_port: "in".into(),
            ty: port_type::FLOAT,
        });
        space.next_link_id = 4;

        Self {
            dock,
            space,
            nodes,
            next_id: 5,
            status: "MMB pan · Ctrl+click multi · Del/x delete · RMB spawn".into(),
            log: String::from("graph ready\n"),
        }
    }
}

impl NodesDemo {
    fn spawn(&mut self, kind: DemoKind, world: Vec2) {
        let id = format!("n{}", self.next_id);
        self.next_id += 1;
        let title = match kind {
            DemoKind::Number => format!("Number {id}"),
            DemoKind::Add => "Add".into(),
            DemoKind::Print => "Print".into(),
        };
        self.log.push_str(&format!("spawn {title} ({id})\n"));
        self.status = format!("Spawned {title}");
        self.nodes.push(DemoNode {
            id,
            kind,
            title,
            pos: world,
            value: 0.0,
            text: String::new(),
        });
    }

    fn evaluate(&mut self) {
        let values: std::collections::HashMap<String, f32> = self
            .nodes
            .iter()
            .filter_map(|n| match n.kind {
                DemoKind::Number => Some((format!("{}:value", n.id), n.value)),
                _ => None,
            })
            .collect();

        let mut resolved = values;
        for n in &self.nodes {
            if let DemoKind::Add = n.kind {
                let a = input_value(&self.space, &resolved, &n.id, "a");
                let b = input_value(&self.space, &resolved, &n.id, "b");
                resolved.insert(format!("{}:sum", n.id), a + b);
            }
        }

        for n in &mut self.nodes {
            match n.kind {
                DemoKind::Add => {
                    n.value = *resolved.get(&format!("{}:sum", n.id)).unwrap_or(&0.0);
                }
                DemoKind::Print => {
                    let v = input_value(&self.space, &resolved, &n.id, "in");
                    n.value = v;
                    n.text = format!("{v:.4}");
                }
                DemoKind::Number => {}
            }
        }
    }

    fn kind_label(kind: &DemoKind) -> &'static str {
        match kind {
            DemoKind::Number => "Number",
            DemoKind::Add => "Add",
            DemoKind::Print => "Print",
        }
    }
}

fn input_value(
    space: &NodeSpace,
    resolved: &std::collections::HashMap<String, f32>,
    node: &str,
    port: &str,
) -> f32 {
    for link in &space.links {
        if link.to_node == node && link.to_port == port {
            let key = format!("{}:{}", link.from_node, link.from_port);
            return *resolved.get(&key).unwrap_or(&0.0);
        }
    }
    0.0
}

impl Scene for NodesDemo {
    fn title() -> &'static str {
        "mega-ui — nodes"
    }

    fn window_size() -> (f64, f64) {
        (1280.0, 800.0)
    }

    fn init(ui: &mut Ui) {
        ui.load_builtin_icons();
    }

    fn build(ui: &mut Ui, state: &mut Self, viewport: Vec2, _dt: f32, _stats: DrawStats) -> bool {
        for id in state.space.take_delete_nodes() {
            state.nodes.retain(|n| n.id != id);
            state.status = format!("Deleted {id}");
            state.log.push_str(&format!("deleted {id}\n"));
        }
        state.evaluate();

        ui.menu_bar(|ui| {
            ui.menu("Graph", |ui| {
                if ui.menu_item("Fit origin").clicked() {
                    state.space.pan = Vec2::new(40.0, 40.0);
                    state.space.zoom = 1.0;
                    state.log.push_str("fit origin\n");
                }
                if ui.menu_item("Clear selection").clicked() {
                    state.space.selected_nodes.clear();
                    state.space.selected_link = None;
                }
                ui.separator();
                if ui.menu_item("Spawn Number").clicked() {
                    state.spawn(DemoKind::Number, Vec2::new(120.0, 120.0));
                }
                if ui.menu_item("Spawn Add").clicked() {
                    state.spawn(DemoKind::Add, Vec2::new(280.0, 140.0));
                }
                if ui.menu_item("Spawn Print").clicked() {
                    state.spawn(DemoKind::Print, Vec2::new(440.0, 140.0));
                }
            });
        });

        let status_h = 24.0 * ui.scale();
        let dock_size = Vec2::new(viewport.x, (viewport.y - 26.0 * ui.scale() - status_h).max(1.0));

        {
            let NodesDemo {
                dock,
                space,
                nodes,
                next_id,
                status,
                log,
                ..
            } = state;

            ui.dock_space("main", dock_size, dock, |ui, tab| match tab {
                "Graph" => {
                    ui.label_styled(
                        "Graph",
                        TextStyle {
                            color: [0.85, 0.85, 0.85, 1.0],
                            size: 15.0,
                        },
                    );
                    ui.label(&status.clone());
                    ui.separator();

                    let size = ui.available_size();
                    let size = Vec2::new(size.x, size.y.max(120.0));
                    ui.node_space("demo_graph", size, space, |ui| {
                        for n in nodes.iter_mut() {
                            let id = n.id.clone();
                            let title = n.title.clone();
                            let kind = n.kind.clone();
                            ui.node(&id, &title, &mut n.pos, |ui| match kind {
                                DemoKind::Number => {
                                    ui.node_port(NodePortSide::Output, "value", port_type::FLOAT);
                                    ui.drag_float("v", &mut n.value, 0.1);
                                }
                                DemoKind::Add => {
                                    ui.node_port(NodePortSide::Input, "a", port_type::FLOAT);
                                    ui.node_port(NodePortSide::Input, "b", port_type::FLOAT);
                                    ui.separator();
                                    ui.label(&format!("= {:.3}", n.value));
                                    ui.node_port(NodePortSide::Output, "sum", port_type::FLOAT);
                                }
                                DemoKind::Print => {
                                    ui.node_port(NodePortSide::Input, "in", port_type::FLOAT);
                                    if n.text.is_empty() {
                                        ui.label("(no input)");
                                    } else {
                                        ui.label(&n.text.clone());
                                    }
                                }
                            });
                        }
                    });

                    let bg = space.background_hovered;
                    let world = space.context_world.unwrap_or(Vec2::new(100.0, 100.0));
                    let mut spawn_kind: Option<DemoKind> = None;
                    ui.context_menu("spawn_menu", bg, |ui| {
                        if ui.menu_item("Number").clicked() {
                            spawn_kind = Some(DemoKind::Number);
                        }
                        if ui.menu_item("Add").clicked() {
                            spawn_kind = Some(DemoKind::Add);
                        }
                        if ui.menu_item("Print").clicked() {
                            spawn_kind = Some(DemoKind::Print);
                        }
                    });
                    if let Some(kind) = spawn_kind {
                        let id = format!("n{next_id}");
                        *next_id += 1;
                        let title = match kind {
                            DemoKind::Number => format!("Number {id}"),
                            DemoKind::Add => "Add".into(),
                            DemoKind::Print => "Print".into(),
                        };
                        log.push_str(&format!("spawn {title} ({id})\n"));
                        *status = format!("Spawned {title}");
                        nodes.push(DemoNode {
                            id,
                            kind,
                            title,
                            pos: world,
                            value: 0.0,
                            text: String::new(),
                        });
                    }
                }
                "Inspector" => {
                    let size = ui.available_size();
                    ui.scroll_area("inspector", size, ScrollAxes::Vertical, |ui| {
                        ui.label_styled(
                            "Inspector",
                            TextStyle {
                                color: [0.85, 0.85, 0.85, 1.0],
                                size: 15.0,
                            },
                        );
                        ui.separator();
                        ui.label(&format!("Zoom: {:.2}", space.zoom));
                        ui.label(&format!("Snap: {:.0}", space.snap));
                        ui.drag_float("snap", &mut space.snap, 1.0);
                        if space.snap < 0.0 {
                            space.snap = 0.0;
                        }
                        ui.label(&format!(
                            "Pan: ({:.0}, {:.0})",
                            space.pan.x, space.pan.y
                        ));
                        ui.label(&format!("Nodes: {}", nodes.len()));
                        ui.label(&format!("Links: {}", space.links.len()));
                        ui.label(&format!("Selected: {}", space.selected_nodes.len()));
                        ui.separator();

                        if space.selected_nodes.len() > 1 {
                            ui.label(&format!(
                                "Multi-select ({} nodes)",
                                space.selected_nodes.len()
                            ));
                            if ui.button("Delete selected").clicked() {
                                let ids = space.selected_nodes.clone();
                                for id in &ids {
                                    space.detach_node(id);
                                }
                                space.request_delete_nodes.extend(ids);
                            }
                            ui.separator();
                        }

                        if let Some(sel) = space.selected_nodes.first().cloned() {
                            if let Some(n) = nodes.iter_mut().find(|n| n.id == sel) {
                                ui.label(&format!("Selected: {}", n.title));
                                ui.label(&format!("Id: {}", n.id));
                                ui.label(&format!("Type: {}", NodesDemo::kind_label(&n.kind)));
                                ui.label(&format!("Pos: ({:.0}, {:.0})", n.pos.x, n.pos.y));
                                ui.separator();
                                match n.kind {
                                    DemoKind::Number => {
                                        ui.label("Value");
                                        ui.drag_float("insp_v", &mut n.value, 0.1);
                                    }
                                    DemoKind::Add => {
                                        ui.label(&format!("Sum = {:.4}", n.value));
                                    }
                                    DemoKind::Print => {
                                        ui.label(&format!("Output: {}", n.text));
                                    }
                                }
                                ui.separator();
                                if ui.button("Delete node").clicked() {
                                    space.detach_node(&sel);
                                    space.request_delete_nodes.push(sel);
                                }
                            }
                        } else if let Some(lid) = space.selected_link {
                            if let Some(link) = space.links.iter().find(|l| l.id == lid) {
                                ui.label("Selected link");
                                ui.label(&format!(
                                    "{}:{} -> {}:{}",
                                    link.from_node, link.from_port, link.to_node, link.to_port
                                ));
                                ui.separator();
                                if ui.button("Delete link").clicked() {
                                    space.remove_link(lid);
                                }
                            }
                        } else {
                            ui.label("Nothing selected");
                            ui.label("Click a node or wire");
                        }
                    });
                }
                "Nodes" => {
                    let size = ui.available_size();
                    ui.scroll_area("nodes_list", size, ScrollAxes::Vertical, |ui| {
                        ui.label_styled(
                            "Nodes",
                            TextStyle {
                                color: [0.85, 0.85, 0.85, 1.0],
                                size: 15.0,
                            },
                        );
                        ui.separator();
                        let mut focus: Option<String> = None;
                        for n in nodes.iter() {
                            let selected = space.selected_nodes.iter().any(|id| id == &n.id);
                            let label = format!(
                                "{}  [{}]  ({:.0},{:.0})",
                                n.title,
                                NodesDemo::kind_label(&n.kind),
                                n.pos.x,
                                n.pos.y
                            );
                            if ui
                                .selectable(&n.id, selected, |ui| {
                                    ui.label(&label);
                                })
                                .clicked()
                            {
                                focus = Some(n.id.clone());
                            }
                        }
                        if let Some(id) = focus {
                            space.selected_nodes = vec![id];
                            space.selected_link = None;
                        }
                    });
                }
                "Log" => {
                    let size = ui.available_size();
                    ui.scroll_area("log", size, ScrollAxes::Vertical, |ui| {
                        ui.label_styled(
                            "Log",
                            TextStyle {
                                color: [0.85, 0.85, 0.85, 1.0],
                                size: 15.0,
                            },
                        );
                        ui.separator();
                        if ui.button("Clear").clicked() {
                            log.clear();
                        }
                        ui.separator();
                        for line in log.lines() {
                            ui.label(line);
                        }
                    });
                }
                _ => {
                    ui.label(tab);
                }
            });
        }

        ui.status_bar(|ui| {
            ui.label(&state.status.clone());
            ui.spacer();
            ui.label(&format!(
                "zoom {:.0}% · {} nodes · {} links",
                state.space.zoom * 100.0,
                state.nodes.len(),
                state.space.links.len()
            ));
        });

        true
    }
}

fn main() {
    Host::run(NodesDemo::default());
}
