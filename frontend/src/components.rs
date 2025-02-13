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

    let base_input_tap = center + orient_y * 0.25;
    let junction_radius = 0.25;

    base_wire.wire(painter, base_in, base_input_tap, selected);
    base_wire.floating().wire(
        painter,
        base_input_tap - orient_x * junction_radius,
        base_input_tap + orient_x * junction_radius,
        selected,
    );

    let conn_radius = 0.10;

    let ty_orient = if p_type { -orient_x } else { orient_x };
    let emitter_input_tap = center + (ty_orient) * 0.25;
    let collector_input_tap = center + (-ty_orient) * 0.25;

    emitter_wire.arrow(
        painter,
        emitter_input_tap,
        base_input_tap + ty_orient * conn_radius,
        selected,
        p_type,
    );

    emitter_wire.wire(painter, emitter_in, emitter_input_tap, selected);

    collector_wire.wire(
        painter,
        collector_input_tap,
        base_input_tap - ty_orient * conn_radius,
        selected,
    );

    collector_wire.wire(painter, collector_in, collector_input_tap, selected);
}

pub fn draw_resistor(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    selected: bool,
) {
    let [begin, end] = pos;
    let [begin_wire, end_wire] = wires;

    let (begin_segment, end_segment, y) = center_cell_segment(begin, end);

    let y = y * CELL_SIZE;
    let x = y.rot90();

    begin_wire.line_segment(painter, begin, begin_segment, selected);
    end_wire.line_segment(painter, end_segment, end, selected);

    let wiggles = 4;

    let mut amplitude = 0.125;

    let mut last = begin_segment;
    for i in 0..=wiggles * 2 {
        amplitude *= -1.0;

        let f = (i as f32) / (wiggles * 2) as f32;

        let new_pos = if i == 0 {
            begin_segment
        } else if i == wiggles * 2 {
            end_segment
        } else {
            begin_segment + y * f + x * amplitude
        };
        begin_wire
            .lerp_voltage(&end_wire, f)
            .line_segment(painter, last, new_pos, selected);

        last = new_pos;
    }

    begin_wire.current(painter, begin, end);
}

fn center_cell_segment(a: Pos2, b: Pos2) -> (Pos2, Pos2, Vec2) {
    let diff = b - a;
    let remain = diff.length() - CELL_SIZE;
    let translate = remain.max(0.0) / 2.0;
    let n = diff.normalized();
    (a + n * translate, a + n * (translate + CELL_SIZE), n)
}
