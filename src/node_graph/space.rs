use std::collections::{HashMap, HashSet};

use glam::Vec2;

use crate::types::Rect;

use super::geom::{FRAME_PAD, NODE_MIN_W, ZOOM_MIN, snap_vec};
use super::types::{NodeFrame, NodeLink, NodePortSide, PortType, default_port_types, port_type};

/// Upsert by `&str`: allocate `String` only when the key is new.
pub(crate) fn map_upsert<V>(map: &mut HashMap<String, V>, key: &str, val: V) {
    if let Some(slot) = map.get_mut(key) {
        *slot = val;
    } else {
        map.insert(key.to_string(), val);
    }
}

#[derive(Clone, Copy, Debug)]
struct PinSlot {
    pos: Vec2,
    epoch: u32,
}

/// World-space pin positions on one node. Nested maps so lookup is `&str` (no `String` clone).
/// `epoch` keeps port `String` keys alive across frames; stale ports are retained away.
#[derive(Clone, Debug, Default)]
pub(crate) struct NodePins {
    inputs: HashMap<String, PinSlot>,
    outputs: HashMap<String, PinSlot>,
    epoch: u32,
}

impl NodePins {
    fn begin_frame(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
    }

    fn retain_current(&mut self) {
        let e = self.epoch;
        self.inputs.retain(|_, s| s.epoch == e);
        self.outputs.retain(|_, s| s.epoch == e);
    }

    fn get(&self, side: NodePortSide, port: &str) -> Option<Vec2> {
        let slot = match side {
            NodePortSide::Input => self.inputs.get(port),
            NodePortSide::Output => self.outputs.get(port),
        }?;
        Some(slot.pos)
    }

    fn insert(&mut self, side: NodePortSide, port: &str, pos: Vec2) {
        let map = match side {
            NodePortSide::Input => &mut self.inputs,
            NodePortSide::Output => &mut self.outputs,
        };
        let epoch = self.epoch;
        if let Some(slot) = map.get_mut(port) {
            slot.pos = pos;
            slot.epoch = epoch;
        } else {
            map.insert(port.to_string(), PinSlot { pos, epoch });
        }
    }

