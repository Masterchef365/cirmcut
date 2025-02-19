pub mod solver;
mod map;
mod stamp;

pub type CellPos = (i32, i32);

/// Represents the simplified topology of the network. This is the input to the simulator.
/// This is an unsimplified representation, suitable for use with human interfaces.
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct PrimitiveDiagram {
    pub num_nodes: usize,
    pub two_terminal: Vec<([usize; 2], TwoTerminalComponent)>,
    pub three_terminal: Vec<([usize; 3], ThreeTerminalComponent)>,
}

/// Output voltage and current, corresponding to the input indices
#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct SimOutputs {
    /// One voltage for each node
    pub voltages: Vec<f64>,
    pub two_terminal_current: Vec<f64>,
    pub three_terminal_current: Vec<[f64; 3]>,
}

/// Represents a single circuit element.
#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub enum TwoTerminalComponent {
    Wire,
    // Resistance
    Resistor(f64),
    // Inductance
    Inductor(f64),
    // Capacitance
    Capacitor(f64),
    Diode,
    Battery(f64),
    Switch(bool),
    CurrentSource(f64),
    /*
    AcSource(Source),
    */
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub enum ThreeTerminalComponent {
    /// Beta
    PTransistor(f64),
    NTransistor(f64),
}

impl TwoTerminalComponent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Wire => "Wire",
            Self::Resistor(_) => "Resistor",
            Self::Capacitor(_) => "Capacitor",
            Self::Inductor(_) => "Inductor",
            Self::Battery(_) => "Battery",
            Self::Diode => "Diode",
            Self::Switch(_) => "Switch",
            Self::CurrentSource(_) => "Current Source",
        }
    }
}

impl ThreeTerminalComponent {
    pub fn name(&self) -> &'static str {
        match self {
            ThreeTerminalComponent::NTransistor(_) => "N-type Transistor (NPN)",
            ThreeTerminalComponent::PTransistor(_) => "P-type Transistor (PNP)",
        }
    }
}

impl PrimitiveDiagram {
    /// Returns (component index, voltage)
    pub fn voltage_sources(&self) -> impl Iterator<Item = (usize, f64)> + '_ {
        self.two_terminal
            .iter()
            .enumerate()
            .filter_map(|(component_idx, &(_, comp))| match comp {
                crate::TwoTerminalComponent::Battery(v) => Some((component_idx, v)),
                _ => None,
            })
    }
}
