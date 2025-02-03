use std::collections::HashMap;

/// Represents a single circuit element.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Component {
    Junction,
    /// Wires don't touch! 
    Crossover,
    Wire,
    // Beta
    Transistor(f32, TransistorType),
    // Resistance
    Resistor(f32),
    // Capacitance
    Capacitor(f32),
    // Inductance
    Inductor(f32),
    Diode,
    Switch(bool),
}

/// Represents the wire states of a component
#[derive(Clone, Copy, Debug)]
pub struct ComponentState {
    pub top: WireState,
    pub bottom: WireState,
    pub left: WireState,
    pub right: WireState,
}

/*
/// Represents a single circuit element.
#[derive(Clone, Copy, Debug)]
pub enum ComponentState {
    /// Horizontal and vertical current
    Junction { top: WireState, bottom: WireState, left: WireState, right: WireState },
    Crossover { horizontal: WireState, vertical: WireState },
    Wire(WireState),
    Transistor { base: WireState, collector: WireState, emitter: WireState },
    Resistor { left: WireState, right: WireState },
    Capacitor { left: WireState, right: WireState },
    Inductor { left: WireState, right: WireState },
    Diode { left: WireState, right: WireState },
    Switch  { left: WireState, right: WireState },
}
*/

#[derive(Clone, Copy, Debug)]
pub struct WireState {
    pub current: f32,
    pub voltage: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TransistorType {
    PType,
    NType,
}

/// Represents the rotation of a Component
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Orientation {
    #[default]
    Orig,
    Rot90,
    Rot180,
    Rot270,
}

type Coord = (usize, usize);

#[derive(Clone, Copy, Debug)]
pub struct DiagramCell {
    pub flip: bool,
    pub orient: Orientation,
    pub comp: Component,
}

/// Represents the pictoral representation of a circuit, 
/// in a way that uniquely defines a circuit (or some open-ended garbage).
pub type Diagram = HashMap<Coord, DiagramCell>;

/// Represents only the state corresponding to a diagram
pub type DiagramState = HashMap<Coord, ComponentState>;
