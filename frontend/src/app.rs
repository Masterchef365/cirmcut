use cirmcut_sim::CellPos;
use egui::{Color32, Id, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::circuit_widget::{
    cellpos_to_egui, cellpos_to_egui_vec, draw_grid, egui_to_cellpos, egui_to_cellvec,
};

//use crate::circuit_widget::{circuit_widget, ComponentButton};

//#[derive(serde::Deserialize, serde::Serialize)]
//#[serde(default)]
pub struct TemplateApp {
    view_rect: Rect,
    editor: DiagramEditor,
}

#[derive(Clone, Debug, Default)]
struct Diagram {
    components: Vec<TwoTerminalComponent>,
}

#[derive(Clone, Copy, Debug)]
struct TwoTerminalComponent {
    begin: CellPos,
    end: CellPos,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            editor: DiagramEditor::new(Diagram::default()),
            view_rect: Rect::ZERO,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        /*
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        */
        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /*
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
    */

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });

        egui::TopBottomPanel::bottom("buttons").show(ctx, |ui| {
            if ui.button("Add wire").clicked() {
                self.editor.new_component((), (0, 0));
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let rect = self.view_rect;
                egui::Scene::new().show(ui, &mut self.view_rect, |ui| {
                    draw_grid(ui, rect, 1.0, Color32::DARK_GRAY);
                    self.editor.edit(ui, rect);
                });
            });
        });
    }
}

struct DiagramEditor {
    components: Vec<TwoTerminalComponent>,
}

impl DiagramEditor {
    pub fn new(diagram: Diagram) -> Self {
        Self {
            components: diagram.components,
        }
    }

    pub fn diagram(&self) -> Diagram {
        todo!()
    }

    pub fn new_component(&mut self, component: (), pos: CellPos) {
        let (x, y) = pos;
        self.components.push(TwoTerminalComponent {
            begin: pos,
            end: (x + 1, y),
        });
    }

