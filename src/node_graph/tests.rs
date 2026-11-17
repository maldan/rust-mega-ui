use std::collections::HashMap;

use glam::Vec2;

use super::NodeSpace;
use super::geom::{
    ZOOM_MIN, dist_point_link, dist_point_polyline, dist_point_segment, link_aabb, link_handle,
    link_seg_count, node_content_scale, rect_from_points, rects_overlap, snap_vec,
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
    space.set_pin_pos("seq", NodePortSide::Input, "clock", Vec2::new(0.0, 0.0));
    space.set_pin_pos("seq", NodePortSide::Output, "clock", Vec2::new(10.0, 0.0));
    assert_eq!(
        space.pin_pos("seq", NodePortSide::Input, "clock"),
        Some(Vec2::new(0.0, 0.0))
    );
    assert_eq!(
        space.pin_pos("seq", NodePortSide::Output, "clock"),
        Some(Vec2::new(10.0, 0.0))
    );
}

#[test]
fn pin_screen_applies_pan_zoom() {
    let mut space = NodeSpace::new();
    space.zoom = 2.0;
    space.pan = Vec2::new(10.0, 5.0);
    space.set_pin_pos("n", NodePortSide::Output, "o", Vec2::new(1.0, 2.0));
    assert_eq!(
        space.pin_screen("n", NodePortSide::Output, "o"),
        Some(Vec2::new(12.0, 9.0))
    );
}

#[test]
fn translate_node_pins_keeps_culled_wires_attached() {
    let mut space = NodeSpace::new();
    space.set_pin_pos("n", NodePortSide::Output, "o", Vec2::new(4.0, 8.0));
    space.translate_node_pins("n", Vec2::new(3.0, -1.0));
    assert_eq!(
        space.pin_pos("n", NodePortSide::Output, "o"),
        Some(Vec2::new(7.0, 7.0))
    );
}

#[test]
fn pin_epoch_drops_removed_ports_keeps_string_keys() {
    let mut space = NodeSpace::new();
    space.begin_node_pins("n");
    space.set_pin_pos("n", NodePortSide::Input, "a", Vec2::new(1.0, 0.0));
    space.set_pin_pos("n", NodePortSide::Input, "b", Vec2::new(2.0, 0.0));
    space.finish_node_pins("n");
    assert!(space.pin_pos("n", NodePortSide::Input, "a").is_some());
    assert!(space.pin_pos("n", NodePortSide::Input, "b").is_some());

    // Same ports again: keys reused (no clear); "b" dropped when omitted.
    space.begin_node_pins("n");
    space.set_pin_pos("n", NodePortSide::Input, "a", Vec2::new(3.0, 4.0));
    space.finish_node_pins("n");
    assert_eq!(
        space.pin_pos("n", NodePortSide::Input, "a"),
        Some(Vec2::new(3.0, 4.0))
    );
    assert!(space.pin_pos("n", NodePortSide::Input, "b").is_none());
}

#[test]
fn link_aabb_skips_wires_far_from_the_canvas() {
    let a = Vec2::new(0.0, 0.0);
    let b = Vec2::new(40.0, 0.0);
    let aabb = link_aabb(a, b, 1.0, 4.0);
    let view = Rect::from_min_size(Vec2::new(400.0, 400.0), Vec2::splat(100.0));
    assert!(!rects_overlap(aabb, view));
}

#[test]
fn link_aabb_keeps_wires_that_cross_the_canvas() {
    let a = Vec2::new(-80.0, 50.0);
    let b = Vec2::new(80.0, 50.0);
    let aabb = link_aabb(a, b, 1.0, 4.0);
    let view = Rect::from_min_size(Vec2::new(0.0, 0.0), Vec2::new(20.0, 100.0));
    assert!(rects_overlap(aabb, view));
}

#[test]
fn link_seg_count_uses_fewer_segments_on_short_wires() {
    let short = link_seg_count(Vec2::ZERO, Vec2::new(20.0, 0.0), 1.0);
    let long = link_seg_count(Vec2::ZERO, Vec2::new(800.0, 0.0), 1.0);
    assert_eq!(short, 4);
    assert_eq!(long, 18);
}

