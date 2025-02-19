use std::ops::Range;

use rsparse::{data::{Sprs, Trpl}, lusol};

use crate::{map::PrimitiveDiagramMapping, PrimitiveDiagram, SimOutputs, TwoTerminalComponent};

pub fn stamp(dt: f64, map: &PrimitiveDiagramMapping, diagram: &PrimitiveDiagram, last_iteration: &[f64], last_timestep: &[f64]) -> (Sprs, Vec<f64>) {
    let n = map.vector_size();

    // (params, state)
    let mut matrix = Trpl::new();
    let mut params = vec![0_f64; n];

    // TODO: Three-terminal components

    // Stamp current laws
    for (component_idx, &(node_indices, _component)) in diagram.two_terminal.iter().enumerate()
    {
        let [begin_node_idx, end_node_idx] = node_indices;

        let current_idx = map.state_map.currents().nth(component_idx).unwrap();
        if let Some(end_current_law_idx) = map.param_map.current_laws().nth(end_node_idx) {
            matrix.append(end_current_law_idx, current_idx, 1.0);
        }
        if let Some(begin_current_law_idx) =
            map.param_map.current_laws().nth(begin_node_idx)
        {
            matrix.append(begin_current_law_idx, current_idx, -1.0);
        }
    }

    // Stamp voltage laws
    for (component_idx, &(node_indices, _component)) in diagram.two_terminal.iter().enumerate()
    {
        let [begin_node_idx, end_node_idx] = node_indices;

        let voltage_law_idx = 
            map
            .param_map
            .voltage_laws()
            .nth(component_idx)
            .unwrap();
        let voltage_drop_idx = 
            map
            .state_map
            .voltage_drops()
            .nth(component_idx)
            .unwrap();

        matrix.append(voltage_law_idx, voltage_drop_idx, 1.0);
        if let Some(end_voltage_idx) = map.state_map.voltages().nth(end_node_idx) {
            matrix.append(voltage_law_idx, end_voltage_idx, 1.0);
        }

        if let Some(begin_voltage_idx) = map.state_map.voltages().nth(begin_node_idx) {
            matrix.append(voltage_law_idx, begin_voltage_idx, -1.0);
        }
    }

    // Stamp components
    for (i, &(node_indices, component)) in diagram.two_terminal.iter().enumerate() {
        let component_idx = map.param_map.components().nth(i).unwrap();

        let current_idx = map.state_map.currents().nth(i).unwrap();
        let voltage_drop_idx = map.state_map.voltage_drops().nth(i).unwrap();

        match component {
            TwoTerminalComponent::Resistor(resistance) => {
                matrix.append(component_idx, current_idx, -resistance);
                matrix.append(component_idx, voltage_drop_idx, 1.0);
            }
            TwoTerminalComponent::Wire => {
                // Vd = 0
                //matrix.append(component_idx, voltage_drop_idx, 1.0);
                let [begin_node_idx, end_node_idx] = node_indices;

                if let Some(voltage_idx) = map.state_map.voltages().nth(end_node_idx) {
                    matrix.append(component_idx, voltage_idx, 1.0);
                }

                if let Some(voltage_idx) = map.state_map.voltages().nth(begin_node_idx) {
                    matrix.append(component_idx, voltage_idx, -1.0);
                }
            }
            TwoTerminalComponent::Switch(is_open) => {
                // Vd = 0
                //matrix.append(component_idx, voltage_drop_idx, 1.0);
                //let [begin_node_idx, end_node_idx] = node_indices;

                if is_open {
                    // Set current through this component to zero
                    matrix.append(component_idx, current_idx, 1.0);
                } else {
                    // Set voltage through this component to zero
                    matrix.append(component_idx, voltage_drop_idx, 1.0);
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
                matrix.append(component_idx, voltage_drop_idx, -1.0);
                params[component_idx] = voltage;
            }
            TwoTerminalComponent::Capacitor(capacitance) => {
                matrix.append(component_idx, current_idx, -dt);
                matrix.append(component_idx, voltage_drop_idx, capacitance);
                params[component_idx] = last_timestep[voltage_drop_idx] * capacitance;
            }
            TwoTerminalComponent::Inductor(inductance) => {
                matrix.append(component_idx, voltage_drop_idx, dt);
                matrix.append(component_idx, current_idx, -inductance);
                params[component_idx] = -last_timestep[current_idx] * inductance;
            }
            TwoTerminalComponent::Diode => {
                // Stolen from falstad.
                let sat_current = 171.4352819281e-9;
                let n = 2.0;
                let temperature = 273.15 + 22.0;
                let thermal_voltage = 8.617e-5 * temperature;
                let nvt = n * thermal_voltage;

                let v0 = last_iteration[voltage_drop_idx];
                /*
                let dfdi = 1.0 - nvt / (sat_current + i);
                let dfdv = -1.0 + sat_current * (v / nvt).exp();
                */

                let ex = (v0 / nvt).exp();
                let coeff = -(sat_current / nvt) * ex;

                matrix.append(component_idx, voltage_drop_idx, coeff);
                matrix.append(component_idx, current_idx, 1.0);

                params[component_idx] = sat_current * (1.0 - ex + v0 * ex / nvt);
            }
            TwoTerminalComponent::CurrentSource(current) => {
                matrix.append(component_idx, current_idx, 1.0);
                params[component_idx] = current;
            }
            //other => eprintln!("{other:?} is not supported yet!!"),
        }
    }

    (matrix.to_sprs(), params)
}
