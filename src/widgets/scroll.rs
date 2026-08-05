use glam::Vec2;

use crate::theme;
use crate::types::{CursorIcon, Id, Rect};
use crate::{new_layer, LayoutDir, ScrollState, Ui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAxes {
    Vertical,
    Horizontal,
    Both,
}

impl ScrollAxes {
    fn vertical(self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }
    fn horizontal(self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }
}

impl Ui {
    pub fn scroll_area(
        &mut self,
        id: &str,
        size: Vec2,
        axes: ScrollAxes,
        add: impl FnOnce(&mut Self),
    ) {
        let widget_id = self.current_id(id);
        let v_id = widget_id.child("vbar");
        let h_id = widget_id.child("hbar");

        let outer = self.allocate(size);
        let mut st = self
            .scrolls
            .get(&widget_id)
            .copied()
            .unwrap_or(ScrollState {
                offset: Vec2::ZERO,
                target: Vec2::ZERO,
                content: Vec2::ZERO,
            });

        let bar = self.s(theme::SCROLL_BAR);
        let gap = self.s(theme::SCROLL_GAP);
        let need_v = axes.vertical() && st.content.y > size.y + 0.5;
        let need_h = axes.horizontal() && st.content.x > size.x + 0.5;
        let view = Rect {
            min: outer.min,
            max: Vec2::new(
                outer.max.x - if need_v { bar + gap } else { 0.0 },
                outer.max.y - if need_h { bar + gap } else { 0.0 },
            ),
        };
        if view.width() < 1.0 || view.height() < 1.0 {
            return;
        }

        let dragging = interact_bars(self, &mut st, view, axes, need_v, need_h, v_id, h_id, bar, gap);

        let hovered = self.hovered_rect(view) || self.hovered_rect(outer);
        if hovered {
            self.want_capture = true;
            self.scroll_hover = Some(widget_id);
        }

        // Wheel is applied after content so children (knob, …) can consume it first.
        let max_scroll = Vec2::new(
            (st.content.x - view.width()).max(0.0),
            (st.content.y - view.height()).max(0.0),
        );
        st.target = st.target.clamp(Vec2::ZERO, max_scroll);
        if dragging {
            st.offset = st.target;
        } else {
            let dt = self.input.dt.max(0.0).min(0.1);
            let t = 1.0 - (-theme::SCROLL_SMOOTH * dt).exp();
            st.offset = st.offset.lerp(st.target, t);
            if (st.offset - st.target).length_squared() > 0.25 {
                self.needs_repaint = true;
            } else {
                st.offset = st.target;
            }
        }
        st.offset = st.offset.clamp(Vec2::ZERO, max_scroll);

        self.push_id(id);
        self.push_clip(view);
        let origin = view.min - st.offset;
        self.layers
            .push(new_layer(LayoutDir::Vertical, origin, self.spacing, view.width(), 0.0));
        add(self);
        let used = self.layers.pop().unwrap().used;
        self.pop_clip();
        self.pop_id();

        st.content = used;

        if self.scroll_wheel_target == Some(widget_id) && !self.scroll_consumed {
            let d = self.input.scroll_delta;
            let view_h = view.height();
            let view_w = view.width();
            if axes.vertical() && d.y.abs() > 0.0 && st.content.y > view_h {
                st.target.y -= d.y;
            }
            if axes.horizontal() && d.x.abs() > 0.0 && st.content.x > view_w {
                st.target.x -= d.x;
            }
            if axes.horizontal()
                && !axes.vertical()
                && d.y.abs() > 0.0
                && st.content.x > view_w
            {
                st.target.x -= d.y;
            }
            if d != Vec2::ZERO {
                self.consume_scroll();
            }
        }

        let need_v = axes.vertical() && st.content.y > view.height() + 0.5;
        let need_h = axes.horizontal() && st.content.x > view.width() + 0.5;
        let view = Rect {
            min: outer.min,
            max: Vec2::new(
                outer.max.x - if need_v { bar + gap } else { 0.0 },
                outer.max.y - if need_h { bar + gap } else { 0.0 },
            ),
        };

        let max_scroll = Vec2::new(
            (st.content.x - view.width()).max(0.0),
            (st.content.y - view.height()).max(0.0),
        );
        st.target = st.target.clamp(Vec2::ZERO, max_scroll);
        st.offset = st.offset.clamp(Vec2::ZERO, max_scroll);

        draw_bars(self, view, outer, &st, axes, need_v, need_h, v_id, h_id, bar, gap);

        self.scrolls.insert(widget_id, st);
    }
}

fn thumb_len(view: f32, content: f32, min: f32) -> f32 {
    if content <= view {
        view
    } else {
        (view * view / content).max(min).min(view)
    }
}

fn interact_bars(
    ui: &mut Ui,
    st: &mut ScrollState,
    view: Rect,
    axes: ScrollAxes,
    need_v: bool,
    need_h: bool,
    v_id: Id,
    h_id: Id,
    bar: f32,
    gap: f32,
) -> bool {
    let mut dragging = false;
    if need_v && axes.vertical() {
        let track = Rect {
            min: Vec2::new(view.max.x + gap, view.min.y),
            max: Vec2::new(view.max.x + gap + bar, view.max.y),
        };
        let th = thumb_len(view.height(), st.content.y, ui.s(theme::SCROLL_THUMB_MIN));
        let travel = (view.height() - th).max(0.0);
        let max_s = (st.content.y - view.height()).max(0.0);
        let ty = if max_s > 0.0 && travel > 0.0 {
            view.min.y + st.offset.y / max_s * travel
        } else {
            view.min.y
        };
        let thumb = Rect::from_min_size(Vec2::new(track.min.x, ty), Vec2::new(bar, th));

        let hot = ui.hovered_rect(thumb) || ui.hovered_rect(track);
        if hot {
            ui.want_capture = true;
            ui.set_cursor(CursorIcon::Pointer);
        }
        if ui.hovered_rect(thumb) && ui.input.mouse_pressed {
            ui.active_id = Some(v_id);
            ui.drag_grab = Some(ui.input.mouse_pos - Vec2::new(0.0, ty));
        }
        if ui.hovered_rect(track) && !ui.hovered_rect(thumb) && ui.input.mouse_pressed {
            let t = ((ui.input.mouse_pos.y - view.min.y - th * 0.5) / travel.max(1.0)).clamp(0.0, 1.0);
            st.target.y = t * max_s;
            st.offset.y = st.target.y;
            ui.active_id = Some(v_id);
            ui.drag_grab = Some(Vec2::new(0.0, th * 0.5));
            dragging = true;
        }
        if ui.active_id == Some(v_id) && ui.input.mouse_down {
            let grab = ui.drag_grab.unwrap_or(Vec2::new(0.0, th * 0.5)).y;
            let t = ((ui.input.mouse_pos.y - view.min.y - grab) / travel.max(1.0)).clamp(0.0, 1.0);
            st.target.y = t * max_s;
            st.offset.y = st.target.y;
            ui.want_capture = true;
            ui.set_cursor(CursorIcon::Pointer);
            dragging = true;
        }
    }

    if need_h && axes.horizontal() {
        let track = Rect {
            min: Vec2::new(view.min.x, view.max.y + gap),
            max: Vec2::new(view.max.x, view.max.y + gap + bar),
        };
        let tw = thumb_len(view.width(), st.content.x, ui.s(theme::SCROLL_THUMB_MIN));
        let travel = (view.width() - tw).max(0.0);
        let max_s = (st.content.x - view.width()).max(0.0);
        let tx = if max_s > 0.0 && travel > 0.0 {
            view.min.x + st.offset.x / max_s * travel
        } else {
            view.min.x
        };
        let thumb = Rect::from_min_size(Vec2::new(tx, track.min.y), Vec2::new(tw, bar));

        let hot = ui.hovered_rect(thumb) || ui.hovered_rect(track);
        if hot {
            ui.want_capture = true;
            ui.set_cursor(CursorIcon::Pointer);
        }
        if ui.hovered_rect(thumb) && ui.input.mouse_pressed {
            ui.active_id = Some(h_id);
            ui.drag_grab = Some(ui.input.mouse_pos - Vec2::new(tx, 0.0));
        }
        if ui.hovered_rect(track) && !ui.hovered_rect(thumb) && ui.input.mouse_pressed {
            let t = ((ui.input.mouse_pos.x - view.min.x - tw * 0.5) / travel.max(1.0)).clamp(0.0, 1.0);
            st.target.x = t * max_s;
            st.offset.x = st.target.x;
            ui.active_id = Some(h_id);
            ui.drag_grab = Some(Vec2::new(tw * 0.5, 0.0));
            dragging = true;
        }
        if ui.active_id == Some(h_id) && ui.input.mouse_down {
            let grab = ui.drag_grab.unwrap_or(Vec2::new(tw * 0.5, 0.0)).x;
            let t = ((ui.input.mouse_pos.x - view.min.x - grab) / travel.max(1.0)).clamp(0.0, 1.0);
            st.target.x = t * max_s;
            st.offset.x = st.target.x;
            ui.want_capture = true;
            ui.set_cursor(CursorIcon::Pointer);
            dragging = true;
        }
    }
    dragging
}

fn draw_bars(
    ui: &mut Ui,
    view: Rect,
    outer: Rect,
    st: &ScrollState,
    axes: ScrollAxes,
    need_v: bool,
    need_h: bool,
    v_id: Id,
    h_id: Id,
    bar: f32,
    gap: f32,
) {
    if need_v && axes.vertical() {
        let track = Rect {
            min: Vec2::new(view.max.x + gap, view.min.y),
            max: Vec2::new(view.max.x + gap + bar, view.max.y),
        };
        ui.round_rect(track, 0.0, theme::SCROLL_BG);
        let th = thumb_len(view.height(), st.content.y, ui.s(theme::SCROLL_THUMB_MIN));
        let travel = (view.height() - th).max(0.0);
        let max_s = (st.content.y - view.height()).max(0.0);
        let ty = if max_s > 0.0 && travel > 0.0 {
            view.min.y + st.offset.y / max_s * travel
        } else {
            view.min.y
        };
        let thumb = Rect::from_min_size(Vec2::new(track.min.x + 1.0, ty), Vec2::new(bar - 2.0, th));
        let hot = ui.active_id == Some(v_id) || ui.hovered_rect(thumb);
        ui.round_rect(
            thumb,
            3.0,
            if hot {
                theme::SCROLL_THUMB_HOT
            } else {
                theme::SCROLL_THUMB
            },
        );
    }

    if need_h && axes.horizontal() {
        let track = Rect {
            min: Vec2::new(view.min.x, view.max.y + gap),
            max: Vec2::new(view.max.x, view.max.y + gap + bar),
        };
        ui.round_rect(track, 0.0, theme::SCROLL_BG);
        let tw = thumb_len(view.width(), st.content.x, ui.s(theme::SCROLL_THUMB_MIN));
        let travel = (view.width() - tw).max(0.0);
        let max_s = (st.content.x - view.width()).max(0.0);
        let tx = if max_s > 0.0 && travel > 0.0 {
            view.min.x + st.offset.x / max_s * travel
        } else {
            view.min.x
        };
        let thumb = Rect::from_min_size(Vec2::new(tx, track.min.y + 1.0), Vec2::new(tw, bar - 2.0));
        let hot = ui.active_id == Some(h_id) || ui.hovered_rect(thumb);
        ui.round_rect(
            thumb,
            3.0,
            if hot {
                theme::SCROLL_THUMB_HOT
            } else {
                theme::SCROLL_THUMB
            },
        );
    }

    if need_v && need_h {
        let corner = Rect {
            min: Vec2::new(view.max.x + gap, view.max.y + gap),
            max: outer.max,
        };
        ui.round_rect(corner, 0.0, theme::SCROLL_BG);
    }
}
