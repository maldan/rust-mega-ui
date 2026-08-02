use glam::{Vec2, Vec3};

use crate::types::Response;
use crate::Ui;

impl Ui {
    pub fn vec2(&mut self, id: &str, v: &mut Vec2, step: f32) -> Response {
        self.push_id(id);
        let mut changed = false;
        self.horizontal(|ui| {
            ui.label("X");
            changed |= ui.drag_float("x", &mut v.x, step).changed;
            ui.label("Y");
            changed |= ui.drag_float("y", &mut v.y, step).changed;
        });
        self.pop_id();
        Response {
            hovered: false,
            clicked: false,
            changed,
        }
    }

    pub fn vec3(&mut self, id: &str, v: &mut Vec3, step: f32) -> Response {
        self.push_id(id);
        let mut changed = false;
        self.horizontal(|ui| {
            ui.label("X");
            changed |= ui.drag_float("x", &mut v.x, step).changed;
            ui.label("Y");
            changed |= ui.drag_float("y", &mut v.y, step).changed;
            ui.label("Z");
            changed |= ui.drag_float("z", &mut v.z, step).changed;
        });
        self.pop_id();
        Response {
            hovered: false,
            clicked: false,
            changed,
        }
    }
}
