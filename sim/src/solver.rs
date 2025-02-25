use std::ops::Range;

use gmres::gmres;
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
    pub n_timesteps: usize,
    #[serde(default)]
    pub linear_sol: LinearSolver,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LinearSolver {
    #[default]
    LUDecomposition,
    BiconjugateGradient,
    GenMinRes,
}

impl Solver {
    pub fn new(diagram: &PrimitiveDiagram, n_timesteps: usize) -> Self {
        let map = PrimitiveDiagramMapping::new(diagram);

        Self {
            soln_vector: vec![0.0; map.vector_size() * n_timesteps],
            map,
        }
    }

    /// Note: Assumes diagram is compatible what a sufficiently large battery (or a battery with very low internal resisith the one this solver was created with!
    pub fn step(&mut self, dt: f64, diagram: &PrimitiveDiagram, cfg: &SolverConfig) -> Result<(), String> {
        match cfg.mode {
            SolverMode::NewtonRaphson => self.nr_step(dt, diagram, cfg),
            SolverMode::Linear => self.linear_step(dt, diagram, cfg),
        }
    }

    fn linear_step(&mut self, dt: f64, diagram: &PrimitiveDiagram, cfg: &SolverConfig) -> Result<(), String> {
        /*
        let prev_time_step_soln = &self.soln_vector;

        let (matrix, params) = stamp(dt, &self.map, diagram, &prev_time_step_soln, &prev_time_step_soln);

        let mut new_soln = params;
        cfg.linear_sol.solve(&matrix, &mut new_soln, cfg.dx_soln_tolerance)?;

        self.soln_vector = new_soln;

        */
        Ok(())
    }

    fn nr_step(&mut self, dt: f64, diagram: &PrimitiveDiagram, cfg: &SolverConfig) -> Result<(), String> {
        let total_vect_len = cfg.n_timesteps * self.map.vector_size();
        let prev_time_step_soln = &self.soln_vector[cfg.n_timesteps.saturating_sub(1) * self.map.vector_size()..];
        dbg!(&prev_time_step_soln.len());

        let mut new_state: Vec<f64> = prev_time_step_soln.iter().cycle().take(total_vect_len).copied().collect();

        let mut last_err = 9e99;
        let mut nr_iters = 0;
        for _ in 0..cfg.max_nr_iters {
            // Calculate A(w_n(K)), b(w_n(K))
            let (matrix, params) = stamp(dt, &self.map, diagram, &new_state, prev_time_step_soln, cfg.n_timesteps);

            if params.len() == 0 {
                return Ok(());
            }

            let dense_b = vector_to_sparse(&params);

            let mut new_state_sparse = Trpl::new();
            for (i, val) in new_state.iter().enumerate() {
                new_state_sparse.append(i, 0, *val);
            }
            let new_state_sparse = new_state_sparse.to_sprs();

            // Calculate -f(w_n(K)) = b(w_n(K)) - A(w_n(K)) w_n(K)
            let ax = &matrix * &new_state_sparse;
            let f = dense_b - ax;

            // Solve A(w_n(K)) dw = -f for dw
            let mut delta: Vec<f64> = sparse_to_vector(&f);
            cfg.linear_sol.solve(&matrix, &mut delta, cfg.dx_soln_tolerance)?;

            // dw dot dw
            let err = delta.iter().map(|f| f*f).sum::<f64>();

            if err > last_err {
                //return Err("Error value increased!".to_string());
                //eprintln!("Error value increased! {}", err - last_err);
            }

            // w += dw * step size
            new_state.iter_mut().zip(&delta).for_each(|(n, delta)| *n += delta * cfg.nr_step_size);

            if err < cfg.nr_tolerance {
                break;
            }
            //dbg!(err);

            last_err = err;
            nr_iters += 1;
        }

        /*
        if nr_iters > 0 {
            dbg!(nr_iters);
        }
        */

        self.soln_vector = new_state;

        Ok(())
    }

