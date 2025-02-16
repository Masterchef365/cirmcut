use std::{ops::Range, time::Instant};

use ndarray::{linalg::kron, Array1, Array2};
use rsparse::{data::{Sprs, Trpl}, lusol, sprs_print};

use crate::{PrimitiveDiagram, SimOutputs, TwoTerminalComponent};

pub struct Solver {
    map: PrimitiveDiagramMapping,
    soln_vector: Vec<f64>,
}

/// Maps indices of the state vector (x from Ax = b) to the corresponding component voltages,
/// currents, etc.
#[derive(Default)]
struct PrimitiveDiagramStateVectorMapping {
    n_currents: usize,
    n_voltage_drops: usize,
    n_voltages: usize,
}

/// Maps indices of the parameters (known values such as input voltage or current or signal).
/// These are the known variables, or b from Ax = b.
#[derive(Default)]
struct PrimitiveDiagramParameterMapping {
    n_components: usize,
    n_current_laws: usize,
    n_voltage_laws: usize,
}

/// Represents the mappings needed to work with either the state vector or the parameter map
struct PrimitiveDiagramMapping {
    state_map: PrimitiveDiagramStateVectorMapping,
    param_map: PrimitiveDiagramParameterMapping,
}

impl PrimitiveDiagramMapping {
    fn new(diagram: &PrimitiveDiagram) -> Self {
        Self {
            state_map: PrimitiveDiagramStateVectorMapping::new(diagram),
            param_map: PrimitiveDiagramParameterMapping::new(diagram),
        }
    }

    fn vector_size(&self) -> usize {
        debug_assert_eq!(self.state_map.total_len(), self.param_map.total_len());
        self.state_map.total_len()
    }
}

impl PrimitiveDiagramParameterMapping {
    fn new(diagram: &PrimitiveDiagram) -> Self {
        Self {
            n_components: diagram.two_terminal.len(),
            n_voltage_laws: diagram.two_terminal.len(),
            n_current_laws: diagram.num_nodes.saturating_sub(1),
        }
    }

    fn components(&self) -> Range<usize> {
        0..self.n_components
    }

    fn current_laws(&self) -> Range<usize> {
        let base = self.components().end;
        base..base + self.n_current_laws
    }

    fn voltage_laws(&self) -> Range<usize> {
        let base = self.current_laws().end;
        base..base + self.n_voltage_laws
    }

    fn total_len(&self) -> usize {
        self.n_current_laws + self.n_voltage_laws + self.n_components
    }
}

impl PrimitiveDiagramStateVectorMapping {
    fn new(diagram: &PrimitiveDiagram) -> Self {
        Self {
            n_currents: diagram.two_terminal.len(),
            n_voltage_drops: diagram.two_terminal.len(),
            n_voltages: diagram.num_nodes.saturating_sub(1),
        }
    }

    fn currents(&self) -> Range<usize> {
        0..self.n_currents
    }

    fn voltage_drops(&self) -> Range<usize> {
        let base = self.currents().end;
        base..base + self.n_voltage_drops
    }

    fn voltages(&self) -> Range<usize> {
        let base = self.voltage_drops().end;
        base..base + self.n_voltages
    }

