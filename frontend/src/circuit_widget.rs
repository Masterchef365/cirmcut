use egui::{Color32, Id, Key, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use std::{collections::HashMap, sync::Arc};

use cirmcut_sim::{CellPos, ThreeTerminalComponent, TwoTerminalComponent};

pub const CELL_SIZE: f32 = 100.0;

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Diagram {
    pub two_terminal: Vec<TwoTerminalDiagramComponent>,
    pub three_terminal: Vec<ThreeTerminalDiagramComponent>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub struct TwoTerminalDiagramComponent {
    pub begin: CellPos,
    pub end: CellPos,
    pub component: TwoTerminalComponent,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub struct ThreeTerminalDiagramComponent {
    pub a: CellPos,
    pub b: CellPos,
    pub c: CellPos,
    pub component: ThreeTerminalComponent,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DiagramEditor {
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
    (
        (v.x / CELL_SIZE) as i32,
        (v.y / CELL_SIZE) as i32,
    )
}



impl Diagram {
    pub fn junctions(&self) -> Vec<CellPos> {
        let mut junctions = HashMap::<CellPos, u32>::new();
        for comp in &self.two_terminal {
            for pos in [comp.begin, comp.end] {
                *junctions.entry(pos).or_default() += 1;
            }
        }
        for comp in &self.three_terminal {
            for pos in [comp.a, comp.b, comp.c] {
                *junctions.entry(pos).or_default() += 1;
            }
        }
        junctions
            .into_iter()
            .filter_map(|(pos, count)| (count > 1).then_some(pos))
            .collect()
    }
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

impl DiagramEditor {
    pub fn new(diagram: Diagram) -> Self {
        let mut inst = Self {
            junctions: vec![],
            diagram,
            selected: None,
        };

        inst.recompute_junctions();

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
        self.diagram.three_terminal.push(ThreeTerminalDiagramComponent {
            a: pos,
            b: (x + 1, y),
            c: (x + 1, y + 1),
            component,
        });
        self.recompute_junctions();
    }

    pub fn new_twoterminal(&mut self, pos: CellPos, component: TwoTerminalComponent) {
        let (x, y) = pos;
        self.diagram.two_terminal.push(TwoTerminalDiagramComponent {
            begin: pos,
            end: (x + 1, y),
            component,
        });
        self.recompute_junctions();
    }

    pub fn edit(&mut self, ui: &mut Ui, debug_draw: bool) {
        if ui.input(|r| r.key_pressed(Key::Escape)) {
            self.selected = None;
        }

        let mut two_body_responses = vec![];
        let mut three_body_responses = vec![];

        let mut any_changed = false;
        let mut new_selection = None;

        for (idx, comp) in self.diagram.two_terminal.iter_mut().enumerate() {
            let ret = interact_with_twoinal_body(
                ui,
                comp,
                Id::new("body").with(idx),
                self.selected == Some((idx, false)),
            );
            if ret.clicked() {
                new_selection = Some((idx, false));
            }
            two_body_responses.push(ret);
        }

        for (idx, comp) in self.diagram.three_terminal.iter_mut().enumerate() {
            let ret = interact_with_threeinal_body(
                ui,
                comp,
                Id::new("threebody").with(idx),
                self.selected == Some((idx, true)),
            );
            if ret.clicked() {
                new_selection = Some((idx, true));
            }
            three_body_responses.push(ret);
        }

        for (idx, (resp, comp)) in two_body_responses
            .drain(..)
            .zip(self.diagram.two_terminal.iter_mut())
            .enumerate()
        {
            if interact_with_twoinal(ui, comp, resp, self.selected == Some((idx, false)), debug_draw) {
                any_changed = true;
            }
        }

        for (idx, (resp, comp)) in three_body_responses
            .drain(..)
            .zip(self.diagram.three_terminal.iter_mut())
            .enumerate()
        {
            if interact_with_threeinal(ui, comp, resp, self.selected == Some((idx, true)), debug_draw) {
                any_changed = true;
            }
        }


        if let Some(sel) = new_selection {
            self.selected = Some(sel);
        }

        if any_changed {
            self.recompute_junctions();
        }

        for junction in &self.junctions {
            ui.painter()
                .circle_filled(cellpos_to_egui(*junction), 5.0, Color32::LIGHT_GRAY);
        }
    }

    fn recompute_junctions(&mut self) {
        self.junctions = Diagram::from(self.diagram()).junctions();
    }
}

// TODO: The following code sucks.

fn interact_with_twoinal_body(
    ui: &mut Ui,
    comp: &mut TwoTerminalDiagramComponent,
    id: Id,
    selected: bool,
) -> egui::Response {
    let begin = cellpos_to_egui(comp.begin);
    let end = cellpos_to_egui(comp.end);
    let body_rect = Rect::from_points(&[begin, end]);

    let horiz = comp.begin.1 == comp.end.1;
    let vert = comp.begin.0 == comp.end.0;
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

fn interact_with_twoinal(
    ui: &mut Ui,
    comp: &mut TwoTerminalDiagramComponent,
    body_resp: Response,
    selected: bool,
    debug_draw: bool,
) -> bool {
    let id = Id::new("twoinal");
    let begin = cellpos_to_egui(comp.begin);
    let end = cellpos_to_egui(comp.end);

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
            comp.begin = egui_to_cellpos(begin + begin_offset);
            comp.end = egui_to_cellpos(end + end_offset);
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

    let color = if selected {
        Color32::from_rgb(0x00, 0xff, 0xff)
    } else {
        Color32::GREEN
    };

    ui.painter().line_segment(
        [begin + begin_offset, end + end_offset],
        Stroke::new(3., color),
    );

    any_changed
}

fn interact_with_threeinal_body(
    ui: &mut Ui,
    comp: &mut ThreeTerminalDiagramComponent,
    id: Id,
    selected: bool,
) -> egui::Response {
    let a = cellpos_to_egui(comp.a);
    let b = cellpos_to_egui(comp.b);
    let c = cellpos_to_egui(comp.c);
    let body_rect = Rect::from_points(&[a, b, c]);

    let body_hitbox = if body_rect.area() == 0.0 {
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

fn interact_with_threeinal(
    ui: &mut Ui,
    comp: &mut ThreeTerminalDiagramComponent,
    body_resp: Response,
    selected: bool,
    debug_draw: bool,
) -> bool {
    let id = Id::new("threeinal");
    let a = cellpos_to_egui(comp.a);
    let b = cellpos_to_egui(comp.b);
    let c = cellpos_to_egui(comp.c);

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

        if body_resp.drag_started() || a_resp.drag_started() || b_resp.drag_started() || c_resp.drag_started()  {
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

        if body_resp.drag_stopped() || a_resp.drag_stopped() || b_resp.drag_stopped() || c_resp.drag_stopped() {
            comp.a = egui_to_cellpos(a + a_offset);
            comp.b = egui_to_cellpos(b + b_offset);
            comp.c = egui_to_cellpos(c + c_offset);
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

    let color = if selected {
        Color32::from_rgb(0x00, 0xff, 0xff)
    } else {
        Color32::GREEN
    };

    let a = a + a_offset;
    let b = b + b_offset;
    let c = c + c_offset;

    let ctr = ((a.to_vec2() + b.to_vec2() + c.to_vec2()) / 3.0).to_pos2();

    ui.painter().line_segment(
        [a, ctr],
        Stroke::new(3., color),
    );

    ui.painter().line_segment(
        [b, ctr],
        Stroke::new(3., color),
    );

    ui.painter().line_segment(
        [c, ctr],
        Stroke::new(3., color),
    );

    any_changed
}

