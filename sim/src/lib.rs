pub type CellPos = (i32, i32);

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub enum Source {
    VoltageDC(f32),
    /*
    CurrentDC(f32),
    VoltageAC {
        /// Hertz
        freq: f32,
        /// RMS voltage
        rms: f32,
    }
    */
}

/// Represents a single circuit element.
#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub enum TwoTerminalComponent {
    Wire,
    // Resistance
    Resistor(f32),
    // Inductance
    Inductor(f32),
    // Capacitance
    Capacitor(f32),
    Diode,
    Battery(f32),
    Switch(bool),
    /*
    AcSource(Source),
    */
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub enum ThreeTerminalComponent {
    /// Beta
    PTransistor(f32),
    NTransistor(f32),
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
