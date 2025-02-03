use cirmcut_sim::{CellPos, Component, ComponentState, Diagram, DiagramCell, DiagramState};
use egui::{Align2, Color32, Painter, PointerButton, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

#[derive(Copy, Clone, Debug)]
pub struct CircuitWidgetCamera {
    /// Egui units per cell
    pub zoom: f32,
    /// Position in cells (fractional)
    pub pos: Pos2,
}

impl Default for CircuitWidgetCamera {
    fn default() -> Self {
        Self {
            zoom: 50.0,
            pos: Pos2::ZERO,
        }
    }
}

pub struct CircuitWidgetCameraTransformation {
    /// Screen area in egui
    pub area: Rect,
    pub camera: CircuitWidgetCamera,
}

impl CircuitWidgetCamera {
    pub fn drive(
        &mut self,
        resp: &egui::Response,
        zoom_delta: f32,
        pivot: Pos2,
    ) -> CircuitWidgetCameraTransformation {
        let old_zoom = self.zoom;
        self.zoom *= zoom_delta;
        self.zoom = self.zoom.clamp(10.0, 300.0);
        let zoom_delta = self.zoom / old_zoom;

        let area = resp.interact_rect;

        let pivot_vect = (1. - zoom_delta) * (pivot - area.center());
        self.pos -= pivot_vect / self.zoom;

        if resp.dragged() {
            self.pos -= resp.drag_delta() / self.zoom;
        }

        if resp.clicked_by(PointerButton::Secondary) {
            *self = Self::default();
        }

        let scroll = resp.ctx.input(|r| r.raw_scroll_delta.x);
        if resp.ctx.input(|m| m.modifiers.shift) {
            if resp.ctx.input(|m| m.modifiers.alt) {
                self.pos.y -= scroll / self.zoom;
            } else {
                self.pos.x -= scroll / self.zoom;
            }
        }

        CircuitWidgetCameraTransformation {
            area,
            camera: *self,
        }
    }
}

impl CircuitWidgetCameraTransformation {
    pub fn screen_center_offset(&self) -> Vec2 {
        self.area.center().to_vec2()
    }

    pub fn sim_to_egui(&self, sim_pos: CellPos) -> egui::Pos2 {
        let (x, y) = sim_pos;
        let sim_pos = Vec2::new(x as f32, y as f32);
        let zoomed = self.camera.zoom * (sim_pos - self.camera.pos.to_vec2());

        zoomed.to_pos2() + self.screen_center_offset()
    }

    pub fn egui_to_sim(&self, egui_pos: egui::Pos2) -> egui::Pos2 {
        let zoomed = egui_pos - self.screen_center_offset();

        self.camera.pos + (zoomed.to_vec2() / self.camera.zoom)
    }

    pub fn egui_to_sim_cellpos(&self, egui_pos: egui::Pos2) -> CellPos {
        let pos = self.egui_to_sim(egui_pos);
        (pos.x.floor() as i32, pos.y.floor() as i32)
    }

    pub fn visible_rect(&self) -> (CellPos, CellPos) {
        (
            self.egui_to_sim_cellpos(self.area.min),
            self.egui_to_sim_cellpos(self.area.max),
        )
    }
}
