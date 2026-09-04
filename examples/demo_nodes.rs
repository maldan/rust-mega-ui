//! Node graph workspace demo (dock layout).
//!
//! ```text
//! cargo run --example demo_nodes --features wgpu
//! ```
//!
//! Controls:
//! - MMB drag — pan
//! - LMB drag empty — marquee select (Ctrl = add to selection)
//! - Ctrl+click node — toggle multi-select
//! - Wheel — zoom
//! - Drag node title — move group (snap default 5)
//! - Drag frame — move all member nodes
//! - Inspector Group selected — wrap selection in a frame
//! - Drag ports — connect
//! - RMB on wire — delete link
//! - Selected wire + Delete — also deletes
//! - Delete / Backspace — delete selected node(s) (host applies)
//! - Inspector Delete — same, host-owned UI
//! - Ctrl+D — clone selected node(s); selection moves to clones
//! - RMB empty canvas — spawn menu

#[path = "framework.rs"]
mod framework;

use std::collections::HashMap;

use framework::{DrawStats, Host, Scene};
use glam::{Mat4, Quat, Vec2, Vec3};
use mega_ui::{
    port_type, DockNode, DockState, GradientStop, NodePortSide, NodeSpace, OpacityStop,
    ScrollAxes, TextStyle, Ui, sample_gradient,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DemoKind {
    Float,
    Add,
    Mul,
    Vec3,
    Vec3Add,
    Vec3Scale,
    Vec3Dot,
    Vec3Cross,
    Vec3Length,
    Vec3Normalize,
    Mat4Identity,
    Mat4Translate,
    Mat4Euler,
    Mat4Scale,
    Mat4Mul,
    Mat4MulVec,
    QuatIdentity,
    QuatEuler,
    QuatMul,
    QuatRotateVec,
    String,
    StringConcat,
    Print,
}

#[derive(Clone, Debug)]
struct DemoNode {
    id: String,
    kind: DemoKind,
    title: String,
    pos: Vec2,
    /// Scalar / vec / mat / quat components (column-major for mat4).
    floats: [f32; 16],
    text: String,
    /// Host-evaluated preview string.
    preview: String,
}

#[derive(Clone, Debug)]
enum Val {
    F(f32),
    V3(Vec3),
    M4(Mat4),
    Q(Quat),
    S(String),
}

struct NodesDemo {
    dock: DockState,
    space: NodeSpace,
    nodes: Vec<DemoNode>,
    next_id: u64,
    status: String,
    log: String,
    scale: f32,
    gradient_stops: Vec<GradientStop>,
    gradient_opacities: Vec<OpacityStop>,
    gradient_sample: f32,
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
        space.pan = Vec2::new(20.0, 20.0);

        let mut nodes = Vec::new();
        let mut next_id = 1u64;

        // Float chain
        let n_a = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Float,
            "Float A",
            Vec2::new(40.0, 40.0),
            |n| n.floats[0] = 2.0,
        );
        let n_b = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Float,
            "Float B",
            Vec2::new(40.0, 180.0),
            |n| n.floats[0] = 3.0,
        );
        let n_add = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Add,
            "Add",
            Vec2::new(260.0, 100.0),
            |_| {},
        );
        let n_print_f = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Print,
            "Print float",
            Vec2::new(480.0, 100.0),
            |_| {},
        );

        // Vec3 chain
        let n_v1 = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Vec3,
            "Vec3 A",
            Vec2::new(40.0, 340.0),
            |n| {
                n.floats[0] = 1.0;
                n.floats[1] = 0.0;
                n.floats[2] = 0.0;
            },
        );
        let n_v2 = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Vec3,
            "Vec3 B",
            Vec2::new(40.0, 520.0),
            |n| {
                n.floats[0] = 0.0;
                n.floats[1] = 1.0;
                n.floats[2] = 0.0;
            },
        );
        let n_cross = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Vec3Cross,
            "Cross",
            Vec2::new(280.0, 420.0),
            |_| {},
        );
        let n_print_v = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Print,
            "Print vec",
            Vec2::new(500.0, 420.0),
            |_| {},
        );

        // Transform chain
        let n_t = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Mat4Translate,
            "Translate",
            Vec2::new(40.0, 720.0),
            |n| {
                n.floats[0] = 1.0;
                n.floats[1] = 2.0;
                n.floats[2] = 0.0;
            },
        );
        let n_e = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::QuatEuler,
            "Quat Euler",
            Vec2::new(40.0, 920.0),
            |n| {
                n.floats[1] = 45.0; // yaw deg
            },
        );
        let n_rot_v = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::QuatRotateVec,
            "Quat × Vec",
            Vec2::new(300.0, 820.0),
            |_| {},
        );
        let n_mv = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Mat4MulVec,
            "Mat4 × Vec",
            Vec2::new(540.0, 780.0),
            |_| {},
        );

        // Strings
        let n_s1 = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::String,
            "Hello",
            Vec2::new(760.0, 40.0),
            |n| n.text = "hello".into(),
        );
        let n_s2 = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::String,
            "World",
            Vec2::new(760.0, 180.0),
            |n| n.text = " world".into(),
        );
        let n_cat = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::StringConcat,
            "Concat",
            Vec2::new(1000.0, 100.0),
            |_| {},
        );
        let n_print_s = push_node(
            &mut nodes,
            &mut next_id,
            DemoKind::Print,
            "Print str",
            Vec2::new(1220.0, 100.0),
            |_| {},
        );

        link(&mut space, &n_a, "value", &n_add, "a", port_type::FLOAT);
        link(&mut space, &n_b, "value", &n_add, "b", port_type::FLOAT);
        link(&mut space, &n_add, "sum", &n_print_f, "in", port_type::FLOAT);
        link(&mut space, &n_v1, "v", &n_cross, "a", port_type::VEC3);
        link(&mut space, &n_v2, "v", &n_cross, "b", port_type::VEC3);
        link(&mut space, &n_cross, "out", &n_print_v, "in", port_type::VEC3);
        link(&mut space, &n_e, "q", &n_rot_v, "q", port_type::QUAT);
        link(&mut space, &n_v1, "v", &n_rot_v, "v", port_type::VEC3);
        link(&mut space, &n_t, "m", &n_mv, "m", port_type::MAT4);
        link(&mut space, &n_rot_v, "out", &n_mv, "v", port_type::VEC3);
        link(&mut space, &n_s1, "text", &n_cat, "a", port_type::STRING);
        link(&mut space, &n_s2, "text", &n_cat, "b", port_type::STRING);
        link(
            &mut space,
            &n_cat,
            "out",
            &n_print_s,
            "in",
            port_type::STRING,
        );

        Self {
            dock,
            space,
            nodes,
            next_id,
            status: "Ctrl+D clone · Delete remove · RMB wire del".into(),
            log: String::from("graph ready\n"),
            scale: 1.0,
            gradient_stops: vec![
                GradientStop {
                    t: 0.0,
                    color: [0.12, 0.32, 0.72, 1.0],
                },
                GradientStop {
                    t: 0.5,
                    color: [0.95, 0.52, 0.14, 1.0],
                },
                GradientStop {
                    t: 1.0,
                    color: [0.85, 0.28, 0.28, 1.0],
                },
            ],
            gradient_opacities: vec![
                OpacityStop {
                    t: 0.0,
                    alpha: 1.0,
                },
                OpacityStop {
                    t: 1.0,
                    alpha: 1.0,
                },
            ],
            gradient_sample: 0.5,
        }
    }
}

