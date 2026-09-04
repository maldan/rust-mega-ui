//! Node graph workspace: pan/zoom canvas, nodes with widget content, typed ports & links.
//!
//! Host owns [`NodeSpace`] (view + links) and its own node list / field data.
//! This module only draws and manipulates — no graph execution.

use std::collections::{HashMap, HashSet};
use std::ptr::NonNull;

use glam::Vec2;

use crate::draw::push_polyline;
use crate::layout::new_layer;
use crate::theme;
use crate::types::{CursorIcon, Rect};
use crate::{CrossAlign, LayoutDir, Ui};

/// Built-in port type ids. Host may register more via [`NodeSpace::register_type`].
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
struct PendingWire {
    from_node: String,
    from_port: String,
    side: NodePortSide,
    ty: u16,
    /// Screen-space start of the wire.
    start: Vec2,
}

#[derive(Clone, Debug)]
pub struct NodeFrame {
    pub id: String,
    pub label: String,
    pub node_ids: Vec<String>,
}

#[derive(Clone, Debug)]
struct NodeDrag {
    primary: String,
    /// `mouse_world - primary_pos` at press.
    grab: Vec2,
    primary_start: Vec2,
    /// World positions at drag start (filled lazily as nodes are visited).
    anchors: HashMap<String, Vec2>,
    /// Selection snapshot at drag start — followers keep moving even if selection changes.
    group: Vec<String>,
}

#[derive(Clone, Debug)]
struct FrameDrag {
    frame_id: String,
    /// `mouse_world - origin` at press.
    grab: Vec2,
    origin_start: Vec2,
    anchors: HashMap<String, Vec2>,
}

#[derive(Clone, Copy, Debug)]
struct BoxSelect {
    /// Screen-space drag origin.
    start: Vec2,
    /// Ctrl held at press → add to selection instead of replace.
    additive: bool,
}

#[derive(Clone, Debug)]
struct PendingNodePress {
    id: String,
    /// True = start / continue group drag from title; false = body select.
    on_title: bool,
    ctrl: bool,
    mouse_world: Vec2,
}

/// Retained view / wiring state for one graph. Host keeps this between frames.
#[derive(Clone, Debug)]
pub struct NodeSpace {
    pub pan: Vec2,
    pub zoom: f32,
    /// Grid snap for node moves in world units. `0` disables. Default `5`.
    pub snap: f32,
    pub links: Vec<NodeLink>,
    pub types: Vec<PortType>,
    /// Unordered type pairs that may connect (auto-cast), e.g. Gray↔Color.
    pub cast_pairs: Vec<(u16, u16)>,
    pub selected_nodes: Vec<String>,
    pub selected_link: Option<u64>,
    /// Visual groups around nodes. Membership is sticky; bounds auto-fit members.
    pub frames: Vec<NodeFrame>,
    pub selected_frame: Option<String>,
    pub next_frame_id: u64,
    /// Host-owned: nodes currently evaluating (green border).
    pub running_nodes: HashSet<String>,
    /// Host-owned: nodes with bypass on (muted chrome).
    pub bypassed_nodes: HashSet<String>,
    /// Cleared by host after applying (remove nodes from its list).
    pub request_delete_nodes: Vec<String>,
    /// Cleared by host after applying (duplicate node payloads + links).
    pub request_clone_nodes: Vec<String>,
    /// Cleared by host after copying selection (Ctrl+C).
    pub request_copy_nodes: Vec<String>,
    /// Cleared by host after pasting (Ctrl+V). World position under cursor.
    pub request_paste_at: Option<Vec2>,
    /// World-space position of last RMB on empty canvas (for spawn menus).
    pub context_world: Option<Vec2>,
    /// True on the frame RMB requested a context menu on empty canvas.
    pub context_menu_request: bool,
    /// True when pointer is over empty canvas (not a node) inside the space.
    pub background_hovered: bool,
    /// True when pointer is inside the node-space rect this frame.
    pub pointer_in_space: bool,
    /// Center nodes in the canvas on the next build (screen-space pan).
    pub fit_view: bool,

    pub next_link_id: u64,
    pending: Option<PendingWire>,
    port_pos: HashMap<(String, NodePortSide, String), Vec2>,
    /// Last frame outer size in **world** units (title + body).
    node_sizes: HashMap<String, Vec2>,
    /// Screen-space node bounds from the current/last build (for marquee).
    node_screen_rects: HashMap<String, Rect>,
    /// Latest world positions seen this/last frame (for group-drag anchors).
    node_world_pos: HashMap<String, Vec2>,
    node_order: Vec<String>,
    pan_grab: Option<Vec2>,
    node_drag: Option<NodeDrag>,
    frame_drag: Option<FrameDrag>,
    box_select: Option<BoxSelect>,
    /// Topmost node press this frame (later draws overwrite → frontmost wins).
    pending_node_press: Option<PendingNodePress>,
    link_hit: Option<u64>,
    /// Set during [`Ui::node`] when the pointer is over a node this frame.
    pointer_over_node: bool,
    pointer_over_frame: bool,
    frame_screen_rects: HashMap<String, Rect>,
}

