use std::f32::consts::{PI, TAU};

use cirmcut_sim::TwoTerminalComponent;
use egui::{emath::TSTransform, Align2, Color32, Painter, Pos2, Shape, Stroke, Vec2};

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

    let (begin_segment, end_segment, y) = center_cell_segment(begin, end, CELL_SIZE);

    let y = y * CELL_SIZE;
    let x = y.rot90();

    begin_wire.line_segment(painter, begin, begin_segment, selected);
    end_wire.line_segment(painter, end_segment, end, selected);

    let wiggles = 6;

    let mut amplitude = 0.095;

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

fn center_cell_segment(a: Pos2, b: Pos2, len: f32) -> (Pos2, Pos2, Vec2) {
    let diff = b - a;
    let remain = (diff.length() - len).max(0.0);
    let translate = remain / 2.0;
    let n = diff.normalized();
    (a + n * translate, a + n * (translate + len), n)
}

pub fn draw_inductor(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    selected: bool,
) {
    let [begin, end] = pos;
    let [begin_wire, end_wire] = wires;

    let (begin_segment, end_segment, y) = center_cell_segment(begin, end, CELL_SIZE);

    let y = y * CELL_SIZE;
    let x = y.rot90();

    begin_wire.line_segment(painter, begin, begin_segment, selected);
    end_wire.line_segment(painter, end_segment, end, selected);

    let steps = 100;

    let mut last = begin_segment;
    for i in 0..=steps {
        let f = i as f32 / steps as f32;

        let n_loops = 5;
        let t = f * TAU * n_loops as f32;

        let k: f32 = 7.44;
        let a = 0.12;

        let xf = t.sin() / 10.0;
        let yf = (((t.cos() - 1.0) * k.cos()) + t * a) / (TAU * n_loops as f32 * a);

        let new_pos = begin_segment + x * xf + y * yf;
        begin_wire
            .lerp_voltage(&end_wire, f)
            .line_segment(painter, last, new_pos, selected);

        last = new_pos;
    }

    begin_wire.current(painter, begin, end);
}

fn draw_capacitorlike(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    selected: bool,
    plate_a: f32,
    plate_b: f32,
) {
    let [begin, end] = pos;
    let [begin_wire, end_wire] = wires;

    let sep = 0.1 * CELL_SIZE;
    let (begin_segment, end_segment, y) = center_cell_segment(begin, end, sep);

    let y = y * CELL_SIZE;
    let x = y.rot90();

    begin_wire.line_segment(painter, begin, begin_segment, selected);
    end_wire.line_segment(painter, end_segment, end, selected);

    begin_wire.line_segment(
        painter,
        begin_segment - x * plate_a,
        begin_segment + x * plate_a,
        selected,
    );

    end_wire.line_segment(
        painter,
        end_segment - x * plate_b,
        end_segment + x * plate_b,
        selected,
    );

    begin_wire.current(painter, begin, end);
}

pub fn draw_capacitor(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    selected: bool,
) {
    let radius = 0.2;
    draw_capacitorlike(painter, pos, wires, selected, radius, radius);
}

pub fn draw_battery(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    selected: bool,
) {
    draw_capacitorlike(painter, pos, wires, selected, 0.3, 0.15);
}

pub fn draw_diode(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    selected: bool,
) {
    let [begin, end] = pos;
    let [begin_wire, end_wire] = wires;

    let size = 0.2;

    let sep = size * 2.0 * CELL_SIZE;
    let (begin_segment, end_segment, y) = center_cell_segment(begin, end, sep);

    let y = y * CELL_SIZE;
    let x = y.rot90();

    begin_wire.line_segment(painter, begin, begin_segment, selected);
    end_wire.line_segment(painter, end_segment, end, selected);

    let plate_radius = size;

    begin_wire.line_segment(
        painter,
        begin_segment - x * plate_radius,
        begin_segment + x * plate_radius,
        selected,
    );

    painter.add(Shape::convex_polygon(vec![begin_segment, end_segment + x * plate_radius, end_segment - x * plate_radius], end_wire.color(selected), 

    Stroke::NONE));

    begin_wire.current(painter, begin, end);
}

pub fn draw_switch(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    selected: bool,
    is_open: bool,
) {
    let [begin, end] = pos;
    let [begin_wire, end_wire] = wires;

    let (begin_segment, end_segment, y) = center_cell_segment(begin, end, CELL_SIZE);

    let y = y * CELL_SIZE;
    let x = y.rot90();

    begin_wire.line_segment(painter, begin, begin_segment, selected);
    end_wire.line_segment(painter, end_segment, end, selected);

    let rot = if is_open {
        PI / 6.
    } else {
        0.0
    };

    let contact = 
        x * rot.sin() + y * rot.cos();

    painter.line_segment([begin_segment, begin_segment + contact], Stroke::new(5., Color32::WHITE));

    begin_wire.current(painter, begin, end);
}

pub fn draw_component_value(painter: &Painter, pos: [Pos2; 2], component: TwoTerminalComponent) {
    if let Some(text) = format_component_value(component) {
        let diff = pos[1] - pos[0];
        let y = diff.normalized() * CELL_SIZE;
        let x = y.rot90();

        let midpt = (pos[0] + pos[1].to_vec2()) / 2.0;

        let pos = midpt + x * 0.25;

        painter.text(pos, Align2::CENTER_CENTER, text, Default::default(), Color32::WHITE);
    }
}

fn format_component_value(component: TwoTerminalComponent) -> Option<String> {
    match component {
        TwoTerminalComponent::Battery(v) => Some(to_metric_prefix(v, 'V')),
        TwoTerminalComponent::Capacitor(c) => Some(to_metric_prefix(c, 'F')),
        TwoTerminalComponent::Inductor(i) => Some(to_metric_prefix(i, 'H')),
        TwoTerminalComponent::Resistor(r) => Some(to_metric_prefix(r, 'Ω')),
        _ => None,
    }
}

// WARNING: Chatgpt did this lol
fn to_metric_prefix(value: f32, unit: char) -> String {
    let prefixes = [
        (-24, "y"), (-21, "z"), (-18, "a"), (-15, "f"), (-12, "p"), (-9, "n"), (-6, "μ"), (-3, "m"),
        (0, ""), (3, "k"), (6, "M"), (9, "G"), (12, "T"), (15, "P"), (18, "E"), (21, "Z"), (24, "Y")
    ];
    
    if value == 0.0 {
        return "0".to_string();
    }
    
    let exponent = (value.abs().log10() / 3.0).floor() as i32 * 3;
    let prefix = prefixes.iter().find(|&&(e, _)| e == exponent);
    
    if let Some((e, symbol)) = prefix {
        format!("{} {}{unit}", value / 10_f32.powi(*e), symbol)
    } else {
        format!("{} {unit}", value) // Fallback in case exponent is out of range
    }
}