fn push_node(
    nodes: &mut Vec<DemoNode>,
    next_id: &mut u64,
    kind: DemoKind,
    title: &str,
    pos: Vec2,
    init: impl FnOnce(&mut DemoNode),
) -> String {
    let id = format!("n{next_id}");
    *next_id += 1;
    let mut n = DemoNode {
        id: id.clone(),
        kind,
        title: title.into(),
        pos,
        floats: [0.0; 16],
        text: String::new(),
        preview: String::new(),
    };
    init(&mut n);
    nodes.push(n);
    id
}

fn link(space: &mut NodeSpace, from: &str, from_port: &str, to: &str, to_port: &str, ty: u16) {
    let id = space.next_link_id;
    space.next_link_id += 1;
    space.links.push(mega_ui::NodeLink {
        id,
        from_node: from.into(),
        from_port: from_port.into(),
        to_node: to.into(),
        to_port: to_port.into(),
        ty,
    });
}

impl NodesDemo {
    fn kind_title(kind: DemoKind) -> &'static str {
        match kind {
            DemoKind::Float => "Float",
            DemoKind::Add => "Add",
            DemoKind::Mul => "Mul",
            DemoKind::Vec3 => "Vec3",
            DemoKind::Vec3Add => "Vec3 Add",
            DemoKind::Vec3Scale => "Vec3 Scale",
            DemoKind::Vec3Dot => "Dot",
            DemoKind::Vec3Cross => "Cross",
            DemoKind::Vec3Length => "Length",
            DemoKind::Vec3Normalize => "Normalize",
            DemoKind::Mat4Identity => "Mat4 Identity",
            DemoKind::Mat4Translate => "Mat4 Translate",
            DemoKind::Mat4Euler => "Mat4 Euler",
            DemoKind::Mat4Scale => "Mat4 Scale",
            DemoKind::Mat4Mul => "Mat4 Mul",
            DemoKind::Mat4MulVec => "Mat4 × Vec",
            DemoKind::QuatIdentity => "Quat Identity",
            DemoKind::QuatEuler => "Quat Euler",
            DemoKind::QuatMul => "Quat Mul",
            DemoKind::QuatRotateVec => "Quat × Vec",
            DemoKind::String => "String",
            DemoKind::StringConcat => "Concat",
            DemoKind::Print => "Print",
        }
    }

    fn spawn(&mut self, kind: DemoKind, world: Vec2) {
        let id = format!("n{}", self.next_id);
        self.next_id += 1;
        let title = Self::kind_title(kind).to_string();
        self.log.push_str(&format!("spawn {title} ({id})\n"));
        self.status = format!("Spawned {title}");
        self.nodes.push(DemoNode {
            id,
            kind,
            title,
            pos: world,
            floats: {
                let mut f = [0.0; 16];
                if kind == DemoKind::Mat4Scale {
                    f[0] = 1.0;
                    f[1] = 1.0;
                    f[2] = 1.0;
                }
                f
            },
            text: String::new(),
            preview: String::new(),
        });
    }

    fn apply_deletes(&mut self) {
        for id in self.space.take_delete_nodes() {
            self.nodes.retain(|n| n.id != id);
            self.status = format!("Deleted {id}");
            self.log.push_str(&format!("deleted {id}\n"));
        }
    }

    fn apply_clones(&mut self) {
        let ids = self.space.take_clone_nodes();
        if ids.is_empty() {
            return;
        }
        let offset = self.space.clone_offset();
        let mut id_map = HashMap::new();
        let mut new_sel = Vec::new();
        for old_id in &ids {
            let Some(src) = self.nodes.iter().find(|n| n.id == *old_id).cloned() else {
                continue;
            };
            let new_id = format!("n{}", self.next_id);
            self.next_id += 1;
            let mut clone = src;
            clone.id = new_id.clone();
            clone.pos += offset;
            id_map.insert(old_id.clone(), new_id.clone());
            new_sel.push(new_id.clone());
            self.log.push_str(&format!("clone {old_id} → {new_id}\n"));
            self.nodes.push(clone);
        }
        self.space.duplicate_links(&id_map);
        self.space.selected_nodes = new_sel;
        self.space.selected_link = None;
        self.status = format!("Cloned {} node(s)", id_map.len());
    }

    fn evaluate(&mut self) {
        let mut resolved: HashMap<String, Val> = HashMap::new();

        // Sources first
        for n in &self.nodes {
            match n.kind {
                DemoKind::Float => {
                    resolved.insert(format!("{}:value", n.id), Val::F(n.floats[0]));
                }
                DemoKind::Vec3 => {
                    resolved.insert(
                        format!("{}:v", n.id),
                        Val::V3(Vec3::new(n.floats[0], n.floats[1], n.floats[2])),
                    );
                }
                DemoKind::Mat4Identity => {
                    resolved.insert(format!("{}:m", n.id), Val::M4(Mat4::IDENTITY));
                }
                DemoKind::Mat4Translate => {
                    let t = Vec3::new(n.floats[0], n.floats[1], n.floats[2]);
                    resolved.insert(format!("{}:m", n.id), Val::M4(Mat4::from_translation(t)));
                }
                DemoKind::Mat4Euler => {
                    let rad = |d: f32| d.to_radians();
                    let m = Mat4::from_euler(
                        glam::EulerRot::YXZ,
                        rad(n.floats[1]),
                        rad(n.floats[0]),
                        rad(n.floats[2]),
                    );
                    resolved.insert(format!("{}:m", n.id), Val::M4(m));
                }
                DemoKind::Mat4Scale => {
                    let s = Vec3::new(n.floats[0], n.floats[1], n.floats[2]);
                    resolved.insert(format!("{}:m", n.id), Val::M4(Mat4::from_scale(s)));
                }
                DemoKind::QuatIdentity => {
                    resolved.insert(format!("{}:q", n.id), Val::Q(Quat::IDENTITY));
                }
                DemoKind::QuatEuler => {
                    let rad = |d: f32| d.to_radians();
                    let q = Quat::from_euler(
                        glam::EulerRot::YXZ,
                        rad(n.floats[1]),
                        rad(n.floats[0]),
                        rad(n.floats[2]),
                    );
                    resolved.insert(format!("{}:q", n.id), Val::Q(q));
                }
                DemoKind::String => {
                    resolved.insert(format!("{}:text", n.id), Val::S(n.text.clone()));
                }
                _ => {}
            }
        }

        // Multi-pass for ops (small graphs; enough for demo)
        for _ in 0..8 {
            for n in &self.nodes {
                match n.kind {
                    DemoKind::Add => {
                        let a = as_f(input_val(&self.space, &resolved, &n.id, "a"));
                        let b = as_f(input_val(&self.space, &resolved, &n.id, "b"));
                        resolved.insert(format!("{}:sum", n.id), Val::F(a + b));
                    }
                    DemoKind::Mul => {
                        let a = as_f(input_val(&self.space, &resolved, &n.id, "a"));
                        let b = as_f(input_val(&self.space, &resolved, &n.id, "b"));
                        resolved.insert(format!("{}:prod", n.id), Val::F(a * b));
                    }
                    DemoKind::Vec3Add => {
                        let a = as_v3(input_val(&self.space, &resolved, &n.id, "a"));
                        let b = as_v3(input_val(&self.space, &resolved, &n.id, "b"));
                        resolved.insert(format!("{}:out", n.id), Val::V3(a + b));
                    }
                    DemoKind::Vec3Scale => {
                        let v = as_v3(input_val(&self.space, &resolved, &n.id, "v"));
                        let s = as_f(input_val(&self.space, &resolved, &n.id, "s"));
                        resolved.insert(format!("{}:out", n.id), Val::V3(v * s));
                    }
                    DemoKind::Vec3Dot => {
                        let a = as_v3(input_val(&self.space, &resolved, &n.id, "a"));
                        let b = as_v3(input_val(&self.space, &resolved, &n.id, "b"));
                        resolved.insert(format!("{}:out", n.id), Val::F(a.dot(b)));
                    }
                    DemoKind::Vec3Cross => {
                        let a = as_v3(input_val(&self.space, &resolved, &n.id, "a"));
                        let b = as_v3(input_val(&self.space, &resolved, &n.id, "b"));
                        resolved.insert(format!("{}:out", n.id), Val::V3(a.cross(b)));
                    }
                    DemoKind::Vec3Length => {
                        let v = as_v3(input_val(&self.space, &resolved, &n.id, "v"));
                        resolved.insert(format!("{}:out", n.id), Val::F(v.length()));
                    }
                    DemoKind::Vec3Normalize => {
                        let v = as_v3(input_val(&self.space, &resolved, &n.id, "v"));
                        resolved.insert(
                            format!("{}:out", n.id),
                            Val::V3(v.try_normalize().unwrap_or(Vec3::ZERO)),
                        );
                    }
                    DemoKind::Mat4Mul => {
                        let a = as_m4(input_val(&self.space, &resolved, &n.id, "a"));
                        let b = as_m4(input_val(&self.space, &resolved, &n.id, "b"));
                        resolved.insert(format!("{}:out", n.id), Val::M4(a * b));
                    }
                    DemoKind::Mat4MulVec => {
                        let m = as_m4(input_val(&self.space, &resolved, &n.id, "m"));
                        let v = as_v3(input_val(&self.space, &resolved, &n.id, "v"));
                        let out = m.transform_point3(v);
                        resolved.insert(format!("{}:out", n.id), Val::V3(out));
                    }
                    DemoKind::QuatMul => {
                        let a = as_q(input_val(&self.space, &resolved, &n.id, "a"));
                        let b = as_q(input_val(&self.space, &resolved, &n.id, "b"));
                        resolved.insert(format!("{}:out", n.id), Val::Q((a * b).normalize()));
                    }
                    DemoKind::QuatRotateVec => {
                        let q = as_q(input_val(&self.space, &resolved, &n.id, "q"));
                        let v = as_v3(input_val(&self.space, &resolved, &n.id, "v"));
                        resolved.insert(format!("{}:out", n.id), Val::V3(q * v));
                    }
                    DemoKind::StringConcat => {
                        let a = as_s(input_val(&self.space, &resolved, &n.id, "a"));
                        let b = as_s(input_val(&self.space, &resolved, &n.id, "b"));
                        resolved.insert(format!("{}:out", n.id), Val::S(format!("{a}{b}")));
                    }
                    DemoKind::Print => {
                        if let Some(v) = input_val(&self.space, &resolved, &n.id, "in") {
                            resolved.insert(format!("{}:out", n.id), v);
                        }
                    }
                    _ => {}
                }
            }
        }

        for n in &mut self.nodes {
            let key = match n.kind {
                DemoKind::Float => format!("{}:value", n.id),
                DemoKind::Add => format!("{}:sum", n.id),
                DemoKind::Mul => format!("{}:prod", n.id),
                DemoKind::Vec3 => format!("{}:v", n.id),
                DemoKind::Mat4Identity
                | DemoKind::Mat4Translate
                | DemoKind::Mat4Euler
                | DemoKind::Mat4Scale => format!("{}:m", n.id),
                DemoKind::QuatIdentity | DemoKind::QuatEuler => format!("{}:q", n.id),
                DemoKind::String => format!("{}:text", n.id),
                DemoKind::Print => format!("{}:out", n.id),
                _ => format!("{}:out", n.id),
            };
            n.preview = resolved
                .get(&key)
                .map(format_val)
                .or_else(|| input_val(&self.space, &resolved, &n.id, "in").map(|v| format_val(&v)))
                .unwrap_or_default();
        }
    }
}

