use cirmcut_sim::{CellPos, Component, ComponentState, DiagramCell, Orientation, WireState};
use egui::{Id, Rect, Vec2};

use crate::circuit_widget::{circuit_widget, ComponentButton};

//#[derive(serde::Deserialize, serde::Serialize)]
//#[serde(default)]
pub struct TemplateApp {
    selection: CellPos,
    scene_rect: Rect,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            scene_rect: Rect::ZERO,
            selection: (0, 0),
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            //return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
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

        egui::SidePanel::left("cfg").show(ctx, |ui| {
            egui::Grid::new("componentbuttons").show(ui, |ui| {
                for row in DEFAULT_COMPONENTS.chunks(2) {
                    for col in row {
                        let cell = DiagramCell { flip: false, orient: Orientation::Orig, comp: *col };
                        let state = ComponentState { top: WireState::default(), bottom: WireState::default(), left: WireState::default(), right: WireState::default() };
                        let size = 50.0;
                        if ui.add(ComponentButton::new(cell, state, size)).clicked() {

                        }
                    }
                    ui.end_row();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                circuit_widget(
                    &mut Default::default(),
                    &Default::default(),
                    &mut self.selection,
                    ui,
                    &mut self.scene_rect,
                )
            });
        });
    }
}

const DEFAULT_COMPONENTS: [Component; 2] = [
    Component::Wire,
    Component::Resistor(1000.0),
];