    pub fn state(&self, diagram: &PrimitiveDiagram, time_step_idx: usize) -> SimOutputs {
        let offset = time_step_idx * self.map.vector_size();

        let mut voltages = self.soln_vector[offset..][self.map.state_map.voltages()].to_vec();
        // Last node voltage is ground!
        voltages.push(0.0);

        let mut total_idx = 0;
        let mut two_terminal_current = vec![];

        for _ in &diagram.two_terminal {
            two_terminal_current.push(self.soln_vector[offset..][total_idx]);
            total_idx += 1;
        }

        let mut three_terminal_current = vec![];
        for _ in &diagram.three_terminal {
            let ab_current = self.soln_vector[offset..][total_idx];
            total_idx += 1;
            let bc_current = self.soln_vector[offset..][total_idx];
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
            linear_sol: LinearSolver::LUDecomposition,
            mode: SolverMode::default(),
            dx_soln_tolerance: 1e-3,
            nr_tolerance: 1e-6,
            nr_step_size: 1e-1,
            max_nr_iters: 2000,
            n_timesteps: 2,
        }
    }
}

impl LinearSolver {
    fn solve(&self, a: &Sprs, b: &mut Vec<f64>, tolerance: f64) -> Result<(), String> {
        let inst = std::time::Instant::now();

        match self {
            Self::LUDecomposition => lusol(a, b, -1, tolerance).map_err(|e| e.to_string()),
            Self::BiconjugateGradient => bicg(a, b, tolerance),
            Self::GenMinRes => gmres(a, b.to_vec().as_slice(), b, 100, tolerance),
        }?;

        let time_ms = inst.elapsed().as_secs_f32() * 1e3;
        println!("{} ms", time_ms);

        Ok(())
    }
}

fn vector_to_sparse(v: &[f64]) -> Sprs {
    let mut mat = Trpl::new();
    for (i, val) in v.iter().enumerate() {
        mat.append(i, 0, *val);
    }
    mat.to_sprs()
}

fn sparse_to_vector(s: &Sprs) -> Vec<f64> {
    s.to_dense().iter().flatten().copied().collect()
}

/// Gets the value of a single element matrix
fn get_one(a: &Sprs) -> f64 {
    debug_assert_eq!(a.to_dense().len(), 1);
    debug_assert_eq!(a.to_dense()[0].len(), 1);
    a.to_dense()[0][0]
}

fn bicg(a: &Sprs, b: &mut [f64], tolerance: f64) -> Result<(), String> {
    let x = vec![1e-12; b.len()];
    let mut x = vector_to_sparse(&x);
    let mut xt = rsparse::transpose(&x);

    let out = b;
    let b = vector_to_sparse(&out);
    let bt = rsparse::transpose(&b);

    let at = rsparse::transpose(&a);

    let mut r = b - a * &x;
    let mut rt = bt - &xt * at;

    let mut p = r.clone();
    let mut pt = rt.clone();

    eprintln!("\nSTART BICG");
    for _ in 0..100 {
        let rr = get_one(&(&rt * &r));
        let pap = get_one(&(&pt * a * &p));
        //dbg!(rr, pap);
        let alpha = rr / pap;
        //dbg!(alpha);

        let res = get_one(&(rsparse::transpose(&r) * &r));
        dbg!(res);
        if res.is_nan() {
            panic!("NaN result");
        }
        if res < tolerance {
            break;
        }

        x = &x + alpha * &p;
        xt = &xt + alpha * &pt;

        r = &r - alpha * a * &p;
        rt = &rt - alpha * &pt * a;

        let rr_new = get_one(&(&rt * &r));
        //dbg!(rr_new);
        let beta = rr_new / rr;
        //dbg!(beta);
        p = &r + beta * &p;
        pt = &rt + beta * &pt;
    }

    out.copy_from_slice(&sparse_to_vector(&x));

    Ok(())
}
