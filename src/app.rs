use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    path::{Path, PathBuf},
};

use cirmcut_sim::{
    solver::{Solver, SolverConfig, SolverMode},
    stamp::stamp,
    PrimitiveDiagram, SimOutputs, ThreeTerminalComponent, TwoTerminalComponent,
};
use egui::{
    Color32, DragValue, Key, Layout, Pos2, Rect, RichText, ScrollArea, Ui, Vec2, ViewportCommand,
};

use crate::circuit_widget::{
    draw_grid, draw_twoterminal_component, draw_twoterminal_component_no_value, egui_to_cellpos,
    show_add_component_buttons, Diagram, DiagramEditor, DiagramState, DiagramWireState,
    SelectionType, VisualizationOptions,
};

/// (capitalized/shift, key, component)
const TWO_TERMINAL_SHORTCUTS: [(bool, Key, TwoTerminalComponent); 8] = [
    (false, Key::W, TwoTerminalComponent::Wire),
    (true, Key::L, TwoTerminalComponent::Inductor(1.0, None)),
    (false, Key::R, TwoTerminalComponent::Resistor(1000.0)),
    (false, Key::C, TwoTerminalComponent::Capacitor(1000.0)),
    (false, Key::D, TwoTerminalComponent::Diode),
    (false, Key::S, TwoTerminalComponent::Switch(false)),
    (false, Key::V, TwoTerminalComponent::Battery(5.0)),
    (false, Key::A, TwoTerminalComponent::CurrentSource(10e-3)),
];

#[derive(serde::Deserialize, serde::Serialize)]
pub struct CircuitApp {
    view_rect: Rect,
    editor: DiagramEditor,
    debug_draw: bool,
    current_path: Option<PathBuf>,
    show_matrix: bool,
    show_componentlist: bool,

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
    cfg: SolverConfig,
    dt: f64,
}

impl Default for CircuitApp {
    fn default() -> Self {
        Self {
            show_matrix: false,
            vis_opt: VisualizationOptions::default(),
            error: None,
            sim: None,
            editor: DiagramEditor::new(),
            current_file: ron::from_str(include_str!("colpitts2.ckt")).unwrap_or_default(),
            paused: false,
            view_rect: Rect::from_center_size(Pos2::ZERO, Vec2::splat(1000.0)),
            debug_draw: false,
            current_path: None,
            show_componentlist: false,
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
            DiagramState::new(&sim.state(&diag.primitive), &diag.primitive)
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
            ctx.send_viewport_cmd(ViewportCommand::Title(format!("Circuit {path}")));
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
                    if ui.button("New").clicked() {
                        self.current_file = CircuitFile::default();
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
                        self.current_file = Self::default().current_file;
                        self.sim = None;
                    }
                    egui::widgets::global_theme_preference_buttons(ui);
                });

                ui.menu_button("View", |ui| {
                    egui::Grid::new("viewgrid").show(ui, |ui| {
                        ui.label("Show matrix");
                        ui.checkbox(&mut self.show_matrix, "On");
                        ui.end_row();

                        ui.label("Show component list");
                        ui.checkbox(&mut self.show_componentlist, "On");
                        ui.end_row();

                        if ui.button("Reset viewbox").clicked() {
                            self.view_rect = Rect::ZERO;
                        }
                        ui.end_row();
                    });
                });

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.hyperlink_to(
                        "Source code on GitHub",
                        "https://github.com/Masterchef365/cirmcut",
                    );
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

                ui.horizontal(|ui| {
                    egui_simpletabs::play_pause_button(ui, &mut self.paused);
                    single_step |= egui_simpletabs::single_step_button(ui).clicked();
                    rebuild_sim |= egui_simpletabs::reset_step_button(ui).clicked();
                });

                ui.horizontal(|ui| {
                    ui.label("Δt: ");
                    ui.add(egui_simpletabs::edit_metric_f64(
                        &mut self.current_file.dt,
                        "s",
                    ));
                });

