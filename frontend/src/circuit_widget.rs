use std::sync::Arc;

use cirmcut_sim::{CellPos, Component, ComponentState, Diagram, DiagramCell, DiagramState};
use egui::{Align2, Color32, Painter, PointerButton, Pos2, Rect, Sense, Vec2};

pub fn circuit_widget(
    diagram: &mut Diagram,
    state: &DiagramState,
    ui: &mut egui::Ui,
    desired_size: Vec2,
    id: egui::Id,
) -> egui::Response {
    let resp = ui.allocate_response(desired_size, Sense::click_and_drag());

    let scroll_speed = 1e-2;
    let zoom_delta = ui.input(|r| r.zoom_delta() * (1.0 - r.smooth_scroll_delta.y * scroll_speed));

    let hover = resp.hover_pos().unwrap_or(Pos2::ZERO);
    let pivot = ui.input(|r| match r.multi_touch() {
        Some(mt) => mt.center_pos,
        None => hover,
    });

    // Use the response to drive the camera
    let mut camera = ui.memory_mut(|mem| {
        *mem.data.get_temp_mut_or_default::<CircuitWidgetCamera>(id)
    });
    let transf = camera.drive(&resp, zoom_delta, pivot);
    ui.memory_mut(|mem| *mem.data.get_temp_mut_or_default(id) = camera);

    let painter = ui.painter_at(transf.area);

    let ((min_x, min_y), (max_x, max_y)) = transf.visible_rect();

    let mut n = 0;
    'outer: for y in min_y..=max_y {
        for x in min_x..=max_x {
            n += 1;
            if n > 1_000_000 {
                break 'outer;
            }

            let pos = (x, y);
            let tl = transf.sim_to_egui(pos);

            // Draw a little dot at the corner of each visible space
            painter.circle_filled(tl, transf.camera.zoom/10., Color32::LIGHT_GRAY);
            painter.text(tl, Align2::CENTER_CENTER, format!("{x},{y}"), Default::default(), Color32::RED);

            // Draw cell
            match (diagram.get_mut(&pos), state.get(&pos)) {
                (Some(cell), Some(state)) => {
                    draw_component(tl, transf.camera.zoom, cell, state, &painter)
                }
                _ => (),
            }
        }
    }
    dbg!(n);

    resp
}

fn draw_component(
    tl: Pos2,
    zoom: f32,
    cell: &mut DiagramCell,
    state: &ComponentState,
    painter: &Painter,
) {
}

#[derive(Copy, Clone, Debug)]
struct CircuitWidgetCamera {
    /// Egui units per cell
    zoom: f32,
    /// Position in cells (fractional)
    pos: Pos2,
}

impl Default for CircuitWidgetCamera {
    fn default() -> Self {
        Self {
            zoom: 100.0,
            pos: Pos2::ZERO,
        }
    }
}

struct CircuitWidgetCameraTransformation {
    /// Screen area in egui
    area: Rect,
    camera: CircuitWidgetCamera,
}

impl CircuitWidgetCamera {
    fn drive(&mut self, resp: &egui::Response, zoom_delta: f32, pivot: Pos2) -> CircuitWidgetCameraTransformation {
        self.zoom *= zoom_delta;
        self.zoom = self.zoom.clamp(2.0, 300.0);

        let area = resp.interact_rect;

        let pivot_vect = (1. - zoom_delta) * (pivot - area.center());

        if resp.dragged() {
            self.pos -= resp.drag_delta() / self.zoom;
        }

        CircuitWidgetCameraTransformation {
            area,
            camera: *self,
        }
    }
}

impl CircuitWidgetCameraTransformation {
    fn screen_center_offset(&self) -> Vec2 {
        self.area.center().to_vec2()
    }

    fn sim_to_egui(&self, sim_pos: CellPos) -> egui::Pos2 {
        let (x, y) = sim_pos;
        let sim_pos = Vec2::new(x as f32, y as f32);
        let zoomed = self.camera.zoom * (sim_pos - self.camera.pos.to_vec2());

        zoomed.to_pos2() + self.screen_center_offset()
    }

    fn egui_to_sim(&self, egui_pos: egui::Pos2) -> egui::Pos2 {
        let zoomed = egui_pos - self.screen_center_offset();

        self.camera.pos + (zoomed.to_vec2() / self.camera.zoom)
    }

    fn egui_to_sim_cellpos(&self, egui_pos: egui::Pos2) -> CellPos {
        let pos = self.egui_to_sim(egui_pos);
        (pos.x.floor() as i32, pos.y.floor() as i32)
    }

    fn visible_rect(&self) -> (CellPos, CellPos) {
        (
            self.egui_to_sim_cellpos(self.area.min),
            self.egui_to_sim_cellpos(self.area.max),
        )
    }
}
