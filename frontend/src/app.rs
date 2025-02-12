use std::collections::{HashMap, HashSet};

use cirmcut_sim::CellPos;
use egui::{Color32, Id, Key, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::circuit_widget::{
    cellpos_to_egui, cellpos_to_egui_vec, draw_grid, egui_to_cellpos, egui_to_cellvec,
};

//use crate::circuit_widget::{circuit_widget, ComponentButton};

#[derive(serde::Deserialize, serde::Serialize)]
//#[serde(default)]
pub struct CircuitApp {
    view_rect: Rect,
    editor: DiagramEditor,
    debug_draw: bool,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
struct Diagram {
    two_terminal: Vec<TwoTerminalComponent>,
    three_terminal: Vec<ThreeTerminalComponent>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
struct TwoTerminalComponent {
    begin: CellPos,
    end: CellPos,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
struct ThreeTerminalComponent {
    a: CellPos,
    b: CellPos,
    c: CellPos,
}

impl Default for CircuitApp {
    fn default() -> Self {
        Self {
            editor: DiagramEditor::new(Diagram::default()),
            view_rect: Rect::from_center_size(Pos2::ZERO, Vec2::splat(1000.0)),
            debug_draw: false,
        }
    }
}

impl CircuitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for CircuitApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });

        egui::TopBottomPanel::bottom("buttons").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Add wire").clicked() {
                    let pos = egui_to_cellpos(self.view_rect.center());
                    self.editor.new_twoterminal(pos);
                }
                if ui.button("Add transistor").clicked() {
                    let pos = egui_to_cellpos(self.view_rect.center());
                    self.editor.new_threeterminal(pos);
                }
                ui.checkbox(&mut self.debug_draw, "Debug draw");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let rect = self.view_rect;
                egui::Scene::new().show(ui, &mut self.view_rect, |ui| {
                    draw_grid(ui, rect, 1.0, Color32::DARK_GRAY);
                    self.editor.edit(ui, self.debug_draw);
                });
            });
        });
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct DiagramEditor {
    diagram: Diagram,
    junctions: Vec<CellPos>,
    selected: Option<(usize, bool)>,
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

    pub fn new_threeterminal(&mut self, pos: CellPos) {
        let (x, y) = pos;
        self.diagram.three_terminal.push(ThreeTerminalComponent {
            a: pos,
            b: (x + 1, y),
            c: (x + 1, y + 1),
        });
        self.recompute_junctions();
    }

    pub fn new_twoterminal(&mut self, pos: CellPos) {
        let (x, y) = pos;
        self.diagram.two_terminal.push(TwoTerminalComponent {
            begin: pos,
            end: (x + 1, y),
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
            let ret = interact_with_twoterminal_body(
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
            let ret = interact_with_threeterminal_body(
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
            if interact_with_twoterminal(ui, comp, resp, self.selected == Some((idx, false)), debug_draw) {
                any_changed = true;
            }
        }

        for (idx, (resp, comp)) in three_body_responses
            .drain(..)
            .zip(self.diagram.three_terminal.iter_mut())
            .enumerate()
        {
            println!("threeterminal {}", idx);
            if interact_with_threeterminal(ui, comp, resp, self.selected == Some((idx, true)), debug_draw) {
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

    /*
    fn nearest_component_idx(&self, cursor: Pos2) -> Option<usize> {
        let mut closest_dist_sq = 100_f32.powi(2);

        let mut closest_idx = None;
        for (idx, comp) in self.diagram.two_terminal.iter().enumerate() {
            let begin = cellpos_to_egui(comp.begin);
            let end = cellpos_to_egui(comp.end);

            // Vector projection
            let cursor_off = cursor - begin;
            let n = (end - begin).normalized();
            let t = n.dot(cursor_off);
            let dist_sq = (n * t - cursor_off).length_sq();
            if dist_sq < closest_dist_sq {
                closest_dist_sq = dist_sq;
                closest_idx = Some(idx);
            }
        }

        closest_idx
    }
    */
}

fn interact_with_twoterminal_body(
    ui: &mut Ui,
    comp: &mut TwoTerminalComponent,
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

fn interact_with_twoterminal(
    ui: &mut Ui,
    comp: &mut TwoTerminalComponent,
    body_resp: Response,
    selected: bool,
    debug_draw: bool,
) -> bool {
    let id = Id::new("twoterminal");
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

fn interact_with_threeterminal_body(
    ui: &mut Ui,
    comp: &mut ThreeTerminalComponent,
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

fn interact_with_threeterminal(
    ui: &mut Ui,
    comp: &mut ThreeTerminalComponent,
    body_resp: Response,
    selected: bool,
    debug_draw: bool,
) -> bool {
    let id = Id::new("threeterminal");
    let a = cellpos_to_egui(comp.a);
    let b = cellpos_to_egui(comp.b);
    let c = cellpos_to_egui(comp.c);

    let handle_hitbox_size = 50.0;
    let a_hitbox = Rect::from_center_size(a, Vec2::splat(handle_hitbox_size));
    let b_hitbox = Rect::from_center_size(b, Vec2::splat(handle_hitbox_size));

    let mut a_offset = Vec2::ZERO;
    let mut b_offset = Vec2::ZERO;
    let mut c_offset = Vec2::ZERO;

    let mut any_changed = false;

    if selected {
        let a_resp = ui.interact(a_hitbox, id.with("a"), Sense::click_and_drag());
        let b_resp = ui.interact(b_hitbox, id.with("b"), Sense::click_and_drag());
        let c_resp = ui.interact(b_hitbox, id.with("c"), Sense::click_and_drag());

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

        /*
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
        */

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

impl Diagram {
    pub fn junctions(&self) -> Vec<CellPos> {
        let mut junctions = HashMap::<CellPos, u32>::new();
        for comp in &self.two_terminal {
            for pos in [comp.begin, comp.end] {
                *junctions.entry(pos).or_default() += 1;
            }
        }
        junctions
            .into_iter()
            .filter_map(|(pos, count)| (count > 1).then_some(pos))
            .collect()
    }
}
