use std::collections::HashMap;

use rsparse::data::{Sprs, Trpl};

use crate::{map::PrimitiveDiagramMapping, PrimitiveDiagram, ThreeTerminalComponent, TwoTerminalComponent};

pub fn stamp(dt: f64, map: &PrimitiveDiagramMapping, diagram: &PrimitiveDiagram, last_iteration: &[f64], last_timestep: &[f64]) -> (Sprs<f64>, Vec<f64>) {
    let n = map.vector_size();

    // (params, state)
    let mut matrix = Trpl::new();
    let mut params = vec![0_f64; n];

    // TODO: Three-terminal components

    // Stamp current laws
    let mut total_current_idx = 0;
    for &(node_indices, _component) in &diagram.two_terminal
    {
        let [begin_node_idx, end_node_idx] = node_indices;

        let current_idx = map.state_map.currents().nth(total_current_idx).unwrap();
        if let Some(end_current_law_idx) = map.param_map.current_laws().nth(end_node_idx) {
            matrix.append(end_current_law_idx, current_idx, 1.0);
        }
        if let Some(begin_current_law_idx) =
            map.param_map.current_laws().nth(begin_node_idx)
        {
            matrix.append(begin_current_law_idx, current_idx, -1.0);
        }

        total_current_idx += 1;
    }

    for &(node_indices, _component) in &diagram.three_terminal
    {
        let [a, b, c] = node_indices;
        let i_ab_idx = map.state_map.currents().nth(total_current_idx).unwrap();
        total_current_idx += 1;
        let i_bc_idx = map.state_map.currents().nth(total_current_idx).unwrap();
        total_current_idx += 1;

        let a_idx = map.param_map.current_laws().nth(a);
        let b_idx = map.param_map.current_laws().nth(b);
        let c_idx = map.param_map.current_laws().nth(c);

        if let Some(a) = a_idx {
            matrix.append(a, i_ab_idx, 1.0);
        }
        if let Some(b) = b_idx {
            matrix.append(b, i_ab_idx, -1.0);
            matrix.append(b, i_bc_idx, 1.0);
        }
        if let Some(c) = c_idx {
            matrix.append(c, i_bc_idx, -1.0);
        }
    }

    // Stamp voltage laws
    let mut total_voltage_idx = 0;
    for &(node_indices, _component) in &diagram.two_terminal
    {
        let [begin_node_idx, end_node_idx] = node_indices;

        let voltage_law_idx = 
            map
            .param_map
            .voltage_laws()
            .nth(total_voltage_idx)
            .unwrap();
        let voltage_drop_idx = 
            map
            .state_map
            .voltage_drops()
            .nth(total_voltage_idx)
            .unwrap();

        total_voltage_idx += 1;

        matrix.append(voltage_law_idx, voltage_drop_idx, 1.0);
        if let Some(end_voltage_idx) = map.state_map.voltages().nth(end_node_idx) {
            matrix.append(voltage_law_idx, end_voltage_idx, 1.0);
        }

        if let Some(begin_voltage_idx) = map.state_map.voltages().nth(begin_node_idx) {
            matrix.append(voltage_law_idx, begin_voltage_idx, -1.0);
        }
    }

    for &(node_indices, _component) in &diagram.three_terminal
    {
        let [a, b, c] = node_indices;

        let v_ab_law_idx = 
            map
            .param_map
            .voltage_laws()
            .nth(total_voltage_idx)
            .unwrap();
        let v_ab_drop_idx = 
            map
            .state_map
            .voltage_drops()
            .nth(total_voltage_idx)
            .unwrap();

        total_voltage_idx += 1;

        matrix.append(v_ab_law_idx, v_ab_drop_idx, 1.0);

        let v_bc_law_idx = 
            map
            .param_map
            .voltage_laws()
            .nth(total_voltage_idx)
            .unwrap();
        let v_bc_drop_idx = 
            map
            .state_map
            .voltage_drops()
            .nth(total_voltage_idx)
            .unwrap();

        total_voltage_idx += 1;

        matrix.append(v_bc_law_idx, v_bc_drop_idx, 1.0);

        if let Some(a) = map.state_map.voltages().nth(a) {
            matrix.append(v_ab_law_idx, a, 1.0);
        }

        if let Some(b) = map.state_map.voltages().nth(b) {
            matrix.append(v_ab_law_idx, b, -1.0);
            matrix.append(v_bc_law_idx, b, 1.0);
        }

        if let Some(c) = map.state_map.voltages().nth(c) {
            matrix.append(v_bc_law_idx, c, -1.0);
        }
    }

    // Maps core ID -> inductance, two terminal component idx
    let mut cores: HashMap<u16, Vec<(f64, usize)>> = HashMap::new();
    for (idx, (_, component)) in diagram.two_terminal.iter().enumerate() {
        if let TwoTerminalComponent::Inductor(value, Some(core_id)) = component {
            cores.entry(*core_id).or_default().push((*value, idx));
        }
    }

    // Stamp components
    let mut total_idx = 0;
    for &(node_indices, component) in &diagram.two_terminal {
        let law_idx = map.param_map.components().nth(total_idx).unwrap();

        let current_idx = map.state_map.currents().nth(total_idx).unwrap();
        let voltage_drop_idx = map.state_map.voltage_drops().nth(total_idx).unwrap();

        match component {
            TwoTerminalComponent::Resistor(resistance) => {
                matrix.append(law_idx, current_idx, -resistance);
                matrix.append(law_idx, voltage_drop_idx, 1.0);
            }
            TwoTerminalComponent::Wire => {
                // Vd = 0
                //matrix.append(component_idx, voltage_drop_idx, 1.0);
                let [begin_node_idx, end_node_idx] = node_indices;

                if let Some(voltage_idx) = map.state_map.voltages().nth(end_node_idx) {
                    matrix.append(law_idx, voltage_idx, 1.0);
                }

                if let Some(voltage_idx) = map.state_map.voltages().nth(begin_node_idx) {
                    matrix.append(law_idx, voltage_idx, -1.0);
                }
            }
            TwoTerminalComponent::Switch(is_open) => {
                // Vd = 0
                //matrix.append(component_idx, voltage_drop_idx, 1.0);
                //let [begin_node_idx, end_node_idx] = node_indices;

                if is_open {
                    // Set current through this component to zero
                    matrix.append(law_idx, current_idx, 1.0);
                } else {
                    // Set voltage through this component to zero
                    matrix.append(law_idx, voltage_drop_idx, 1.0);
                    /*
                    // Set voltages of connected nodes to be equal
                    if let Some(voltage_idx) = map.state_map.voltages().nth(end_node_idx) {
                        matrix.append(component_idx, voltage_idx, 1.0);
                    }

                    if let Some(voltage_idx) = map.state_map.voltages().nth(begin_node_idx)
                    {
                        matrix.append(component_idx, voltage_idx, -1.0);
                    }
                    */
                }
            }
            TwoTerminalComponent::Battery(voltage) => {
                matrix.append(law_idx, voltage_drop_idx, -1.0);
                params[law_idx] = voltage;
            }
            TwoTerminalComponent::Capacitor(capacitance) => {
                matrix.append(law_idx, current_idx, -dt);
                matrix.append(law_idx, voltage_drop_idx, capacitance);
                params[law_idx] = last_timestep[voltage_drop_idx] * capacitance;
            }
            TwoTerminalComponent::Inductor(inductance, core_id) => {
                matrix.append(law_idx, current_idx, -inductance);
                params[law_idx] = -last_timestep[current_idx] * inductance;
                let mut coeff = dt;
                if let Some(others) = core_id.and_then(|id| cores.get(&id)) {
                    for (value, twoterm_idx) in others {
                        if *twoterm_idx != total_idx {
                            coeff += -value.sqrt();
                            let other_voltage_idx = map.state_map.voltage_drops().nth(*twoterm_idx).unwrap();
                            matrix.append(law_idx, other_voltage_idx, inductance.sqrt());
                        }
                    }
                }
                matrix.append(law_idx, voltage_drop_idx, coeff);
            }
            TwoTerminalComponent::Diode => {
                let (coeff, param) = diode_eq(last_iteration[voltage_drop_idx]);
                matrix.append(law_idx, voltage_drop_idx, coeff);
                matrix.append(law_idx, current_idx, 1.0);
                params[law_idx] = param;
            }
            TwoTerminalComponent::CurrentSource(current) => {
                matrix.append(law_idx, current_idx, 1.0);
                params[law_idx] = current;
            }
            //other => eprintln!("{other:?} is not supported yet!!"),
        }

        total_idx += 1;
    }

    for &(_, component) in &diagram.three_terminal {
        let ab_law_idx = map.param_map.components().nth(total_idx).unwrap();
        let ab_current_idx = map.state_map.currents().nth(total_idx).unwrap();
        let ab_voltage_drop_idx = map.state_map.voltage_drops().nth(total_idx).unwrap();
        total_idx += 1;

        let bc_law_idx = map.param_map.components().nth(total_idx).unwrap();
        let bc_current_idx = map.state_map.currents().nth(total_idx).unwrap();
        let bc_voltage_drop_idx = map.state_map.voltage_drops().nth(total_idx).unwrap();
        total_idx += 1;

        match component {
            ThreeTerminalComponent::NTransistor(_) | ThreeTerminalComponent::PTransistor(_) => {
                let sign = match component {
                    ThreeTerminalComponent::NTransistor(_) => 1.0,
                    _ => -1.0,
                };

                let (diode_coeff_ab, mut diode_param_ab) = diode_eq(sign * last_iteration[ab_voltage_drop_idx]);

                let (diode_coeff_bc, mut diode_param_bc) = diode_eq(-sign * last_iteration[bc_voltage_drop_idx]);

                let af = 0.98;
                let ar = 0.1;

                diode_param_bc += af * last_iteration[ab_current_idx];
                diode_param_ab += ar * last_iteration[bc_current_idx];

                matrix.append(ab_law_idx, ab_voltage_drop_idx, diode_coeff_ab);
                matrix.append(ab_law_idx, ab_current_idx, 1.0);
                params[ab_law_idx] = diode_param_ab;

                matrix.append(bc_law_idx, bc_voltage_drop_idx, diode_coeff_bc);
                matrix.append(bc_law_idx, bc_current_idx, 1.0);
                params[bc_law_idx] = diode_param_bc;
            }
        }
    }

    (matrix.to_sprs(), params)
}

// Solves for the backwards difference, using the taylor expansion of 
// the diode equation about `last_iteration_voltage`.
fn diode_eq(last_iteration_voltage: f64) -> (f64, f64) {
    // Stolen from falstad.
    let sat_current = 171.4352819281e-9;
    let n = 2.0;
    let temperature = 273.15 + 22.0;
    let thermal_voltage = 8.617e-5 * temperature;
    let nvt = n * thermal_voltage;

    let v0 = last_iteration_voltage;

    let ex = (v0 / nvt).exp();
    let coeff = -(sat_current / nvt) * ex;

    let param = sat_current * (1.0 - ex + v0 * ex / nvt);

    (coeff, param)
}
