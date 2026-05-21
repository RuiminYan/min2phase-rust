// WASM bindings for min2phase-rust.
//
// Usage (JS):
//   import init, { Min2Phase } from './m2p_wasm.js';
//   await init();
//   const m = new Min2Phase();              // builds tables (~80ms)
//   const sol = m.solve(facelets);          // returns " R U R' ... "
//   const f = m.fromScramble("R U R' U'"); // 54-char facelet string
//   const r = m.randomCube();              // random cube facelets

use std::sync::Arc;

use m2p_core::{tools, Solver, SolverError, Tables};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Min2Phase {
    tables: Arc<Tables>,
    solver: Solver,
}

#[wasm_bindgen]
impl Min2Phase {
    /// Build with full pruning tables. ~80-150ms in WASM (slower than native).
    #[wasm_bindgen(constructor)]
    pub fn new() -> Min2Phase {
        let tables = Arc::new(Tables::build(true));
        let solver = Solver::with_tables(tables.clone());
        Min2Phase { tables, solver }
    }

    /// Solve a 54-char facelet string with max_depth=21, probe_max=100_000.
    /// Returns the FORWARD solution (apply it to the input cube to reach
    /// solved). Throws on parse / verify / probe-limit errors.
    #[wasm_bindgen]
    pub fn solve(&mut self, facelets: &str) -> Result<String, JsError> {
        self.solver
            .solve(facelets, 21, 100_000, 0, 0)
            .map_err(err_to_js)
    }

    /// Solve with custom parameters.
    #[wasm_bindgen(js_name = solveEx)]
    pub fn solve_ex(
        &mut self,
        facelets: &str,
        max_depth: i32,
        probe_max: u32,
        probe_min: u32,
        verbose_bits: u32,
    ) -> Result<String, JsError> {
        self.solver
            .solve(facelets, max_depth, probe_max as u64, probe_min as u64, verbose_bits)
            .map_err(err_to_js)
    }

    /// Continue searching for shorter solutions after a previous solve().
    /// Same forward-solution semantics as `solve`.
    #[wasm_bindgen]
    pub fn next(&mut self, probe_max: u32, probe_min: u32) -> Result<String, JsError> {
        self.solver
            .next(probe_max as u64, probe_min as u64, 0)
            .map_err(err_to_js)
    }

    /// Convert WCA scramble string ("R U R'") to 54-char facelet string.
    #[wasm_bindgen(js_name = fromScramble)]
    pub fn from_scramble(&self, scramble: &str) -> String {
        tools::from_scramble(scramble, &self.tables)
    }

    /// Apply a WCA scramble to an existing facelet state. Useful for
    /// verifying a solution by applying it back to the scrambled cube.
    #[wasm_bindgen(js_name = applyMoves)]
    pub fn apply_moves(&self, facelets: &str, scramble: &str) -> Result<String, JsError> {
        tools::apply_moves(facelets, scramble, &self.tables)
            .map_err(|e| JsError::new(&format!("{:?}: {}", e, e)))
    }

    /// Generate a uniformly-random cube as a 54-char facelet string.
    #[wasm_bindgen(js_name = randomCube)]
    pub fn random_cube(&self) -> String {
        tools::random_cube_thread()
    }

    /// Super-flip state — known 20-move-optimal hard case.
    #[wasm_bindgen(js_name = superFlip)]
    pub fn super_flip(&self) -> String {
        tools::super_flip()
    }

    /// Length of the last solution (in moves).
    #[wasm_bindgen(js_name = lastLength)]
    pub fn last_length(&self) -> usize {
        self.solver.length()
    }

    /// Number of phase-2 probes used by the last solve.
    #[wasm_bindgen(js_name = lastProbes)]
    pub fn last_probes(&self) -> u32 {
        self.solver.number_of_probes() as u32
    }
}

impl Default for Min2Phase {
    fn default() -> Self {
        Self::new()
    }
}

fn err_to_js(e: SolverError) -> JsError {
    JsError::new(&format!("{:?}: {}", e, e))
}