    fn translate(&mut self, delta: Vec2) {
        for v in self.inputs.values_mut() {
            v.pos += delta;
        }
        for v in self.outputs.values_mut() {
            v.pos += delta;
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PendingWire {
    pub(crate) from_node: String,
    pub(crate) from_port: String,
    pub(crate) side: NodePortSide,
    pub(crate) ty: u16,
    /// Screen-space start of the wire.
    pub(crate) start: Vec2,
}

#[derive(Clone, Debug)]
pub(crate) struct NodeDrag {
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
pub(crate) struct FrameDrag {
    frame_id: String,
    /// `mouse_world - origin` at press.
    grab: Vec2,
    origin_start: Vec2,
    anchors: HashMap<String, Vec2>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BoxSelect {
    /// Screen-space drag origin.
    pub(crate) start: Vec2,
    /// Ctrl held at press → add to selection instead of replace.
    pub(crate) additive: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingNodePress {
    pub(crate) id: String,
    /// True = start / continue group drag from title; false = body select.
    pub(crate) on_title: bool,
    pub(crate) ctrl: bool,
    pub(crate) mouse_world: Vec2,
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
    pub(crate) pending: Option<PendingWire>,
    pub(crate) port_pos: HashMap<String, NodePins>,
    /// Last frame outer size in **world** units (title + body).
    pub(crate) node_sizes: HashMap<String, Vec2>,
    /// Screen-space node bounds from the current/last build (for marquee).
    pub(crate) node_screen_rects: HashMap<String, Rect>,
    /// Latest world positions seen this/last frame (for group-drag anchors).
    pub(crate) node_world_pos: HashMap<String, Vec2>,
    pub(crate) node_order: Vec<String>,
    pub(crate) pan_grab: Option<Vec2>,
    pub(crate) node_drag: Option<NodeDrag>,
    pub(crate) frame_drag: Option<FrameDrag>,
    pub(crate) box_select: Option<BoxSelect>,
    /// Topmost node press this frame (later draws overwrite → frontmost wins).
    pub(crate) pending_node_press: Option<PendingNodePress>,
    pub(crate) link_hit: Option<u64>,
    /// Set during [`crate::Ui::node`] when the pointer is over a node this frame.
    pub(crate) pointer_over_node: bool,
    pub(crate) pointer_over_frame: bool,
    pub(crate) frame_screen_rects: HashMap<String, Rect>,
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
        if self
            .frame_drag
            .as_ref()
            .is_some_and(|d| d.frame_id == frame_id)
        {
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
        self.port_pos.remove(node_id);
        self.node_sizes.remove(node_id);
        self.node_screen_rects.remove(node_id);
        self.node_world_pos.remove(node_id);
        self.node_order.retain(|id| id != node_id);
        for frame in &mut self.frames {
            frame.node_ids.retain(|n| n != node_id);
        }
        self.frames.retain(|f| !f.node_ids.is_empty());
        let fid = self.selected_frame.clone();
        if let Some(id) = fid
            && !self.frames.iter().any(|f| f.id == id)
        {
            self.selected_frame = None;
        }
    }

    pub fn remove_link(&mut self, id: u64) {
        self.links.retain(|l| l.id != id);
        if self.selected_link == Some(id) {
            self.selected_link = None;
        }
    }

    pub(crate) fn pin_pos(&self, node: &str, side: NodePortSide, port: &str) -> Option<Vec2> {
        self.port_pos.get(node)?.get(side, port)
    }

    pub(crate) fn pin_screen(&self, node: &str, side: NodePortSide, port: &str) -> Option<Vec2> {
        Some(self.world_to_screen(self.pin_pos(node, side, port)?))
    }

    pub(crate) fn set_pin_pos(&mut self, node: &str, side: NodePortSide, port: &str, pos: Vec2) {
        if let Some(pins) = self.port_pos.get_mut(node) {
            pins.insert(side, port, pos);
            return;
        }
        let mut pins = NodePins::default();
        pins.insert(side, port, pos);
        self.port_pos.insert(node.to_string(), pins);
    }

    /// Start a pin rebuild for `node` this frame (keeps `String` keys; bumps epoch).
    pub(crate) fn begin_node_pins(&mut self, node: &str) {
        if let Some(pins) = self.port_pos.get_mut(node) {
            pins.begin_frame();
        }
    }

    /// Drop ports not touched since [`Self::begin_node_pins`].
    pub(crate) fn finish_node_pins(&mut self, node: &str) {
        if let Some(pins) = self.port_pos.get_mut(node) {
            pins.retain_current();
        }
    }

    pub(crate) fn translate_node_pins(&mut self, node: &str, delta: Vec2) {
        if delta.length_squared() < 1e-10 {
            return;
        }
        if let Some(pins) = self.port_pos.get_mut(node) {
            pins.translate(delta);
        }
    }

    pub(crate) fn world_to_screen(&self, world: Vec2) -> Vec2 {
        world * self.zoom + self.pan
    }

    pub(crate) fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        (screen - self.pan) / self.zoom.max(1e-4)
    }

    pub(crate) fn is_selected(&self, id: &str) -> bool {
        self.selected_nodes.iter().any(|x| x == id)
    }

    pub(crate) fn clear_frame_sel(&mut self) {
        self.selected_frame = None;
        self.frame_drag = None;
    }

    /// Click select: Ctrl toggles; click unselected replaces; click selected keeps multi.
    pub(crate) fn select_for_click(&mut self, id: &str, ctrl: bool) {
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
    pub(crate) fn select_for_drag(&mut self, id: &str, ctrl: bool) {
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

    pub(crate) fn begin_node_drag(&mut self, id: &str, pos: Vec2, mouse_world: Vec2, ctrl: bool) {
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

    pub(crate) fn begin_frame_drag(&mut self, frame_id: &str, mouse_world: Vec2) {
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
    pub(crate) fn apply_node_drag(&mut self, id: &str, pos: &mut Vec2, mouse_world: Vec2) -> bool {
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

    pub(crate) fn frame_world_rect_id(&self, frame_id: &str) -> Option<Rect> {
        let frame = self.frames.iter().find(|f| f.id == frame_id)?;
        self.frame_world_rect(frame)
    }

    pub(crate) fn frame_world_rect(&self, frame: &NodeFrame) -> Option<Rect> {
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

    pub(crate) fn frame_at(&self, screen: Vec2) -> Option<String> {
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

    pub(crate) fn compatible(&self, a: u16, b: u16) -> bool {
        if a == port_type::ANY || b == port_type::ANY || a == b {
            return true;
        }
        let pair = if a < b { (a, b) } else { (b, a) };
        self.cast_pairs.contains(&pair)
    }

    pub(crate) fn bring_front(&mut self, id: &str) {
        self.node_order.retain(|x| x != id);
        self.node_order.push(id.to_string());
    }

    pub(crate) fn bring_frame_front(&mut self, id: &str) {
        if let Some(i) = self.frames.iter().position(|f| f.id == id) {
            let frame = self.frames.remove(i);
            self.frames.push(frame);
        }
    }
}
