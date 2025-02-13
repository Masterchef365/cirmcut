use cirmcut_sim::{ThreeTerminalComponent, TwoTerminalComponent};
use egui::{Color32, Id, Key, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

use crate::circuit_widget::{cellpos_to_egui, draw_grid, egui_to_cellpos, Diagram, DiagramEditor};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CircuitApp {
    view_rect: Rect,
    editor: DiagramEditor,
    debug_draw: bool,
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
                    let pos = egui_to_cellpos(self.view_rect.center());
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Wire);
                }
                if ui.button("Add resistor").clicked() {
                    let pos = egui_to_cellpos(self.view_rect.center());
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Resistor(1000.0));
                }
                if ui.button("Add inductor").clicked() {
                    let pos = egui_to_cellpos(self.view_rect.center());
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Inductor(1.0));
                }
                if ui.button("Add PNP").clicked() {
                    let pos = egui_to_cellpos(self.view_rect.center());
                    self.editor
                        .new_threeterminal(pos, ThreeTerminalComponent::PTransistor(100.0));
                }
                if ui.button("Add NPN").clicked() {
                    let pos = egui_to_cellpos(self.view_rect.center());
                    self.editor
                        .new_threeterminal(pos, ThreeTerminalComponent::NTransistor(100.0));
                }
                if ui.button("Delete").clicked() {
                    self.editor.delete();
                }
                ui.checkbox(&mut self.debug_draw, "Debug draw");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let rect = self.view_rect;
                let resp = egui::Scene::new().show(ui, &mut self.view_rect, |ui| {
                    draw_grid(ui, rect, 1.0, Color32::DARK_GRAY);
                    self.editor.edit(ui, self.debug_draw);
                });

                if ui.input(|r| r.key_pressed(Key::Delete) || r.key_pressed(Key::Backspace)) {
                    self.editor.delete();
                }

                if resp.response.clicked() || ui.input(|r| r.key_pressed(Key::Escape)) {
                    self.editor.reset_selection();
                }
            });
        });
    }
}
