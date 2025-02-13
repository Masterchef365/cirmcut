use cirmcut_sim::{ThreeTerminalComponent, TwoTerminalComponent};
use egui::{Color32, Id, Key, Pos2, Rect, Response, ScrollArea, Sense, Stroke, Ui, Vec2};

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

        egui::SidePanel::left("cfg").show(ctx, |ui| {
            self.editor.edit_component(ui);
        });

        egui::TopBottomPanel::bottom("buttons").show(ctx, |ui| {
            ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Add component: ");
                let pos = egui_to_cellpos(self.view_rect.center());
                if ui.button("Wire").clicked() {
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Wire);
                }
                if ui.button("Resistor").clicked() {
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Resistor(1000.0));
                }
                if ui.button("Inductor").clicked() {
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Inductor(1.0));
                }
                if ui.button("Capacitor").clicked() {
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Capacitor(10e-6));
                }
                if ui.button("Diode").clicked() {
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Diode);
                }
                if ui.button("Battery").clicked() {
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Battery(5.0));
                }
                if ui.button("Switch").clicked() {
                    self.editor.new_twoterminal(pos, TwoTerminalComponent::Switch(true));
                }
                if ui.button("PNP").clicked() {
                    self.editor
                        .new_threeterminal(pos, ThreeTerminalComponent::PTransistor(100.0));
                }
                if ui.button("NPN").clicked() {
                    self.editor
                        .new_threeterminal(pos, ThreeTerminalComponent::NTransistor(100.0));
                }
                /*
                if ui.button("Delete").clicked() {
                    self.editor.delete();
                }
                ui.checkbox(&mut self.debug_draw, "Debug draw");
                */
            });
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
