use egui::{Color32, Painter, Pos2};

use crate::circuit_widget::DiagramWireState;

pub fn draw_transistor(
    painter: &Painter,
    pos: [Pos2; 3],
    wires: [DiagramWireState; 3],
    selected: bool,
    p_type: bool,
) {
    let [a, b, c] = pos;
    let ctr = ((a.to_vec2() + b.to_vec2() + c.to_vec2()) / 3.0).to_pos2();

    wires[0].draw(painter, a, ctr, selected);
    wires[1].draw(painter, b, ctr, selected);
    wires[2].draw(painter, c, ctr, selected);
}

pub fn draw_wire(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    selected: bool,
) {
    let [a, b] = pos;
    wires[0].draw(painter, a, b, selected);
}
