use std::{
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

use cirmcut_sim::{
    dense_solver::{NewtonRaphsonConfig, Solver},
    PrimitiveDiagram, SimOutputs, ThreeTerminalComponent, TwoTerminalComponent,
};
use egui::{
    Align2, Color32, DragValue, Id, Key, Pos2, Rect, Response, RichText, ScrollArea, Sense, Stroke,
    Ui, Vec2, ViewportCommand,
};

use crate::circuit_widget::{
    cellpos_to_egui, draw_grid, egui_to_cellpos, Diagram, DiagramEditor, DiagramState,
    DiagramWireState, VisualizationOptions,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CircuitApp {
    view_rect: Rect,
    editor: DiagramEditor,
    debug_draw: bool,
    current_path: Option<PathBuf>,

    current_file: CircuitFile,
    vis_opt: VisualizationOptions,

    #[serde(skip)]
    sim: Option<Solver>,

    #[serde(skip)]
    error: Option<String>,

    paused: bool,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CircuitFile {
    diagram: Diagram,
    cfg: NewtonRaphsonConfig,
    dt: f64,
}

impl Default for CircuitApp {
    fn default() -> Self {
        Self {
            vis_opt: VisualizationOptions::default(),
            error: None,
            sim: None,
            editor: DiagramEditor::new(),
            current_file: CircuitFile::default(),
            paused: false,
            view_rect: Rect::from_center_size(Pos2::ZERO, Vec2::splat(1000.0)),
            debug_draw: false,
            current_path: None,
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

    fn state(&self) -> Option<DiagramState> {
        self.sim.as_ref().map(|sim| {
            let diag = self.current_file.diagram.to_primitive_diagram();
            solver_to_diagramstate(sim.state(&diag), &diag)
        })
    }

    fn save_file(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let maybe_path = match &self.current_path {
                Some(current) => Some(current.clone()),
                None => rfd::FileDialog::new()
                    .add_filter("CKT", &["ckt"])
                    .save_file(),
            };

            if let Some(mut path) = maybe_path {
                if path.extension() != Some(OsStr::new("ckt")) {
                    path.set_extension("ckt");
                }

                write_file(&self.current_file, &path);
            }

            self.update_title(ctx);
        }
    }

    fn open_file(&mut self, ctx: &egui::Context) {
        //self.save_file(ctx);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let maybe_path = match &self.current_path {
                Some(current) => Some(current.clone()),
                None => rfd::FileDialog::new()
                    .add_filter("CKT", &["ckt"])
                    .pick_file(),
            };

            if let Some(path) = maybe_path {
                if let Some(data) = read_file(&path) {
                    self.current_file = data;
                    self.sim = None;
                }
            }

            self.update_title(ctx);
        }
    }

    fn update_title(&self, ctx: &egui::Context) {
        if let Some(path) = self.current_path.as_ref().and_then(|file| file.to_str()) {
            ctx.send_viewport_cmd(ViewportCommand::Title(format!("Cirmcut {path}")));
        }
    }
}

impl eframe::App for CircuitApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        //ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.current_file = CircuitFile::default();
                        self.sim = None;
                    }
                    ui.separator();
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

        let mut rebuild_sim = self.sim.is_none();

        // TODO: Cache this?
        let state = self.state();

        let mut single_step = false;

        egui::SidePanel::left("cfg").show(ctx, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.strong("Simulation");
                let text = if self.paused { "Run" } else { "Pause" };
                ui.horizontal(|ui| {
                    if ui.button(text).clicked() {
                        self.paused ^= true;
                    }
                    if self.paused {
                        single_step |= ui.button("Single-step").clicked();
                    }
                });

                rebuild_sim |= ui.button("Reset").clicked();

                ui.add(
                    DragValue::new(&mut self.current_file.dt)
                        .prefix("dt: ")
                        .speed(1e-7)
                        .suffix(" s"),
                );

                if let Some(error) = &self.error {
                    ui.label(RichText::new(error).color(Color32::RED));
                }

                ui.separator();
                ui.strong("Advanced");

                ui.add(
                    DragValue::new(&mut self.current_file.cfg.max_nr_iters)
                        .prefix("Max NR iters: "),
                );
                ui.add(
                    DragValue::new(&mut self.current_file.cfg.nr_step_size)
                        .speed(1e-6)
                        .prefix("NR step size: "),
                );
                ui.add(
                    DragValue::new(&mut self.current_file.cfg.nr_tolerance)
                        .speed(1e-6)
                        .prefix("NR tolerance: "),
                );
                ui.add(
                    DragValue::new(&mut self.current_file.cfg.dx_soln_tolerance)
                        .speed(1e-6)
                        .prefix("Matrix solve tol: "),
                );

                ui.separator();

                if let Some(state) = &state {
                    rebuild_sim |=
                        self.editor
                            .edit_component(ui, &mut self.current_file.diagram, state);
                }

                ui.separator();
                ui.strong("Visualization");
                ui.add(
                    DragValue::new(&mut self.vis_opt.voltage_scale)
                        .prefix("Voltage scale: ")
                        .speed(1e-2),
                );
                ui.add(
                    DragValue::new(&mut self.vis_opt.current_scale)
                        .prefix("Current scale: ")
                        .speed(1e-2),
                );
                if ui.button("Auto scale").clicked() {
                    if let Some(state) = &state {
                        let all_wires = state.two_terminal.iter().copied().flatten();
                        self.vis_opt.voltage_scale = all_wires
                            .clone()
                            .map(|wire| wire.voltage)
                            .max_by(|a, b| {
                                a.abs()
                                    .partial_cmp(&b.abs())
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .unwrap_or(VisualizationOptions::default().voltage_scale);
                        self.vis_opt.current_scale = all_wires
                            .map(|wire| wire.current)
                            .max_by(|a, b| {
                                a.abs()
                                    .partial_cmp(&b.abs())
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .unwrap_or(VisualizationOptions::default().current_scale);
                    }
                    //self.vis_opt.voltage_scale =
                }
            });
        });

        egui::TopBottomPanel::bottom("buttons").show(ctx, |ui| {
            ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Add component: ");
                    let pos = egui_to_cellpos(self.view_rect.center());
                    if ui.button("Wire").clicked() {
                        rebuild_sim = true;
                        self.editor.new_twoterminal(
                            &mut self.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Wire,
                        );
                    }
                    if ui.button("Resistor").clicked() {
                        rebuild_sim = true;
                        self.editor.new_twoterminal(
                            &mut self.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Resistor(1000.0),
                        );
                    }
                    if ui.button("Inductor").clicked() {
                        rebuild_sim = true;
                        self.editor.new_twoterminal(
                            &mut self.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Inductor(1.0),
                        );
                    }
                    if ui.button("Capacitor").clicked() {
                        rebuild_sim = true;
                        self.editor.new_twoterminal(
                            &mut self.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Capacitor(10e-6),
                        );
                    }
                    if ui.button("Diode").clicked() {
                        rebuild_sim = true;
                        self.editor.new_twoterminal(
                            &mut self.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Diode,
                        );
                    }
                    if ui.button("Battery").clicked() {
                        rebuild_sim = true;
                        self.editor.new_twoterminal(
                            &mut self.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Battery(5.0),
                        );
                    }
                    if ui.button("Switch").clicked() {
                        rebuild_sim = true;
                        self.editor.new_twoterminal(
                            &mut self.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Switch(true),
                        );
                    }
                    if ui.button("Current source").clicked() {
                        rebuild_sim = true;
                        self.editor.new_twoterminal(
                            &mut self.current_file.diagram,
                            pos,
                            TwoTerminalComponent::CurrentSource(0.1),
                        );
                    }
                    /*if ui.button("PNP").clicked() {
                        rebuild_sim = true;
                        self.editor.new_threeterminal(
                            &mut self.current_file.diagram,
                            pos,
                            ThreeTerminalComponent::PTransistor(100.0),
                        );
                    }
                    if ui.button("NPN").clicked() {
                        rebuild_sim = true;
                        self.editor.new_threeterminal(
                            &mut self.current_file.diagram,
                            pos,
                            ThreeTerminalComponent::NTransistor(100.0),
                        );
                    }*/
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
                    if let Some(state) = state {
                        rebuild_sim |= self.editor.edit(
                            ui,
                            &mut self.current_file.diagram,
                            &state,
                            self.debug_draw,
                            &self.vis_opt,
                        );
                    }
                });

                if ui.input(|r| r.key_pressed(Key::Delete)) {
                    rebuild_sim = true;
                    self.editor.delete(&mut self.current_file.diagram);
                }

                if resp.response.clicked() || ui.input(|r| r.key_pressed(Key::Escape)) {
                    self.editor.reset_selection();
                }
            });
        });

        // Reset
        if rebuild_sim {
            self.sim = Some(Solver::new(
                &self.current_file.diagram.to_primitive_diagram(),
            ));
        }

        if !self.paused || rebuild_sim || single_step {
            ctx.request_repaint();

            if let Some(sim) = &mut self.sim {
                if let Err(e) = sim.step(
                    self.current_file.dt,
                    &self.current_file.diagram.to_primitive_diagram(),
                    &self.current_file.cfg,
                ) {
                    eprintln!("{}", e);
                    self.error = Some(e);
                    self.paused = true;
                } else {
                    self.error = None;
                }
            }
        }
    }
}