fn input_val(
    space: &NodeSpace,
    resolved: &HashMap<String, Val>,
    node: &str,
    port: &str,
) -> Option<Val> {
    for link in &space.links {
        if link.to_node == node && link.to_port == port {
            let key = format!("{}:{}", link.from_node, link.from_port);
            return resolved.get(&key).cloned();
        }
    }
    None
}

fn as_f(v: Option<Val>) -> f32 {
    match v {
        Some(Val::F(x)) => x,
        Some(Val::V3(x)) => x.x,
        _ => 0.0,
    }
}

fn as_v3(v: Option<Val>) -> Vec3 {
    match v {
        Some(Val::V3(x)) => x,
        Some(Val::F(x)) => Vec3::splat(x),
        Some(Val::Q(q)) => Vec3::new(q.x, q.y, q.z),
        _ => Vec3::ZERO,
    }
}

fn as_m4(v: Option<Val>) -> Mat4 {
    match v {
        Some(Val::M4(m)) => m,
        _ => Mat4::IDENTITY,
    }
}

fn as_q(v: Option<Val>) -> Quat {
    match v {
        Some(Val::Q(q)) => q,
        _ => Quat::IDENTITY,
    }
}

fn as_s(v: Option<Val>) -> String {
    match v {
        Some(Val::S(s)) => s,
        Some(other) => format_val(&other),
        None => String::new(),
    }
}