#[test]
fn dist_point_link_is_near_zero_on_the_curve() {
    let a = Vec2::ZERO;
    let b = Vec2::new(100.0, 0.0);
    let d = dist_point_link(Vec2::new(50.0, 0.0), a, b, 1.0);
    assert!(d < 2.0);
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
fn annotation_is_optional_owned_and_removed_on_detach() {
    let mut space = NodeSpace::new();
    assert_eq!(space.node_annotation("a"), None);
    let text = String::from("128 ms");
    space.set_node_annotation("a", Some(&text));
    drop(text);
    assert_eq!(space.node_annotation("a"), Some("128 ms"));
    space.set_node_annotation("a", Some("42 items\nsecond line"));
    assert_eq!(space.node_annotation("a"), Some("42 items"));
    for hidden in [None, Some(""), Some("\nsecond line")] {
        space.set_node_annotation("a", Some("128 ms"));
        space.set_node_annotation("a", hidden);
        assert_eq!(space.node_annotation("a"), None);
    }
    space.set_node_annotation("a", Some("128 ms"));
    space.set_node_annotation("b", Some("42 items"));
    space.detach_node("a");
    assert_eq!(space.node_annotation("a"), None);
    assert_eq!(space.node_annotation("b"), Some("42 items"));
}

#[test]
fn annotation_is_above_node_clipped_and_preserves_layout() {
    use crate::{Ui, UiInput};

    let mut ui = Ui::new();
    let color = ui.theme.node.annotation;
    let mut space = NodeSpace::new();
    let mut pos = Vec2::new(30.0, 100.0);
    let canvas = Rect::from_min_size(Vec2::ZERO, Vec2::splat(2000.0));
    for zoom in [0.35, 1.0, 2.0] {
        space.zoom = zoom;
        let mut baseline = None;
        for text in [
            None,
            Some("128 ms"),
            Some("A very long annotation that exceeds the node width"),
            None,
        ] {
            space.set_node_annotation("a", text);
            ui.begin_frame(UiInput {
                viewport: canvas.max,
                mouse_pos: Vec2::splat(-100.0),
                ..Default::default()
            });
            ui.node_space_rect("graph", canvas, &mut space, |ui| {
                ui.node("a", "Node", &mut pos, |ui| {
                    ui.node_port(NodePortSide::Input, "in", port_type::ANY);
                });
            });
            let out = ui.end_frame();
            let glyphs: Vec<_> = out
                .draw_list
                .iter()
                .filter(|d| d.colors == [color; 4])
                .collect();
            assert_eq!(!glyphs.is_empty(), text.is_some());
            let bounds = space.node_screen_rects["a"];
            for glyph in &glyphs {
                assert!(glyph.rect.max.y <= bounds.min.y - 3.0 * zoom + 1e-4);
                assert!(glyph.rect.min.x >= bounds.min.x + 4.0 * zoom - 1e-4);
                assert!(glyph.rect.max.x <= bounds.max.x - 4.0 * zoom + 1e-4);
            }
            if let Some("128 ms") = text {
                let px = ui.theme.metrics.font_size * node_content_scale(ui.scale(), zoom) * 0.82;
                let expected_x = bounds.max.x - 4.0 * zoom - ui.text_width_at("128 ms", px);
                assert!((glyphs[0].rect.min.x - expected_x).abs() < px * 0.2);
            }
            let layout = (
                space.node_sizes["a"],
                space.pin_pos("a", NodePortSide::Input, "in"),
            );
            if let Some(expected) = baseline {
                assert_eq!(layout, expected);
            }
            baseline = Some(layout);
            assert_eq!(space.node_annotation("a"), text);
        }
    }
}

#[test]
fn annotation_remains_visible_when_node_body_is_culled() {
    use crate::{Ui, UiInput};

    let mut ui = Ui::new();
    ui.theme.metrics.font_size = 28.0;
    let color = ui.theme.node.annotation;
    let mut space = NodeSpace::new();
    space.set_node_annotation("a", Some("128 ms"));
    let canvas = Rect::from_min_size(Vec2::ZERO, Vec2::splat(200.0));
    let mut pos = Vec2::new(10.0, 80.0);
    for culled in [false, true] {
        if culled {
            pos.y = 218.0;
        }
        ui.begin_frame(UiInput {
            viewport: canvas.max,
            ..Default::default()
        });
        let mut built = false;
        ui.node_space_rect("graph", canvas, &mut space, |ui| {
            ui.node("a", "Node", &mut pos, |_| {
                built = true;
            });
        });
        assert_eq!(built, !culled);
        let out = ui.end_frame();
        let glyphs: Vec<_> = out
            .draw_list
            .iter()
            .filter(|d| d.colors == [color; 4])
            .collect();
        assert!(!glyphs.is_empty());
        for glyph in glyphs {
            assert!(glyph.rect.max.y <= canvas.max.y);
            assert!(glyph.rect.min.y >= canvas.min.y);
        }
    }
}

#[test]
fn progress_is_optional_clamped_and_removed_on_detach() {
    let mut space = NodeSpace::new();
    assert_eq!(space.node_progress("a"), None);
    for (value, expected) in [(-0.5, 0.0), (0.45, 0.45), (1.5, 1.0)] {
        space.set_node_progress("a", Some(value));
        assert_eq!(space.node_progress("a"), Some(expected));
    }
    for value in [
        None,
        Some(f32::NAN),
        Some(f32::INFINITY),
        Some(f32::NEG_INFINITY),
    ] {
        space.set_node_progress("a", Some(0.5));
        space.set_node_progress("a", value);
        assert_eq!(space.node_progress("a"), None);
    }
    space.set_node_progress("a", Some(0.5));
    space.set_node_progress("b", Some(1.0));
    space.detach_node("a");
    assert_eq!(space.node_progress("a"), None);
    assert_eq!(space.node_progress("b"), Some(1.0));
}

#[test]
fn progress_strip_scales_and_preserves_node_and_port_layout() {
    use crate::{Ui, UiInput};

    let mut ui = Ui::new();
    let mut space = NodeSpace::new();
    let mut pos = Vec2::new(20.0, 20.0);
    let fill = ui.theme.node.progress;
    let track = ui.theme.node.progress_track;
    let canvas = Rect::from_min_size(Vec2::ZERO, Vec2::splat(2000.0));
    for zoom in [0.35, 1.0, 2.0] {
        space.zoom = zoom;
        let mut baseline = None;
        // Warm up the retained node dimensions, then compare all states.
        for progress in [None, None, Some(0.0), Some(0.25), Some(1.0), None] {
            space.set_node_progress("a", progress);
            ui.begin_frame(UiInput {
                viewport: canvas.max,
                mouse_pos: Vec2::splat(-100.0),
                ..Default::default()
            });
            ui.node_space_rect("graph", canvas, &mut space, |ui| {
                ui.node("a", "Node", &mut pos, |ui| {
                    ui.node_port(NodePortSide::Input, "in", port_type::ANY);
                });
            });
            let out = ui.end_frame();
            let tracks: Vec<_> = out
                .draw_list
                .iter()
                .filter(|d| d.colors == [track; 4])
                .collect();
            let fills: Vec<_> = out
                .draw_list
                .iter()
                .filter(|d| d.colors == [fill; 4])
                .collect();
            assert_eq!(tracks.len(), usize::from(progress.is_some()));
            assert_eq!(fills.len(), usize::from(progress.is_some_and(|v| v > 0.0)));
            if let Some(value) = progress {
                assert!((tracks[0].rect.height() - 3.0 * zoom).abs() < 1e-4);
                let bounds = space.node_screen_rects["a"];
                assert!(
                    (tracks[0].rect.min.y - bounds.min.y - ui.theme.metrics.window_title_h * zoom)
                        .abs()
                        < 1e-4
                );
                if value > 0.0 {
                    assert!((fills[0].rect.width() - tracks[0].rect.width() * value).abs() < 1e-4);
                }
            }
            let layout = (
                space.node_sizes["a"],
                space.pin_pos("a", NodePortSide::Input, "in"),
            );
            if let Some(expected) = baseline {
                assert_eq!(layout, expected);
            }
            baseline = Some(layout);
            assert_eq!(space.node_progress("a"), progress);
        }
    }
}

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
