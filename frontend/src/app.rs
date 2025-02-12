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

#[derive(Clone, Debug, Default)]
struct Diagram {
    components: Vec<TwoTerminalComponent>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, Debug)]
struct TwoTerminalComponent {
    begin: CellPos,
    end: CellPos,
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
        ctx.request_repaint();
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
                    self.editor.new_component((), (0, 0));
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
    components: Vec<TwoTerminalComponent>,
    junctions: Vec<CellPos>,
    selected: Option<usize>,
}

impl DiagramEditor {
    pub fn new(diagram: Diagram) -> Self {
        let mut inst = Self {
            junctions: vec![],
            components: diagram.components,
            selected: None,
        };

        inst.recompute_junctions();

        inst
    }

    pub fn diagram(&self) -> Diagram {
        Diagram::new(self.components.clone())
    }

    pub fn new_component(&mut self, component: (), pos: CellPos) {
        let (x, y) = pos;
        self.components.push(TwoTerminalComponent {
            begin: pos,
            end: (x + 1, y),
        });
        self.recompute_junctions();
    }

    pub fn edit(&mut self, ui: &mut Ui, debug_draw: bool) {
        if ui.input(|r| r.key_pressed(Key::Escape)) {
            self.selected = None;
        }

        let mut body_responses = vec![];

        let mut any_changed = false;
        let mut new_selection = None;
        for (idx, comp) in self.components.iter_mut().enumerate() {
            let ret = interact_with_component_body(
                ui,
                comp,
                Id::new("body").with(idx),
                self.selected == Some(idx),
            );
            if ret.clicked() {
                new_selection = Some(idx);
            }
            body_responses.push(ret);
        }

        for (idx, (resp, comp)) in body_responses
            .drain(..)
            .zip(self.components.iter_mut())
            .enumerate()
        {
            if interact_with_component(ui, comp, resp, self.selected == Some(idx), debug_draw)
            {
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

    fn nearest_component_idx(&self, cursor: Pos2) -> Option<usize> {
        let mut closest_dist_sq = 100_f32.powi(2);

        let mut closest_idx = None;
        for (idx, comp) in self.components.iter().enumerate() {
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
}

fn interact_with_component_body(
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
    let body_hitbox = if horiz == vert { body_rect } else { body_rect.expand(10.0) };

    let sense = if selected {
        Sense::drag()
    } else {
        Sense::click_and_drag()
    };

    ui.interact(body_hitbox, id, sense)
}

fn interact_with_component(
    ui: &mut Ui,
    comp: &mut TwoTerminalComponent,
    body_resp: Response,
    selected: bool,
    debug_draw: bool,
) -> bool {
    let id = Id::new("component");
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

impl Diagram {
    pub fn new(components: Vec<TwoTerminalComponent>) -> Self {
        Self { components }
    }

    pub fn junctions(&self) -> Vec<CellPos> {
        let mut junctions = HashMap::<CellPos, u32>::new();
        for comp in &self.components {
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