fn format_val(v: &Val) -> String {
    match v {
        Val::F(x) => format!("{x:.4}"),
        Val::V3(v) => format!("({:.3}, {:.3}, {:.3})", v.x, v.y, v.z),
        Val::M4(m) => {
            let t = m.w_axis;
            format!("T({:.2},{:.2},{:.2})", t.x, t.y, t.z)
        }
        Val::Q(q) => format!("q({:.3},{:.3},{:.3},{:.3})", q.x, q.y, q.z, q.w),
        Val::S(s) => s.clone(),
    }
}

fn edit_vec3(ui: &mut Ui, id: &str, f: &mut [f32; 16], step: f32, default: Vec3) {
    let mut v = Vec3::new(f[0], f[1], f[2]);
    ui.vec3(id, &mut v, step, default);
    f[0] = v.x;
    f[1] = v.y;
    f[2] = v.z;
}

fn draw_node_body(
    ui: &mut Ui,
    kind: DemoKind,
    floats: &mut [f32; 16],
    text: &mut String,
    preview: &str,
) {
    match kind {
        DemoKind::Float => {
            ui.node_port(NodePortSide::Output, "value", port_type::FLOAT);
            ui.drag_float("v", &mut floats[0], 0.1);
        }
        DemoKind::Add => {
            ui.node_port(NodePortSide::Input, "a", port_type::FLOAT);
            ui.node_port(NodePortSide::Input, "b", port_type::FLOAT);
            ui.separator();
            ui.label(&format!("= {preview}"));
            ui.node_port(NodePortSide::Output, "sum", port_type::FLOAT);
        }
        DemoKind::Mul => {
            ui.node_port(NodePortSide::Input, "a", port_type::FLOAT);
            ui.node_port(NodePortSide::Input, "b", port_type::FLOAT);
            ui.separator();
            ui.label(&format!("= {preview}"));
            ui.node_port(NodePortSide::Output, "prod", port_type::FLOAT);
        }
        DemoKind::Vec3 => {
            ui.node_port(NodePortSide::Output, "v", port_type::VEC3);
            edit_vec3(ui, "v3", floats, 0.1, Vec3::ZERO);
        }
        DemoKind::Vec3Add | DemoKind::Vec3Cross => {
            ui.node_port(NodePortSide::Input, "a", port_type::VEC3);
            ui.node_port(NodePortSide::Input, "b", port_type::VEC3);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::VEC3);
        }
        DemoKind::Vec3Scale => {
            ui.node_port(NodePortSide::Input, "v", port_type::VEC3);
            ui.node_port(NodePortSide::Input, "s", port_type::FLOAT);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::VEC3);
        }
        DemoKind::Vec3Dot => {
            ui.node_port(NodePortSide::Input, "a", port_type::VEC3);
            ui.node_port(NodePortSide::Input, "b", port_type::VEC3);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::FLOAT);
        }
        DemoKind::Vec3Length => {
            ui.node_port(NodePortSide::Input, "v", port_type::VEC3);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::FLOAT);
        }
        DemoKind::Vec3Normalize => {
            ui.node_port(NodePortSide::Input, "v", port_type::VEC3);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::VEC3);
        }
        DemoKind::Mat4Identity => {
            ui.label("identity");
            ui.node_port(NodePortSide::Output, "m", port_type::MAT4);
        }
        DemoKind::Mat4Translate => {
            edit_vec3(ui, "t", floats, 0.1, Vec3::ZERO);
            ui.node_port(NodePortSide::Output, "m", port_type::MAT4);
        }
        DemoKind::Mat4Scale => {
            edit_vec3(ui, "s", floats, 0.01, Vec3::ONE);
            ui.node_port(NodePortSide::Output, "m", port_type::MAT4);
        }
        DemoKind::Mat4Euler => {
            ui.label("pitch/yaw/roll °");
            edit_vec3(ui, "e", floats, 1.0, Vec3::ZERO);
            ui.node_port(NodePortSide::Output, "m", port_type::MAT4);
        }
        DemoKind::Mat4Mul => {
            ui.node_port(NodePortSide::Input, "a", port_type::MAT4);
            ui.node_port(NodePortSide::Input, "b", port_type::MAT4);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::MAT4);
        }
        DemoKind::Mat4MulVec => {
            ui.node_port(NodePortSide::Input, "m", port_type::MAT4);
            ui.node_port(NodePortSide::Input, "v", port_type::VEC3);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::VEC3);
        }
        DemoKind::QuatIdentity => {
            ui.label("identity");
            ui.node_port(NodePortSide::Output, "q", port_type::QUAT);
        }
        DemoKind::QuatEuler => {
            ui.label("pitch/yaw/roll °");
            edit_vec3(ui, "qe", floats, 1.0, Vec3::ZERO);
            ui.node_port(NodePortSide::Output, "q", port_type::QUAT);
        }
        DemoKind::QuatMul => {
            ui.node_port(NodePortSide::Input, "a", port_type::QUAT);
            ui.node_port(NodePortSide::Input, "b", port_type::QUAT);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::QUAT);
        }
        DemoKind::QuatRotateVec => {
            ui.node_port(NodePortSide::Input, "q", port_type::QUAT);
            ui.node_port(NodePortSide::Input, "v", port_type::VEC3);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::VEC3);
        }
        DemoKind::String => {
            ui.node_port(NodePortSide::Output, "text", port_type::STRING);
            ui.text_input("s", text);
        }
        DemoKind::StringConcat => {
            ui.node_port(NodePortSide::Input, "a", port_type::STRING);
            ui.node_port(NodePortSide::Input, "b", port_type::STRING);
            ui.separator();
            ui.label(preview);
            ui.node_port(NodePortSide::Output, "out", port_type::STRING);
        }
        DemoKind::Print => {
            ui.node_port(NodePortSide::Input, "in", port_type::ANY);
            if preview.is_empty() {
                ui.label("(no input)");
            } else {
                ui.label(preview);
            }
        }
    }
}

