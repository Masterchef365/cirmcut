use std::collections::HashMap;

pub type CellPos = (i32, i32);

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct Diagram {
    pub two_terminal: Vec<TwoTerminalComponent>,
    pub three_terminal: Vec<ThreeTerminalComponent>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub struct TwoTerminalComponent {
    pub begin: CellPos,
    pub end: CellPos,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug)]
pub struct ThreeTerminalComponent {
    pub a: CellPos,
    pub b: CellPos,
    pub c: CellPos,
}

/// Represents a single circuit element.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Component {
    Wire,
    /*
    // Resistance
    Resistor(f32),
    // Beta
    Transistor(f32, TransistorType),
    // Capacitance
    Capacitor(f32),
    // Inductance
    Inductor(f32),
    Diode,
    Switch(bool),
    */
}

impl Diagram {
    pub fn junctions(&self) -> Vec<CellPos> {
        let mut junctions = HashMap::<CellPos, u32>::new();
        for comp in &self.two_terminal {
            for pos in [comp.begin, comp.end] {
                *junctions.entry(pos).or_default() += 1;
            }
        }
        for comp in &self.three_terminal {
            for pos in [comp.a, comp.b, comp.c] {
                *junctions.entry(pos).or_default() += 1;
            }
        }
        junctions
            .into_iter()
            .filter_map(|(pos, count)| (count > 1).then_some(pos))
            .collect()
    }
}