impl Default for NodeSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeSpace {
    pub fn new() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            snap: 5.0,
            links: Vec::new(),
            types: default_port_types(),
            cast_pairs: Vec::new(),
            selected_nodes: Vec::new(),
            selected_link: None,
            frames: Vec::new(),
            selected_frame: None,
            next_frame_id: 1,
            running_nodes: HashSet::new(),
            bypassed_nodes: HashSet::new(),
            request_delete_nodes: Vec::new(),
            request_clone_nodes: Vec::new(),
            request_copy_nodes: Vec::new(),
            request_paste_at: None,
            context_world: None,
            context_menu_request: false,
            background_hovered: false,
            pointer_in_space: false,
            fit_view: false,
            next_link_id: 1,
            pending: None,
            port_pos: HashMap::new(),
            node_sizes: HashMap::new(),
            node_screen_rects: HashMap::new(),
            node_world_pos: HashMap::new(),
            node_order: Vec::new(),
            pan_grab: None,
            node_drag: None,
            frame_drag: None,
            box_select: None,
            pending_node_press: None,
            link_hit: None,
            pointer_over_node: false,
            pointer_over_frame: false,
            frame_screen_rects: HashMap::new(),
        }
    }

    pub fn register_type(&mut self, id: u16, name: impl Into<String>, color: [f32; 4]) {
        if let Some(t) = self.types.iter_mut().find(|t| t.id == id) {
            t.name = name.into();
            t.color = color;
        } else {
            self.types.push(PortType {
                id,
                name: name.into(),
                color,
            });
        }
    }

    /// Allow wiring between two distinct port types (both directions).
    pub fn allow_cast(&mut self, a: u16, b: u16) {
        if a == b {
            return;
        }
        let pair = if a < b { (a, b) } else { (b, a) };
        if !self.cast_pairs.contains(&pair) {
            self.cast_pairs.push(pair);
        }
    }

    /// Pan/zoom so known nodes sit in `canvas` (window-space rect of this space).
    pub fn fit_nodes_in_rect(&mut self, canvas: Rect) {
        if self.node_world_pos.is_empty() {
            return;
        }
        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        for (id, pos) in &self.node_world_pos {
            let sz = self
                .node_sizes
                .get(id)
                .copied()
                .unwrap_or(Vec2::new(180.0, 80.0));
            min = min.min(*pos);
            max = max.max(*pos + sz);
        }
        let size = (max - min).max(Vec2::splat(8.0));
        let view = Vec2::new(canvas.width(), canvas.height());
        let pad = 36.0;
        let zx = (view.x - pad * 2.0).max(32.0) / size.x;
        let zy = (view.y - pad * 2.0).max(32.0) / size.y;
        self.zoom = zx.min(zy).clamp(ZOOM_MIN, 1.25);
        let center = (min + max) * 0.5;
        self.pan = canvas.min + view * 0.5 - center * self.zoom;
    }

    pub fn type_color(&self, id: u16) -> [f32; 4] {
        self.types
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.color)
            .unwrap_or([0.55, 0.55, 0.55, 1.0])
    }

    pub fn take_delete_nodes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.request_delete_nodes)
    }

    pub fn take_clone_nodes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.request_clone_nodes)
    }

    pub fn take_copy_nodes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.request_copy_nodes)
    }

    pub fn take_paste_at(&mut self) -> Option<Vec2> {
        self.request_paste_at.take()
    }

    /// Suggested world offset when the host duplicates a selection.
    pub fn clone_offset(&self) -> Vec2 {
        let s = if self.snap > 1e-6 { self.snap } else { 5.0 };
        // Large enough that clones don't sit on top of originals (hit-test overlap).
        Vec2::new(s * 16.0, s * 16.0)
    }

    /// Wrap `ids` in a new frame. A node belongs to at most one frame. Needs ≥2 ids.
    pub fn group_nodes(&mut self, ids: &[String]) -> Option<String> {
        let mut seen = HashSet::new();
        let mut node_ids = Vec::new();
        for id in ids {
            if seen.insert(id.clone()) {
                node_ids.push(id.clone());
            }
        }
        if node_ids.len() < 2 {
            return None;
        }
        for frame in &mut self.frames {
            frame.node_ids.retain(|n| !seen.contains(n));
        }
        self.frames.retain(|f| !f.node_ids.is_empty());
        let id = format!("f{}", self.next_frame_id);
        self.next_frame_id += 1;
        self.frames.push(NodeFrame {
            id: id.clone(),
            label: "Group".into(),
            node_ids,
        });
        self.selected_frame = Some(id.clone());
        self.selected_nodes.clear();
        self.selected_link = None;
        Some(id)
    }

    pub fn ungroup_frame(&mut self, frame_id: &str) {
        self.frames.retain(|f| f.id != frame_id);
        if self.selected_frame.as_deref() == Some(frame_id) {
            self.selected_frame = None;
        }
        self.frame_screen_rects.remove(frame_id);
        if self.frame_drag.as_ref().is_some_and(|d| d.frame_id == frame_id) {
            self.frame_drag = None;
        }
    }

    /// Duplicate links whose both endpoints are in `id_map` (old → new).
    pub fn duplicate_links(&mut self, id_map: &HashMap<String, String>) {
        let mut extras = Vec::new();
        for link in &self.links {
            let Some(from) = id_map.get(&link.from_node) else {
                continue;
            };
            let Some(to) = id_map.get(&link.to_node) else {
                continue;
            };
            let id = self.next_link_id;
            self.next_link_id += 1;
            extras.push(NodeLink {
                id,
                from_node: from.clone(),
                from_port: link.from_port.clone(),
                to_node: to.clone(),
                to_port: link.to_port.clone(),
                ty: link.ty,
            });
        }
        self.links.extend(extras);
    }

    /// Remove links that reference `node_id`.
    pub fn detach_node(&mut self, node_id: &str) {
        self.links
            .retain(|l| l.from_node != node_id && l.to_node != node_id);
        self.selected_nodes.retain(|id| id != node_id);
        self.port_pos.retain(|(n, _, _), _| n != node_id);
        self.node_sizes.remove(node_id);
        self.node_screen_rects.remove(node_id);
        self.node_world_pos.remove(node_id);
        self.node_order.retain(|id| id != node_id);
        for frame in &mut self.frames {
            frame.node_ids.retain(|n| n != node_id);
        }
        self.frames.retain(|f| !f.node_ids.is_empty());
        let fid = self.selected_frame.clone();
        if let Some(id) = fid {
            if !self.frames.iter().any(|f| f.id == id) {
                self.selected_frame = None;
            }
        }
    }

    pub fn remove_link(&mut self, id: u64) {
        self.links.retain(|l| l.id != id);
        if self.selected_link == Some(id) {
            self.selected_link = None;
        }
    }

    fn world_to_screen(&self, world: Vec2) -> Vec2 {
        world * self.zoom + self.pan
    }

    fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        (screen - self.pan) / self.zoom.max(1e-4)
    }

    fn is_selected(&self, id: &str) -> bool {
        self.selected_nodes.iter().any(|x| x == id)
    }

    fn clear_frame_sel(&mut self) {
        self.selected_frame = None;
        self.frame_drag = None;
    }

    /// Click select: Ctrl toggles; click unselected replaces; click selected keeps multi.
    fn select_for_click(&mut self, id: &str, ctrl: bool) {
        self.clear_frame_sel();
        if ctrl {
            if let Some(i) = self.selected_nodes.iter().position(|x| x == id) {
                self.selected_nodes.remove(i);
            } else {
                self.selected_nodes.push(id.to_string());
            }
        } else if !self.is_selected(id) {
            self.selected_nodes = vec![id.to_string()];
        }
        self.selected_link = None;
    }

    /// Press on title to drag: Ctrl adds; click unselected replaces; click selected keeps group.
    fn select_for_drag(&mut self, id: &str, ctrl: bool) {
        self.clear_frame_sel();
        if ctrl {
            if !self.is_selected(id) {
                self.selected_nodes.push(id.to_string());
            }
        } else if !self.is_selected(id) {
            self.selected_nodes = vec![id.to_string()];
        }
        self.selected_link = None;
    }

    fn begin_node_drag(&mut self, id: &str, pos: Vec2, mouse_world: Vec2, ctrl: bool) {
        self.frame_drag = None;
        self.select_for_drag(id, ctrl);
        self.bring_front(id);
        let group = self.selected_nodes.clone();
        let mut anchors = HashMap::new();
        for gid in &group {
            if let Some(&p) = self.node_world_pos.get(gid) {
                anchors.insert(gid.clone(), p);
            }
        }
        anchors.insert(id.to_string(), pos);
        self.node_drag = Some(NodeDrag {
            primary: id.to_string(),
            grab: mouse_world - pos,
            primary_start: pos,
            anchors,
            group,
        });
    }

    fn begin_frame_drag(&mut self, frame_id: &str, mouse_world: Vec2) {
        self.node_drag = None;
        self.bring_frame_front(frame_id);
        self.selected_frame = Some(frame_id.to_string());
        self.selected_nodes.clear();
        self.selected_link = None;
        let origin = self
            .frame_world_rect_id(frame_id)
            .map(|r| r.min)
            .unwrap_or(mouse_world);
        let mut anchors = HashMap::new();
        if let Some(frame) = self.frames.iter().find(|f| f.id == frame_id) {
            for nid in &frame.node_ids {
                if let Some(&p) = self.node_world_pos.get(nid) {
                    anchors.insert(nid.clone(), p);
                }
            }
        }
        self.frame_drag = Some(FrameDrag {
            frame_id: frame_id.to_string(),
            grab: mouse_world - origin,
            origin_start: origin,
            anchors,
        });
    }

    /// Update `pos` while a group/single drag is active.
    fn apply_node_drag(&mut self, id: &str, pos: &mut Vec2, mouse_world: Vec2) -> bool {
        let snap = self.snap;
        if self.node_drag.is_none() {
            let in_frame = match self.frame_drag.as_ref() {
                None => false,
                Some(drag) if drag.anchors.contains_key(id) => true,
                Some(drag) => self
                    .frames
                    .iter()
                    .any(|f| f.id == drag.frame_id && f.node_ids.iter().any(|n| n == id)),
            };
            if in_frame {
                let Some(drag) = self.frame_drag.as_mut() else {
                    return false;
                };
                let raw = mouse_world - drag.grab;
                let origin_now = snap_vec(raw, snap);
                let start = *drag.anchors.entry(id.to_string()).or_insert(*pos);
                *pos = start + (origin_now - drag.origin_start);
                return true;
            }
            return false;
        }
        let Some(drag) = self.node_drag.as_mut() else {
            return false;
        };
        let in_group = id == drag.primary || drag.group.iter().any(|x| x == id);
        if !in_group {
            return false;
        }
        let raw = mouse_world - drag.grab;
        let primary_now = snap_vec(raw, snap);
        if id == drag.primary {
            *pos = primary_now;
            true
        } else {
            let start = *drag.anchors.entry(id.to_string()).or_insert(*pos);
            *pos = start + (primary_now - drag.primary_start);
            true
        }
    }

    fn frame_world_rect_id(&self, frame_id: &str) -> Option<Rect> {
        let frame = self.frames.iter().find(|f| f.id == frame_id)?;
        self.frame_world_rect(frame)
    }

    fn frame_world_rect(&self, frame: &NodeFrame) -> Option<Rect> {
        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        let mut any = false;
        for nid in &frame.node_ids {
            let Some(&p) = self.node_world_pos.get(nid) else {
                continue;
            };
            let size = self
                .node_sizes
                .get(nid)
                .copied()
                .unwrap_or(Vec2::new(NODE_MIN_W, 80.0));
            any = true;
            min = Vec2::new(min.x.min(p.x), min.y.min(p.y));
            max = Vec2::new(max.x.max(p.x + size.x), max.y.max(p.y + size.y));
        }
        if !any {
            return None;
        }
        Some(Rect {
            min: min - Vec2::splat(FRAME_PAD),
            max: max + Vec2::splat(FRAME_PAD),
        })
    }

    fn frame_at(&self, screen: Vec2) -> Option<String> {
        for frame in self.frames.iter().rev() {
            if self
                .frame_screen_rects
                .get(&frame.id)
                .is_some_and(|r| r.contains(screen))
            {
                return Some(frame.id.clone());
            }
        }
        None
    }

    fn compatible(&self, a: u16, b: u16) -> bool {
        if a == port_type::ANY || b == port_type::ANY || a == b {
            return true;
        }
        let pair = if a < b { (a, b) } else { (b, a) };
        self.cast_pairs.contains(&pair)
    }

    fn bring_front(&mut self, id: &str) {
        self.node_order.retain(|x| x != id);
        self.node_order.push(id.to_string());
    }

    fn bring_frame_front(&mut self, id: &str) {
        if let Some(i) = self.frames.iter().position(|f| f.id == id) {
            let frame = self.frames.remove(i);
            self.frames.push(frame);
        }
    }
}

