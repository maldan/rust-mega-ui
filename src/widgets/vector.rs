use glam::{Vec2, Vec3};

use crate::theme;
use crate::types::Response;
use crate::widgets::drag_float::format_float;
use crate::Ui;

impl Ui {
    /// Editable `Vec2` with RGB grips (X/Y), lock (uniform edit), and reset to `default`.
    pub fn vec2(&mut self, id: &str, v: &mut Vec2, step: f32, default: Vec2) -> Response {
        self.push_id(id);
        let lock_key = self.current_id("__lock");
        let mut locked = self.vec_locks.get(&lock_key).copied().unwrap_or(false);
        let mut changed = false;
        let mut hovered = false;

        self.horizontal(|ui| {
            let icon = if locked { "lock" } else { "unlock" };
            if ui.icon_button("lock", icon, locked) {
                locked = !locked;
            }

            let rx = ui.drag_float_grip("x", &mut v.x, step, Some(theme::AXIS_X));
            let ry = ui.drag_float_grip("y", &mut v.y, step, Some(theme::AXIS_Y));
            hovered |= rx.hovered || ry.hovered;

            if locked {
                if rx.changed {
                    v.y = v.x;
                    changed = true;
                } else if ry.changed {
                    v.x = v.y;
                    changed = true;
                }
            } else {
                changed |= rx.changed || ry.changed;
            }

            let is_default =
                (v.x - default.x).abs() < f32::EPSILON && (v.y - default.y).abs() < f32::EPSILON;
            if ui.icon_button("reset", "reset", !is_default) && *v != default {
                *v = default;
                changed = true;
            }
        });

        if changed {
            self.sync_axis_bufs(&[("x", v.x), ("y", v.y)], step);
        }

        self.vec_locks.insert(lock_key, locked);
        self.pop_id();
        Response {
            hovered,
            clicked: false,
            changed,
        }
    }

    /// Editable `Vec3` with RGB grips (X/Y/Z), lock (uniform edit), and reset to `default`.
    pub fn vec3(&mut self, id: &str, v: &mut Vec3, step: f32, default: Vec3) -> Response {
        self.push_id(id);
        let lock_key = self.current_id("__lock");
        let mut locked = self.vec_locks.get(&lock_key).copied().unwrap_or(false);
        let mut changed = false;
        let mut hovered = false;

        self.horizontal(|ui| {
            let icon = if locked { "lock" } else { "unlock" };
            if ui.icon_button("lock", icon, locked) {
                locked = !locked;
            }

            let rx = ui.drag_float_grip("x", &mut v.x, step, Some(theme::AXIS_X));
            let ry = ui.drag_float_grip("y", &mut v.y, step, Some(theme::AXIS_Y));
            let rz = ui.drag_float_grip("z", &mut v.z, step, Some(theme::AXIS_Z));
            hovered |= rx.hovered || ry.hovered || rz.hovered;

            if locked {
                if rx.changed {
                    v.y = v.x;
                    v.z = v.x;
                    changed = true;
                } else if ry.changed {
                    v.x = v.y;
                    v.z = v.y;
                    changed = true;
                } else if rz.changed {
                    v.x = v.z;
                    v.y = v.z;
                    changed = true;
                }
            } else {
                changed |= rx.changed || ry.changed || rz.changed;
            }

            let is_default = (v.x - default.x).abs() < f32::EPSILON
                && (v.y - default.y).abs() < f32::EPSILON
                && (v.z - default.z).abs() < f32::EPSILON;
            if ui.icon_button("reset", "reset", !is_default) && *v != default {
                *v = default;
                changed = true;
            }
        });

        if changed {
            self.sync_axis_bufs(&[("x", v.x), ("y", v.y), ("z", v.z)], step);
        }

        self.vec_locks.insert(lock_key, locked);
        self.pop_id();
        Response {
            hovered,
            clicked: false,
            changed,
        }
    }

    /// Keep focused/edit buffers in sync when lock or reset rewrites sibling axes.
    fn sync_axis_bufs(&mut self, axes: &[(&str, f32)], step: f32) {
        for &(axis, val) in axes {
            let axis_id = self.current_id(axis);
            if let Some(buf) = self.num_bufs.get_mut(&axis_id) {
                *buf = format_float(val, step);
            }
        }
    }
}
