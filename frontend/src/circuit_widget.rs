use std::sync::Arc;

use crate::camera::*;
use cirmcut_sim::{CellPos, Component, ComponentState, Diagram, DiagramCell, DiagramState};
use egui::{Align2, Color32, Painter, PointerButton, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

pub fn circuit_widget(
    diagram: &mut Diagram,
    selection: &mut CellPos,
    state: &DiagramState,
    ui: &mut egui::Ui,
    desired_size: Vec2,
    id: egui::Id,
) -> egui::Response {
    let resp = ui.allocate_response(desired_size, Sense::click_and_drag());

    // Handle scrolling
    let scroll_speed = -3e-3;
    let zoom_delta = ui.input(|r| r.zoom_delta() * (1.0 - r.smooth_scroll_delta.y * scroll_speed));

    // Handle pinching or dragging
    let pointer_pos = ui.input(|r| r.pointer.latest_pos().unwrap_or(Pos2::ZERO));

    let zoom_pivot = ui.input(|r| match r.multi_touch() {
        Some(mt) => mt.center_pos,
        None => pointer_pos,
    });

    // Use the response to drive the camera
    let mut camera =
        ui.memory_mut(|mem| *mem.data.get_temp_mut_or_default::<CircuitWidgetCamera>(id));
    let transf = camera.drive(&resp, zoom_delta, zoom_pivot);
    ui.memory_mut(|mem| *mem.data.get_temp_mut_or_default(id) = camera);

    let painter = ui.painter_at(transf.area);

    let ((min_x, min_y), (max_x, max_y)) = transf.visible_rect();

    // Draw visible circuit elements
    let mut n = 0;
    'outer: for y in min_y..=max_y + 1 {
        for x in min_x..=max_x + 1 {
            n += 1;
            if n > 1_000_000 {
                break 'outer;
            }

            let pos = (x, y);
            let tl = transf.sim_to_egui(pos);

            // Draw a little dot at the corner of each visible space
            painter.circle_filled(tl, transf.camera.zoom / 50., Color32::LIGHT_GRAY);
            //painter.text(tl, Align2::CENTER_CENTER, format!("{x},{y}"), Default::default(), Color32::RED);

            // Draw cell
            match (diagram.get_mut(&pos), state.get(&pos)) {
                (Some(cell), Some(state)) => {
                    draw_component(tl, transf.camera.zoom, cell, state, &painter)
                }
                _ => (),
            }
        }
    }

    // Selection
    if resp.clicked() {
        *selection = transf.egui_to_sim_cellpos(resp.interact_pointer_pos().unwrap_or_default());
    }

    let tl = transf.sim_to_egui(*selection);
    let rect = Rect::from_min_size(tl, Vec2::splat(transf.camera.zoom));
    painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(1., Color32::RED));

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