fn read_file(path: &Path) -> Option<CircuitFile> {
    let file = File::open(path).ok()?;
    ron::de::from_reader(file).ok()
}

fn write_file(diagram: &CircuitFile, path: &Path) {
    // TODO: Show dialog on fail.
    let file = match File::create(path) {
        Err(e) => {
            eprintln!("{e}");
            return;
        }
        Ok(f) => f,
    };

    match ron::ser::to_writer(&file, diagram) {
        Err(e) => {
            eprintln!("{e}");
            return;
        }
        Ok(()) => (),
    };
}

fn solver_to_diagramstate(output: SimOutputs, diagram: &PrimitiveDiagram) -> DiagramState {
    DiagramState {
        two_terminal: output
            .two_terminal_current
            .iter()
            .zip(&diagram.two_terminal)
            .map(|(&current, (indices, _))| {
                indices.map(|idx| DiagramWireState {
                    voltage: output.voltages[idx],
                    current,
                })
            })
            .collect(),
        three_terminal: diagram
            .three_terminal
            .iter()
            .map(|(indices, _)| indices.map(|_| DiagramWireState::default()))
            .collect(),
    }
}

impl Default for CircuitFile {
    fn default() -> Self {
        Self {
            diagram: Diagram::default(),
            dt: 5e-3,
            cfg: NewtonRaphsonConfig {
                dx_soln_tolerance: 1e-3,
                nr_tolerance: 1e-9,
                nr_step_size: 1e-2,
                max_nr_iters: 200,
            },
        }
    }
}