                if let Some(error) = &self.error {
                    ui.label(RichText::new(error).color(Color32::RED));
                }

                ui.collapsing("Advanced", |ui| {
                    ui.add(
                        DragValue::new(&mut self.current_file.cfg.max_nr_iters)
                            .prefix("Max NR iters: "),
                    );
                    ui.horizontal(|ui| {
                        ui.add(
                            DragValue::new(&mut self.current_file.cfg.nr_step_size)
                                .speed(1e-6)
                                .prefix("Initial NR step size: "),
                        );
                        ui.checkbox(&mut self.current_file.cfg.adaptive_step_size, "Adaptive");
                    });

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

                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.current_file.cfg.mode,
                            SolverMode::NewtonRaphson,
                            "Newton-Raphson",
                        );
                        ui.selectable_value(
                            &mut self.current_file.cfg.mode,
                            SolverMode::Linear,
                            "Linear",
                        );
                    });

                    if ui.button("Default cfg").clicked() {
                        self.current_file.cfg = Default::default();
                    }
                });

                ui.separator();
                ui.strong("Visualization");
                ui.add(
                    egui_simpletabs::edit_metric_f64(&mut self.vis_opt.voltage_scale, "V")
                        .prefix("Voltage scale: ")
                        .speed(1e-2),
                );
                ui.add(
                    egui_simpletabs::edit_metric_f64(&mut self.vis_opt.current_scale, "A")
                        .prefix("Current scale: ")
                        .speed(1e-2),
                );
                if ui.button("Auto scale").clicked() {
                    if let Some(state) = &state {
                        let all_wires = state.two_terminal.iter().copied().flatten();
                        self.vis_opt.voltage_scale = all_wires
                            .clone()
                            .map(|wire| wire.voltage.abs())
                            .max_by(|a, b| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap_or(VisualizationOptions::default().voltage_scale);
                        self.vis_opt.current_scale = all_wires
                            .map(|wire| wire.current.abs())
                            .max_by(|a, b| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap_or(VisualizationOptions::default().current_scale);
                    }
                    //self.vis_opt.voltage_scale =
                }
            });
        });

        if let Some(state) = &state {
            egui::SidePanel::right("component").show(ctx, |ui| {
                ui.strong("Component");
                rebuild_sim |=
                    self.editor
                        .edit_component(ui, &mut self.current_file.diagram, state);
            });
        }

        if self.show_matrix {
            egui::Window::new("Matrix").open(&mut self.show_matrix).show(ctx, |ui| {
                ui.heading("Matrix");
                if let Some(solver) = &self.sim {
                    let diagram = self.current_file.diagram.to_primitive_diagram();
                    let mut selection = None;
                    if let Some((idx, SelectionType::TwoTerminal)) = self.editor.selected {
                        selection = Some(idx);
                    }

                    if let Some((idx, SelectionType::ThreeTerminal)) = self.editor.selected {
                        selection = Some(idx + diagram.primitive.two_terminal.len());
                    }

                    show_parameter_matrix(
                        ui,
                        self.current_file.dt,
                        solver,
                        &diagram.primitive,
                        selection,
                    );
                }
            });
        }

        if self.show_componentlist {
            egui::Window::new("Component list").open(&mut self.show_componentlist).show(ctx, |ui| {
                ui.heading("Components");
                show_component_list(ui, &mut self.current_file.diagram, &mut self.editor);
            });
        }

        egui::TopBottomPanel::bottom("buttons").show(ctx, |ui| {
            ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    rebuild_sim |= show_add_component_buttons(
                        ui,
                        self.view_rect.center(),
                        &mut self.editor,
                        &mut self.current_file.diagram,
                    );
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

                // Delete
                if ui.input(|r| r.key_pressed(Key::Delete)) {
                    rebuild_sim = true;
                    self.editor.delete(&mut self.current_file.diagram);
                }

                // Reset selection
                if resp.response.clicked() || ui.input(|r| r.key_pressed(Key::Escape)) {
                    self.editor.reset_selection();
                }

                // Shortcuts
                if let Some(mouse_pos) = resp.response.hover_pos() {
                    for (shift, key, component) in TWO_TERMINAL_SHORTCUTS {
                        if ui.input(|r| r.key_pressed(key) && r.modifiers.shift == shift) {
                            self.editor.new_twoterminal(
                                &mut self.current_file.diagram,
                                egui_to_cellpos(mouse_pos),
                                component,
                            );
                            rebuild_sim = true;
                        }
                    }
                }

            });
        });

        // Reset
        if rebuild_sim {
            self.sim = Some(Solver::new(
                &self.current_file.diagram.to_primitive_diagram().primitive,
            ));
        }

        if !self.paused || rebuild_sim || single_step {
            ctx.request_repaint();

            if let Some(sim) = &mut self.sim {
                //let start = std::time::Instant::now();
                if let Err(e) = sim.step(
                    self.current_file.dt,
                    &self.current_file.diagram.to_primitive_diagram().primitive,
                    &self.current_file.cfg,
                    None,
                ) {
                    eprintln!("{}", e);
                    self.error = Some(e);
                    self.paused = true;
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

impl Default for CircuitFile {
    fn default() -> Self {
        Self {
            diagram: Diagram::default(),
            dt: 5e-3,
            cfg: Default::default(),
        }
    }
}

fn to_subscript(s: String) -> String {
    s.chars()
        .map(|c| {
            "₀₁₂₃₄₅₆₇₈₉"
                .chars()
                .nth(c.to_digit(10).unwrap_or(10) as usize)
                .unwrap_or(c)
        })
        .collect()
}

fn display_number(ui: &mut Ui, value: f64) {
    if value == 0.0 {
        ui.weak("0");
        return;
    }

    if value == 1.0 {
        ui.strong("1");
        return;
    }

    if value == -1.0 {
        ui.strong("-1");
        return;
    }

    ui.strong(format!("{value:.2e}"));
}

fn show_parameter_matrix(
    ui: &mut Ui,
    dt: f64,
    sim: &Solver,
    diagram: &PrimitiveDiagram,
    selected_idx: Option<usize>,
) {
    //let map: HashMap<usize, ()>;
    let (matrix, params) = stamp(
        dt,
        &sim.map,
        diagram,
        &sim.soln_vector,
        &sim.soln_vector,
        None,
    );
    // TODO: Slow!
    let dense = matrix.to_dense();

    let mut state_names = vec![];
    let mut parameter_names = vec![];

    let mut component_names = vec![];
    for (_, component) in diagram.two_terminal.iter() {
        component_names.push(component.name());
    }
    for (_, component) in diagram.three_terminal.iter() {
        component_names.push(component.name());
        component_names.push(component.name());
    }

    for (idx, _) in sim.map.param_map.components().enumerate() {
        parameter_names.push(component_names[idx].to_string());
    }
    for (idx, _) in sim.map.param_map.current_laws().enumerate() {
        parameter_names.push(format!("Current law {idx}"));
    }
    for (idx, _) in sim.map.param_map.voltage_laws().enumerate() {
        parameter_names.push(format!("Voltage law {idx}"));
    }

    for (idx, _) in sim.map.state_map.currents().enumerate() {
        state_names.push(to_subscript(format!("I{idx}")));
    }
    for (idx, _) in sim.map.state_map.voltage_drops().enumerate() {
        state_names.push(to_subscript(format!("Vd{idx}")));
    }
    for (idx, _) in sim.map.state_map.voltages().enumerate() {
        state_names.push(to_subscript(format!("V{idx}")));
    }

    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("circuitmatrix")
            .striped(true)
            .show(ui, |ui| {
                let n_cols = dense.get(0).map(|v| v.len()).unwrap_or(0);
                let nrows = dense.len();

                // Matrix
                ui.strong("Matrix");
                for col_idx in 0..n_cols {
                    ui.strong(&state_names[col_idx]);
                }

                // Multiply sign
                ui.label("");

                // Column headers for the rest
                ui.strong("Solution vector");
                ui.label("");

                // Equals sign
                ui.label("");

                ui.strong("Parameters");
                ui.label("");
                ui.end_row();

                for (row_idx, row) in dense.iter().enumerate() {
                    let old_fg_stroke = ui.style_mut().visuals.widgets.active.fg_stroke;
                    if selected_idx == Some(row_idx) {
                        ui.style_mut().visuals.override_text_color = Some(Color32::CYAN);
                        ui.style_mut().visuals.widgets.active.fg_stroke.color = Color32::CYAN;
                    }

                    // Matrix
                    ui.strong(&parameter_names[row_idx]);
                    for col in row {
                        display_number(ui, *col);
                    }

                    // Multiply sign
                    if row_idx == nrows / 2 {
                        ui.strong("x");
                    } else {
                        ui.label("");
                    }

                    // Solution vector
                    ui.weak(&state_names[row_idx]);
                    display_number(ui, sim.soln_vector[row_idx]);

                    // Equals sign
                    if row_idx == nrows / 2 {
                        ui.strong("=");
                    } else {
                        ui.label("");
                    }

                    // Parameters
                    ui.weak(&parameter_names[row_idx]);
                    display_number(ui, params[row_idx]);

                    ui.style_mut().visuals.override_text_color = None;
                    ui.style_mut().visuals.widgets.active.fg_stroke = old_fg_stroke;

                    ui.end_row();
                }
            });
    });
}

fn show_component_list(ui: &mut Ui, diagram: &mut Diagram, editor: &mut DiagramEditor) {
    ui.heading("Two terminal");
    let mut del_idx = None;
    egui::Grid::new("twoterminal").striped(true).show(ui, |ui| {
        ui.strong("Name");
        ui.strong("Location");
        ui.strong("Controls");
        ui.end_row();
        for (idx, (pos, comp)) in diagram.two_terminal.iter().enumerate() {
            ui.label(comp.name());
            ui.label(format!("{pos:?}"));
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    del_idx = Some(idx);
                }
                ui.selectable_value(&mut editor.selected, Some((idx, SelectionType::TwoTerminal)), "Select");
            });
            ui.end_row();
        }
    });
    if let Some(idx) = del_idx {
        diagram.two_terminal.remove(idx);
    }

    ui.heading("Three terminal");
    let mut del_idx = None;
    egui::Grid::new("threeterminal").striped(true).show(ui, |ui| {
        ui.strong("Name");
        ui.strong("Location");
        ui.strong("Controls");
        ui.end_row();
        for (idx, (pos, comp)) in diagram.three_terminal.iter().enumerate() {
            ui.label(comp.name());
            ui.label(format!("{pos:?}"));
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    del_idx = Some(idx);
                }
                ui.selectable_value(&mut editor.selected, Some((idx, SelectionType::ThreeTerminal)), "Select");
            });
            ui.end_row();
        }
    });
    if let Some(idx) = del_idx {
        diagram.three_terminal.remove(idx);
    }


    ui.heading("Ports");
    let mut del_idx = None;
    egui::Grid::new("ports").striped(true).show(ui, |ui| {
        ui.strong("Name");
        ui.strong("Location");
        ui.strong("Controls");
        ui.end_row();
        for (idx, (pos, comp)) in diagram.ports.iter_mut().enumerate() {
            ui.text_edit_singleline(comp);
            ui.label(format!("{pos:?}"));
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    del_idx = Some(idx);
                }
                ui.selectable_value(&mut editor.selected, Some((idx, SelectionType::Port)), "Select");
            });
            ui.end_row();
        }
    });
    if let Some(idx) = del_idx {
        diagram.ports.remove(idx);
    }

   //let mut del_idx = None;
}