fn spawn_menu_items(ui: &mut Ui) -> Option<DemoKind> {
    let mut kind = None;
    if ui.menu_item("Float").clicked() {
        kind = Some(DemoKind::Float);
    }
    if ui.menu_item("Add").clicked() {
        kind = Some(DemoKind::Add);
    }
    if ui.menu_item("Mul").clicked() {
        kind = Some(DemoKind::Mul);
    }
    ui.separator();
    if ui.menu_item("Vec3").clicked() {
        kind = Some(DemoKind::Vec3);
    }
    if ui.menu_item("Vec3 Add").clicked() {
        kind = Some(DemoKind::Vec3Add);
    }
    if ui.menu_item("Vec3 Scale").clicked() {
        kind = Some(DemoKind::Vec3Scale);
    }
    if ui.menu_item("Dot").clicked() {
        kind = Some(DemoKind::Vec3Dot);
    }
    if ui.menu_item("Cross").clicked() {
        kind = Some(DemoKind::Vec3Cross);
    }
    if ui.menu_item("Length").clicked() {
        kind = Some(DemoKind::Vec3Length);
    }
    if ui.menu_item("Normalize").clicked() {
        kind = Some(DemoKind::Vec3Normalize);
    }
    ui.separator();
    if ui.menu_item("Mat4 Identity").clicked() {
        kind = Some(DemoKind::Mat4Identity);
    }
    if ui.menu_item("Mat4 Translate").clicked() {
        kind = Some(DemoKind::Mat4Translate);
    }
    if ui.menu_item("Mat4 Euler").clicked() {
        kind = Some(DemoKind::Mat4Euler);
    }
    if ui.menu_item("Mat4 Scale").clicked() {
        kind = Some(DemoKind::Mat4Scale);
    }
    if ui.menu_item("Mat4 Mul").clicked() {
        kind = Some(DemoKind::Mat4Mul);
    }
    if ui.menu_item("Mat4 × Vec").clicked() {
        kind = Some(DemoKind::Mat4MulVec);
    }
    ui.separator();
    if ui.menu_item("Quat Identity").clicked() {
        kind = Some(DemoKind::QuatIdentity);
    }
    if ui.menu_item("Quat Euler").clicked() {
        kind = Some(DemoKind::QuatEuler);
    }
    if ui.menu_item("Quat Mul").clicked() {
        kind = Some(DemoKind::QuatMul);
    }
    if ui.menu_item("Quat × Vec").clicked() {
        kind = Some(DemoKind::QuatRotateVec);
    }
    ui.separator();
    if ui.menu_item("String").clicked() {
        kind = Some(DemoKind::String);
    }
    if ui.menu_item("Concat").clicked() {
        kind = Some(DemoKind::StringConcat);
    }
    if ui.menu_item("Print").clicked() {
        kind = Some(DemoKind::Print);
    }
    kind
}

