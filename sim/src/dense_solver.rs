use std::ops::{Range, RangeFrom};

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

    fn known_voltages(&self) -> Range<usize> {
        0..self.n_known_voltages
    }

    fn zeros(&self) -> RangeFrom<usize> {
        self.n_known_voltages..
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

        let mut output_vect = Array1::<f32>::zeros(n);

        // Stamp known voltages in output vector
        for (output_idx, voltage) in self.map.param_map.known_voltages().zip(self.diagram.voltage_sources()) {
            output_vect[output_idx] = voltage;
        }

        // Stamp current laws
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
