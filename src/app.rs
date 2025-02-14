use std::{ffi::OsStr, fs::File, path::{Path, PathBuf}};

use cirmcut_sim::{ThreeTerminalComponent, TwoTerminalComponent};
use egui::{Color32, Id, Key, Pos2, Rect, Response, ScrollArea, Sense, Stroke, Ui, Vec2, ViewportCommand};

use crate::circuit_widget::{cellpos_to_egui, draw_grid, egui_to_cellpos, Diagram, DiagramEditor};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CircuitApp {
    view_rect: Rect,
    editor: DiagramEditor,
    debug_draw: bool,
    current_file: Option<PathBuf>,
}

impl Default for CircuitApp {
    fn default() -> Self {
        Self {
            editor: DiagramEditor::new(Diagram::default()),
            view_rect: Rect::from_center_size(Pos2::ZERO, Vec2::splat(1000.0)),
            debug_draw: false,
            current_file: None,
        }
    }
}

impl CircuitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        let inst = Self::default();
        inst.update_title(&cc.egui_ctx);

        inst
    }

    fn save_file(&mut self, ctx: &egui::Context) {
        let maybe_path = match &self.current_file {
            Some(current) => Some(current.clone()),
            None => rfd::FileDialog::new().add_filter("CKT", &["ckt"]).save_file(),
        };

        if let Some(mut path) = maybe_path {
            if path.extension() != Some(OsStr::new("ckt")) {
                path.set_extension("ckt");
            }

            write_file(&self.editor.diagram(), &path);
        }

        self.update_title(ctx);
    }

    fn open_file(&mut self, ctx: &egui::Context) {
        //self.save_file(ctx);

        let maybe_path = match &self.current_file {
            Some(current) => Some(current.clone()),
            None => rfd::FileDialog::new().add_filter("CKT", &["ckt"]).pick_file(),
        };

        if let Some(path) = maybe_path {
            if let Some(diagram) = read_file(&path) {
                self.editor = DiagramEditor::new(diagram);
            }
        }

        self.update_title(ctx);
    }

    fn update_title(&self, ctx: &egui::Context) {
        if let Some(path) = self.current_file.as_ref().and_then(|file| file.to_str()) {
            ctx.send_viewport_cmd(ViewportCommand::Title(format!("Cirmcut {path}")));
        }
    }
}

impl eframe::App for CircuitApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        self.open_file(ui.ctx());
                    }
                    if ui.button("Save").clicked() {
                        self.save_file(ui.ctx());
                    }
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });

        egui::SidePanel::left("cfg").show(ctx, |ui| {
            if ui.button("Print diagram").clicked() {
                dbg!(self.editor.diagram().to_primitive_diagram());
            }
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

                if ui.input(|r| r.key_pressed(Key::Delete)) {
                    self.editor.delete();
                }

                if resp.response.clicked() || ui.input(|r| r.key_pressed(Key::Escape)) {
                    self.editor.reset_selection();
                }
            });
        });
    }
}

fn read_file(path: &Path) -> Option<Diagram> {
    let file = File::open(path).ok()?;
    ron::de::from_reader(file).ok()
}

fn write_file(diagram: &Diagram, path: &Path) {
    // TODO: Show dialog on fail.
    let file = match File::create(path) {
        Err(e) => { eprintln!("{e}"); return; },
        Ok(f) => f,
    };
    
    match ron::ser::to_writer(&file, diagram) {
        Err(e) => { eprintln!("{e}"); return; },
        Ok(()) => (),
    };
}
