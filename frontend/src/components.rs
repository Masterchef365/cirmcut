use egui::{emath::TSTransform, Color32, Painter, Pos2, Vec2};

use crate::circuit_widget::{DiagramWireState, CELL_SIZE};

pub fn draw_transistor(
    painter: &Painter,
    pos: [Pos2; 3],
    wires: [DiagramWireState; 3],
    selected: bool,
    p_type: bool,
) {
    let [emitter_in, base_in, collector_in] = pos;
    let [emitter_wire, base_wire, collector_wire] = wires;

    let orient = (base_in - (emitter_in + collector_in.to_vec2()) / 2.0).normalized() * CELL_SIZE;
    let center = (emitter_in + base_in.to_vec2() + collector_in.to_vec2()) / 3.0;

    let orient_x = orient.rot90();
    let orient_y = orient;

    let base_input_tap = center + orient_y * 0.125;
    let junction_radius = 0.25;

    base_wire.draw(painter, base_in, base_input_tap, selected);
    base_wire.floating().draw(
        painter,
        base_input_tap - orient_x * junction_radius,
        base_input_tap + orient_x * junction_radius,
        selected,
    );

    let conn_radius = 0.125;

    let emitter_input_tap = center + (-orient_x) * 0.125;
    let collector_input_tap = center + (orient_x) * 0.125;

    base_wire.draw(
        painter,
        emitter_input_tap,
        base_input_tap - orient_x * conn_radius,
        selected,
    );

    base_wire.draw(
        painter,
        collector_input_tap,
        base_input_tap + orient_x * conn_radius,
        selected,
    );

    emitter_wire.draw(painter, emitter_in, emitter_input_tap, selected);
    emitter_wire.draw(painter, collector_in, collector_input_tap, selected);

    /*
    let orient =

    let ctr = ((a.to_vec2() + b.to_vec2() + c.to_vec2()) / 3.0).to_pos2();

    wires[0].draw(painter, a, ctr, selected);
    wires[1].draw(painter, b, ctr, selected);
    wires[2].draw(painter, c, ctr, selected);
    */
}

pub fn draw_wire(painter: &Painter, pos: [Pos2; 2], wires: [DiagramWireState; 2], selected: bool) {
    let [a, b] = pos;
    wires[0].draw(painter, a, b, selected);
}

impl DiagramWireState {
    /// Zeroes current
    fn floating(self) -> Self {
        Self {
            voltage: self.voltage,
            current: 0.0,
        }
    }
}
