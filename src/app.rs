use std::{
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

use cirmcut_sim::{
    solver::{Solver, SolverConfig, SolverMode}, PrimitiveDiagram, SimOutputs, ThreeTerminalComponent, TwoTerminalComponent
};
use egui::{Color32, DragValue, Key, Layout, Pos2, Rect, RichText, ScrollArea, Vec2, ViewportCommand};

use crate::circuit_widget::{
    draw_grid, egui_to_cellpos, Diagram, DiagramEditor, DiagramState, DiagramWireState,
    VisualizationOptions,
};

pub struct CircuitApp {
    sim: Option<Solver>,
    error: Option<String>,
    save: CircuitAppSaveData,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CircuitAppSaveData {
    view_rect: Rect,
    editor: DiagramEditor,
    debug_draw: bool,
    current_path: Option<PathBuf>,
    current_file: CircuitFile,
    vis_opt: VisualizationOptions,
    paused: bool,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CircuitFile {
    diagram: Diagram,
    cfg: SolverConfig,
    dt: f64,
}

impl Default for CircuitAppSaveData {
    fn default() -> Self {
        Self {
            vis_opt: VisualizationOptions::default(),
            editor: DiagramEditor::new(),
            current_file: ron::from_str(include_str!("colpitts2.ckt")).unwrap_or_default(),
            paused: false,
            view_rect: Rect::from_center_size(Pos2::ZERO, Vec2::splat(1000.0)),
            debug_draw: false,
            current_path: None,
        }
    }
}

impl CircuitApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let save = cc.storage.and_then(|storage| eframe::get_value(storage, eframe::APP_KEY)).unwrap_or_default();

        Self {
            save,
            error: None,
            sim: None,
        }
    }

    fn state(&self) -> Option<DiagramState> {
        self.sim.as_ref().map(|sim| {
            let diag = self.save.current_file.diagram.to_primitive_diagram();
            solver_to_diagramstate(sim.state(&diag), &diag)
        })
    }

    fn save_file(&mut self, ctx: &egui::Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let maybe_path = match &self.save.current_path {
                Some(current) => Some(current.clone()),
                None => rfd::FileDialog::new()
                    .add_filter("CKT", &["ckt"])
                    .save_file(),
            };

            if let Some(mut path) = maybe_path {
                if path.extension() != Some(OsStr::new("ckt")) {
                    path.set_extension("ckt");
                }

                write_file(&self.save.current_file, &path);
            }

            //self.save.update_title(ctx);
        }
    }

    fn open_file(&mut self, ctx: &egui::Context) {
        //self.save.save_file(ctx);

        #[cfg(not(target_arch = "wasm32"))]
        {
            let maybe_path = match &self.save.current_path {
                Some(current) => Some(current.clone()),
                None => rfd::FileDialog::new()
                    .add_filter("CKT", &["ckt"])
                    .pick_file(),
            };

            if let Some(path) = maybe_path {
                if let Some(data) = read_file(&path) {
                    self.save.current_file = data;
                    self.sim = None;
                }
            }

            self.update_title(ctx);
        }
    }

    fn update_title(&self, ctx: &egui::Context) {
        if let Some(path) = self.save.current_path.as_ref().and_then(|file| file.to_str()) {
            ctx.send_viewport_cmd(ViewportCommand::Title(format!("Cirmcut {path}")));
        }
    }
}

