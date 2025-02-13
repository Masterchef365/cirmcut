pub type CellPos = (i32, i32);


/// Represents a single circuit element.
#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub enum TwoTerminalComponent {
    Wire,
    // Resistance
    Resistor(f32),
    /*
    // Capacitance
    Capacitor(f32),
    // Inductance
    Inductor(f32),
    Diode,
    Switch(bool),
    */
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub enum ThreeTerminalComponent {
    /// Beta
    PTransistor(f32),
    NTransistor(f32),
}
