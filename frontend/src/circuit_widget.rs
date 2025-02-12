use std::sync::Arc;

use cirmcut_sim::CellPos;
use egui::{Align2, Color32, Painter, PointerButton, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

pub const CELL_SIZE: f32 = 100.0;

pub fn cellpos_to_egui((x, y): CellPos) -> Pos2 {
    Pos2::new(x as f32, y as f32) * CELL_SIZE
}

pub fn cellpos_to_egui_vec((x, y): CellPos) -> Vec2 {
    Vec2::new(x as f32, y as f32) * CELL_SIZE
}

pub fn egui_to_cellpos(pos: Pos2) -> CellPos {
    (
        (pos.x / CELL_SIZE).round() as i32,
        (pos.y / CELL_SIZE).round() as i32,
    )
}

pub fn egui_to_cellvec(v: Vec2) -> CellPos {
    (
        (v.x / CELL_SIZE) as i32,
        (v.y / CELL_SIZE) as i32,
    )
}


pub fn draw_grid(
    ui: &mut egui::Ui,
    rect: Rect,
    radius: f32,
    color: Color32,
) {
    let (min_x, min_y) = egui_to_cellpos(rect.min.floor());
    let (max_x, max_y) = egui_to_cellpos(rect.max.ceil());

    let painter = ui.painter();

    // Draw visible circuit elements
    let mut n = 0;
    const MAX_N: i32 = 100_000;
    'outer: for y in min_y..=max_y {
        for x in min_x..=max_x {
            n += 1;
            if n > MAX_N {
                break 'outer;
            }

            painter.circle_filled(cellpos_to_egui((x, y)), radius, color);
        }
    }
    if n > MAX_N {
        eprintln!("WARNING: zoomed out too far!");
    }

}
