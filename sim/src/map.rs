use std::ops::Range;

use crate::PrimitiveDiagram;

/// Maps indices of the state vector (x from Ax = b) to the corresponding component voltages,
/// currents, etc.
#[derive(Default)]
pub struct PrimitiveDiagramStateVectorMapping {
    pub n_currents: usize,
    pub n_voltage_drops: usize,
    pub n_voltages: usize,
}

/// Maps indices of the parameters (known values such as input voltage or current or signal).
/// These are the known variables, or b from Ax = b.
#[derive(Default)]
pub struct PrimitiveDiagramParameterMapping {
    pub n_components: usize,
    pub n_current_laws: usize,
    pub n_voltage_laws: usize,
}

/// Represents the mappings needed to work with either the state vector or the parameter map
pub struct PrimitiveDiagramMapping {
    pub state_map: PrimitiveDiagramStateVectorMapping,
    pub param_map: PrimitiveDiagramParameterMapping,
}

impl PrimitiveDiagramMapping {
    pub fn new(diagram: &PrimitiveDiagram) -> Self {
        Self {
            state_map: PrimitiveDiagramStateVectorMapping::new(diagram),
            param_map: PrimitiveDiagramParameterMapping::new(diagram),
        }
    }

    pub fn vector_size(&self) -> usize {
        debug_assert_eq!(self.state_map.total_len(), self.param_map.total_len());
        self.state_map.total_len()
    }
}

impl PrimitiveDiagramParameterMapping {
    pub fn new(diagram: &PrimitiveDiagram) -> Self {
        Self {
            n_components: diagram.two_terminal.len(),
            n_voltage_laws: diagram.two_terminal.len(),
            n_current_laws: diagram.num_nodes.saturating_sub(1),
        }
    }

    pub fn components(&self) -> Range<usize> {
        0..self.n_components
    }

    pub fn current_laws(&self) -> Range<usize> {
        let base = self.components().end;
        base..base + self.n_current_laws
    }

    pub fn voltage_laws(&self) -> Range<usize> {
        let base = self.current_laws().end;
        base..base + self.n_voltage_laws
    }

    pub fn total_len(&self) -> usize {
        self.n_current_laws + self.n_voltage_laws + self.n_components
    }
}

impl PrimitiveDiagramStateVectorMapping {
    pub fn new(diagram: &PrimitiveDiagram) -> Self {
        Self {
            n_currents: diagram.two_terminal.len(),
            n_voltage_drops: diagram.two_terminal.len(),
            n_voltages: diagram.num_nodes.saturating_sub(1),
        }
    }

    pub fn currents(&self) -> Range<usize> {
        0..self.n_currents
    }

    pub fn voltage_drops(&self) -> Range<usize> {
        let base = self.currents().end;
        base..base + self.n_voltage_drops
    }

    pub fn voltages(&self) -> Range<usize> {
        let base = self.voltage_drops().end;
        base..base + self.n_voltages
    }

    pub fn total_len(&self) -> usize {
        self.n_currents + self.n_voltages + self.n_voltage_drops
    }
}