impl Scene for NodesDemo {
    fn title() -> &'static str {
        "mega-ui — nodes"
    }

    fn window_size() -> (f64, f64) {
        (1400.0, 900.0)
    }

    fn init(ui: &mut Ui) {
        ui.load_builtin_icons();
    }

    fn build(ui: &mut Ui, state: &mut Self, viewport: Vec2, dt: f32, stats: DrawStats) -> bool {
        state.apply_deletes();
        state.apply_clones();
        state.evaluate();

        ui.set_scale(state.scale);

        ui.menu_bar(|ui| {
            ui.menu("Graph", |ui| {
                if ui.menu_item("Fit origin").clicked() {
                    state.space.pan = Vec2::new(20.0, 20.0);
                    state.space.zoom = 1.0;
                    state.log.push_str("fit origin\n");
                }
                if ui.menu_item("Clear selection").clicked() {
                    state.space.selected_nodes.clear();
                    state.space.selected_link = None;
                }
                if ui.menu_item("Clone selected (Ctrl+D)").clicked() {
                    if !state.space.selected_nodes.is_empty() {
                        state.space.request_clone_nodes = state.space.selected_nodes.clone();
                    }
                }
                ui.separator();
                ui.menu("Spawn", |ui| {
                    if let Some(kind) = spawn_menu_items(ui) {
                        state.spawn(kind, Vec2::new(120.0, 120.0));
                    }
                });
            });
            ui.menu("View", |ui| {
                ui.menu("UI Scale", |ui| {
                    for (label, v) in [
                        ("100%", 1.0),
                        ("125%", 1.25),
                        ("150%", 1.5),
                        ("175%", 1.75),
                        ("200%", 2.0),
                    ] {
                        if ui.menu_item(label).clicked() {
                            state.scale = v;
                        }
                    }
                });
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
                gradient_stops,
                gradient_opacities,
                gradient_sample,
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
                            let kind = n.kind;
                            let preview = n.preview.clone();
                            let pos = &mut n.pos;
                            let floats = &mut n.floats;
                            let text = &mut n.text;
                            ui.node(&id, &title, pos, |ui| {
                                draw_node_body(ui, kind, floats, text, &preview);
                            });
                        }
                    });

                    let bg = space.background_hovered;
                    let world = space.context_world.unwrap_or(Vec2::new(100.0, 100.0));
                    let mut spawn_kind: Option<DemoKind> = None;
                    ui.context_menu("spawn_menu", bg, |ui| {
                        spawn_kind = spawn_menu_items(ui);
                    });
                    if let Some(kind) = spawn_kind {
                        let id = format!("n{next_id}");
                        *next_id += 1;
                        let title = NodesDemo::kind_title(kind).to_string();
                        log.push_str(&format!("spawn {title} ({id})\n"));
                        *status = format!("Spawned {title}");
                        let mut floats = [0.0; 16];
                        if kind == DemoKind::Mat4Scale {
                            floats[0] = 1.0;
                            floats[1] = 1.0;
                            floats[2] = 1.0;
                        }
                        nodes.push(DemoNode {
                            id,
                            kind,
                            title,
                            pos: world,
                            floats,
                            text: String::new(),
                            preview: String::new(),
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
                        ui.label("Gradient editor");
                        let _ = ui.gradient_editor(
                            "nodes_gradient",
                            gradient_stops,
                            gradient_opacities,
                            Vec2::new(0.0, 32.0),
                        );
                        ui.slider("nodes_grad_sample", gradient_sample, 0.0..=1.0);
                        let sampled =
                            sample_gradient(gradient_stops, gradient_opacities, *gradient_sample);
                        ui.horizontal(|ui| {
                            ui.label(&format!("Sampled @ {:.2}", *gradient_sample));
                            ui.color_box(22.0, sampled);
                        });
                        ui.separator();

                        if let Some(fid) = space.selected_frame.clone() {
                            if space.frames.iter().any(|f| f.id == fid) {
                                ui.label("Group");
                                ui.label(&format!("Id: {fid}"));
                                ui.label("Label");
                                if let Some(frame) =
                                    space.frames.iter_mut().find(|f| f.id == fid)
                                {
                                    let _ = ui.text_input("frame_label", &mut frame.label);
                                }
                                if ui.button("Ungroup").clicked() {
                                    space.ungroup_frame(&fid);
                                }
                                ui.separator();
                            }
                        }

                        if space.selected_nodes.len() > 1 {
                            ui.label(&format!(
                                "Multi-select ({} nodes)",
                                space.selected_nodes.len()
                            ));
                            if ui.button("Group selected").clicked() {
                                space.group_nodes(&space.selected_nodes.clone());
                            }
                            if ui.button("Clone selected").clicked() {
                                space.request_clone_nodes = space.selected_nodes.clone();
                            }
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
                                ui.label(&format!("Type: {}", NodesDemo::kind_title(n.kind)));
                                ui.label(&format!("Pos: ({:.0}, {:.0})", n.pos.x, n.pos.y));
                                if !n.preview.is_empty() {
                                    ui.label(&format!("Value: {}", n.preview));
                                }
                                ui.separator();
                                if ui.button("Clone node").clicked() {
                                    space.request_clone_nodes = vec![sel.clone()];
                                }
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
                                NodesDemo::kind_title(n.kind),
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
                            space.selected_frame = None;
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

        let fps = (1.0 / dt.max(1e-4)).min(999.0);
        ui.status_bar(|ui| {
            ui.label(&state.status.clone());
            ui.spacer();
            ui.label(&format!(
                "zoom {:.0}% · {} nodes · {} links · quads {} · FPS {:.0}",
                state.space.zoom * 100.0,
                state.nodes.len(),
                state.space.links.len(),
                stats.quads,
                fps
            ));
        });

        true
    }
}

fn main() {
    Host::run(NodesDemo::default());
}