fn default_port_types() -> Vec<PortType> {
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

fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

/// Widget scale inside a node. Must track zoom (no 0.45 floor): chrome uses `z`
/// for the box, so a higher floor made ports/layout sit in a different space.
fn node_content_scale(ui_scale: f32, zoom: f32) -> f32 {
    (ui_scale * zoom).clamp(0.05, 16.0)
}

fn link_handle(from: Vec2, to: Vec2, zoom: f32) -> f32 {
    ((to.x - from.x).abs() * 0.5).max(48.0 * zoom.clamp(ZOOM_MIN, 1.0))
}

fn link_curve(from: Vec2, to: Vec2, zoom: f32) -> Vec<Vec2> {
    let dx = link_handle(from, to, zoom);
    let c1 = from + Vec2::new(dx, 0.0);
    let c2 = to - Vec2::new(dx, 0.0);
    let n = 18;
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        pts.push(cubic_bezier(from, c1, c2, to, t));
    }
    pts
}

fn dist_point_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-8 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

fn dist_point_polyline(p: Vec2, pts: &[Vec2]) -> f32 {
    let mut best = f32::MAX;
    for w in pts.windows(2) {
        best = best.min(dist_point_segment(p, w[0], w[1]));
    }
    best
}

fn rect_from_points(a: Vec2, b: Vec2) -> Rect {
    Rect {
        min: Vec2::new(a.x.min(b.x), a.y.min(b.y)),
        max: Vec2::new(a.x.max(b.x), a.y.max(b.y)),
    }
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
}

