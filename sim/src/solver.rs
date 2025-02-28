use std::ops::Range;

use rsparse::{data::{Sprs, Trpl}, lusol};

use crate::{map::PrimitiveDiagramMapping, stamp::stamp, PrimitiveDiagram, SimOutputs, TwoTerminalComponent};

pub struct Solver {
    map: PrimitiveDiagramMapping,
    soln_vector: Vec<f64>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum SolverMode {
    Linear,
    #[default]
    NewtonRaphson,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, Debug)]
pub struct SolverConfig {
    pub max_nr_iters: usize, 
    pub nr_step_size: f64,
    /// NR-Iterate until error reaches this value
    pub nr_tolerance: f64,
    /// When solving F Delta x = -f, which tolerance do we solve the system to?
    pub dx_soln_tolerance: f64,
    pub mode: SolverMode,
    #[serde(default)]
    pub adaptive_step_size: bool,
    pub n_timesteps: usize,
}

impl Solver {
    pub fn new(diagram: &PrimitiveDiagram) -> Self {
        let map = PrimitiveDiagramMapping::new(diagram);

        Self {
            soln_vector: vec![0.0; map.vector_size()],
            map,
        }
    }

    /// Note: Assumes diagram is compatible what a sufficiently large battery (or a battery with very low internal resisith the one this solver was created with!
    pub fn step(&mut self, dt: f64, diagram: &PrimitiveDiagram, cfg: &SolverConfig) -> Result<(), String> {
        for _ in 0..cfg.n_timesteps {
            match cfg.mode {
                SolverMode::NewtonRaphson => self.nr_step(dt, diagram, cfg)?,
                SolverMode::Linear => self.linear_step(dt, diagram, cfg)?,
            }
        }

        Ok(())
    }

    fn linear_step(&mut self, dt: f64, diagram: &PrimitiveDiagram, cfg: &SolverConfig) -> Result<(), String> {
        let prev_time_step_soln = &self.soln_vector;

        let (matrix, params) = stamp(dt, &self.map, diagram, &prev_time_step_soln, &prev_time_step_soln);

        let mut new_soln = params;
        lusol(&matrix, &mut new_soln, -1, cfg.dx_soln_tolerance).map_err(|e| e.to_string())?;

        self.soln_vector = new_soln;

        Ok(())
    }

    fn nr_step(&mut self, dt: f64, diagram: &PrimitiveDiagram, cfg: &SolverConfig) -> Result<(), String> {
        let prev_time_step_soln = &self.soln_vector;

        let mut new_state = [prev_time_step_soln.clone()];

        let mut step_size: f64 = cfg.nr_step_size;

        let mut last_err = 9e99;
        let mut nr_iters = 0;
        for _ in 0..cfg.max_nr_iters {
            // Calculate A(w_n(K)), b(w_n(K))
            let (matrix, params) = stamp(dt, &self.map, diagram, &new_state[0], &prev_time_step_soln);

            if params.len() == 0 {
                return Ok(());
            }

            let mut dense_b = Trpl::new();
            for (i, val) in params.iter().enumerate() {
                dense_b.append(i, 0, *val);
            }
            let dense_b = dense_b.to_sprs();


            let mut new_state_sparse = Trpl::new();
            for (i, val) in new_state[0].iter().enumerate() {
                new_state_sparse.append(i, 0, *val);
            }
            let new_state_sparse = new_state_sparse.to_sprs();

            // Calculate -f(w_n(K)) = b(w_n(K)) - A(w_n(K)) w_n(K)
            let ax = &matrix * &new_state_sparse;
            let f = dense_b - ax;

            // Solve A(w_n(K)) dw = -f for dw
            let mut delta: Vec<f64> = f.to_dense().iter().flatten().copied().collect();
            lusol(&matrix, &mut delta, -1, cfg.dx_soln_tolerance).map_err(|e| e.to_string())?;

            // dw dot dw
            let err = delta.iter().map(|f| (f * step_size).powi(2)).sum::<f64>();

            if err > last_err && cfg.adaptive_step_size {
                last_err = err;
                //dbg!(step_size);
                step_size /= 2.0;
                continue;
                //return Err("Error value increased!".to_string());
                //eprintln!("Error value increased! {}", err - last_err);
            }

            // w += dw * step size
            new_state[0].iter_mut().zip(&delta).for_each(|(n, delta)| *n += delta * step_size);

            if err < cfg.nr_tolerance {
                break;
            }
            //dbg!(err);

            last_err = err;
            nr_iters += 1;
        }

        if nr_iters > 0 {
            //dbg!(nr_iters);
        }

        [self.soln_vector] = new_state;

        Ok(())
    }

    pub fn state(&self, diagram: &PrimitiveDiagram) -> SimOutputs {
        let mut voltages = self.soln_vector[self.map.state_map.voltages()].to_vec();
        // Last node voltage is ground!
        voltages.push(0.0);

        let mut total_idx = 0;
        let mut two_terminal_current = vec![];

        for _ in &diagram.two_terminal {
            two_terminal_current.push(self.soln_vector[total_idx]);
            total_idx += 1;
        }

        let mut three_terminal_current = vec![];
        for _ in &diagram.three_terminal {
            let ab_current = self.soln_vector[total_idx];
            total_idx += 1;
            let bc_current = self.soln_vector[total_idx];
            total_idx += 1;

            let c = bc_current;
            let b = bc_current - ab_current;
            let a = ab_current;

            three_terminal_current.push([a, b, c]);
        }

        // TODO: Transistors!

        SimOutputs {
            voltages,
            two_terminal_current,
            three_terminal_current,
        }
    }
}

impl Default for SolverConfig {
    fn default() -> Self {
        SolverConfig {
            n_timesteps: 1,
            adaptive_step_size: true,
            mode: SolverMode::default(),
            dx_soln_tolerance: 1e-3,
            nr_tolerance: 1e-3,
            nr_step_size: 1e-1,
            max_nr_iters: 20,
        }
    }
}