impl eframe::App for CircuitApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.save);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.save.current_file = CircuitFile::default();
                        self.sim = None;
                    }
                    ui.separator();
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if ui.button("Open").clicked() {
                            self.open_file(ui.ctx());
                        }
                        if ui.button("Save").clicked() {
                            self.save_file(ui.ctx());
                        }
                        ui.separator();
                    }

                    if ui.button("Load Example circuit").clicked() {
                        self.save.current_file = CircuitAppSaveData::default().current_file;
                        self.sim = None;
                    }
                    egui::widgets::global_theme_preference_buttons(ui);
                });

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to("Source code on GitHub", "https://github.com/Masterchef365/cirmcut");
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
                let text = if self.save.paused { "Run" } else { "Pause" };
                ui.horizontal(|ui| {
                    if ui.button(text).clicked() {
                        self.save.paused ^= true;
                    }
                    if self.save.paused {
                        single_step |= ui.button("Single-step").clicked();
                    }
                });

                rebuild_sim |= ui.button("Reset").clicked();

                ui.add(
                    DragValue::new(&mut self.save.current_file.dt)
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
                    DragValue::new(&mut self.save.current_file.cfg.max_nr_iters)
                        .prefix("Max NR iters: "),
                );
                ui.horizontal(|ui| {
                    ui.add(
                        DragValue::new(&mut self.save.current_file.cfg.nr_step_size)
                        .speed(1e-6)
                        .prefix("Initial NR step size: "),
                    );
                    ui.checkbox(
                        &mut self.save.current_file.cfg.adaptive_step_size,
                        "Adaptive",
                    );
                });

                ui.add(
                    DragValue::new(&mut self.save.current_file.cfg.nr_tolerance)
                        .speed(1e-6)
                        .prefix("NR tolerance: "),
                );
                ui.add(
                    DragValue::new(&mut self.save.current_file.cfg.dx_soln_tolerance)
                        .speed(1e-6)
                        .prefix("Matrix solve tol: "),
                );

                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.save.current_file.cfg.mode,
                        SolverMode::NewtonRaphson,
                        "Newton-Raphson",
                    );
                    ui.selectable_value(
                        &mut self.save.current_file.cfg.mode,
                        SolverMode::Linear,
                        "Linear",
                    );
                });

                if ui.button("Default cfg").clicked() {
                    self.save.current_file.cfg = Default::default();
                }

                ui.separator();

                if let Some(state) = &state {
                    rebuild_sim |=
                        self.save.editor
                            .edit_component(ui, &mut self.save.current_file.diagram, state);
                }

                ui.separator();
                ui.strong("Visualization");
                ui.add(
                    DragValue::new(&mut self.save.vis_opt.voltage_scale)
                        .prefix("Voltage scale: ")
                        .speed(1e-2),
                );
                ui.add(
                    DragValue::new(&mut self.save.vis_opt.current_scale)
                        .prefix("Current scale: ")
                        .speed(1e-2),
                );
                if ui.button("Auto scale").clicked() {
                    if let Some(state) = &state {
                        let all_wires = state.two_terminal.iter().copied().flatten();
                        self.save.vis_opt.voltage_scale = all_wires
                            .clone()
                            .map(|wire| wire.voltage.abs())
                            .max_by(|a, b| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap_or(VisualizationOptions::default().voltage_scale);
                        self.save.vis_opt.current_scale = all_wires
                            .map(|wire| wire.current.abs())
                            .max_by(|a, b| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))
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
                    let pos = egui_to_cellpos(self.save.view_rect.center());
                    if ui.button("Wire").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_twoterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Wire,
                        );
                    }
                    if ui.button("Resistor").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_twoterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Resistor(1000.0),
                        );
                    }
                    if ui.button("Inductor").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_twoterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Inductor(1.0, None),
                        );
                    }
                    if ui.button("Capacitor").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_twoterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Capacitor(10e-6),
                        );
                    }
                    if ui.button("Diode").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_twoterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Diode,
                        );
                    }
                    if ui.button("Battery").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_twoterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Battery(5.0),
                        );
                    }
                    if ui.button("Switch").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_twoterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            TwoTerminalComponent::Switch(true),
                        );
                    }
                    if ui.button("Current source").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_twoterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            TwoTerminalComponent::CurrentSource(0.1),
                        );
                    }
                    if ui.button("PNP").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_threeterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            ThreeTerminalComponent::PTransistor(100.0),
                        );
                    }
                    if ui.button("NPN").clicked() {
                        rebuild_sim = true;
                        self.save.editor.new_threeterminal(
                            &mut self.save.current_file.diagram,
                            pos,
                            ThreeTerminalComponent::NTransistor(100.0),
                        );
                    }
                    /*
                    if ui.button("Delete").clicked() {
                        self.save.editor.delete();
                    }
                    ui.checkbox(&mut self.debug_draw, "Debug draw");
                    */
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let rect = self.save.view_rect;
                let resp = egui::Scene::new().show(ui, &mut self.save.view_rect, |ui| {
                    draw_grid(ui, rect, 1.0, Color32::DARK_GRAY);
                    if let Some(state) = state {
                        rebuild_sim |= self.save.editor.edit(
                            ui,
                            &mut self.save.current_file.diagram,
                            &state,
                            self.save.debug_draw,
                            &self.save.vis_opt,
                        );
                    }
                });

                if ui.input(|r| r.key_pressed(Key::Delete)) {
                    rebuild_sim = true;
                    self.save.editor.delete(&mut self.save.current_file.diagram);
                }

                if resp.response.clicked() || ui.input(|r| r.key_pressed(Key::Escape)) {
                    self.save.editor.reset_selection();
                }
            });
        });

        // Reset
        if rebuild_sim {
            self.sim = Some(Solver::new(
                &self.save.current_file.diagram.to_primitive_diagram(),
            ));
        }

        if !self.save.paused || rebuild_sim || single_step {
            ctx.request_repaint();

            if let Some(sim) = &mut self.sim {
                //let start = std::time::Instant::now();
                if let Err(e) = sim.step(
                    self.save.current_file.dt,
                    &self.save.current_file.diagram.to_primitive_diagram(),
                    &self.save.current_file.cfg,
                ) {
                    eprintln!("{}", e);
                    self.error = Some(e);
                    self.save.paused = true;
                } else {
                    self.error = None;
                }
                //println!("Time: {:.03} ms = {:.03} fps", start.elapsed().as_secs_f32() * 1000.0, 1.0 / (start.elapsed().as_secs_f32()));
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
        three_terminal: output.three_terminal_current.iter().zip(&diagram
            .three_terminal)
            .map(|(&current, (indices, _))| {
                [0, 1, 2].map(|offset| DiagramWireState {
                    voltage: output.voltages[indices[offset]],
                    current: current[offset],
                })
            })
            .collect(),
    }
}

