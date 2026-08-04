//! Toast / notification stack (top-right).

use glam::Vec2;

use crate::theme;
use crate::types::Rect;
use crate::Ui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warn,
    Error,
}

#[derive(Clone)]
pub(crate) struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub age: f32,
}

impl ToastKind {
    fn color(self) -> [f32; 4] {
        match self {
            Self::Info => theme::TOAST_INFO,
            Self::Success => theme::TOAST_SUCCESS,
            Self::Warn => theme::TOAST_WARN,
            Self::Error => theme::TOAST_ERROR,
        }
    }
}

impl Ui {
    pub fn notify(&mut self, text: impl Into<String>) {
        self.push_toast(text.into(), ToastKind::Info);
    }

    pub fn notify_success(&mut self, text: impl Into<String>) {
        self.push_toast(text.into(), ToastKind::Success);
    }

    pub fn notify_warn(&mut self, text: impl Into<String>) {
        self.push_toast(text.into(), ToastKind::Warn);
    }

    pub fn notify_error(&mut self, text: impl Into<String>) {
        self.push_toast(text.into(), ToastKind::Error);
    }

    fn push_toast(&mut self, text: String, kind: ToastKind) {
        self.toasts.push(Toast {
            text,
            kind,
            age: 0.0,
        });
        // Cap stack
        if self.toasts.len() > 6 {
            self.toasts.remove(0);
        }
        self.needs_repaint = true;
    }

    pub(crate) fn tick_toasts(&mut self) {
        let dt = self.input.dt;
        for t in &mut self.toasts {
            t.age += dt;
        }
        let life = theme::TOAST_LIFETIME;
        self.toasts.retain(|t| t.age < life);
        if !self.toasts.is_empty() {
            self.needs_repaint = true;
        }
    }

    pub(crate) fn draw_toasts(&mut self) {
        if self.toasts.is_empty() {
            return;
        }
        let pad = self.s(12.0);
        let gap = self.s(8.0);
        let width = self.s(280.0);
        let mut y = pad;
        let vp = self.input.viewport;
        let life = theme::TOAST_LIFETIME;

        // Clone texts to avoid borrow issues while drawing
        let items: Vec<_> = self.toasts.clone();
        for t in items.iter().rev() {
            let fade = if t.age > life - 0.45 {
                ((life - t.age) / 0.45).clamp(0.0, 1.0)
            } else if t.age < 0.15 {
                (t.age / 0.15).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let th = self.text_height();
            let h = th + self.s(16.0);
            let x = vp.x - width - pad;
            let rect = Rect::from_min_size(Vec2::new(x, y), Vec2::new(width, h));
            let mut bg = theme::TOAST_BG;
            bg[3] *= fade;
            let mut accent = t.kind.color();
            accent[3] *= fade;
            let mut text_c = theme::TEXT;
            text_c[3] *= fade;

            let radius = self.s(5.0);
            self.round_rect_overlay(rect, radius, theme::BTN_BORDER);
            self.round_rect_overlay(rect.inset(1.0), (radius - 1.0).max(0.0), bg);
            let strip = Rect {
                min: rect.min + Vec2::new(1.0, 1.0),
                max: Vec2::new(rect.min.x + self.s(4.0), rect.max.y - 1.0),
            };
            self.round_rect_overlay(strip, 2.0, accent);
            self.text_overlay(
                Vec2::new(rect.min.x + self.s(12.0), rect.min.y + (h - th) * 0.5),
                &t.text,
                text_c,
            );
            y += h + gap;
        }
    }
}
