use std::collections::HashMap;

use glam::Vec2;

use super::NodeSpace;
use super::geom::{
    ZOOM_MIN, dist_point_polyline, dist_point_segment, link_handle, node_content_scale,
    rect_from_points, rects_overlap, snap_vec,
};
use super::types::{NodeLink, NodePortSide, port_type};
use crate::types::Rect;

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
    assert_eq!(space.frames[1].node_ids, ["b".to_string(), "c".to_string()]);
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
        space
            .port_pos
            .get(&("seq".into(), NodePortSide::Input, "clock".into())),
        Some(&Vec2::new(0.0, 0.0))
    );
    assert_eq!(
        space
            .port_pos
            .get(&("seq".into(), NodePortSide::Output, "clock".into())),
        Some(&Vec2::new(10.0, 0.0))
    );
}

// -- coordinate transforms ---------------------------------------------------

#[test]
fn screen_to_world_inverts_world_to_screen() {
    let mut space = NodeSpace::new();
    space.pan = Vec2::new(37.0, -12.0);
    space.zoom = 1.6;
    let world = Vec2::new(123.0, -45.0);
    let screen = space.world_to_screen(world);
    let back = space.screen_to_world(screen);
    assert!((back - world).length() < 1e-3);
}

#[test]
fn screen_to_world_does_not_divide_by_zero_zoom() {
    let mut space = NodeSpace::new();
    space.zoom = 0.0;
    let w = space.screen_to_world(Vec2::new(10.0, 10.0));
    assert!(w.x.is_finite() && w.y.is_finite());
}

// -- is_selected --------------------------------------------------------------

#[test]
fn is_selected_reflects_selected_nodes_list() {
    let mut space = NodeSpace::new();
    space.selected_nodes.push("a".into());
    assert!(space.is_selected("a"));
    assert!(!space.is_selected("b"));
}

// -- link management ----------------------------------------------------------

#[test]
fn remove_link_by_id_only_removes_matching_link() {
    let mut space = NodeSpace::new();
    space.links.push(NodeLink {
        id: 1,
        from_node: "a".into(),
        from_port: "out".into(),
        to_node: "b".into(),
        to_port: "in".into(),
        ty: port_type::ANY,
    });
    space.links.push(NodeLink {
        id: 2,
        from_node: "b".into(),
        from_port: "out".into(),
        to_node: "c".into(),
        to_port: "in".into(),
        ty: port_type::ANY,
    });
    space.selected_link = Some(1);
    space.remove_link(1);
    assert_eq!(space.links.len(), 1);
    assert_eq!(space.links[0].id, 2);
    assert!(
        space.selected_link.is_none(),
        "removing the selected link must clear selection"
    );
}

#[test]
fn remove_link_missing_id_is_noop() {
    let mut space = NodeSpace::new();
    space.links.push(NodeLink {
        id: 1,
        from_node: "a".into(),
        from_port: "out".into(),
        to_node: "b".into(),
        to_port: "in".into(),
        ty: port_type::ANY,
    });
    space.remove_link(999);
    assert_eq!(space.links.len(), 1);
}

#[test]
fn duplicate_links_only_copies_links_fully_covered_by_id_map() {
    let mut space = NodeSpace::new();
    space.links.push(NodeLink {
        id: 1,
        from_node: "a".into(),
        from_port: "out".into(),
        to_node: "b".into(),
        to_port: "in".into(),
        ty: port_type::ANY,
    });
    // Link to an external node "x" not present in the duplication set.
    space.links.push(NodeLink {
        id: 2,
        from_node: "b".into(),
        from_port: "out".into(),
        to_node: "x".into(),
        to_port: "in".into(),
        ty: port_type::ANY,
    });
    let mut id_map = HashMap::new();
    id_map.insert("a".to_string(), "a2".to_string());
    id_map.insert("b".to_string(), "b2".to_string());

    // Mirror what the real API does: manually-pushed links must advance the
    // id counter themselves, otherwise duplication can mint a colliding id.
    space.next_link_id = 3;
    space.duplicate_links(&id_map);

    assert_eq!(
        space.links.len(),
        3,
        "only the fully-mapped link should be duplicated"
    );
    let dup = space.links.last().unwrap();
    assert_eq!(dup.from_node, "a2");
    assert_eq!(dup.to_node, "b2");
    assert_ne!(dup.id, 1);
    assert_ne!(dup.id, 2);
}