    pub fn edit(&mut self, ui: &mut Ui, view_rect: Rect) {
        for (idx, comp) in self.components.iter_mut().enumerate() {
            interact_with_component(ui, comp);
        }
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

fn interact_with_component(
    ui: &mut Ui,
    comp: &mut TwoTerminalComponent,
) -> egui::Response {
    let id = Id::new("component body");
    let begin = cellpos_to_egui(comp.begin);
    let end = cellpos_to_egui(comp.end);
    let body_rect = Rect::from_points(&[begin, end]);
    let mut body_rect = body_rect.expand(10.0);

    let handle_hitbox_size = 50.0;
    let begin_handle = Rect::from_center_size(begin, Vec2::splat(handle_hitbox_size));
    let end_handle = Rect::from_center_size(end, Vec2::splat(handle_hitbox_size));

    let body_resp = ui.allocate_rect(body_rect, Sense::click_and_drag());
    let end_resp = ui.allocate_rect(end_handle, Sense::click_and_drag());
    let begin_resp = ui.allocate_rect(begin_handle, Sense::click_and_drag());

    let focus = |resp: egui::Response| {
        if resp.clicked() {
            resp.request_focus();
        }
        if resp.has_focus() {
            Color32::RED
        } else {
            Color32::WHITE
        }
    };

    let interact_pos = body_resp.interact_pointer_pos().or(begin_resp.interact_pointer_pos()).or(end_resp.interact_pointer_pos());

    if body_resp.drag_started() || begin_resp.drag_started() || end_resp.drag_started() {
        if let Some(interact_pos) = interact_pos {
            ui.memory_mut(|mem| *mem.data.get_temp_mut_or_default::<Pos2>(id) = interact_pos);
        }
    }

    let interact_begin_pos = ui.memory_mut(|mem| mem.data.get_temp::<Pos2>(id));

    let interact_delta = interact_begin_pos.zip(interact_pos).map(|(start, stop)| stop - start);

    let mut begin_offset = Vec2::ZERO;
    let mut end_offset = Vec2::ZERO;
    if body_resp.dragged() || body_resp.drag_stopped() {
        begin_offset = interact_delta.unwrap_or(Vec2::ZERO);
        end_offset = interact_delta.unwrap_or(Vec2::ZERO);
    } else if begin_resp.dragged() || begin_resp.drag_stopped() {
        begin_offset = interact_delta.unwrap_or(Vec2::ZERO);
    } else if end_resp.dragged() || end_resp.drag_stopped() {
        end_offset = interact_delta.unwrap_or(Vec2::ZERO);
    }

    if body_resp.drag_stopped() || begin_resp.drag_stopped() || end_resp.drag_stopped() {
        comp.begin = egui_to_cellpos(begin + dbg!(begin_offset));
        comp.end = egui_to_cellpos(end + dbg!(end_offset));
    }

    if body_resp.drag_stopped() || begin_resp.drag_stopped() || end_resp.drag_stopped() {
        ui.memory_mut(|mem| mem.data.remove::<Pos2>(id));
    }

    let body_color = focus(body_resp.clone());
    ui.painter().rect_stroke(
        body_rect.translate((begin_offset + end_offset) / 2.0),
        0.0,
        Stroke::new(1., body_color),
        egui::StrokeKind::Inside,
    );

    let begin_color = focus(begin_resp);
    ui.painter().rect_stroke(
        begin_handle.translate(begin_offset),
        0.0,
        Stroke::new(1., begin_color),
        egui::StrokeKind::Inside,
    );

    let end_color = focus(end_resp);
    ui.painter().rect_stroke(
        end_handle.translate(end_offset),
        0.0,
        Stroke::new(1., end_color),
        egui::StrokeKind::Inside,
    );

    ui.painter().line_segment([begin + begin_offset, end + end_offset], Stroke::new(1., Color32::RED));


    body_resp
    /*

    let drag_begin_pos = ui.memory_mut(|mem| *mem.data.get_temp_mut_or_default::<Vec2>(id));

    let begin = cellpos_to_egui(comp.begin);
    let end = cellpos_to_egui(comp.end);

    let mut body_rect = Rect::from_points(&[drag_begin_pos, (begin - end) + drag_begin_pos]);
    // Expand dong.
    let mut body_rect = body_rect.expand(10.0);

    let body_resp = ui.allocate_rect(body_rect, Sense::click_and_drag());

    if body_resp.clicked() {
        body_resp.request_focus();
    }

    if body_resp.has_focus() {
        if body_resp.drag_started() {
            if let Some(interact) = body_resp.interact_pointer_pos() {
                ui.memory_mut(|mem| *mem.data.get_temp_mut_or_default::<Vec2>(id) = interact - begin);
            }
        }

        if body_resp.dragged() {
            ui.memory_mut(|mem| *mem.data.get_temp_mut_or_default::<Vec2>(id) += delta);
        }
    } else {
        let offset = ui.memory_mut(|mem| {
            let val = mem.data.get_temp_mut_or_default::<Vec2>(id);
            let tmp = *val;
            *val = Vec2::ZERO;
            tmp
        });

        egui_to_cellvec(pos)
    }

    body_resp
    */

    /*
    //ui.memory_mut(|mem| mem.data.remove::<Vec2>("component body".into()));
    if body_resp.has_focus() {
        //let begin = drag_handle(ui, &mut comp.begin);
        //let end = drag_handle(ui, &mut comp.end);
        if dbg!(body_resp.dragged()) {
            let delta = body_resp.drag_delta();
            dbg!(delta);

            let cellpos_delta = ui.memory_mut(|mem| {
                let offset = mem.data.get_temp_mut_or_default::<Vec2>("component body".into());
                *offset += delta;
                let floor = egui_to_cellvec(*offset);
                let fract = *offset - cellpos_to_egui_vec(floor);
                *offset = dbg!(fract);
                floor
            });

            dbg!(cellpos_delta);

            comp.begin.0 += cellpos_delta.0;
            comp.begin.1 += cellpos_delta.1;
            comp.end.0 += cellpos_delta.0;
            comp.end.1 += cellpos_delta.1;
        }
    }

    let body_color = if body_resp.has_focus() {
        Color32::BLUE
    } else {
        Color32::LIGHT_GRAY
    };

    ui.painter().line_segment(
        [cellpos_to_egui(comp.begin) + offset, cellpos_to_egui(comp.end) + offset],
        Stroke::new(1., body_color),
    );

    body_resp
    */
}

fn drag_handle(ui: &mut Ui, pos: &mut CellPos) -> egui::Response {
    let handle_hitbox_size = 10.0;

    let egui_pos = cellpos_to_egui(*pos);

    let rect = Rect::from_center_size(egui_pos, Vec2::splat(handle_hitbox_size));

    let resp = ui.allocate_rect(rect, Sense::FOCUSABLE | Sense::DRAG | Sense::CLICK);

    let handle_radius = 10.0;
    let handle_color = if resp.has_focus() {
        Color32::RED
    } else {
        Color32::WHITE
    };

    if resp.hovered() || resp.has_focus() {
        ui.painter()
            .circle_filled(egui_pos, handle_radius, handle_color);
    }

    if resp.dragged() || resp.clicked() {
        resp.request_focus();
    }

    if resp.has_focus() {}

    resp
}
