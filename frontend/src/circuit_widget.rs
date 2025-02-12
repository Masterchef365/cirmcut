use egui::{Color32, Id, Key, Painter, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use std::{collections::HashMap, sync::Arc};

use cirmcut_sim::{CellPos, ThreeTerminalComponent, TwoTerminalComponent};

pub const CELL_SIZE: f32 = 100.0;

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

#[derive(Clone, Copy, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct DiagramWireState {
    pub voltage: f32,
    pub current: f32,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DiagramEditor {
    state: DiagramState,
    diagram: Diagram,
    junctions: Vec<CellPos>,
    selected: Option<(usize, bool)>,
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
    pub fn new(diagram: Diagram) -> Self {
        let mut inst = Self {
            state: DiagramState::default_from_diagram(&diagram),
            junctions: vec![],
            diagram,
            selected: None,
        };

        inst.recompute_cached();

        inst
    }

    pub fn diagram(&self) -> Diagram {
        self.diagram.clone()
    }

    pub fn delete(&mut self) {
        if let Some((idx, three)) = self.selected.take() {
            if three {
                self.diagram.three_terminal.remove(idx);
            } else {
                self.diagram.two_terminal.remove(idx);
            }
        }
    }

    pub fn new_threeterminal(&mut self, pos: CellPos, component: ThreeTerminalComponent) {
        let (x, y) = pos;
        self.diagram
            .three_terminal
            .push(([pos, (x + 1, y), (x + 1, y + 1)], component));
        self.recompute_cached();
    }

    pub fn new_twoterminal(&mut self, pos: CellPos, component: TwoTerminalComponent) {
        let (x, y) = pos;
        self.diagram
            .two_terminal
            .push(([pos, (x + 1, y)], component));
        self.recompute_cached();
    }

    pub fn edit(&mut self, ui: &mut Ui, debug_draw: bool) {
        if ui.input(|r| r.key_pressed(Key::Escape)) {
            self.selected = None;
        }

        let mut two_body_responses = vec![];
        let mut three_body_responses = vec![];

        let mut any_changed = false;
        let mut new_selection = None;

        for (idx, (pos, comp)) in self.diagram.two_terminal.iter_mut().enumerate() {
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

        for (idx, (pos, comp)) in self.diagram.three_terminal.iter_mut().enumerate() {
            let ret = interact_with_threeterminal_body(
                ui,
                *pos,
                Id::new("threebody").with(idx),
                self.selected == Some((idx, true)),
            );
            if ret.clicked() {
                new_selection = Some((idx, true));
            }
            three_body_responses.push(ret);
        }

        for (idx, ((resp, (pos, comp)), wires)) in two_body_responses
            .drain(..)
            .zip(self.diagram.two_terminal.iter_mut())
            .zip(self.state.two_terminal.iter())
            .enumerate()
        {
            if interact_with_twoterminal(
                ui,
                pos,
                *comp,
                *wires,
                resp,
                self.selected == Some((idx, false)),
                debug_draw,
            ) {
                any_changed = true;
            }
        }

        for (idx, ((resp, (pos, comp)), wires)) in three_body_responses
            .drain(..)
            .zip(self.diagram.three_terminal.iter_mut())
            .zip(self.state.three_terminal.iter())
            .enumerate()
        {
            if interact_with_threeterminal(
                ui,
                pos,
                *comp,
                *wires,
                resp,
                self.selected == Some((idx, true)),
                debug_draw,
            ) {
                any_changed = true;
            }
        }

        if let Some(sel) = new_selection {
            self.selected = Some(sel);
        }

        if any_changed {
            self.recompute_cached();

        }

        for junction in &self.junctions {
            ui.painter()
                .circle_filled(cellpos_to_egui(*junction), 5.0, Color32::LIGHT_GRAY);
        }
    }

    fn recompute_cached(&mut self) {
        self.junctions = Diagram::from(self.diagram()).junctions();
        self.state = DiagramState::default_from_diagram(&self.diagram);
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

    let sense = if selected {
        Sense::drag()
    } else {
        Sense::click_and_drag()
    };

    ui.interact(body_hitbox, id, sense)
}

fn interact_with_twoterminal(
    ui: &mut Ui,
    pos: &mut [CellPos; 2],
    component: TwoTerminalComponent,
    wires: [DiagramWireState; 2],
    body_resp: Response,
    selected: bool,
    debug_draw: bool,
) -> bool {
    let id = Id::new("twoterminal");
    let begin = cellpos_to_egui(pos[0]);
    let end = cellpos_to_egui(pos[1]);

    let handle_hitbox_size = 50.0;
    let begin_hitbox = Rect::from_center_size(begin, Vec2::splat(handle_hitbox_size));
    let end_hitbox = Rect::from_center_size(end, Vec2::splat(handle_hitbox_size));

    let mut begin_offset = Vec2::ZERO;
    let mut end_offset = Vec2::ZERO;

    let mut any_changed = false;

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
            any_changed = true;
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

    draw_twoterminal_component(
        ui.painter(),
        [begin + begin_offset, end + end_offset],
        wires,
        component,
        selected,
    );

    any_changed
}

fn interact_with_threeterminal_body(
    ui: &mut Ui,
    pos: [CellPos; 3],
    id: Id,
    selected: bool,
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
    debug_draw: bool,
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

    let mut any_changed = false;

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
            any_changed = true;
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

    draw_threeterminal_component(ui.painter(), [a, b, c], wires, component, selected);

    any_changed
}

impl DiagramWireState {
    pub fn draw(&self, a: Pos2, b: Pos2) {}
}

fn voltage_color(voltage: f32) -> Color32 {
    todo!()
}

fn draw_threeterminal_component(
    painter: &Painter,
    pos: [Pos2; 3],
    wires: [DiagramWireState; 3],
    component: ThreeTerminalComponent,
    selected: bool,
) {
    let [a, b, c] = pos;
    let ctr = ((a.to_vec2() + b.to_vec2() + c.to_vec2()) / 3.0).to_pos2();

    let color = if selected {
        Color32::from_rgb(0x00, 0xff, 0xff)
    } else {
        Color32::GREEN
    };

    painter.line_segment([a, ctr], Stroke::new(3., color));

    painter.line_segment([b, ctr], Stroke::new(3., color));

    painter.line_segment([c, ctr], Stroke::new(3., color));
}

fn draw_twoterminal_component(
    painter: &Painter,
    pos: [Pos2; 2],
    wires: [DiagramWireState; 2],
    component: TwoTerminalComponent,
    selected: bool,
) {
    let color = if selected {
        Color32::from_rgb(0x00, 0xff, 0xff)
    } else {
        Color32::GREEN
    };

    painter.line_segment(pos, Stroke::new(3., color));
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