    fn total_len(&self) -> usize {
        self.n_currents + self.n_voltages + self.n_voltage_drops
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct NewtonRaphsonConfig {
    pub max_nr_iters: usize, 
    pub step_size: f64,
    /// NR-Iterate until error reaches this value
    pub nr_tolerance: f64,
    /// When solving F Delta x = -f, which tolerance do we solve the system to?
    pub dx_soln_tolerance: f64,
}

impl Solver {
    pub fn new(diagram: &PrimitiveDiagram) -> Self {
        let map = PrimitiveDiagramMapping::new(diagram);

        Self {
            soln_vector: vec![0.0; map.vector_size()],
            map,
        }
    }

    /// Note: Assumes diagram is compatible with the one this solver was created with!
    pub fn step(&mut self, dt: f64, diagram: &PrimitiveDiagram, cfg: &NewtonRaphsonConfig) -> Result<(), String> {
        let prev_time_step_soln = &self.soln_vector;

        let mut new_state = [prev_time_step_soln.clone()];

        let mut last_err = 9e99;
        for _ in 0..cfg.max_nr_iters {
            // Calculate A(w_n(K)), b(w_n(K))
            let (matrix, params) = stamp(dt, &self.map, diagram, &new_state[0]);

            if params.len() == 0 {
                return Ok(());
            }

            let mut dense_b = Trpl::new();
            for (i, val) in params.iter().enumerate() {
                dense_b.append(i, 0, *val);
            }
            let dense_b = dense_b.to_sprs();


            let mut new_state_sparse = Trpl::new();
            for (i, val) in new_state[0].iter().enumerate() {
                new_state_sparse.append(i, 0, *val);
            }
            let new_state_sparse = new_state_sparse.to_sprs();

            // Calculate -f(w_n(K)) = b(w_n(K)) - A(w_n(K)) w_n(K)
            let ax = &matrix * &new_state_sparse;
            let f = dense_b - ax;

            // Solve A(w_n(K)) dw = -f for dw
            let mut delta: Vec<f64> = f.to_dense().iter().flatten().copied().collect();
            lusol(&matrix, &mut delta, -1, 1e-3)?;

            // dw dot dw
            let err = delta.iter().map(|f| f*f).sum::<f64>();
            dbg!(err);

            if err > last_err {
                return Err("Error value increased!".to_string());
                //eprintln!("Error value increased! {}", err - last_err);
            }

            // w += dw * step size
            new_state[0].iter_mut().zip(&delta).for_each(|(n, delta)| *n += delta * cfg.step_size);

            if err < cfg.nr_tolerance {
                break;
            }

            last_err = err;
        }

        [self.soln_vector] = new_state;

        Ok(())
    }

    pub fn state(&self, diagram: &PrimitiveDiagram) -> SimOutputs {
        let mut voltages = self.soln_vector[self.map.state_map.voltages()].to_vec();
        // Last node voltage is ground!
        voltages.push(0.0);

        let two_terminal_current = self.soln_vector[self.map.state_map.currents()].to_vec();

        // TODO: Transistors!
        let three_terminal_current = diagram.three_terminal.iter().map(|_| [0.0; 3]).collect();

        SimOutputs {
            voltages,
            two_terminal_current,
            three_terminal_current,
        }
    }
}

fn stamp(dt: f64, map: &PrimitiveDiagramMapping, diagram: &PrimitiveDiagram, last_state: &[f64]) -> (Sprs, Vec<f64>) {
    let n = map.vector_size();

    // (params, state)
    let mut matrix = Trpl::new();
    let mut params = vec![0_f64; n];

    // TODO: Three-terminal components

    // Stamp current laws
    for (component_idx, &(node_indices, _component)) in diagram.two_terminal.iter().enumerate()
    {
        let [begin_node_idx, end_node_idx] = node_indices;

        let current_idx = map.state_map.currents().nth(component_idx).unwrap();
        if let Some(end_current_law_idx) = map.param_map.current_laws().nth(end_node_idx) {
            matrix.append(end_current_law_idx, current_idx, 1.0);
        }
        if let Some(begin_current_law_idx) =
            map.param_map.current_laws().nth(begin_node_idx)
        {
            matrix.append(begin_current_law_idx, current_idx, -1.0);
        }
    }

    // Stamp voltage laws
    for (component_idx, &(node_indices, _component)) in diagram.two_terminal.iter().enumerate()
    {
        let [begin_node_idx, end_node_idx] = node_indices;

        let voltage_law_idx = 
            map
            .param_map
            .voltage_laws()
            .nth(component_idx)
            .unwrap();
        let voltage_drop_idx = 
            map
            .state_map
            .voltage_drops()
            .nth(component_idx)
            .unwrap();

        matrix.append(voltage_law_idx, voltage_drop_idx, 1.0);
        if let Some(end_voltage_idx) = map.state_map.voltages().nth(end_node_idx) {
            matrix.append(voltage_law_idx, end_voltage_idx, 1.0);
        }

        if let Some(begin_voltage_idx) = map.state_map.voltages().nth(begin_node_idx) {
            matrix.append(voltage_law_idx, begin_voltage_idx, -1.0);
        }
    }

    // Stamp components
    for (i, &(node_indices, component)) in diagram.two_terminal.iter().enumerate() {
        let component_idx = map.param_map.components().nth(i).unwrap();

        let current_idx = map.state_map.currents().nth(i).unwrap();
        let voltage_drop_idx = map.state_map.voltage_drops().nth(i).unwrap();

        match component {
            TwoTerminalComponent::Resistor(resistance) => {
                matrix.append(component_idx, current_idx, -resistance);
                matrix.append(component_idx, voltage_drop_idx, 1.0);
            }
            TwoTerminalComponent::Wire => {
                // Vd = 0
                //matrix.append(component_idx, voltage_drop_idx, 1.0);
                let [begin_node_idx, end_node_idx] = node_indices;

                if let Some(voltage_idx) = map.state_map.voltages().nth(end_node_idx) {
                    matrix.append(component_idx, voltage_idx, 1.0);
                }

                if let Some(voltage_idx) = map.state_map.voltages().nth(begin_node_idx) {
                    matrix.append(component_idx, voltage_idx, -1.0);
                }
            }
            TwoTerminalComponent::Switch(is_open) => {
                // Vd = 0
                //matrix.append(component_idx, voltage_drop_idx, 1.0);
                let [begin_node_idx, end_node_idx] = node_indices;

                if is_open {
                    // Set current through this component to zero
                    matrix.append(component_idx, current_idx, 1.0);
                } else {
                    // Set voltages of connected nodes to be equal
                    if let Some(voltage_idx) = map.state_map.voltages().nth(end_node_idx) {
                        matrix.append(component_idx, voltage_idx, 1.0);
                    }

                    if let Some(voltage_idx) = map.state_map.voltages().nth(begin_node_idx)
                    {
                        matrix.append(component_idx, voltage_idx, -1.0);
                    }
                }
            }
            TwoTerminalComponent::Battery(voltage) => {
                matrix.append(component_idx, voltage_drop_idx, -1.0);
                params[component_idx] = voltage;
            }
            TwoTerminalComponent::Capacitor(capacitance) => {
                matrix.append(component_idx, current_idx, -dt);
                matrix.append(component_idx, voltage_drop_idx, capacitance);
                params[component_idx] = last_state[voltage_drop_idx] * capacitance;
            }
            TwoTerminalComponent::Inductor(inductance) => {
                matrix.append(component_idx, voltage_drop_idx, dt);
                matrix.append(component_idx, current_idx, -inductance);
                params[component_idx] = -last_state[current_idx] * inductance;
            }
            TwoTerminalComponent::Diode => {
                // Stolen from falstad.
                //let sat_current = 171.4352819281e-9;
                let n = 2.0;
                let temperature = 273.15 + 22.0;
                let thermal_voltage = 8.617e-5 * temperature;
                let vt = thermal_voltage / n;

                let last_voltage = last_state[voltage_drop_idx];
                let vn = last_voltage / vt;

                params[component_idx] = 1.0 - last_state[voltage_drop_idx];
                matrix.append(component_idx, voltage_drop_idx, -1.0);
                matrix.append(component_idx, current_idx, (-vn).exp());
            }
            other => eprintln!("{other:?} is not supported yet!!"),
        }
    }

    (matrix.to_sprs(), params)
}