#[test]
fn detach_node_removes_its_links_but_keeps_others() {
    let mut space = NodeSpace::new();
    space.links.push(NodeLink {
        id: 1,
        from_node: "a".into(),
        from_port: "out".into(),
        to_node: "b".into(),
        to_port: "in".into(),
        ty: port_type::ANY,
    });
    space.links.push(NodeLink {
        id: 2,
        from_node: "c".into(),
        from_port: "out".into(),
        to_node: "d".into(),
        to_port: "in".into(),
        ty: port_type::ANY,
    });
    space.detach_node("a");
    assert_eq!(space.links.len(), 1);
    assert_eq!(space.links[0].id, 2);
}

// -- geometry helpers -----------------------------------------------------

#[test]
fn dist_point_segment_zero_on_segment() {
    let d = dist_point_segment(Vec2::new(5.0, 0.0), Vec2::ZERO, Vec2::new(10.0, 0.0));
    assert!(d < 1e-4);
}

#[test]
fn dist_point_segment_perpendicular_distance() {
    let d = dist_point_segment(Vec2::new(5.0, 3.0), Vec2::ZERO, Vec2::new(10.0, 0.0));
    assert!((d - 3.0).abs() < 1e-4);
}

#[test]
fn dist_point_segment_clamps_to_nearest_endpoint() {
    // Point beyond segment end — nearest point is the endpoint, not the infinite line.
    let d = dist_point_segment(Vec2::new(20.0, 0.0), Vec2::ZERO, Vec2::new(10.0, 0.0));
    assert!((d - 10.0).abs() < 1e-4);
}

#[test]
fn dist_point_segment_degenerate_zero_length_segment() {
    let d = dist_point_segment(Vec2::new(3.0, 4.0), Vec2::ZERO, Vec2::ZERO);
    assert!((d - 5.0).abs() < 1e-4);
}

#[test]
fn dist_point_polyline_picks_closest_segment() {
    let pts = [
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(10.0, 10.0),
    ];
    let d = dist_point_polyline(Vec2::new(10.0, 5.0), &pts);
    assert!(d < 1e-4);
}

#[test]
fn rect_from_points_normalizes_min_max_regardless_of_order() {
    let r = rect_from_points(Vec2::new(10.0, 10.0), Vec2::new(2.0, 8.0));
    assert_eq!(r.min, Vec2::new(2.0, 8.0));
    assert_eq!(r.max, Vec2::new(10.0, 10.0));
}

#[test]
fn rects_overlap_detects_intersection_and_touching_is_not_overlap() {
    let a = Rect::from_min_size(Vec2::ZERO, Vec2::splat(10.0));
    let overlapping = Rect::from_min_size(Vec2::new(5.0, 5.0), Vec2::splat(10.0));
    let touching = Rect::from_min_size(Vec2::new(10.0, 0.0), Vec2::splat(10.0));
    let disjoint = Rect::from_min_size(Vec2::new(20.0, 20.0), Vec2::splat(5.0));
    assert!(rects_overlap(a, overlapping));
    assert!(!rects_overlap(a, touching));
    assert!(!rects_overlap(a, disjoint));
}

#[test]
fn snap_vec_rounds_to_grid() {
    let p = snap_vec(Vec2::new(13.0, 27.0), 5.0);
    assert_eq!(p, Vec2::new(15.0, 25.0));
}

#[test]
fn snap_vec_disabled_when_snap_is_near_zero() {
    let p = Vec2::new(13.37, -8.21);
    assert_eq!(snap_vec(p, 0.0), p);
}
