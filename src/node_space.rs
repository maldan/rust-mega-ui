//! Node graph workspace: pan/zoom canvas, nodes with widget content, typed ports & links.
//!
//! Host owns [`NodeSpace`] (view + links) and its own node list / field data.
//! This module only draws and manipulates — no graph execution.

use std::collections::HashMap;
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
struct NodeDrag {
    primary: String,
    /// `mouse_world - primary_pos` at press.
    grab: Vec2,
    primary_start: Vec2,
    /// World positions at drag start (filled lazily as nodes are visited).
    anchors: HashMap<String, Vec2>,
}

/// Retained view / wiring state for one graph. Host keeps this between frames.
#[derive(Clone, Debug)]
pub struct NodeSpace {
    pub pan: Vec2,
    pub zoom: f32,
    /// Grid snap for node moves in world units. `0` disables. Default `10`.
    pub snap: f32,
    pub links: Vec<NodeLink>,
    pub types: Vec<PortType>,
    pub selected_nodes: Vec<String>,
    pub selected_link: Option<u64>,
    /// Cleared by host after applying (remove nodes from its list).
    pub request_delete_nodes: Vec<String>,
    /// World-space position of last RMB on empty canvas (for spawn menus).
    pub context_world: Option<Vec2>,
    /// True when pointer is over empty canvas (not a node) inside the space.
    pub background_hovered: bool,

    pub next_link_id: u64,
    pending: Option<PendingWire>,
    port_pos: HashMap<(String, String), Vec2>,
    /// Last frame outer size in **world** units (title + body).
    node_sizes: HashMap<String, Vec2>,
    node_order: Vec<String>,
    pan_grab: Option<Vec2>,
    node_drag: Option<NodeDrag>,
    link_hit: Option<u64>,
    /// Set during [`Ui::node`] when the pointer is over a node this frame.
    pointer_over_node: bool,
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
            snap: 10.0,
            links: Vec::new(),
            types: default_port_types(),
            selected_nodes: Vec::new(),
            selected_link: None,
            request_delete_nodes: Vec::new(),
            context_world: None,
            background_hovered: false,
            next_link_id: 1,
            pending: None,
            port_pos: HashMap::new(),
            node_sizes: HashMap::new(),
            node_order: Vec::new(),
            pan_grab: None,
            node_drag: None,
            link_hit: None,
            pointer_over_node: false,
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

    /// Remove links that reference `node_id`.
    pub fn detach_node(&mut self, node_id: &str) {
        self.links
            .retain(|l| l.from_node != node_id && l.to_node != node_id);
        self.selected_nodes.retain(|id| id != node_id);
        self.port_pos.retain(|(n, _), _| n != node_id);
        self.node_sizes.remove(node_id);
        self.node_order.retain(|id| id != node_id);
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

    /// Click select: Ctrl toggles; otherwise replace (unless already selected — keep multi).
    fn select_for_click(&mut self, id: &str, ctrl: bool) {
        if ctrl {
            if let Some(i) = self.selected_nodes.iter().position(|x| x == id) {
                self.selected_nodes.remove(i);
            } else {
                self.selected_nodes.push(id.to_string());
            }
        } else {
            self.selected_nodes = vec![id.to_string()];
        }
        self.selected_link = None;
    }

    /// Press on title to drag: Ctrl adds; click unselected replaces; click selected keeps group.
    fn select_for_drag(&mut self, id: &str, ctrl: bool) {
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
        self.select_for_drag(id, ctrl);
        self.bring_front(id);
        let mut anchors = HashMap::new();
        anchors.insert(id.to_string(), pos);
        self.node_drag = Some(NodeDrag {
            primary: id.to_string(),
            grab: mouse_world - pos,
            primary_start: pos,
            anchors,
        });
    }

    /// Update `pos` while a group/single drag is active.
    fn apply_node_drag(&mut self, id: &str, pos: &mut Vec2, mouse_world: Vec2) -> bool {
        let is_sel = self.is_selected(id);
        let snap = self.snap;
        let Some(drag) = self.node_drag.as_mut() else {
            return false;
        };
        let raw = mouse_world - drag.grab;
        let primary_now = if snap <= 1e-6 {
            raw
        } else {
            Vec2::new((raw.x / snap).round() * snap, (raw.y / snap).round() * snap)
        };
        if id == drag.primary {
            *pos = primary_now;
            true
        } else if is_sel {
            let start = *drag.anchors.entry(id.to_string()).or_insert(*pos);
            *pos = start + (primary_now - drag.primary_start);
            true
        } else {
            false
        }
    }

    fn compatible(a: u16, b: u16) -> bool {
        a == port_type::ANY || b == port_type::ANY || a == b
    }

    fn bring_front(&mut self, id: &str) {
        self.node_order.retain(|x| x != id);
        self.node_order.push(id.to_string());
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
            color: [0.45, 0.85, 0.45, 1.0],
        },
        PortType {
            id: port_type::STRING,
            name: "string".into(),
            color: [0.95, 0.55, 0.35, 1.0],
        },
        PortType {
            id: port_type::BOOL,
            name: "bool".into(),
            color: [0.90, 0.40, 0.55, 1.0],
        },
    ]
}

fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
}

fn link_curve(from: Vec2, to: Vec2) -> Vec<Vec2> {
    let dx = ((to.x - from.x).abs() * 0.5).max(48.0);
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

const NODE_MIN_W: f32 = 168.0;
const NODE_PAD: f32 = 10.0;
const PIN_R: f32 = 5.5;
const PIN_HIT: f32 = 10.0;

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

        space.zoom = space.zoom.clamp(0.35, 2.5);
        space.background_hovered = false;
        space.link_hit = None;
        space.pointer_over_node = false;

        // Background
        self.round_rect(rect, self.s(4.0), [0.02, 0.02, 0.02, 1.0]);
        self.push_clip(rect);
        self.draw_node_grid(rect, space);

        let mouse = self.input.mouse_pos;
        let in_rect =
            rect.contains(mouse) && !self.block_input && !self.mouse_over_absorb();

        // Zoom toward cursor
        if in_rect && self.input.scroll_delta.y.abs() > 0.0 && space.pending.is_none() {
            let old_z = space.zoom;
            let factor = if self.input.scroll_delta.y > 0.0 {
                1.08
            } else {
                1.0 / 1.08
            };
            let new_z = (old_z * factor).clamp(0.35, 2.5);
            if (new_z - old_z).abs() > 1e-5 {
                let world = space.screen_to_world(mouse);
                space.zoom = new_z;
                space.pan = mouse - world * space.zoom;
                self.request_repaint();
            }
            self.consume_scroll();
            self.want_capture = true;
        }

        // Links (last-frame port positions) + hit-test
        if in_rect && space.pending.is_none() && space.node_drag.is_none() && space.pan_grab.is_none()
        {
            let mut best: Option<(u64, f32)> = None;
            for link in &space.links {
                let Some(&a) = space
                    .port_pos
                    .get(&(link.from_node.clone(), link.from_port.clone()))
                else {
                    continue;
                };
                let Some(&b) = space
                    .port_pos
                    .get(&(link.to_node.clone(), link.to_port.clone()))
                else {
                    continue;
                };
                let pts = link_curve(a, b);
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
                let Some(&a) = space
                    .port_pos
                    .get(&(link.from_node.clone(), link.from_port.clone()))
                else {
                    continue;
                };
                let Some(&b) = space
                    .port_pos
                    .get(&(link.to_node.clone(), link.to_port.clone()))
                else {
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
                let pts = link_curve(a, b);
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

        add(self);

        // Pending wire
        if let Some(ref pending) = space.pending {
            let color = space.type_color(pending.ty);
            let end = mouse;
            let (a, b) = match pending.side {
                NodePortSide::Output => (pending.start, end),
                NodePortSide::Input => (end, pending.start),
            };
            let pts = link_curve(a, b);
            let thick = 2.4 * space.zoom.clamp(0.7, 1.4);
            self.draw_polyline(&pts, thick, color);
            self.want_capture = true;
            self.request_repaint();
        }

        let hovered_bg = in_rect
            && space.pending.is_none()
            && space.node_drag.is_none()
            && !space.pointer_over_node
            && space.link_hit.is_none()
            && !self.input.mouse_middle_down;
        space.background_hovered = hovered_bg;

        if in_rect {
            self.want_capture = true;
        }

        // Pan: MMB anywhere, or LMB on empty background
        let pan_id = space_id.child("#pan");
        let start_mmb_pan = in_rect && self.input.mouse_middle_pressed && space.pending.is_none();
        let start_lmb_pan = hovered_bg && self.input.mouse_pressed;
        if start_mmb_pan || start_lmb_pan {
            self.active_id = Some(pan_id);
            space.pan_grab = Some(mouse - space.pan);
            if start_lmb_pan {
                space.selected_nodes.clear();
                space.selected_link = None;
            }
        }

        // Link select / Alt+click delete
        if space.link_hit.is_some()
            && self.input.mouse_pressed
            && space.pending.is_none()
            && !self.input.mouse_middle_down
        {
            let lid = space.link_hit.unwrap();
            if self.input.key_shift || space.selected_link == Some(lid) {
                // Shift+click, or click already-selected wire → delete
                space.remove_link(lid);
                space.selected_link = None;
            } else {
                space.selected_link = Some(lid);
                space.selected_nodes.clear();
                self.active_id = Some(space_id.child("#link"));
            }
        }

        let panning = self.active_id == Some(pan_id)
            && (self.input.mouse_down || self.input.mouse_middle_down);
        if panning {
            if let Some(grab) = space.pan_grab {
                space.pan = mouse - grab;
                self.set_cursor(CursorIcon::Move);
                self.want_capture = true;
                self.request_repaint();
            }
        }
        if self.input.mouse_released || self.input.mouse_middle_released {
            if self.input.mouse_middle_released || self.active_id == Some(pan_id) {
                space.pan_grab = None;
            }
            space.node_drag = None;
            if self.input.mouse_released && space.pending.is_some() {
                space.pending = None;
            }
        }

        if in_rect && self.input.mouse_right_pressed && hovered_bg {
            space.context_world = Some(space.screen_to_world(mouse));
        }

        // Delete selection: Delete always; Backspace when not typing
        let want_delete = self.input.key_delete
            || (self.input.key_backspace && self.focus_id.is_none());
        if in_rect && want_delete {
            if let Some(lid) = space.selected_link.take() {
                space.remove_link(lid);
            } else if !space.selected_nodes.is_empty() {
                let ids = space.selected_nodes.clone();
                for nid in &ids {
                    space.detach_node(nid);
                }
                space.request_delete_nodes.extend(ids);
            }
        }

        self.node_space_ptr = prev_ptr;
        self.node_space_clip = prev_clip;
        self.node_space_id = prev_id;
        self.pop_clip();
        self.pop_id();
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

        // Apply ongoing group drag before layout (followers move same frame).
        let dragging = space.apply_node_drag(id, pos, mouse_world);
        if dragging {
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
        let close_id = node_id.child("#close");

        // Close button (always) — top-right of title
        let btn = 16.0 * z.clamp(0.75, 1.35);
        let btn_pad = 4.0 * z;
        let mut close_r = Rect::from_min_size(
            Vec2::new(
                rect.max.x - btn_pad - btn,
                rect.min.y + (title_h * z - btn) * 0.5,
            ),
            Vec2::splat(btn),
        );
        let close_hover = hovered && close_r.contains(mouse) && !self.block_input;
        if close_hover {
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
            if self.input.mouse_pressed {
                self.active_id = Some(close_id);
            }
        }
        if self.active_id == Some(close_id) && self.input.mouse_released && close_hover {
            space.detach_node(id);
            space.request_delete_nodes.push(id.to_string());
            self.pop_id();
            return;
        }

        // Title drag (ignore close button)
        let title_hover = hovered && title_bar.contains(mouse) && !close_r.contains(mouse);
        if title_hover {
            self.set_cursor(CursorIcon::Move);
            self.want_capture = true;
        }
        if title_hover && self.input.mouse_pressed && space.node_drag.is_none() {
            self.active_id = Some(title_id);
            space.begin_node_drag(id, *pos, mouse_world, ctrl);
            let _ = space.apply_node_drag(id, pos, mouse_world);
        }

        // Body click → selection (Ctrl toggles)
        if hovered && self.input.mouse_pressed && !title_hover && !close_hover {
            space.select_for_click(id, ctrl);
            space.bring_front(id);
        }

        let selected = space.is_selected(id);
        let border = if selected {
            theme::ACCENT
        } else {
            theme::WIN_BORDER
        };
        let radius = self.s(theme::WIN_RADIUS) * z.clamp(0.75, 1.25);

        // Temporarily scale widgets with zoom
        let old_scale = self.scale;
        let old_spacing = self.spacing;
        self.scale = (old_scale * z).clamp(0.45, 3.0);
        self.spacing = self.base_spacing * self.scale;

        // Refresh geometry after drag write
        screen_pos = space.world_to_screen(*pos);
        rect = Rect::from_min_size(screen_pos, screen_size);
        close_r = Rect::from_min_size(
            Vec2::new(
                rect.max.x - btn_pad - btn,
                rect.min.y + (title_h * z - btn) * 0.5,
            ),
            Vec2::splat(btn),
        );

        self.round_rect(rect, radius, border);
        self.round_rect(rect.inset(1.0), (radius - 1.0).max(0.0), theme::WIN_BODY);

        let title_color = if self.active_id == Some(title_id) || dragging {
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
            theme::TITLE_TEXT,
        );

        // Close chrome
        let close_bg = if self.active_id == Some(close_id) && self.input.mouse_down {
            theme::BTN_PRESS
        } else if close_hover {
            theme::BTN_HOVER
        } else {
            [0.20, 0.20, 0.20, 1.0]
        };
        self.round_rect(close_r, 3.0 * z.clamp(0.75, 1.2), close_bg);
        let xw = self.text_width("x");
        self.text(
            Vec2::new(
                close_r.min.x + (close_r.width() - xw) * 0.5,
                close_r.min.y + (close_r.height() - th) * 0.5,
            ),
            "x",
            theme::TITLE_TEXT,
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
        add(self);
        self.current_node_id = None;

        let used = self.layer().used;
        self.layers.pop();
        self.pop_clip();

        let body_h = used.y + pad * 1.2;
        let world_h = (title_h + body_h / z.max(1e-4)).max(title_h + 24.0);
        let world_w = (used.x / z.max(1e-4) + NODE_PAD * 2.0).max(min_w);
        space
            .node_sizes
            .insert(id.to_string(), Vec2::new(world_w, world_h));

        self.scale = old_scale;
        self.spacing = old_spacing;
        self.pop_id();
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
        let pin_d = PIN_R * 2.0 * z.clamp(0.75, 1.5);
        let gap = self.s(8.0);
        let width = self.layer().fill_w.max(tw + pin_d + gap + self.s(4.0));
        let row = self.allocate(Vec2::new(width, row_h));

        let pin_center = match side {
            NodePortSide::Input => {
                Vec2::new(row.min.x + pin_d * 0.5, row.min.y + row_h * 0.5)
            }
            NodePortSide::Output => {
                Vec2::new(row.max.x - pin_d * 0.5, row.min.y + row_h * 0.5)
            }
        };
        // Pins sit on the node edge — nudge outward slightly relative to content pad
        let pin_center = match side {
            NodePortSide::Input => Vec2::new(
                pin_center.x - NODE_PAD * z * 0.35,
                pin_center.y,
            ),
            NodePortSide::Output => Vec2::new(
                pin_center.x + NODE_PAD * z * 0.35,
                pin_center.y,
            ),
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

        let pin_rect = Rect::from_min_size(
            pin_center - Vec2::splat(pin_d * 0.5),
            Vec2::splat(pin_d),
        );
        self.round_rect(pin_rect, pin_d * 0.5, color);
        self.round_rect(pin_rect.inset(1.5), (pin_d * 0.5 - 1.5).max(0.5), theme::WIN_BODY);
        self.round_rect(pin_rect.inset(3.0), (pin_d * 0.5 - 3.0).max(0.5), color);

        space
            .port_pos
            .insert((node_id.clone(), port_id.to_string()), pin_center);

        let hit_r = PIN_HIT * z.clamp(0.75, 1.5);
        let hit = Rect::from_min_size(pin_center - Vec2::splat(hit_r), Vec2::splat(hit_r * 2.0));
        let mouse = self.input.mouse_pos;
        let hovered = !self.block_input
            && !self.mouse_over_absorb()
            && clip.contains(mouse)
            && hit.contains(mouse);

        if hovered {
            self.want_capture = true;
            self.set_cursor(CursorIcon::Pointer);
            // highlight
            self.round_rect(
                Rect::from_min_size(pin_center - Vec2::splat(hit_r * 0.7), Vec2::splat(hit_r * 1.4)),
                hit_r * 0.7,
                [color[0], color[1], color[2], 0.25],
            );
        }

        let port_wid = self.current_id(port_id);

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
                if out_n != in_n && NodeSpace::compatible(out_ty, in_ty) {
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