const NODE_MIN_W: f32 = 168.0;
const NODE_PAD: f32 = 10.0;
const PIN_R: f32 = 5.5;
const PIN_HIT: f32 = 10.0;
const FRAME_PAD: f32 = 20.0;
const FRAME_LABEL: f32 = 20.0;
const ZOOM_MIN: f32 = 0.1;
const ZOOM_MAX: f32 = 8.0;

fn snap_vec(p: Vec2, snap: f32) -> Vec2 {
    if snap <= 1e-6 {
        p
    } else {
        Vec2::new((p.x / snap).round() * snap, (p.y / snap).round() * snap)
    }
}

impl Ui {
    /// Fill `size` (or remaining layout space if zero) with a node graph canvas.
    pub fn node_space(&mut self, id: &str, size: Vec2, space: &mut NodeSpace, add: impl FnOnce(&mut Self)) {
        let alloc = if size.x > 0.0 && size.y > 0.0 {
            size
        } else {
            let a = self.available_size();
            Vec2::new(
                if size.x > 0.0 { size.x } else { a.x },
                if size.y > 0.0 { size.y } else { a.y },
            )
        };
        let rect = self.allocate(alloc);
        self.node_space_rect(id, rect, space, add);
    }

    /// Node graph in an explicit screen rect (e.g. dock leaf).
    pub fn node_space_rect(
        &mut self,
        id: &str,
        rect: Rect,
        space: &mut NodeSpace,
        add: impl FnOnce(&mut Self),
    ) {
        self.push_id(id);
        let space_id = *self.id_stack.last().unwrap();

        space.zoom = space.zoom.clamp(ZOOM_MIN, ZOOM_MAX);
        space.background_hovered = false;
        space.context_menu_request = false;
        space.link_hit = None;
        space.pointer_over_node = false;
        space.pointer_over_frame = false;
        space.pending_node_press = None;
        space.pointer_in_space = false;
        space.request_paste_at = None;
        space.request_copy_nodes.clear();
        space.node_screen_rects.clear();
        space.frame_screen_rects.clear();

        // Background
        self.round_rect(rect, self.s(4.0), [0.02, 0.02, 0.02, 1.0]);
        self.push_clip(rect);
        self.draw_node_grid(rect, space);
        self.draw_node_frames(space);

        let mouse = self.input.mouse_pos;
        // Context menus / selects are drawn after the canvas; block canvas input while
        // a menu is open or last-frame overlay covered the pointer.
        let over_popup = self.context_menu.is_some()
            || self.mouse_over_absorb()
            || self.overlay_block.iter().any(|r| r.contains(mouse));
        let in_rect = rect.contains(mouse) && !self.block_input && !over_popup;
        space.pointer_in_space = in_rect;

        // Links hit-test (skip while boxing / dragging / panning)
        if in_rect
            && space.pending.is_none()
            && space.node_drag.is_none()
            && space.pan_grab.is_none()
            && space.box_select.is_none()
        {
            let mut best: Option<(u64, f32)> = None;
            for link in &space.links {
                let Some(&a) = space.port_pos.get(&(
                    link.from_node.clone(),
                    NodePortSide::Output,
                    link.from_port.clone(),
                )) else {
                    continue;
                };
                let Some(&b) = space.port_pos.get(&(
                    link.to_node.clone(),
                    NodePortSide::Input,
                    link.to_port.clone(),
                )) else {
                    continue;
                };
                let pts = link_curve(a, b, space.zoom);
                let d = dist_point_polyline(mouse, &pts);
                if d < 9.0 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                    best = Some((link.id, d));
                }
            }
            space.link_hit = best.map(|(id, _)| id);
        }

        {
            let clip = self.clip();
            let out = &mut self.draw_list;
            for link in &space.links {
                let Some(&a) = space.port_pos.get(&(
                    link.from_node.clone(),
                    NodePortSide::Output,
                    link.from_port.clone(),
                )) else {
                    continue;
                };
                let Some(&b) = space.port_pos.get(&(
                    link.to_node.clone(),
                    NodePortSide::Input,
                    link.to_port.clone(),
                )) else {
                    continue;
                };
                let mut color = space.type_color(link.ty);
                let selected =
                    space.selected_link == Some(link.id) || space.link_hit == Some(link.id);
                if selected {
                    color[0] = (color[0] + 0.25).min(1.0);
                    color[1] = (color[1] + 0.25).min(1.0);
                    color[2] = (color[2] + 0.25).min(1.0);
                }
                let thick = if selected { 3.2 } else { 2.2 } * space.zoom.clamp(0.7, 1.4);
                let pts = link_curve(a, b, space.zoom);
                push_polyline(out, &pts, thick, color, clip);
            }
        }

        // Clear port positions; nodes rewrite this frame
        space.port_pos.clear();

        let prev_ptr = self.node_space_ptr;
        let prev_clip = self.node_space_clip;
        let prev_id = self.node_space_id;
        self.node_space_ptr = NonNull::new(space as *mut NodeSpace);
        self.node_space_clip = Some(rect);
        self.node_space_id = Some(space_id);

        if space.fit_view
            && rect.width() > 80.0
            && rect.height() > 80.0
            && !space.node_world_pos.is_empty()
        {
            space.fit_nodes_in_rect(rect);
            space.fit_view = false;
        }

        add(self);

        if space.fit_view && !space.node_world_pos.is_empty() {
            self.request_repaint();
        }

        // Zoom after children so open selects can consume the wheel first.
        if in_rect
            && !self.scroll_consumed
            && !self.mouse_over_absorb()
            && self.input.scroll_delta.y.abs() > 0.0
            && space.pending.is_none()
        {
            let old_z = space.zoom;
            let factor = if self.input.scroll_delta.y > 0.0 {
                1.08
            } else {
                1.0 / 1.08
            };
            let new_z = (old_z * factor).clamp(ZOOM_MIN, ZOOM_MAX);
            if (new_z - old_z).abs() > 1e-5 {
                let world = space.screen_to_world(mouse);
                space.zoom = new_z;
                space.pan = mouse - world * space.zoom;
                self.request_repaint();
            }
            self.consume_scroll();
            self.want_capture = true;
        }

        // Wire drag wins over node press
        if space.pending.is_some() {
            space.pending_node_press = None;
        }
        // Frontmost node under cursor handles the press (later draws overwrite pending).
        if let Some(press) = space.pending_node_press.take() {
            if let Some(&pos) = space.node_world_pos.get(&press.id) {
                if press.on_title {
                    space.begin_node_drag(&press.id, pos, press.mouse_world, press.ctrl);
                } else {
                    space.select_for_click(&press.id, press.ctrl);
                    space.bring_front(&press.id);
                }
            }
        }

