//! Node graph workspace: pan/zoom canvas, nodes with widget content, typed ports & links.
//!
//! Host owns [`NodeSpace`] (view + links) and its own node list / field data.
//! This module only draws and manipulates — no graph execution.

mod geom;
mod space;
mod types;
mod ui;

pub use space::NodeSpace;
pub use types::{NodeFrame, NodeLink, NodePortSide, PortType, port_type};

#[cfg(test)]
mod tests;
