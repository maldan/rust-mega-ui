/// Built-in port type ids. Host may register more via [`crate::NodeSpace::register_type`].
pub mod port_type {
    pub const ANY: u16 = 0;
    pub const FLOAT: u16 = 1;
    pub const INT: u16 = 2;
    pub const STRING: u16 = 3;
    pub const BOOL: u16 = 4;
    pub const VEC2: u16 = 5;
    pub const VEC3: u16 = 6;
    pub const VEC4: u16 = 7;
    pub const MAT4: u16 = 8;
    pub const QUAT: u16 = 9;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodePortSide {
    Input,
    Output,
}

#[derive(Clone, Debug)]
pub struct PortType {
    pub id: u16,
    pub name: String,
    pub color: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct NodeLink {
    pub id: u64,
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
    /// Color source (usually the output port type).
    pub ty: u16,
}

#[derive(Clone, Debug)]
pub struct NodeFrame {
    pub id: String,
    pub label: String,
    pub node_ids: Vec<String>,
}

pub(crate) fn default_port_types() -> Vec<PortType> {
    vec![
        PortType {
            id: port_type::ANY,
            name: "any".into(),
            color: [0.70, 0.70, 0.70, 1.0],
        },
        PortType {
            id: port_type::FLOAT,
            name: "float".into(),
            color: [0.35, 0.72, 0.95, 1.0],
        },
        PortType {
            id: port_type::INT,
            name: "int".into(),
            color: [0.95, 0.55, 0.35, 1.0],
        },
        PortType {
            id: port_type::STRING,
            name: "string".into(),
            color: [0.35, 0.82, 0.40, 1.0],
        },
        PortType {
            id: port_type::BOOL,
            name: "bool".into(),
            color: [0.90, 0.40, 0.55, 1.0],
        },
        PortType {
            id: port_type::VEC2,
            name: "vec2".into(),
            color: [0.55, 0.85, 0.95, 1.0],
        },
        PortType {
            id: port_type::VEC3,
            name: "vec3".into(),
            color: [0.40, 0.80, 0.70, 1.0],
        },
        PortType {
            id: port_type::VEC4,
            name: "vec4".into(),
            color: [0.35, 0.70, 0.85, 1.0],
        },
        PortType {
            id: port_type::MAT4,
            name: "mat4".into(),
            color: [0.75, 0.55, 0.95, 1.0],
        },
        PortType {
            id: port_type::QUAT,
            name: "quat".into(),
            color: [0.95, 0.75, 0.35, 1.0],
        },
    ]
}
