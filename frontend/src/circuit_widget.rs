use std::sync::Arc;

use cirmcut_sim::{CellPos, Component, ComponentState, Diagram, DiagramCell, DiagramState};
use egui::{Align2, Color32, Painter, PointerButton, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

const CELL_SIZE: f32 = 100.0;

fn cellpos_to_egui((x, y): CellPos) -> Pos2 {
    Pos2::new(x as f32, y as f32) * CELL_SIZE
}

fn egui_to_cellpos(pos: Pos2) -> CellPos {
    (
        (pos.x.floor() / CELL_SIZE) as i32,
        (pos.y.floor() / CELL_SIZE) as i32,
    )
}

pub fn circuit_widget(
    diagram: &mut Diagram,
    state: &DiagramState,
    ui: &mut egui::Ui,
    scene_rect: &mut Rect,
) {
    let rect = *scene_rect;
    egui::Scene::new()
        .show(ui, scene_rect, |ui| {
            let (min_x, min_y) = egui_to_cellpos(rect.min.floor());
            let (max_x, max_y) = egui_to_cellpos(rect.max.ceil());

            let default_area = ui.allocate_rect(
                Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0)),
                Sense::HOVER,
            );
            let painter = ui.painter();
            painter.rect_stroke(
                default_area.rect,
                0.0,
                Stroke::new(1.0, Color32::WHITE),
                egui::StrokeKind::Inside,
            );

            dbg!(min_x, max_x);

            // Draw visible circuit elements
            let mut n = 0;
            'outer: for y in min_y..=max_y {
                for x in min_x..=max_x {
                    n += 1;
                    if n > 100_000 {
                        break 'outer;
                    }

                    // Draw a little dot at the corner of each visible space
                    painter.circle_filled(cellpos_to_egui((x, y)), 1.0, Color32::LIGHT_GRAY);
                    //painter.text(tl, Align2::CENTER_CENTER, format!("{x},{y}"), Default::default(), Color32::RED);

                    // Draw cell
                    match (diagram.get_mut(&(x, y)), state.get(&(x, y))) {
                        (Some(cell), Some(state)) => {
                            //draw_component(egui_pos, transf.camera.zoom, cell, state, &painter)
                        }
                        _ => (),
                    }
                }
            }
            dbg!(n);

            /*
            // Selection
            if resp.clicked() {
                *selection = egui_to_cellpos(resp.interact_pointer_pos().unwrap_or_default());
            }

            let tl = transf.sim_to_egui(*selection);
            let rect = Rect::from_min_size(tl, Vec2::splat(transf.camera.zoom));
            painter.rect_stroke(rect, 0.0, Stroke::new(1., Color32::RED), egui::StrokeKind::Inside);

            resp
            */
        })
        .inner
}

fn draw_component(
    tl: Pos2,
    zoom: f32,
    cell: &DiagramCell,
    state: &ComponentState,
    painter: &Painter,
) {
    let rect = Rect::from_min_size(tl, Vec2::splat(zoom));
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("{:?}", cell.comp),
        Default::default(),
        Color32::WHITE,
    );
}

pub struct ComponentButton {
    cell: DiagramCell,
    state: ComponentState,
    size: f32,
}

impl ComponentButton {
    pub fn new(cell: DiagramCell, state: ComponentState, size: f32) -> Self {
        Self { cell, state, size }
    }
}

impl egui::Widget for ComponentButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let resp = ui.allocate_response(Vec2::splat(self.size), Sense::click_and_drag());

        draw_component(
            resp.rect.min,
            self.size,
            &self.cell,
            &self.state,
            ui.painter(),
        );

        resp
    }
}
