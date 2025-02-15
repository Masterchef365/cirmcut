use std::ops::Range;

use crate::{PrimitiveDiagram, SimOutputs};

#[derive(Default)]
pub struct Solver {
    diagram: PrimitiveDiagram,
    map: PrimitiveDiagramVectorMapping,

    soln_vector: Vec<f32>,
}

#[derive(Default)]
struct PrimitiveDiagramVectorMapping {
    n_currents: usize,
    n_voltage_drops: usize,
    n_voltages: usize,
}

impl PrimitiveDiagramVectorMapping {
    fn new(diagram: &PrimitiveDiagram) -> Self {
        Self {
            n_currents: diagram.two_terminal.len(),
            n_voltage_drops: diagram.two_terminal.len(),
            n_voltages: diagram.num_nodes.checked_sub(1).unwrap_or(0),
        }
    }

    fn currents(&self) -> Range<usize> {
        0 .. self.n_currents
    }

    fn voltage_drops(&self) -> Range<usize> {
        let base = self.n_currents;
        base .. base + self.n_voltage_drops
    }

    fn voltages(&self) -> Range<usize> {
        let base = self.n_currents + self.n_voltage_drops;
        base .. base + self.n_voltages
    }

    fn total_len(&self) -> usize {
        self.n_currents + self.n_voltages + self.n_voltage_drops
    }
}

impl Solver {
    pub fn new(diagram: PrimitiveDiagram) -> Self {
        let map = PrimitiveDiagramVectorMapping::new(&diagram);
        
        Self {
            soln_vector: vec![0.0; map.total_len()],
            map,
            diagram,
        }
    }

    pub fn step(&mut self, dt: f32) {
    }

    pub fn state(&self) -> SimOutputs {
        let mut voltages = self.soln_vector[self.map.voltages()].to_vec();
        // Last node voltage is ground!
        voltages.push(0.0);

        let two_terminal_current = self.soln_vector[self.map.currents()].to_vec();

        // TODO: Transistors!
        let three_terminal_current = self.diagram.three_terminal.iter().map(|_| [0.0; 3]).collect();

        SimOutputs { voltages, two_terminal_current, three_terminal_current }
    }
}
