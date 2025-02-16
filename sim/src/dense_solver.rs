use std::ops::Range;

use ndarray::{Array1, Array2};
use ndarray_linalg::LeastSquaresSvd;

use crate::{PrimitiveDiagram, SimOutputs, TwoTerminalComponent};

pub struct Solver {
    diagram: PrimitiveDiagram,
    map: PrimitiveDiagramMapping,

    soln_vector: Vec<f32>,
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
            n_current_laws: diagram.num_nodes - 1,
        }
    }

    fn components(&self) -> Range<usize> {
        0..self.n_components
    }

    fn current_laws(&self) -> Range<usize> {
        let base = self.components().end;
        base .. base + self.n_current_laws
    }

    fn voltage_laws(&self) -> Range<usize> {
        let base = self.current_laws().end;
        base .. base + self.n_voltage_laws
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
            n_voltages: diagram.num_nodes - 1,
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

impl Solver {
    pub fn new(diagram: PrimitiveDiagram) -> Self {
        let map = PrimitiveDiagramMapping::new(&diagram);

        Self {
            soln_vector: vec![0.0; map.vector_size()],
            map,
            diagram,
        }
    }

    pub fn step(&mut self, dt: f32) {
        let n = self.map.vector_size();

        // (params, state)
        let mut matrix = Array2::<f32>::zeros((n, n));
        let mut param_vect = Array1::<f32>::zeros(n);

        // TODO: Three-terminal components 
        // Stamp current laws
        for (component_idx, &(node_indices, _component)) in self.diagram.two_terminal.iter().enumerate() {
            let [begin_node_idx, end_node_idx] = node_indices;

            let current_idx = self.map.state_map.currents().nth(component_idx).unwrap();
            if let Some(end_current_law_idx) = self.map.param_map.current_laws().nth(end_node_idx) {
                matrix[(end_current_law_idx, current_idx)] = 1.0;
            }
            if let Some(begin_current_law_idx) = self.map.param_map.current_laws().nth(begin_node_idx) {
                matrix[(begin_current_law_idx, current_idx)] = -1.0;
            }
        }

        // Stamp voltage laws
        for (component_idx, &(node_indices, _component)) in self.diagram.two_terminal.iter().enumerate() {
            let [begin_node_idx, end_node_idx] = node_indices;

            let voltage_law_idx = self.map.param_map.voltage_laws().nth(component_idx).unwrap();
            let voltage_drop_idx = self.map.state_map.voltage_drops().nth(component_idx).unwrap();

            matrix[(voltage_law_idx, voltage_drop_idx)] = 1.0;
            if let Some(end_voltage_idx) = self.map.state_map.voltages().nth(end_node_idx) {
                matrix[(voltage_law_idx, end_voltage_idx)] = 1.0;
            }

            if let Some(begin_voltage_idx) = self.map.state_map.voltages().nth(begin_node_idx) {
                matrix[(voltage_law_idx, begin_voltage_idx)] = -1.0;
            }
        }

        // Stamp components
        for (i, &(node_indices, component)) in self.diagram.two_terminal.iter().enumerate() {
            let component_idx = self.map.param_map.components().nth(i).unwrap();

            let current_idx = self.map.state_map.currents().nth(i).unwrap();
            let voltage_drop_idx = self.map.state_map.voltage_drops().nth(i).unwrap();

            match component {
                TwoTerminalComponent::Resistor(resistance) => {
                    matrix[(component_idx, current_idx)] = -resistance;
                    matrix[(component_idx, voltage_drop_idx)] = 1.0;
                },
                TwoTerminalComponent::Wire => {
                    // Vd = 0
                    //matrix[(component_idx, voltage_drop_idx)] = 1.0;
                    let [begin_node_idx, end_node_idx] = node_indices;

                    if let Some(voltage_idx) = self.map.state_map.voltages().nth(end_node_idx) {
                        matrix[(component_idx, voltage_idx)] = 1.0;
                    }

                    if let Some(voltage_idx) = self.map.state_map.voltages().nth(begin_node_idx) {
                        matrix[(component_idx, voltage_idx)] = -1.0;
                    }
                },
                TwoTerminalComponent::Switch(is_open) => {
                    // Vd = 0
                    //matrix[(component_idx, voltage_drop_idx)] = 1.0;
                    let [begin_node_idx, end_node_idx] = node_indices;

                    if is_open {
                        // Set current through this component to zero
                        matrix[(component_idx, current_idx)] = 1.0;
                    } else {
                        // Set voltages of connected nodes to be equal
                        if let Some(voltage_idx) = self.map.state_map.voltages().nth(end_node_idx) {
                            matrix[(component_idx, voltage_idx)] = 1.0;
                        }

                        if let Some(voltage_idx) = self.map.state_map.voltages().nth(begin_node_idx) {
                            matrix[(component_idx, voltage_idx)] = -1.0;
                        }
                    }
                },
                TwoTerminalComponent::Battery(voltage) => {
                    matrix[(component_idx, voltage_drop_idx)] = -1.0;
                    param_vect[component_idx] = voltage;
                },
                TwoTerminalComponent::Capacitor(capacitance) => {
                    matrix[(component_idx, current_idx)] = -dt;
                    matrix[(component_idx, voltage_drop_idx)] = capacitance;
                    param_vect[component_idx] = self.soln_vector[voltage_drop_idx] * capacitance;
                }
                other => eprintln!("{other:?} is not supported yet!!"),
            }

        }

        //println!("Param {}", param_vect);

        //println!("{:>2}", matrix);
        if !matrix.is_empty() {
            if let Ok(inv) = ndarray_linalg::Inverse::inv(&matrix) {
                let res = inv.dot(&param_vect);
                self.soln_vector = res.to_vec();
                //dbg!(&self.soln_vector);

                // println!("Currents: {:?}", &self.soln_vector[self.map.state_map.currents()]);
                // println!("Voltage drops: {:?}", &self.soln_vector[self.map.state_map.voltage_drops()]);
                // println!("Voltages: {:?}", &self.soln_vector[self.map.state_map.voltages()]);
            } else {
                eprintln!("Warn: Unsolved");
                //let lst = matrix.least_squares(&param_vect).unwrap();
                //dbg!(matrix.dot(&lst.solution) - param_vect);
                //self.soln_vector = lst.solution.to_vec();
            }
            //println!("Invertible? {}", ndarray_linalg::Inverse::inv(&matrix).is_ok());

        }

    }

    pub fn state(&self) -> SimOutputs {
        let mut voltages = self.soln_vector[self.map.state_map.voltages()].to_vec();
        // Last node voltage is ground!
        voltages.push(0.0);

        let two_terminal_current = self.soln_vector[self.map.state_map.currents()].to_vec();

        // TODO: Transistors!
        let three_terminal_current = self
            .diagram
            .three_terminal
            .iter()
            .map(|_| [0.0; 3])
            .collect();

        SimOutputs {
            voltages,
            two_terminal_current,
            three_terminal_current,
        }
    }
}