        // Frame under pointer (padding / title) — nodes already claimed the press if overlapping.
        let mouse_world = space.screen_to_world(mouse);
        let frame_hit = if in_rect
            && space.pending.is_none()
            && space.node_drag.is_none()
            && space.pan_grab.is_none()
            && !space.pointer_over_node
            && space.link_hit.is_none()
        {
            space.frame_at(mouse)
        } else {
            None
        };
        space.pointer_over_frame = frame_hit.is_some();
        if let Some(fid) = frame_hit.as_deref() {
            self.set_cursor(CursorIcon::Move);
            if self.input.mouse_pressed
                && space.frame_drag.is_none()
                && space.node_drag.is_none()
                && space.box_select.is_none()
            {
                space.begin_frame_drag(fid, mouse_world);
                self.active_id = Some(space_id.child("#frame"));
            }
        }

        // Pending wire
        if let Some(ref pending) = space.pending {
            let color = space.type_color(pending.ty);
            let end = mouse;
            let (a, b) = match pending.side {
                NodePortSide::Output => (pending.start, end),
                NodePortSide::Input => (end, pending.start),
            };
            let pts = link_curve(a, b, space.zoom);
            let thick = 2.4 * space.zoom.clamp(0.7, 1.4);
            self.draw_polyline(&pts, thick, color);
            self.want_capture = true;
            self.request_repaint();
        }

        if space.frame_drag.is_some() {
            self.want_capture = true;
            self.request_repaint();
        }

        let hovered_bg = in_rect
            && space.pending.is_none()
            && space.node_drag.is_none()
            && space.frame_drag.is_none()
            && space.box_select.is_none()
            && !space.pointer_over_node
            && !space.pointer_over_frame
            && space.link_hit.is_none()
            && !self.input.mouse_middle_down;
        space.background_hovered = hovered_bg;

        if in_rect {
            self.want_capture = true;
        }

        // Pan: MMB anywhere
        let pan_id = space_id.child("#pan");
        if in_rect && self.input.mouse_middle_pressed && space.pending.is_none() {
            self.active_id = Some(pan_id);
            space.pan_grab = Some(mouse - space.pan);
            space.box_select = None;
        }

        // Marquee: LMB drag on empty canvas (Ctrl = additive)
        if hovered_bg && self.input.mouse_pressed {
            let additive = self.input.key_ctrl;
            space.box_select = Some(BoxSelect {
                start: mouse,
                additive,
            });
            if !additive {
                space.selected_nodes.clear();
                space.selected_link = None;
                space.selected_frame = None;
            }
            self.active_id = Some(space_id.child("#box"));
        }

        // Link LMB → select only (never when over a node / already dragging)
        if space.link_hit.is_some()
            && self.input.mouse_pressed
            && space.pending.is_none()
            && space.node_drag.is_none()
            && !space.pointer_over_node
            && !space.pointer_over_frame
            && space.frame_drag.is_none()
            && !self.input.mouse_middle_down
            && space.box_select.is_none()
        {
            let lid = space.link_hit.unwrap();
            space.selected_link = Some(lid);
            space.selected_nodes.clear();
            space.selected_frame = None;
            self.active_id = Some(space_id.child("#link"));
        }

        // Link RMB → delete
        if space.link_hit.is_some()
            && self.input.mouse_right_pressed
            && space.pending.is_none()
            && !space.pointer_over_node
        {
            let lid = space.link_hit.unwrap();
            space.remove_link(lid);
            space.selected_link = None;
        }

        // Draw / update marquee
        if let Some(box_sel) = space.box_select {
            let r = rect_from_points(box_sel.start, mouse);
            let fill = [0.20, 0.45, 0.90, 0.12];
            let border = [0.35, 0.60, 1.0, 0.55];
            self.round_rect(r, 0.0, fill);
            // border as thin lines
            let t = 1.0;
            self.line(Vec2::new(r.min.x, r.min.y), Vec2::new(r.max.x, r.min.y), t, border);
            self.line(Vec2::new(r.min.x, r.max.y), Vec2::new(r.max.x, r.max.y), t, border);
            self.line(Vec2::new(r.min.x, r.min.y), Vec2::new(r.min.x, r.max.y), t, border);
            self.line(Vec2::new(r.max.x, r.min.y), Vec2::new(r.max.x, r.max.y), t, border);
            self.want_capture = true;
            self.request_repaint();
        }

        let panning = self.active_id == Some(pan_id) && self.input.mouse_middle_down;
        if panning {
            if let Some(grab) = space.pan_grab {
                space.pan = mouse - grab;
                self.set_cursor(CursorIcon::Move);
                self.want_capture = true;
                self.request_repaint();
            }
        }

        if self.input.mouse_released {
            if let Some(box_sel) = space.box_select.take() {
                let r = rect_from_points(box_sel.start, mouse);
                let drag_len = (mouse - box_sel.start).length();
                if drag_len >= 4.0 {
                    let mut hits: Vec<String> = space
                        .node_screen_rects
                        .iter()
                        .filter(|(_, nr)| rects_overlap(r, **nr))
                        .map(|(id, _)| id.clone())
                        .collect();
                    hits.sort();
                    if box_sel.additive {
                        for id in hits {
                            if !space.is_selected(&id) {
                                space.selected_nodes.push(id);
                            }
                        }
                    } else {
                        space.selected_nodes = hits;
                    }
                    space.selected_link = None;
                    space.selected_frame = None;
                }
                // tiny drag = click on empty: selection already cleared if !additive
            }
            space.node_drag = None;
            space.frame_drag = None;
            if space.pending.is_some() {
                space.pending = None;
            }
        }
        if self.input.mouse_middle_released {
            space.pan_grab = None;
        }

        // RMB on empty → context spawn pos (not when deleting a link)
        if in_rect
            && self.input.mouse_right_pressed
            && hovered_bg
            && space.link_hit.is_none()
        {
            space.context_world = Some(space.screen_to_world(mouse));
            space.context_menu_request = true;
        }

        // Delete selection: Delete always; Backspace when not typing
        let want_delete = self.input.key_delete
            || (self.input.key_backspace && self.focus_id.is_none());
        if in_rect && want_delete {
            if let Some(lid) = space.selected_link.take() {
                space.remove_link(lid);
            } else if let Some(fid) = space.selected_frame.take() {
                space.ungroup_frame(&fid);
            } else if !space.selected_nodes.is_empty() {
                let ids = space.selected_nodes.clone();
                for nid in &ids {
                    space.detach_node(nid);
                }
                space.request_delete_nodes.extend(ids);
            }
        }