impl Default for CircuitFile {
    fn default() -> Self {
        Self {
            diagram: Diagram::default(),
            dt: 5e-3,
            cfg: Default::default(),
        }
    }
}
/*
 *

enum AudioCommand {
    Reset(CircuitFile),
    UpdateDiagram(CircuitFile),
    Select(Option<Selection>),
}

enum AudioReturn {
    State(DiagramState),
    Error(String),
}

struct InteractiveCircuitSource {
    rx: Receiver<AudioCommand>,
    tx: Sender<AudioReturn>,
    circuit_file: CircuitFile,
    sim: Solver,
    select: Option<Selection>,
    frame_timer: Instant,
}

impl InteractiveCircuitSource {
    fn new(
    rx: Receiver<AudioCommand>,
    tx: Sender<AudioReturn>,
    circuit_file: CircuitFile,
        ) -> Self {
        todo!()

    }
}

impl Iterator for InteractiveCircuitSource {
    type Item = f32;
    fn next(&mut self) -> Option<Self::Item> {
        let mut reset = false;
        for cmd in self.rx.try_iter() {
            match cmd {
                AudioCommand::Reset(f) => {
                    self.circuit_file = f;
                    reset = true;
                }
                AudioCommand::UpdateDiagram(f) => self.circuit_file = f,
                AudioCommand::Select(selection) => self.select = selection,
            }
        }

        let primitive = self.circuit_file.diagram.to_primitive_diagram();
        if reset {
            self.sim = Solver::new(&primitive);
        }

        if let Err(e) = self.sim.step(self.circuit_file.dt, &primitive, &self.circuit_file.cfg) {
            eprintln!("{:?}", e);
            return Some(0.0);
        }

        let state = solver_to_diagramstate(self.sim.state(&primitive), &primitive);

        if self.frame_timer.elapsed().as_millis() > 1000 / 24 {
            self.frame_timer = Instant::now();
            self.tx.send(AudioReturn::State(state.clone())).unwrap();
        }

        if let Some((idx, false)) = self.select {
            let [wire_a, wire_b] = state.two_terminal[idx];
            let dv = wire_b.voltage - wire_a.voltage;
            Some(dv as f32)
        } else {
            Some(0.)
        }
    }
}
*/
