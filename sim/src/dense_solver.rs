use std::ops::Range;

use ndarray::{Array1, Array2};

use crate::{PrimitiveDiagram, SimOutputs};

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
    n_known_voltages: usize,
    n_current_laws: usize,
    //n_known_currents: usize,
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
        self.state_map.total_len().max(self.param_map.total_len())
    }
}

impl PrimitiveDiagramParameterMapping {
    fn new(diagram: &PrimitiveDiagram) -> Self {
        Self {
            n_known_voltages: diagram.voltage_sources().count(),
            n_current_laws: diagram.num_nodes,
        }
    }

    fn known_voltage_indices(&self) -> Range<usize> {
        0..self.n_known_voltages
    }

    fn current_laws(&self) -> Range<usize> {
        let base = self.known_voltage_indices().end;
        base .. base + self.n_current_laws
    }

    fn total_len(&self) -> usize {
        self.n_known_voltages
    }
}

impl PrimitiveDiagramStateVectorMapping {
    fn new(diagram: &PrimitiveDiagram) -> Self {
        Self {
            n_currents: diagram.two_terminal.len(),
            n_voltage_drops: diagram.two_terminal.len(),
            n_voltages: diagram.num_nodes.checked_sub(1).unwrap_or(0),
        }
    }

    fn currents(&self) -> Range<usize> {
        0..self.n_currents
    }

    fn voltage_drops(&self) -> Range<usize> {
        let base = self.n_currents;
        base..base + self.n_voltage_drops
    }

    fn voltages(&self) -> Range<usize> {
        let base = self.n_currents + self.n_voltage_drops;
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

        let mut matrix = Array2::<f32>::zeros((n, n));

        // TODO: Three-terminal components 

        // Stamp parameters
        let mut param_vect = Array1::<f32>::zeros(n);
        for (param_vect_idx, (component_idx, voltage)) in self
            .map
            .param_map
            .known_voltage_indices()
            .zip(self.diagram.voltage_sources())
        {
            // Stamps known voltages in parameter vector
            param_vect[param_vect_idx] = voltage;

            // Connects voltage parameter to voltage drop
            let voltage_drop_idx = self.map.state_map.voltage_drops().nth(component_idx).unwrap();
            matrix[(voltage_drop_idx, param_vect_idx)] = 1.0;
        }

        // Stamp current laws
        for (component_idx, &(node_indices, _component)) in self.diagram.two_terminal.iter().enumerate() {
            let [begin_node_idx, end_node_idx] = node_indices;

            let current_idx = self.map.state_map.currents().nth(component_idx).unwrap();
            let end_current_law_idx = self.map.param_map.current_laws().nth(end_node_idx).unwrap();
            let begin_current_law_idx = self.map.param_map.current_laws().nth(begin_node_idx).unwrap();

            matrix[(current_idx, end_current_law_idx)] = -1.0;
            matrix[(current_idx, begin_current_law_idx)] = 1.0;
        }

        // Stamp voltage laws
        for (component_idx, &(node_indices, _component)) in self.diagram.two_terminal.iter().enumerate() {
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