        // Clone selection (Ctrl+D) — host duplicates payloads via take_clone_nodes
        if in_rect && self.input.key_duplicate && !space.selected_nodes.is_empty() {
            space.request_clone_nodes = space.selected_nodes.clone();
        }

        // Copy / paste (Ctrl+C / Ctrl+V) — skip while a text field has focus
        let typing = self.focus_id.is_some();
        if in_rect && !typing && self.input.key_copy && !space.selected_nodes.is_empty() {
            space.request_copy_nodes = space.selected_nodes.clone();
        }
        if in_rect && !typing && self.input.key_paste {
            space.request_paste_at = Some(space.screen_to_world(mouse));
        }

        self.node_space_ptr = prev_ptr;
        self.node_space_clip = prev_clip;
        self.node_space_id = prev_id;
        self.pop_clip();
        self.pop_id();
    }

    fn draw_node_frames(&mut self, space: &mut NodeSpace) {
        let z = space.zoom;
        let radius = self.s(theme::WIN_RADIUS);
        let frames: Vec<(String, String, Rect, bool)> = space
            .frames
            .iter()
            .filter_map(|frame| {
                let wr = space.frame_world_rect(frame)?;
                let rect = Rect {
                    min: space.world_to_screen(wr.min),
                    max: space.world_to_screen(wr.max),
                };
                let selected = space.selected_frame.as_deref() == Some(frame.id.as_str());
                Some((frame.id.clone(), frame.label.clone(), rect, selected))
            })
            .collect();
        for (id, label, rect, selected) in frames {
            let px = (self.font_size() * 1.45 * z).max(FRAME_LABEL * z);
            let lh = self.text_height_at(px);
            let gap = 10.0 * z;
            let hit = Rect {
                min: Vec2::new(rect.min.x, rect.min.y - lh - gap),
                max: rect.max,
            };
            space.frame_screen_rects.insert(id, hit);
            let fill = if selected {
                theme::NODE_FRAME_SEL
            } else {
                theme::NODE_FRAME
            };
            let border = if selected {
                theme::NODE_FRAME_BORDER_SEL
            } else {
                theme::NODE_FRAME_BORDER
            };
            self.round_rect(rect, radius, fill);
            let t = 1.0;
            self.line(Vec2::new(rect.min.x, rect.min.y), Vec2::new(rect.max.x, rect.min.y), t, border);
            self.line(Vec2::new(rect.min.x, rect.max.y), Vec2::new(rect.max.x, rect.max.y), t, border);
            self.line(Vec2::new(rect.min.x, rect.min.y), Vec2::new(rect.min.x, rect.max.y), t, border);
            self.line(Vec2::new(rect.max.x, rect.min.y), Vec2::new(rect.max.x, rect.max.y), t, border);
            if !label.is_empty() {
                self.text_sized(
                    Vec2::new(rect.min.x, rect.min.y - lh - gap),
                    &label,
                    theme::TITLE_TEXT,
                    px,
                );
            }
        }
    }

    fn draw_node_grid(&mut self, rect: Rect, space: &NodeSpace) {
        let step = 32.0 * space.zoom;
        if step < 6.0 {
            return;
        }
        let major_every = 4;
        let origin = space.pan;
        let start_x = ((rect.min.x - origin.x) / step).floor() as i32 - 1;
        let end_x = ((rect.max.x - origin.x) / step).ceil() as i32 + 1;
        let start_y = ((rect.min.y - origin.y) / step).floor() as i32 - 1;
        let end_y = ((rect.max.y - origin.y) / step).ceil() as i32 + 1;
        let minor = [1.0, 1.0, 1.0, 0.028];
        let major = [1.0, 1.0, 1.0, 0.07];
        for ix in start_x..=end_x {
            let x = origin.x + ix as f32 * step;
            let color = if ix.rem_euclid(major_every) == 0 {
                major
            } else {
                minor
            };
            self.line(
                Vec2::new(x, rect.min.y),
                Vec2::new(x, rect.max.y),
                1.0,
                color,
            );
        }
        for iy in start_y..=end_y {
            let y = origin.y + iy as f32 * step;
            let color = if iy.rem_euclid(major_every) == 0 {
                major
            } else {
                minor
            };
            self.line(
                Vec2::new(rect.min.x, y),
                Vec2::new(rect.max.x, y),
                1.0,
                color,
            );
        }
    }

    /// Draw one node. `pos` is **world-space** top-left (host-owned).
    pub fn node(&mut self, id: &str, title: &str, pos: &mut Vec2, add: impl FnOnce(&mut Self)) {
        let Some(mut space_ptr) = self.node_space_ptr else {
            return;
        };
        let space = unsafe { space_ptr.as_mut() };
        let clip = self.node_space_clip.unwrap_or(Rect {
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        });

        if !space.node_order.iter().any(|x| x == id) {
            space.node_order.push(id.to_string());
        }

        let mouse = self.input.mouse_pos;
        let mouse_world = space.screen_to_world(mouse);
        let ctrl = self.input.key_ctrl;

        space.node_world_pos.insert(id.to_string(), *pos);

        // Apply ongoing group drag before layout (followers move same frame).
        let dragging = space.apply_node_drag(id, pos, mouse_world);
        if dragging {
            space.node_world_pos.insert(id.to_string(), *pos);
            self.set_cursor(CursorIcon::Move);
            self.want_capture = true;
            self.request_repaint();
        }

        let z = space.zoom;
        let title_h = theme::WIN_TITLE_H;
        let min_w = NODE_MIN_W;
        let last = space
            .node_sizes
            .get(id)
            .copied()
            .unwrap_or(Vec2::new(min_w, title_h + 48.0));
        let world_size = Vec2::new(last.x.max(min_w), last.y.max(title_h + 24.0));

        let screen_size = world_size * z;
        let mut screen_pos = space.world_to_screen(*pos);
        let mut rect = Rect::from_min_size(screen_pos, screen_size);
        let title_bar = Rect::from_min_size(screen_pos, Vec2::new(screen_size.x, title_h * z));

        let hovered = !self.block_input
            && !self.mouse_over_absorb()
            && clip.contains(mouse)
            && rect.contains(mouse);
        if hovered {
            space.pointer_over_node = true;
        }

        self.push_id(id);
        let node_id = *self.id_stack.last().unwrap();
        let title_id = node_id.child("#title");

        // Title hover chrome; press is deferred so the frontmost overlapping node wins.
        let title_hover = hovered && title_bar.contains(mouse);
        if title_hover {
            self.set_cursor(CursorIcon::Move);
            self.want_capture = true;
        }
        if hovered && self.input.mouse_pressed && space.node_drag.is_none() {
            space.pending_node_press = Some(PendingNodePress {
                id: id.to_string(),
                on_title: title_hover,
                ctrl,
                mouse_world,
            });
            if title_hover {
                self.active_id = Some(title_id);
            }
        }

        let selected = space.is_selected(id);
        let bypassed = space.bypassed_nodes.contains(id);
        let running = space.running_nodes.contains(id);
        let border = if bypassed {
            theme::NODE_BYPASS
        } else if running {
            theme::NODE_RUNNING
        } else if selected {
            theme::ACCENT
        } else {
            theme::WIN_BORDER
        };

        // Widgets layout in screen pixels; scale must track zoom (not a lower cap).
        // A 3.0 cap made zoom > ~2.5/dpi desync: chrome grew, content froze,
        // world size was rebuilt as used/z and nodes jumped. A 0.45 floor did
        // the same in reverse: box followed z, ports/layout stuck larger.
        let old_scale = self.scale;
        let old_spacing = self.spacing;
        self.scale = node_content_scale(old_scale, z);
        self.spacing = self.base_spacing * self.scale;
        let radius = self.s(theme::WIN_RADIUS);

        // Refresh geometry after drag write
        screen_pos = space.world_to_screen(*pos);
        rect = Rect::from_min_size(screen_pos, screen_size);

        self.round_rect(rect, radius, border);
        let body = if bypassed {
            theme::WIN_BODY_BYPASS
        } else {
            theme::WIN_BODY
        };
        self.round_rect(rect.inset(1.0), (radius - 1.0).max(0.0), body);
        space.node_screen_rects.insert(id.to_string(), rect);

        let title_color = if bypassed {
            if title_hover || dragging {
                theme::WIN_TITLE_BYPASS_HOVER
            } else {
                theme::WIN_TITLE_BYPASS
            }
        } else if self.active_id == Some(title_id) || dragging {
            theme::WIN_TITLE_PRESS
        } else if title_hover {
            theme::WIN_TITLE_HOVER
        } else {
            theme::WIN_TITLE
        };
        let title_draw = Rect {
            min: rect.min + Vec2::new(1.0, 1.0),
            max: Vec2::new(rect.max.x - 1.0, rect.min.y + title_h * z),
        };
        self.round_rect_corners(title_draw, (radius - 1.0).max(0.0), title_color, true, false);
        let th = self.text_height();
        self.text(
            rect.min + Vec2::new(self.s(10.0), (title_h * z - th) * 0.5),
            title,
            if bypassed {
                theme::TEXT_DISABLED
            } else {
                theme::TITLE_TEXT
            },
        );

        let pad = NODE_PAD * z;
        let content_origin = Vec2::new(rect.min.x + pad, rect.min.y + title_h * z + pad * 0.6);
        let content_w = (screen_size.x - pad * 2.0).max(0.0);
        let content_clip = Rect {
            min: Vec2::new(rect.min.x + 1.0, rect.min.y + title_h * z),
            max: rect.max - Vec2::splat(1.0),
        };
        self.push_clip(content_clip);
        self.layers.push(new_layer(
            LayoutDir::Vertical,
            content_origin,
            self.spacing,
            content_w,
            0.0,
            CrossAlign::Start,
        ));

        self.current_node_id = Some(id.to_string());
        self.current_node_caption = None;
        self.node_port_in = 0;
        self.node_port_out = 0;
        self.node_port_rows.clear();
        add(self);
        let caption = self.current_node_caption.take();
        self.current_node_id = None;
        self.node_port_rows.clear();

        let used = self.layer().used;
        let hug_x = self.layer().hug_x;
        self.layers.pop();
        self.pop_clip();

        if let Some(text) = caption.as_deref().filter(|s| !s.is_empty()) {
            let cap_px = (self.font_size() * 0.82).max(8.0);
            let tw = self.text_width_at(text, cap_px);
            let x = rect.min.x + ((rect.width() - tw) * 0.5).max(0.0);
            let y = rect.max.y + 3.0 * z;
            self.text_sized(Vec2::new(x, y), text, theme::TEXT_DISABLED, cap_px);
        }

        let body_h = used.y + pad * 1.2;
        let layout_z = (self.scale / old_scale.max(1e-4)).max(1e-4);
        let world_h = (title_h + body_h / layout_z).max(title_h + 24.0);
        let world_w = (hug_x / layout_z + NODE_PAD * 2.0).max(min_w);
        space
            .node_sizes
            .insert(id.to_string(), Vec2::new(world_w, world_h));

        self.scale = old_scale;
        self.spacing = old_spacing;
        self.pop_id();
    }

    /// Optional caption under the current node body (call inside [`Self::node`]).
    pub fn node_caption(&mut self, text: &str) {
        if self.current_node_id.is_some() {
            self.current_node_caption = Some(text.to_string());
        }
    }

    /// Declare a typed port on the current node (call inside [`Self::node`] content).
    pub fn node_port(&mut self, side: NodePortSide, port_id: &str, ty: u16) {
        let Some(node_id) = self.current_node_id.clone() else {
            return;
        };
        let Some(mut space_ptr) = self.node_space_ptr else {
            return;
        };
        let space = unsafe { space_ptr.as_mut() };
        let clip = self.node_space_clip.unwrap_or(Rect {
            min: Vec2::ZERO,
            max: Vec2::ZERO,
        });
        let z = space.zoom;
        let color = space.type_color(ty);

        let row_h = self.text_height().max(self.s(18.0));
        let label = port_id;
        let tw = self.text_width(label);
        let pin_d = PIN_R * 2.0 * z;
        let gap = self.s(8.0);
        let width = self.layer().fill_w.max(tw + pin_d + gap + self.s(4.0));
        let slot = match side {
            NodePortSide::Input => {
                let i = self.node_port_in;
                self.node_port_in += 1;
                i
            }
            NodePortSide::Output => {
                let i = self.node_port_out;
                self.node_port_out += 1;
                i
            }
        } as usize;
        let row = if let Some(row) = self.node_port_rows.get(slot).copied() {
            row
        } else {
            let row = self.allocate_fill_x(Vec2::new(width, row_h));
            self.node_port_rows.push(row);
            row
        };

        let node_rect = space
            .node_screen_rects
            .get(&node_id)
            .copied()
            .unwrap_or(row);
        let pin_center = match side {
            NodePortSide::Input => Vec2::new(node_rect.min.x, row.min.y + row_h * 0.5),
            NodePortSide::Output => Vec2::new(node_rect.max.x, row.min.y + row_h * 0.5),
        };

        let text_x = match side {
            NodePortSide::Input => pin_center.x + pin_d * 0.5 + gap * 0.5,
            NodePortSide::Output => pin_center.x - pin_d * 0.5 - gap * 0.5 - tw,
        };
        self.text(
            Vec2::new(text_x, row.min.y + (row_h - self.text_height()) * 0.5),
            label,
            theme::TEXT,
        );

        space.port_pos.insert(
            (node_id.clone(), side, port_id.to_string()),
            pin_center,
        );

        let pin_rect = Rect::from_min_size(
            pin_center - Vec2::splat(pin_d * 0.5),
            Vec2::splat(pin_d),
        );
        let hit_r = PIN_HIT * z;
        let hit = Rect::from_min_size(pin_center - Vec2::splat(hit_r), Vec2::splat(hit_r * 2.0));
        let mouse = self.input.mouse_pos;
        let hovered = !self.block_input
            && !self.mouse_over_absorb()
            && clip.contains(mouse)
            && hit.contains(mouse);

        // Content clip is the node interior — drop it so the outer half of the pin
        // is visible, then restore so later widgets stay inside the body.
        let content_clip = self.clip();
        if content_clip.is_some() {
            self.pop_clip();
        }
        let ring = (1.5 * z).max(0.4);
        let core = (3.0 * z).max(0.8);
        self.round_rect(pin_rect, pin_d * 0.5, color);
        self.round_rect(pin_rect.inset(ring), (pin_d * 0.5 - ring).max(0.4), theme::WIN_BODY);
        self.round_rect(pin_rect.inset(core), (pin_d * 0.5 - core).max(0.4), color);
        if hovered {
            self.round_rect(
                Rect::from_min_size(pin_center - Vec2::splat(hit_r * 0.7), Vec2::splat(hit_r * 1.4)),
                hit_r * 0.7,
                [color[0], color[1], color[2], 0.25],
            );
        }
        if let Some(c) = content_clip {
            self.push_clip(c);
        }

        if hovered {
            space.pointer_over_node = true;
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
        }

        let port_wid = match side {
            NodePortSide::Input => self.current_id(&format!("in:{port_id}")),
            NodePortSide::Output => self.current_id(&format!("out:{port_id}")),
        };

        // Start wire
        if hovered && self.input.mouse_pressed {
            self.active_id = Some(port_wid);
            space.pending = Some(PendingWire {
                from_node: node_id.clone(),
                from_port: port_id.to_string(),
                side,
                ty,
                start: pin_center,
            });
            space.selected_link = None;
        }

        // Complete wire on hover + release
        if hovered && self.input.mouse_released {
            if let Some(pending) = space.pending.clone() {
                let (out_n, out_p, in_n, in_p, out_ty, in_ty) = match (pending.side, side) {
                    (NodePortSide::Output, NodePortSide::Input) => (
                        pending.from_node,
                        pending.from_port,
                        node_id.clone(),
                        port_id.to_string(),
                        pending.ty,
                        ty,
                    ),
                    (NodePortSide::Input, NodePortSide::Output) => (
                        node_id.clone(),
                        port_id.to_string(),
                        pending.from_node,
                        pending.from_port,
                        ty,
                        pending.ty,
                    ),
                    _ => {
                        space.pending = None;
                        return;
                    }
                };
                if out_n != in_n && space.compatible(out_ty, in_ty) {
                    space
                        .links
                        .retain(|l| !(l.to_node == in_n && l.to_port == in_p));
                    let id = space.next_link_id;
                    space.next_link_id += 1;
                    space.links.push(NodeLink {
                        id,
                        from_node: out_n,
                        from_port: out_p,
                        to_node: in_n,
                        to_port: in_p,
                        ty: out_ty,
                    });
                }
                space.pending = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_needs_two_nodes() {
        let mut space = NodeSpace::new();
        assert!(space.group_nodes(&["a".into()]).is_none());
        let id = space.group_nodes(&["a".into(), "b".into()]).unwrap();
        assert_eq!(space.frames.len(), 1);
        assert_eq!(space.selected_frame.as_deref(), Some(id.as_str()));
        assert!(space.selected_nodes.is_empty());
    }

    #[test]
    fn node_belongs_to_one_frame() {
        let mut space = NodeSpace::new();
        space.group_nodes(&["a".into(), "b".into()]);
        space.group_nodes(&["b".into(), "c".into()]);
        assert_eq!(space.frames.len(), 2);
        assert_eq!(space.frames[0].node_ids, vec!["a".to_string()]);
        assert_eq!(
            space.frames[1].node_ids,
            ["b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn detach_prunes_empty_frame() {
        let mut space = NodeSpace::new();
        space.group_nodes(&["a".into(), "b".into()]);
        space.detach_node("a");
        assert_eq!(space.frames[0].node_ids, vec!["b".to_string()]);
        space.detach_node("b");
        assert!(space.frames.is_empty());
        assert!(space.selected_frame.is_none());
    }

    #[test]
    fn node_content_scale_tracks_zoom_out() {
        assert!((node_content_scale(1.0, 0.35) - 0.35).abs() < 1e-6);
        assert!(node_content_scale(1.0, 0.35) < 0.45);
        assert!((node_content_scale(1.25, 0.35) - 1.25 * 0.35).abs() < 1e-6);
        assert!((node_content_scale(1.0, ZOOM_MIN) - ZOOM_MIN).abs() < 1e-6);
    }

    #[test]
    fn link_handle_shrinks_when_zoomed_out() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(8.0, 0.0);
        assert!((link_handle(a, b, 1.0) - 48.0).abs() < 1e-4);
        assert!((link_handle(a, b, 0.35) - 48.0 * 0.35).abs() < 1e-4);
        assert!((link_handle(a, b, ZOOM_MIN) - 48.0 * ZOOM_MIN).abs() < 1e-4);
    }

    #[test]
    fn same_port_name_keeps_input_and_output() {
        let mut space = NodeSpace::new();
        space.port_pos.insert(
            ("seq".into(), NodePortSide::Input, "clock".into()),
            Vec2::new(0.0, 0.0),
        );
        space.port_pos.insert(
            ("seq".into(), NodePortSide::Output, "clock".into()),
            Vec2::new(10.0, 0.0),
        );
        assert_eq!(
            space.port_pos.get(&("seq".into(), NodePortSide::Input, "clock".into())),
            Some(&Vec2::new(0.0, 0.0))
        );
        assert_eq!(
            space.port_pos.get(&("seq".into(), NodePortSide::Output, "clock".into())),
            Some(&Vec2::new(10.0, 0.0))
        );
    }
}
