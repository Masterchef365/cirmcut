use egui::{Color32, DragValue, Id, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};
use std::collections::HashMap;

use cirmcut_sim::{CellPos, PrimitiveDiagram, ThreeTerminalComponent, TwoTerminalComponent};

use crate::{
    components::{
        draw_battery, draw_capacitor, draw_component_value, draw_current_source, draw_diode,
        draw_inductor, draw_resistor, draw_switch, draw_transistor,
    },
    to_metric_prefix,
};

pub const CELL_SIZE: f32 = 100.0;

#[derive(Copy, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct VisualizationOptions {
    /// Volts
    pub voltage_scale: f64,
    /// Amps
    pub current_scale: f64,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Diagram {
    pub two_terminal: Vec<([CellPos; 2], TwoTerminalComponent)>,
    pub three_terminal: Vec<([CellPos; 3], ThreeTerminalComponent)>,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct DiagramState {
    pub two_terminal: Vec<[DiagramWireState; 2]>,
    pub three_terminal: Vec<[DiagramWireState; 3]>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub struct DiagramWireState {
    pub voltage: f64,
    pub current: f64,
}

impl Default for DiagramWireState {
    fn default() -> Self {
        Self {
            voltage: 5.0,
            current: 1e-3,
        }
    }
}

pub type Selection = (usize, bool);

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DiagramEditor {
    selected: Option<Selection>,
}

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
    ((v.x / CELL_SIZE) as i32, (v.y / CELL_SIZE) as i32)
}

impl Diagram {
    pub fn junctions(&self) -> Vec<CellPos> {
        let mut junctions = HashMap::<CellPos, u32>::new();
        for (positions, _) in &self.two_terminal {
            for &pos in positions {
                *junctions.entry(pos).or_default() += 1;
            }
        }
        for (positions, _) in &self.three_terminal {
            for &pos in positions {
                *junctions.entry(pos).or_default() += 1;
            }
        }
        junctions
            .into_iter()
            .filter_map(|(pos, count)| (count > 1).then_some(pos))
            .collect()
    }

    pub fn to_primitive_diagram(&self) -> PrimitiveDiagram {
        let mut all_positions: HashMap<CellPos, usize> = HashMap::new();

        for (positions, _) in &self.two_terminal {
            for pos in positions {
                let idx = all_positions.len();
                if !all_positions.contains_key(&pos) {
                    all_positions.insert(*pos, idx);
                }
            }
        }

        for (positions, _) in &self.three_terminal {
            for pos in positions {
                let idx = all_positions.len();
                if !all_positions.contains_key(&pos) {
                    all_positions.insert(*pos, idx);
                }
            }
        }

        let two_terminal = self
            .two_terminal
            .iter()
            .map(|(positions, component)| (positions.map(|pos| all_positions[&pos]), *component))
            .collect();

        let three_terminal = self
            .three_terminal
            .iter()
            .map(|(positions, component)| (positions.map(|pos| all_positions[&pos]), *component))
            .collect();

        PrimitiveDiagram {
            num_nodes: all_positions.len(),
            two_terminal,
            three_terminal,
        }
    }
}

pub fn draw_grid(ui: &mut egui::Ui, rect: Rect, radius: f32, color: Color32) {
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

impl DiagramEditor {
    pub fn new() -> Self {
        Self { selected: None }
    }

    pub fn delete(&mut self, diagram: &mut Diagram) {
        if let Some((idx, three)) = self.selected.take() {
            if three {
                diagram.three_terminal.remove(idx);
            } else {
                diagram.two_terminal.remove(idx);
            }
        }
    }

    pub fn new_threeterminal(
        &mut self,
        diagram: &mut Diagram,
        pos: CellPos,
        component: ThreeTerminalComponent,
    ) {
        let (x, y) = pos;
        self.selected = Some((diagram.two_terminal.len(), true));
        diagram
            .three_terminal
            .push(([pos, (x + 1, y + 1), (x + 1, y)], component));
    }

    pub fn new_twoterminal(
        &mut self,
        diagram: &mut Diagram,
        pos: CellPos,
        component: TwoTerminalComponent,
    ) {
        let (x, y) = pos;
        self.selected = Some((diagram.two_terminal.len(), false));
        diagram.two_terminal.push(([pos, (x + 1, y)], component));
    }

    pub fn reset_selection(&mut self) {
        self.selected = None;
    }

    pub fn selection(&self) -> Option<Selection> {
        self.selected
    }

    pub fn edit(
        &mut self,
        ui: &mut Ui,
        diagram: &mut Diagram,
        state: &DiagramState,
        debug_draw: bool,
        vis: &VisualizationOptions,
    ) -> bool {
        let mut two_body_responses = vec![];
        let mut three_body_responses = vec![];

        let mut destructive_change = false;
        let mut new_selection = None;

        for (idx, (pos, comp)) in diagram.two_terminal.iter_mut().enumerate() {
            let ret = interact_with_twoterminal_body(
                ui,
                *pos,
                Id::new("body").with(idx),
                self.selected == Some((idx, false)),
            );
            if ret.clicked() {
                new_selection = Some((idx, false));
            }
            two_body_responses.push(ret);
        }

        for (idx, (pos, _)) in diagram.three_terminal.iter_mut().enumerate() {
            let ret = interact_with_threeterminal_body(
                ui,
                *pos,
                Id::new("threebody").with(idx),
                self.selected == Some((idx, true)),
                vis,
            );
            if ret.clicked() {
                new_selection = Some((idx, true));
            }
            three_body_responses.push(ret);
        }

        for (idx, ((resp, (pos, comp)), wires)) in two_body_responses
            .drain(..)
            .zip(diagram.two_terminal.iter_mut())
            .zip(state.two_terminal.iter())
            .enumerate()
        {
            if interact_with_twoterminal(
                ui,
                pos,
                comp,
                *wires,
                resp,
                self.selected == Some((idx, false)),
                debug_draw,
                vis,
            ) {
                destructive_change = true;
            }
        }

        for (idx, ((resp, (pos, comp)), wires)) in three_body_responses
            .drain(..)
            .zip(diagram.three_terminal.iter_mut())
            .zip(state.three_terminal.iter())
            .enumerate()
        {
            if interact_with_threeterminal(
                ui,
                pos,
                *comp,
                *wires,
                resp,
                self.selected == Some((idx, true)),
                vis,
            ) {
                destructive_change = true;
            }
        }

        if let Some(sel) = new_selection {
            self.selected = Some(sel);
        }

        for junction in diagram.junctions() {
            ui.painter()
                .circle_filled(cellpos_to_egui(junction), 5.0, Color32::LIGHT_GRAY);
        }

        destructive_change
    }

    /// Returns true if the sim needs rebuilding
    pub fn edit_component(
        &mut self,
        ui: &mut Ui,
        diagram: &mut Diagram,
        state: &DiagramState,
    ) -> bool {
        if let Some((idx, is_threeterminal)) = self.selected {
            if is_threeterminal {
                if let Some((_, component)) = diagram.three_terminal.get_mut(idx) {
                edit_threeterminal_component(
                    ui,
                    component,
                    state.three_terminal[idx],
                );
                }
            } else {
                if let Some((terminals, component)) = diagram.two_terminal.get_mut(idx) {
                    edit_twoterminal_component(ui, component, state.two_terminal[idx]);

                    if ui.button("Flip").clicked() {
                        terminals.swap(0, 1);
                        return true;
                    }
                } else {
                    eprintln!("Warning: Couldn't find {idx} in diagram");
                }
            }

            if ui.button("Delete").clicked() {
                self.delete(diagram);
                return true;
            }
        } else {
            ui.weak("Click on a component to edit");
        }

        false
    }
}

// TODO: The following code sucks.

fn interact_with_twoterminal_body(
    ui: &mut Ui,
    pos: [CellPos; 2],
    id: Id,
    selected: bool,
) -> egui::Response {
    let begin = cellpos_to_egui(pos[0]);
    let end = cellpos_to_egui(pos[1]);
    let body_rect = Rect::from_points(&[begin, end]);

    let horiz = pos[0].1 == pos[1].1;
    let vert = pos[0].0 == pos[1].0;
    let body_hitbox = if horiz == vert {
        body_rect
    } else {
        body_rect.expand(10.0)
    };

    ui.interact(body_hitbox, id, Sense::click_and_drag())
}

fn interact_with_twoterminal(
    ui: &mut Ui,
    pos: &mut [CellPos; 2],
    component: &mut TwoTerminalComponent,
    wires: [DiagramWireState; 2],
    body_resp: Response,
    selected: bool,
    debug_draw: bool,
    vis: &VisualizationOptions,
) -> bool {
    let id = Id::new("twoterminal");
    let begin = cellpos_to_egui(pos[0]);
    let end = cellpos_to_egui(pos[1]);

    let handle_hitbox_size = 50.0;
    let begin_hitbox = Rect::from_center_size(begin, Vec2::splat(handle_hitbox_size));
    let end_hitbox = Rect::from_center_size(end, Vec2::splat(handle_hitbox_size));

    let mut begin_offset = Vec2::ZERO;
    let mut end_offset = Vec2::ZERO;

    let mut destructive_change = false;

    if selected {
        let end_resp = ui.interact(end_hitbox, id.with("end"), Sense::click_and_drag());
        let begin_resp = ui.interact(begin_hitbox, id.with("begin"), Sense::click_and_drag());

        let interact_pos = body_resp
            .interact_pointer_pos()
            .or(begin_resp.interact_pointer_pos())
            .or(end_resp.interact_pointer_pos());

        if body_resp.drag_started() || begin_resp.drag_started() || end_resp.drag_started() {
            if let Some(interact_pos) = interact_pos {
                ui.memory_mut(|mem| *mem.data.get_temp_mut_or_default::<Pos2>(id) = interact_pos);
            }
        }

        let interact_begin_pos = ui.memory_mut(|mem| mem.data.get_temp::<Pos2>(id));

        let interact_delta = interact_begin_pos
            .zip(interact_pos)
            .map(|(start, stop)| stop - start);

        if body_resp.dragged() || body_resp.drag_stopped() {
            begin_offset = interact_delta.unwrap_or(Vec2::ZERO);
            end_offset = interact_delta.unwrap_or(Vec2::ZERO);
        } else if begin_resp.dragged() || begin_resp.drag_stopped() {
            begin_offset = interact_delta.unwrap_or(Vec2::ZERO);
        } else if end_resp.dragged() || end_resp.drag_stopped() {
            end_offset = interact_delta.unwrap_or(Vec2::ZERO);
        }

        if body_resp.drag_stopped() || begin_resp.drag_stopped() || end_resp.drag_stopped() {
            pos[0] = egui_to_cellpos(begin + begin_offset);
            pos[1] = egui_to_cellpos(end + end_offset);
            destructive_change = true;
        }

        if body_resp.drag_stopped() || begin_resp.drag_stopped() || end_resp.drag_stopped() {
            ui.memory_mut(|mem| mem.data.remove::<Pos2>(id));
        }

        if debug_draw {
            ui.painter().rect_stroke(
                begin_hitbox.translate(begin_offset),
                0.0,
                Stroke::new(1., Color32::WHITE),
                egui::StrokeKind::Inside,
            );

            ui.painter().rect_stroke(
                end_hitbox.translate(end_offset),
                0.0,
                Stroke::new(1., Color32::WHITE),
                egui::StrokeKind::Inside,
            );
        }

        ui.painter().circle_stroke(
            begin + begin_offset,
            handle_hitbox_size / 2.0,
            Stroke::new(1., Color32::WHITE),
        );

        ui.painter().circle_stroke(
            end + end_offset,
            handle_hitbox_size / 2.0,
            Stroke::new(1., Color32::WHITE),
        );
    }

    if debug_draw {
        ui.painter().rect_stroke(
            body_resp.rect.translate((begin_offset + end_offset) / 2.0),
            0.0,
            Stroke::new(
                1.,
                if selected {
                    Color32::RED
                } else {
                    Color32::WHITE
                },
            ),
            egui::StrokeKind::Inside,
        );
    }

    if let TwoTerminalComponent::Switch(is_open) = component {
        if body_resp.clicked() && selected {
            *is_open ^= true;
        }
    }

    draw_twoterminal_component(
        ui.painter(),
        [begin + begin_offset, end + end_offset],
        wires,
        *component,
        selected,
        vis,
    );

    destructive_change
}

fn interact_with_threeterminal_body(
    ui: &mut Ui,
    pos: [CellPos; 3],
    id: Id,
    selected: bool,
    vis: &VisualizationOptions,
) -> egui::Response {
    let a = cellpos_to_egui(pos[0]);
    let b = cellpos_to_egui(pos[1]);
    let c = cellpos_to_egui(pos[2]);
    let body_rect = Rect::from_points(&[a, b, c]);

    let body_hitbox = if body_rect.area() == 0.0 {
        body_rect
    } else {
        body_rect.expand(10.0)
    };

    ui.interact(body_hitbox, id, Sense::click_and_drag())
}

fn interact_with_threeterminal(
    ui: &mut Ui,
    pos: &mut [CellPos; 3],
    component: ThreeTerminalComponent,
    wires: [DiagramWireState; 3],
    body_resp: Response,
    selected: bool,
    vis: &VisualizationOptions,
) -> bool {
    let id = Id::new("threeterminal");
    let a = cellpos_to_egui(pos[0]);
    let b = cellpos_to_egui(pos[1]);
    let c = cellpos_to_egui(pos[2]);

    let handle_hitbox_size = 50.0;
    let a_hitbox = Rect::from_center_size(a, Vec2::splat(handle_hitbox_size));
    let b_hitbox = Rect::from_center_size(b, Vec2::splat(handle_hitbox_size));
    let c_hitbox = Rect::from_center_size(c, Vec2::splat(handle_hitbox_size));

    let mut a_offset = Vec2::ZERO;
    let mut b_offset = Vec2::ZERO;
    let mut c_offset = Vec2::ZERO;

    let mut destructive_change = false;

    if selected {
        let a_resp = ui.interact(a_hitbox, id.with("a"), Sense::click_and_drag());
        let b_resp = ui.interact(b_hitbox, id.with("b"), Sense::click_and_drag());
        let c_resp = ui.interact(c_hitbox, id.with("c"), Sense::click_and_drag());

        let interact_pos = body_resp
            .interact_pointer_pos()
            .or(a_resp.interact_pointer_pos())
            .or(b_resp.interact_pointer_pos())
            .or(c_resp.interact_pointer_pos());

        if body_resp.drag_started()
            || a_resp.drag_started()
            || b_resp.drag_started()
            || c_resp.drag_started()
        {
            if let Some(interact_pos) = interact_pos {
                ui.memory_mut(|mem| *mem.data.get_temp_mut_or_default::<Pos2>(id) = interact_pos);
            }
        }

        let interact_begin_pos = ui.memory_mut(|mem| mem.data.get_temp::<Pos2>(id));

        let interact_delta = interact_begin_pos
            .zip(interact_pos)
            .map(|(start, stop)| stop - start);

        if body_resp.dragged() || body_resp.drag_stopped() {
            a_offset = interact_delta.unwrap_or(Vec2::ZERO);
            b_offset = interact_delta.unwrap_or(Vec2::ZERO);
            c_offset = interact_delta.unwrap_or(Vec2::ZERO);
        } else if a_resp.dragged() || a_resp.drag_stopped() {
            a_offset = interact_delta.unwrap_or(Vec2::ZERO);
        } else if b_resp.dragged() || b_resp.drag_stopped() {
            b_offset = interact_delta.unwrap_or(Vec2::ZERO);
        } else if c_resp.dragged() || c_resp.drag_stopped() {
            c_offset = interact_delta.unwrap_or(Vec2::ZERO);
        }

        if body_resp.drag_stopped()
            || a_resp.drag_stopped()
            || b_resp.drag_stopped()
            || c_resp.drag_stopped()
        {
            pos[0] = egui_to_cellpos(a + a_offset);
            pos[1] = egui_to_cellpos(b + b_offset);
            pos[2] = egui_to_cellpos(c + c_offset);
            destructive_change = true;
            ui.memory_mut(|mem| mem.data.remove::<Pos2>(id));
        }

        ui.painter().circle_stroke(
            a + a_offset,
            handle_hitbox_size / 2.0,
            Stroke::new(1., Color32::WHITE),
        );

        ui.painter().circle_stroke(
            b + b_offset,
            handle_hitbox_size / 2.0,
            Stroke::new(1., Color32::WHITE),
        );

        ui.painter().circle_stroke(
            c + c_offset,
            handle_hitbox_size / 2.0,
            Stroke::new(1., Color32::WHITE),
        );
    }

    let a = a + a_offset;
    let b = b + b_offset;
    let c = c + c_offset;

    draw_threeterminal_component(ui.painter(), [a, b, c], wires, component, selected, vis);

    destructive_change
}

impl DiagramWireState {
    /// Zeroes current
    pub fn floating(self) -> Self {
        Self {
            voltage: self.voltage,
            current: 0.0,
        }
    }

    pub fn color(&self, selected: bool, vis: &VisualizationOptions) -> Color32 {
        if selected {
            Color32::from_rgb(0x00, 0xff, 0xff)
        } else {
            voltage_color(self.voltage / vis.voltage_scale)
        }
    }

    pub fn wire(
        &self,
        painter: &Painter,
        a: Pos2,
        b: Pos2,
        selected: bool,
        vis: &VisualizationOptions,
    ) {
        self.line_segment(painter, a, b, selected, vis);
        self.current(painter, a, b, vis);
    }

    pub fn arrow(
        &self,
        painter: &Painter,
        a: Pos2,
        b: Pos2,
        selected: bool,
        direction: bool,
        vis: &VisualizationOptions,
    ) {
        {
            let (rev_a, rev_b) = if direction { (a, b) } else { (b, a) };
            self.arrow_segment(painter, rev_a, rev_b, selected, vis);
        }

        self.current(painter, a, b, vis);
    }

    pub fn line_segment(
        &self,
        painter: &Painter,
        a: Pos2,
        b: Pos2,
        selected: bool,
        vis: &VisualizationOptions,
    ) {
        painter.line_segment([a, b], Stroke::new(3., self.color(selected, vis)));
    }

    pub fn arrow_segment(
        &self,
        painter: &Painter,
        a: Pos2,
        b: Pos2,
        selected: bool,
        vis: &VisualizationOptions,
    ) {
        painter.line_segment([a, b], Stroke::new(3., self.color(selected, vis)));

        let y = (b - a).normalized();
        let x = y.rot90();

        let vp = (y + x / 3.0) * CELL_SIZE * 0.15;
        let vm = (y - x / 3.0) * CELL_SIZE * 0.15;

        painter.add(Shape::convex_polygon(
            vec![a, a + vp, a + vm],
            self.color(selected, vis),
            Stroke::NONE,
        ));
        //painter.arrow(a, b - a, Stroke::new(3., self.color(selected)));
    }

    pub fn current(&self, painter: &Painter, a: Pos2, b: Pos2, vis: &VisualizationOptions) {
        if self.current == 0.0 {
            return;
        }

        let spacing = CELL_SIZE / 5.0;

        let n = ((b - a).length() / spacing) as usize;
        let n = n.max(1);

        let time = painter
            .ctx()
            .input(|r| r.time * self.current.abs() as f64 / vis.current_scale)
            .fract() as f32;

        let rect_size = 5.0;

        for i in 0..n {
            let mut t = (i as f32 + time) / n as f32;
            if self.current < 0.0 {
                t = 1.0 - t
            }
            let pos = a.lerp(b, t);
            let rect = Rect::from_center_size(pos, Vec2::splat(rect_size));
            painter.rect_filled(rect, 0.0, Color32::YELLOW);
        }
    }

    /// Copies current from this
    pub fn lerp_voltage(&self, other: &Self, t: f64) -> Self {
        Self {
            voltage: (1.0 - t) * self.voltage + t * other.voltage,
            current: self.current,
        }
    }
}

fn voltage_color(voltage: f64) -> Color32 {
    let v = voltage.clamp(-1.0, 1.0);

    let neutral = Color32::DARK_GRAY;

    if v > 0.0 {
        neutral.lerp_to_gamma(Color32::GREEN, v as f32)
    } else {
        neutral.lerp_to_gamma(Color32::RED, -v as f32)
    }
}

fn draw_threeterminal_component(
    painter: &Painter,
    pos: [Pos2; 3],
    wires: [DiagramWireState; 3],
    component: ThreeTerminalComponent,
    selected: bool,
    vis: &VisualizationOptions,
) {
    match component {
        ThreeTerminalComponent::PTransistor(_) => {
            draw_transistor(painter, pos, wires, selected, true, vis)
        }
        ThreeTerminalComponent::NTransistor(_) => {
            draw_transistor(painter, pos, wires, selected, false, vis)
        }
    }
}

fn draw_twoterminal_component(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    component: TwoTerminalComponent,
    selected: bool,
    vis: &VisualizationOptions,
) {
    match component {
        TwoTerminalComponent::Wire => wires[0].wire(painter, pos[0], pos[1], selected, vis),
        TwoTerminalComponent::Resistor(_) => draw_resistor(painter, pos, wires, selected, vis),
        TwoTerminalComponent::Inductor(_,_) => draw_inductor(painter, pos, wires, selected, vis),
        TwoTerminalComponent::Capacitor(_) => draw_capacitor(painter, pos, wires, selected, vis),
        TwoTerminalComponent::Diode => draw_diode(painter, pos, wires, selected, vis),
        TwoTerminalComponent::Battery(_) => draw_battery(painter, pos, wires, selected, vis),
        TwoTerminalComponent::Switch(is_open) => {
            draw_switch(painter, pos, wires, selected, is_open, vis)
        }
        TwoTerminalComponent::CurrentSource(_) => {
            draw_current_source(painter, pos, wires, selected, vis)
        }
    }
    draw_component_value(painter, pos, component);
}

impl DiagramState {
    fn default_from_diagram(diagram: &Diagram) -> Self {
        Self {
            two_terminal: diagram
                .two_terminal
                .iter()
                .map(|_| [DiagramWireState::default(); 2])
                .collect(),
            three_terminal: diagram
                .three_terminal
                .iter()
                .map(|_| [DiagramWireState::default(); 3])
                .collect(),
        }
    }
}

fn edit_transistor(ui: &mut Ui, beta: &mut f64) -> Response {
    ui.add(DragValue::new(beta).speed(1e-2).prefix("Beta: "))
}

fn edit_threeterminal_component(
    ui: &mut Ui,
    component: &mut ThreeTerminalComponent,
    wires: [DiagramWireState; 3],
) {
    ui.strong(component.name());
    match component {
        ThreeTerminalComponent::PTransistor(beta) => edit_transistor(ui, beta),
        ThreeTerminalComponent::NTransistor(beta) => edit_transistor(ui, beta),
    };
}

fn edit_twoterminal_component(
    ui: &mut Ui,
    component: &mut TwoTerminalComponent,
    wires: [DiagramWireState; 2],
) {
    ui.strong(component.name());
    match component {
        TwoTerminalComponent::Battery(v) => ui.add(DragValue::new(v).suffix(" V").speed(1e-2)),
        TwoTerminalComponent::Inductor(i, maybe_coreid) => {
            ui.add(DragValue::new(i).suffix(" H").speed(1e-2));
            let mut has_core = maybe_coreid.is_some();
            if ui.checkbox(&mut has_core, "Core ID").changed() {
                *maybe_coreid = has_core.then(|| 0);
            }
            ui.add_enabled(has_core, DragValue::new(maybe_coreid.as_mut().unwrap_or(&mut 0)))
        },
        TwoTerminalComponent::Capacitor(c) => ui.add(DragValue::new(c).suffix(" F").speed(1e-2)),
        TwoTerminalComponent::Resistor(r) => ui.add(DragValue::new(r).suffix(" Ω").speed(1e-2)),
        TwoTerminalComponent::Wire => ui.response(),
        TwoTerminalComponent::Diode => ui.response(),
        TwoTerminalComponent::Switch(is_open) => ui.checkbox(is_open, "Switch open"),
        TwoTerminalComponent::CurrentSource(i) => {
            ui.add(DragValue::new(i).suffix(" A").speed(1e-2))
        }
    };

    let voltage = wires[1].voltage - wires[0].voltage;
    ui.label(format!("Vd: {}", to_metric_prefix(voltage, 'V')));
    let current = wires[0].current;
    ui.label(format!("I: {}", to_metric_prefix(current, 'A')));
    ui.weak(format!("P: {}", to_metric_prefix(voltage * current, 'W')));
}

impl Default for VisualizationOptions {
    fn default() -> Self {
        Self {
            voltage_scale: 5.0,
            current_scale: 5.0,
        }
    }
}
