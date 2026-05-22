// from Java: cs.min2phase.Search
//
// Two-phase IDA* solver. State (move buffer, node stacks, premove stacks,
// urf cubes) lives in `SearchState`; the public `Solver` adds an `Arc<Tables>`
// so several solver instances can share a single 95ms build.

use std::sync::Arc;

use crate::cubie::{self, CubieCube, URF_MOVE};
use crate::coord::CoordCube;
use crate::util::{Solution, UD2STD};
use crate::tools::{self, VerifyError};
use crate::{Tables, N_COMB, N_MOVES2, N_MPERM, USE_CONJ_PRUN};

// Match Java's package-private knobs.
const MAX_PRE_MOVES: i32 = 20;
const TRY_INVERSE: bool = true;
const TRY_THREE_AXES: bool = true;
const MIN_P1LENGTH_PRE: i32 = 7;
const MAX_DEPTH2_DEFAULT: i32 = 12;

// Verbose flag bits (re-exported via lib::verbose for users).
pub const USE_SEPARATOR: u32 = 0x1;
pub const INVERSE_SOLUTION: u32 = 0x2;
pub const APPEND_LENGTH: u32 = 0x4;
pub const OPTIMAL_SOLUTION: u32 = 0x8;

#[derive(Debug, Clone, Copy)]
pub enum VerboseFlag {
    UseSeparator,
    InverseSolution,
    AppendLength,
    OptimalSolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverError {
    /// Error 1 — facelet parse failure.
    FaceletParse,
    /// Error 2 — edge missing.
    EdgeMissing,
    /// Error 3 — edge flip.
    EdgeFlip,
    /// Error 4 — corner missing.
    CornerMissing,
    /// Error 5 — corner twist.
    CornerTwist,
    /// Error 6 — parity.
    Parity,
    /// Error 7 — no solution exists within `max_depth`.
    NoSolutionInDepth,
    /// Error 8 — probe limit exhausted before a solution was found.
    ProbeLimitExceeded,
}

impl std::fmt::Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SolverError::FaceletParse => "Error 1: facelet parse",
            SolverError::EdgeMissing => "Error 2: edge missing",
            SolverError::EdgeFlip => "Error 3: edge flip",
            SolverError::CornerMissing => "Error 4: corner missing",
            SolverError::CornerTwist => "Error 5: corner twist",
            SolverError::Parity => "Error 6: parity",
            SolverError::NoSolutionInDepth => "Error 7: no solution within max_depth",
            SolverError::ProbeLimitExceeded => "Error 8: probe limit exceeded",
        };
        f.write_str(s)
    }
}
impl std::error::Error for SolverError {}

impl From<VerifyError> for SolverError {
    fn from(e: VerifyError) -> Self {
        match e {
            VerifyError::FaceletParse => SolverError::FaceletParse,
            VerifyError::EdgeMissing => SolverError::EdgeMissing,
            VerifyError::EdgeFlip => SolverError::EdgeFlip,
            VerifyError::CornerMissing => SolverError::CornerMissing,
            VerifyError::CornerTwist => SolverError::CornerTwist,
            VerifyError::Parity => SolverError::Parity,
        }
    }
}

// ===== SearchState =====

pub struct SearchState {
    // Solution move buffer (Java: int[] move = new int[31]).
    pub move_: [i32; 31],

    // Node stacks for phase1 (UD axis + RL/FB for searchopt).
    pub node_ud: [CoordCube; 21],
    pub node_rl: [CoordCube; 21],
    pub node_fb: [CoordCube; 21],

    pub self_sym: u64,
    pub conj_mask: i32,
    pub urf_idx: i32,
    pub length1: i32,
    pub depth1: i32,
    pub max_dep2: i32,
    pub sol_len: i32,
    pub solution: Option<Solution>,
    pub probe: u64,
    pub probe_max: u64,
    pub probe_min: u64,
    pub verbose: u32,
    pub valid1: i32,
    pub allow_shorter: bool,

    pub cc: CubieCube,
    pub urf_cubie_cube: [CubieCube; 6],
    pub urf_coord_cube: [CoordCube; 6],
    pub phase1_cubie: Vec<CubieCube>, // 21

    pub pre_move_cubes: Vec<CubieCube>, // MAX_PRE_MOVES+1, index 0 unused
    pub pre_moves: [i32; MAX_PRE_MOVES as usize],
    pub pre_move_len: i32,
    pub max_pre_moves: i32,

    pub is_rec: bool,
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            move_: [0; 31],
            node_ud: [CoordCube::default(); 21],
            node_rl: [CoordCube::default(); 21],
            node_fb: [CoordCube::default(); 21],
            self_sym: 0,
            conj_mask: 0,
            urf_idx: 0,
            length1: 0,
            depth1: 0,
            max_dep2: 0,
            sol_len: 0,
            solution: None,
            probe: 0,
            probe_max: 0,
            probe_min: 0,
            verbose: 0,
            valid1: 0,
            allow_shorter: false,
            cc: CubieCube::solved(),
            urf_cubie_cube: [
                CubieCube::solved(), CubieCube::solved(), CubieCube::solved(),
                CubieCube::solved(), CubieCube::solved(), CubieCube::solved(),
            ],
            urf_coord_cube: [CoordCube::default(); 6],
            phase1_cubie: vec![CubieCube::solved(); 21],
            pre_move_cubes: vec![CubieCube::solved(); MAX_PRE_MOVES as usize + 1],
            pre_moves: [0; MAX_PRE_MOVES as usize],
            pre_move_len: 0,
            max_pre_moves: 0,
            is_rec: false,
        }
    }
}

// ===== Solver =====

pub struct Solver {
    pub tables: Arc<Tables>,
    pub state: SearchState,
}

impl Solver {
    /// Build a solver with full table init (full BFS depth, fastest solves;
    /// ~100ms cold setup).
    pub fn new() -> Self {
        Self::with_tables(Arc::new(Tables::build(true)))
    }

    /// Build a solver with partial table init (BFS only up to MIN_DEPTH;
    /// faster setup but solves are 5-10x slower until tables warm).
    pub fn new_partial() -> Self {
        Self::with_tables(Arc::new(Tables::build(false)))
    }

    pub fn with_tables(tables: Arc<Tables>) -> Self {
        Self {
            tables,
            state: SearchState::new(),
        }
    }

    pub fn length(&self) -> usize {
        self.state.sol_len.max(0) as usize
    }

    pub fn number_of_probes(&self) -> u64 {
        self.state.probe
    }

    /// from Java: Search.solution(facelets, maxDepth, probeMax, probeMin, verbose)
    pub fn solve(
        &mut self,
        facelets: &str,
        max_depth: i32,
        probe_max: u64,
        probe_min: u64,
        verbose: u32,
    ) -> Result<String, SolverError> {
        // 1. verify the facelet string (fills self.state.cc on success).
        let tables = self.tables.clone();
        let mut cc = CubieCube::solved();
        tools::verify_into(facelets, &mut cc, &tables)?;
        self.state.cc = cc;

        self.state.sol_len = max_depth + 1;
        self.state.probe = 0;
        self.state.probe_max = probe_max;
        self.state.probe_min = probe_min.min(probe_max);
        self.state.verbose = verbose;
        self.state.solution = None;
        self.state.is_rec = false;

        self.init_search();

        if (verbose & OPTIMAL_SOLUTION) == 0 {
            self.search()
        } else {
            self.search_opt()
        }
    }

    /// from Java: Search.next — continue searching for shorter solutions
    /// after a previous `solve` call.
    pub fn next(
        &mut self,
        probe_max: u64,
        probe_min: u64,
        verbose: u32,
    ) -> Result<String, SolverError> {
        self.state.probe = 0;
        self.state.probe_max = probe_max;
        self.state.probe_min = probe_min.min(probe_max);
        self.state.solution = None;
        self.state.is_rec =
            (self.state.verbose & OPTIMAL_SOLUTION) == (verbose & OPTIMAL_SOLUTION);
        self.state.verbose = verbose;
        if (verbose & OPTIMAL_SOLUTION) == 0 {
            self.search()
        } else {
            self.search_opt()
        }
    }

    fn init_search(&mut self) {
        let tables = self.tables.clone();
        let cc = self.state.cc.clone();

        let mut conj_mask =
            (if TRY_INVERSE { 0 } else { 0x38 }) | (if TRY_THREE_AXES { 0 } else { 0x36 });
        let mut self_sym = cc.self_symmetry(&tables);
        conj_mask |= if (self_sym >> 16 & 0xffff) != 0 { 0x12 } else { 0 };
        conj_mask |= if (self_sym >> 32 & 0xffff) != 0 { 0x24 } else { 0 };
        conj_mask |= if (self_sym >> 48 & 0xffff) != 0 { 0x38 } else { 0 };
        self_sym &= 0xffffffffffffu64;
        self.state.conj_mask = conj_mask;
        self.state.self_sym = self_sym;
        self.state.max_pre_moves = if conj_mask > 7 { 0 } else { MAX_PRE_MOVES };

        // Build the 6 URF rotations of the input cube.
        let mut walker = cc;
        for i in 0..6 {
            self.state.urf_cubie_cube[i] = walker.clone();
            let mut coord = CoordCube::default();
            coord.set_with_prun(&tables, &walker, 20);
            self.state.urf_coord_cube[i] = coord;
            walker.urf_conjugate(&tables);
            if i % 3 == 2 {
                walker.inv_cubie_cube();
            }
        }
    }

    // ===== search =====

    fn search(&mut self) -> Result<String, SolverError> {
        let tables = self.tables.clone();
        let sol_len_target = self.state.sol_len;
        let start_length1 = if self.state.is_rec { self.state.length1 } else { 0 };
        let start_urf = if self.state.is_rec { self.state.urf_idx } else { 0 };

        let mut length1 = start_length1;
        let mut first_iter = true;
        while length1 < self.state.sol_len {
            self.state.length1 = length1;
            self.state.max_dep2 = MAX_DEPTH2_DEFAULT.min(self.state.sol_len - length1 - 1);

            let mut urf = if first_iter { start_urf } else { 0 };
            while urf < 6 {
                self.state.urf_idx = urf;
                if (self.state.conj_mask & (1 << urf)) == 0 {
                    let cubie = self.state.urf_cubie_cube[urf as usize].clone();
                    let ssym = (self.state.self_sym & 0xffff) as i32;
                    let max_pre = self.state.max_pre_moves;
                    let ret = self.phase1_pre_moves(&tables, max_pre, -30, cubie, ssym);
                    if ret == 0 {
                        return match self.state.solution.as_ref() {
                            Some(sol) => Ok(sol.render(&cubie::URF_MOVE)),
                            None => Err(SolverError::ProbeLimitExceeded),
                        };
                    }
                }
                urf += 1;
            }
            length1 += 1;
            first_iter = false;
        }
        let _ = sol_len_target;
        match self.state.solution.as_ref() {
            Some(sol) => Ok(sol.render(&cubie::URF_MOVE)),
            None => Err(SolverError::NoSolutionInDepth),
        }
    }

    // ===== phase1PreMoves =====

    fn phase1_pre_moves(
        &mut self,
        tables: &Tables,
        maxl: i32,
        lm: i32,
        cc: CubieCube,
        ssym: i32,
    ) -> i32 {
        self.state.pre_move_len = self.state.max_pre_moves - maxl;
        let pre_len = self.state.pre_move_len;
        let length1 = self.state.length1;

        // Java int shift uses (lm & 31). lm can be -30 in the initial call.
        let cond_normal = pre_len == 0
            || ((0x36FB7i32 >> ((lm & 31) as u32)) & 1) == 0;
        let cond_rec = self.state.depth1 == length1 - pre_len;
        let try_phase1 = if self.state.is_rec { cond_rec } else { cond_normal };

        if try_phase1 {
            self.state.depth1 = length1 - pre_len;
            self.state.phase1_cubie[0] = cc.clone();
            self.state.allow_shorter = self.state.depth1 == MIN_P1LENGTH_PRE && pre_len != 0;

            let d1 = self.state.depth1 as usize;
            let mut node = CoordCube::default();
            if node.set_with_prun(tables, &cc, self.state.depth1) {
                self.state.node_ud[d1 + 1] = node;
                let n = self.state.node_ud[d1 + 1];
                if self.phase1(tables, n, ssym, self.state.depth1, -1) == 0 {
                    return 0;
                }
            }
        }

        if maxl == 0 || pre_len + MIN_P1LENGTH_PRE >= length1 {
            return 1;
        }

        let mut skip_moves = cubie::get_skip_moves(ssym as u64, tables);
        if maxl == 1 || pre_len + 1 + MIN_P1LENGTH_PRE >= length1 {
            skip_moves |= 0x36FB7;
        }

        let lm_axis = lm / 3 * 3;
        let mut m = 0i32;
        while m < 18 {
            if m == lm_axis || m == lm_axis - 9 || m == lm_axis + 9 {
                m += 3;
                continue;
            }
            let preview_pre = self.state.max_pre_moves - maxl;
            if self.state.is_rec && m != self.state.pre_moves[preview_pre as usize] {
                m += 1;
                continue;
            }
            if (skip_moves & (1 << m)) != 0 {
                m += 1;
                continue;
            }
            // CornMult(moveCube[m], cc, preMoveCubes[maxl]);
            // EdgeMult(moveCube[m], cc, preMoveCubes[maxl]);
            let mv = tables.move_cube[m as usize].clone();
            let mut next = CubieCube::empty();
            CubieCube::corn_mult(&mv, &cc, &mut next);
            CubieCube::edge_mult(&mv, &cc, &mut next);
            self.state.pre_move_cubes[maxl as usize] = next.clone();
            self.state.pre_moves[preview_pre as usize] = m;
            let new_ssym = ssym & (tables.move_cube_sym[m as usize] as i32);
            let ret = self.phase1_pre_moves(tables, maxl - 1, m, next, new_ssym);
            if ret == 0 {
                return 0;
            }
            m += 1;
        }
        1
    }

    // ===== phase1 =====

    fn phase1(
        &mut self,
        tables: &Tables,
        node: CoordCube,
        ssym: i32,
        maxl: i32,
        lm: i32,
    ) -> i32 {
        if node.prun == 0 && maxl < 5 {
            if self.state.allow_shorter || maxl == 0 {
                self.state.depth1 -= maxl;
                let ret = self.init_phase2_pre(tables);
                self.state.depth1 += maxl;
                return ret;
            } else {
                return 1;
            }
        }

        let skip_moves = cubie::get_skip_moves(ssym as u64, tables);

        let mut axis = 0i32;
        while axis < 18 {
            if axis == lm || axis == lm - 9 {
                axis += 3;
                continue;
            }
            let mut power = 0;
            while power < 3 {
                let m = (axis + power) as usize;

                if self.state.is_rec
                    && m as i32 != self.state.move_[(self.state.depth1 - maxl) as usize]
                {
                    power += 1;
                    continue;
                }
                if skip_moves != 0 && (skip_moves & (1 << m)) != 0 {
                    power += 1;
                    continue;
                }

                let mut work = CoordCube::default();
                let prun = work.do_move_prun(tables, &node, m, true);
                if prun > maxl {
                    break;
                } else if prun == maxl {
                    power += 1;
                    continue;
                }
                if USE_CONJ_PRUN {
                    let prun_c = work.do_move_prun_conj(tables, &node, m);
                    if prun_c > maxl {
                        break;
                    } else if prun_c == maxl {
                        power += 1;
                        continue;
                    }
                }

                // store work as node_ud[maxl], set move and recurse
                self.state.node_ud[maxl as usize] = work;
                let idx = (self.state.depth1 - maxl) as usize;
                self.state.move_[idx] = m as i32;
                self.state.valid1 = self.state.valid1.min(self.state.depth1 - maxl);
                let new_ssym = ssym & (tables.move_cube_sym[m] as i32);
                let next_node = self.state.node_ud[maxl as usize];
                let ret = self.phase1(tables, next_node, new_ssym, maxl - 1, axis);
                if ret == 0 {
                    return 0;
                } else if ret >= 2 {
                    break;
                }
                power += 1;
            }
            axis += 3;
        }
        1
    }

    // ===== initPhase2Pre / initPhase2 / phase2 =====

    fn init_phase2_pre(&mut self, tables: &Tables) -> i32 {
        self.state.is_rec = false;
        let lim = if self.state.solution.is_none() {
            self.state.probe_max
        } else {
            self.state.probe_min
        };
        if self.state.probe >= lim {
            return 0;
        }
        self.state.probe += 1;

        // Apply moves valid1..depth1 to phase1Cubie.
        for i in self.state.valid1..self.state.depth1 {
            let m = self.state.move_[i as usize] as usize;
            let a = self.state.phase1_cubie[i as usize].clone();
            let mv = tables.move_cube[m].clone();
            let mut prod = CubieCube::empty();
            CubieCube::corn_mult(&a, &mv, &mut prod);
            CubieCube::edge_mult(&a, &mv, &mut prod);
            self.state.phase1_cubie[(i + 1) as usize] = prod;
        }
        self.state.valid1 = self.state.depth1;

        let d1 = self.state.depth1 as usize;
        let cube_d1 = self.state.phase1_cubie[d1].clone();
        let mut p2corn = cube_d1.get_cperm_sym(tables);
        let mut p2csym = p2corn & 0xf;
        p2corn >>= 4;
        let mut p2edge = cube_d1.get_eperm_sym(tables);
        let mut p2esym = p2edge & 0xf;
        p2edge >>= 4;
        let mut p2mid = cube_d1.get_mperm();
        let mut edgei = cubie::get_perm_sym_inv(p2edge, p2esym, false, tables);
        let mut corni = cubie::get_perm_sym_inv(p2corn, p2csym, true, tables);

        let last_move = if self.state.depth1 == 0 {
            -1
        } else {
            self.state.move_[d1 - 1]
        };
        let last_pre = if self.state.pre_move_len == 0 {
            -1
        } else {
            self.state.pre_moves[(self.state.pre_move_len - 1) as usize]
        };

        let mut ret = 0i32;
        let pre_len_pos = if self.state.pre_move_len == 0 { 1 } else { 2 };
        let depth1_pos = if self.state.depth1 == 0 { 1 } else { 2 };
        let p2switch_max = pre_len_pos * depth1_pos;
        let mut p2switch_mask = (1 << p2switch_max) - 1;
        let mut p2switch = 0;
        while p2switch < p2switch_max {
            if (p2switch_mask >> p2switch) & 1 != 0 {
                p2switch_mask &= !(1 << p2switch);
                ret = self.init_phase2(tables, p2corn, p2csym, p2edge, p2esym, p2mid, edgei, corni);
                if ret == 0 || ret > 2 {
                    break;
                } else if ret == 2 {
                    p2switch_mask &= 0x4 << p2switch;
                }
            }
            if p2switch_mask == 0 {
                break;
            }
            if (p2switch & 1) == 0 && self.state.depth1 > 0 {
                let m = tables.std2ud[((last_move / 3) * 3 + 1) as usize] as i32;
                self.state.move_[d1 - 1] = (UD2STD[m as usize] as i32) * 2 - self.state.move_[d1 - 1];

                p2mid = tables.m_perm_move[(p2mid as usize) * N_MOVES2 + m as usize] as i32;
                let cm = tables.c_perm_move[(p2corn as usize) * N_MOVES2 + tables.sym_move_ud[p2csym as usize][m as usize] as usize] as i32;
                p2corn = cm;
                p2csym = tables.sym_mult[(p2corn & 0xf) as usize][p2csym as usize] as i32;
                p2corn >>= 4;
                let em = tables.e_perm_move[(p2edge as usize) * N_MOVES2 + tables.sym_move_ud[p2esym as usize][m as usize] as usize] as i32;
                p2edge = em;
                p2esym = tables.sym_mult[(p2edge & 0xf) as usize][p2esym as usize] as i32;
                p2edge >>= 4;
                corni = cubie::get_perm_sym_inv(p2corn, p2csym, true, tables);
                edgei = cubie::get_perm_sym_inv(p2edge, p2esym, false, tables);
            } else if self.state.pre_move_len > 0 {
                let m = tables.std2ud[((last_pre / 3) * 3 + 1) as usize] as i32;
                let pml = (self.state.pre_move_len - 1) as usize;
                self.state.pre_moves[pml] = (UD2STD[m as usize] as i32) * 2 - self.state.pre_moves[pml];

                let inv_mid = tables.m_perm_inv[p2mid as usize] as i32;
                let mid_moved = tables.m_perm_move[(inv_mid as usize) * N_MOVES2 + m as usize] as i32;
                p2mid = tables.m_perm_inv[mid_moved as usize] as i32;

                p2corn = tables.c_perm_move[((corni >> 4) as usize) * N_MOVES2 + tables.sym_move_ud[(corni & 0xf) as usize][m as usize] as usize] as i32;
                corni = (p2corn & !0xf) | (tables.sym_mult[(p2corn & 0xf) as usize][(corni & 0xf) as usize] as i32);
                p2corn = cubie::get_perm_sym_inv(corni >> 4, corni & 0xf, true, tables);
                p2csym = p2corn & 0xf;
                p2corn >>= 4;

                p2edge = tables.e_perm_move[((edgei >> 4) as usize) * N_MOVES2 + tables.sym_move_ud[(edgei & 0xf) as usize][m as usize] as usize] as i32;
                edgei = (p2edge & !0xf) | (tables.sym_mult[(p2edge & 0xf) as usize][(edgei & 0xf) as usize] as i32);
                p2edge = cubie::get_perm_sym_inv(edgei >> 4, edgei & 0xf, false, tables);
                p2esym = p2edge & 0xf;
                p2edge >>= 4;
            }
            p2switch += 1;
        }
        if self.state.depth1 > 0 {
            self.state.move_[d1 - 1] = last_move;
        }
        if self.state.pre_move_len > 0 {
            let pml = (self.state.pre_move_len - 1) as usize;
            self.state.pre_moves[pml] = last_pre;
        }
        if ret == 0 {
            0
        } else {
            2
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn init_phase2(
        &mut self,
        tables: &Tables,
        p2corn: i32,
        p2csym: i32,
        p2edge: i32,
        p2esym: i32,
        p2mid: i32,
        edgei: i32,
        corni: i32,
    ) -> i32 {
        let prun_a = crate::coord::get_pruning(
            &tables.e_perm_c_comb_p_prun,
            ((edgei >> 4) as usize) * N_COMB
                + tables.c_comb_p_conj
                    [((tables.perm2_comb_p[(corni >> 4) as usize] as usize) & 0xff) * 16
                        + tables.sym_mult_inv[(edgei & 0xf) as usize][(corni & 0xf) as usize] as usize]
                    as usize,
        ) as i32;
        let prun_b = crate::coord::get_pruning(
            &tables.e_perm_c_comb_p_prun,
            (p2edge as usize) * N_COMB
                + tables.c_comb_p_conj
                    [((tables.perm2_comb_p[p2corn as usize] as usize) & 0xff) * 16
                        + tables.sym_mult_inv[p2esym as usize][p2csym as usize] as usize]
                    as usize,
        ) as i32;
        let prun_c = crate::coord::get_pruning(
            &tables.mc_perm_prun,
            (p2corn as usize) * N_MPERM
                + tables.m_perm_conj[(p2mid as usize) * 16 + p2csym as usize] as usize,
        ) as i32;
        let prun = prun_a.max(prun_b.max(prun_c));

        if prun > self.state.max_dep2 {
            return prun - self.state.max_dep2;
        }

        let mut depth2 = self.state.max_dep2;
        while depth2 >= prun {
            let ret = self.phase2(
                tables, p2edge, p2esym, p2corn, p2csym, p2mid, depth2,
                self.state.depth1, 10,
            );
            if ret < 0 {
                break;
            }
            depth2 -= ret;
            self.state.sol_len = 0;
            let mut sol = Solution::new();
            sol.set_args(self.state.verbose, self.state.urf_idx, self.state.depth1);
            for i in 0..(self.state.depth1 + depth2) {
                sol.append_sol_move(self.state.move_[i as usize]);
            }
            for i in (0..self.state.pre_move_len).rev() {
                sol.append_sol_move(self.state.pre_moves[i as usize]);
            }
            self.state.sol_len = sol.length as i32;
            self.state.solution = Some(sol);
            depth2 -= 1;
        }

        if depth2 != self.state.max_dep2 {
            self.state.max_dep2 =
                MAX_DEPTH2_DEFAULT.min(self.state.sol_len - self.state.length1 - 1);
            return if self.state.probe >= self.state.probe_min { 0 } else { 1 };
        }
        1
    }

    #[allow(clippy::too_many_arguments)]
    fn phase2(
        &mut self,
        tables: &Tables,
        edge: i32,
        esym: i32,
        corn: i32,
        csym: i32,
        mid: i32,
        maxl: i32,
        depth: i32,
        lm: i32,
    ) -> i32 {
        if edge == 0 && corn == 0 && mid == 0 {
            return maxl;
        }
        let move_mask = tables.ckmv2bit[lm as usize];
        let mut m = 0i32;
        while m < 10 {
            if (move_mask >> m) & 1 != 0 {
                m += ((0x42i32 >> m) & 3) + 1;
                continue;
            }
            let midx = tables.m_perm_move[(mid as usize) * N_MOVES2 + m as usize] as i32;
            let mut cornx = tables.c_perm_move
                [(corn as usize) * N_MOVES2 + tables.sym_move_ud[csym as usize][m as usize] as usize] as i32;
            let csymx = tables.sym_mult[(cornx & 0xf) as usize][csym as usize] as i32;
            cornx >>= 4;
            let mut edgex = tables.e_perm_move
                [(edge as usize) * N_MOVES2 + tables.sym_move_ud[esym as usize][m as usize] as usize] as i32;
            let esymx = tables.sym_mult[(edgex & 0xf) as usize][esym as usize] as i32;
            edgex >>= 4;
            let edgei = cubie::get_perm_sym_inv(edgex, esymx, false, tables);
            let corni = cubie::get_perm_sym_inv(cornx, csymx, true, tables);

            let mut prun = crate::coord::get_pruning(
                &tables.e_perm_c_comb_p_prun,
                ((edgei >> 4) as usize) * N_COMB
                    + tables.c_comb_p_conj
                        [((tables.perm2_comb_p[(corni >> 4) as usize] as usize) & 0xff) * 16
                            + tables.sym_mult_inv[(edgei & 0xf) as usize][(corni & 0xf) as usize] as usize]
                        as usize,
            ) as i32;
            if prun > maxl + 1 {
                return maxl - prun + 1;
            } else if prun >= maxl {
                m += ((0x42i32 >> m) & 3 & (maxl - prun)) + 1;
                continue;
            }
            prun = crate::coord::get_pruning(
                &tables.mc_perm_prun,
                (cornx as usize) * N_MPERM
                    + tables.m_perm_conj[(midx as usize) * 16 + csymx as usize] as usize,
            ) as i32;
            let prun_b = crate::coord::get_pruning(
                &tables.e_perm_c_comb_p_prun,
                (edgex as usize) * N_COMB
                    + tables.c_comb_p_conj
                        [((tables.perm2_comb_p[cornx as usize] as usize) & 0xff) * 16
                            + tables.sym_mult_inv[esymx as usize][csymx as usize] as usize]
                        as usize,
            ) as i32;
            let prun = prun.max(prun_b);
            if prun >= maxl {
                m += ((0x42i32 >> m) & 3 & (maxl - prun)) + 1;
                continue;
            }
            let ret = self.phase2(tables, edgex, esymx, cornx, csymx, midx, maxl - 1, depth + 1, m);
            if ret >= 0 {
                self.state.move_[depth as usize] = UD2STD[m as usize] as i32;
                return ret;
            }
            if ret < -2 {
                break;
            }
            if ret < -1 {
                m += (0x42i32 >> m) & 3;
            }
            m += 1;
        }
        -1
    }

    // ===== searchopt / phase1opt =====

    fn search_opt(&mut self) -> Result<String, SolverError> {
        let tables = self.tables.clone();
        let mut maxprun1 = 0i32;
        let mut maxprun2 = 0i32;
        for i in 0..6 {
            // calc_pruning is in-place
            let mut node = self.state.urf_coord_cube[i];
            node.calc_pruning(&tables, false);
            self.state.urf_coord_cube[i] = node;
            if i < 3 {
                maxprun1 = maxprun1.max(node.prun);
            } else {
                maxprun2 = maxprun2.max(node.prun);
            }
        }
        let urf_start = if maxprun2 > maxprun1 { 3 } else { 0 };
        self.state.urf_idx = urf_start;
        self.state.phase1_cubie[0] = self.state.urf_cubie_cube[urf_start as usize].clone();

        let start_length1 = if self.state.is_rec { self.state.length1 } else { 0 };
        let mut length1 = start_length1;
        while length1 < self.state.sol_len {
            self.state.length1 = length1;
            let ud = self.state.urf_coord_cube[urf_start as usize];
            let rl = self.state.urf_coord_cube[(urf_start + 1) as usize];
            let fb = self.state.urf_coord_cube[(urf_start + 2) as usize];

            if ud.prun <= length1 && rl.prun <= length1 && fb.prun <= length1 {
                let ssym = self.state.self_sym;
                let ret = self.phase1_opt(&tables, ud, rl, fb, ssym, length1, -1);
                if ret == 0 {
                    return match self.state.solution.as_ref() {
                        Some(sol) => Ok(sol.render(&cubie::URF_MOVE)),
                        None => Err(SolverError::ProbeLimitExceeded),
                    };
                }
            }
            length1 += 1;
        }
        match self.state.solution.as_ref() {
            Some(sol) => Ok(sol.render(&cubie::URF_MOVE)),
            None => Err(SolverError::NoSolutionInDepth),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn phase1_opt(
        &mut self,
        tables: &Tables,
        ud: CoordCube,
        rl: CoordCube,
        fb: CoordCube,
        ssym: u64,
        maxl: i32,
        lm: i32,
    ) -> i32 {
        if ud.prun == 0 && rl.prun == 0 && fb.prun == 0 && maxl < 5 {
            self.state.max_dep2 = maxl;
            self.state.depth1 = self.state.length1 - maxl;
            return if self.init_phase2_pre(tables) == 0 { 0 } else { 1 };
        }

        let skip_moves = cubie::get_skip_moves(ssym, tables);

        let mut axis = 0i32;
        while axis < 18 {
            if axis == lm || axis == lm - 9 {
                axis += 3;
                continue;
            }
            let mut power = 0;
            while power < 3 {
                let mut m = (axis + power) as usize;

                if self.state.is_rec
                    && m as i32
                        != self.state.move_[(self.state.length1 - maxl) as usize]
                {
                    power += 1;
                    continue;
                }
                if skip_moves != 0 && (skip_moves & (1 << m)) != 0 {
                    power += 1;
                    continue;
                }

                // UD axis
                let mut work_ud = CoordCube::default();
                let p_ud_a = work_ud.do_move_prun(tables, &ud, m, false);
                let p_ud = if USE_CONJ_PRUN {
                    p_ud_a.max(work_ud.do_move_prun_conj(tables, &ud, m))
                } else {
                    p_ud_a
                };
                if p_ud > maxl {
                    break;
                } else if p_ud == maxl {
                    power += 1;
                    continue;
                }

                // RL axis: m = urfMove[2][m]
                m = URF_MOVE[2][m] as usize;
                let mut work_rl = CoordCube::default();
                let p_rl_a = work_rl.do_move_prun(tables, &rl, m, false);
                let p_rl = if USE_CONJ_PRUN {
                    p_rl_a.max(work_rl.do_move_prun_conj(tables, &rl, m))
                } else {
                    p_rl_a
                };
                if p_rl > maxl {
                    break;
                } else if p_rl == maxl {
                    power += 1;
                    continue;
                }

                // FB axis
                m = URF_MOVE[2][m] as usize;
                let mut work_fb = CoordCube::default();
                let p_fb_a = work_fb.do_move_prun(tables, &fb, m, false);
                let mut p_fb = if USE_CONJ_PRUN {
                    p_fb_a.max(work_fb.do_move_prun_conj(tables, &fb, m))
                } else {
                    p_fb_a
                };
                if p_ud == p_rl && p_rl == p_fb && p_fb != 0 {
                    p_fb += 1;
                }
                if p_fb > maxl {
                    break;
                } else if p_fb == maxl {
                    power += 1;
                    continue;
                }

                m = URF_MOVE[2][m] as usize;
                self.state.node_ud[maxl as usize] = work_ud;
                self.state.node_rl[maxl as usize] = work_rl;
                self.state.node_fb[maxl as usize] = work_fb;
                self.state.move_[(self.state.length1 - maxl) as usize] = m as i32;
                self.state.valid1 = self.state.valid1.min(self.state.length1 - maxl);
                let new_ssym = ssym & tables.move_cube_sym[m];
                let ret = self.phase1_opt(
                    tables,
                    self.state.node_ud[maxl as usize],
                    self.state.node_rl[maxl as usize],
                    self.state.node_fb[maxl as usize],
                    new_ssym,
                    maxl - 1,
                    axis,
                );
                if ret == 0 {
                    return 0;
                }
                power += 1;
            }
            axis += 3;
        }
        1
    }
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools;
    use std::fs;
    use std::path::PathBuf;
    use std::time::Instant;

    #[test]
    fn solve_super_flip_roundtrip() {
        let mut solver = Solver::new();
        let sf = tools::super_flip();
        let sol = solver.solve(&sf, 21, 100_000, 0, INVERSE_SOLUTION).expect("solve");
        let regen = tools::from_scramble(&sol, &solver.tables);
        assert_eq!(regen, sf);
        // Super flip is a known 20-mover; min2phase finds ≤ 21 reliably.
        assert!(solver.length() <= 21);
    }

    #[test]
    fn solve_short_scramble() {
        let mut solver = Solver::new();
        let facelets = tools::from_scramble("R U", &solver.tables);
        let sol = solver.solve(&facelets, 21, 100_000, 0, INVERSE_SOLUTION).expect("solve");
        let regen = tools::from_scramble(&sol, &solver.tables);
        assert_eq!(regen, facelets);
        // 2-mover should be found at length ≤ 2.
        assert!(solver.length() <= 2);
    }

    #[test]
    fn solve_solved_cube() {
        let mut solver = Solver::new();
        let solved = "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB";
        let _sol = solver.solve(solved, 21, 100_000, 0, 0).expect("solve");
        assert_eq!(solver.length(), 0);
    }

    #[test]
    fn solver_error_on_bad_facelets() {
        let mut solver = Solver::new();
        // 54 chars but the count check fails (extra U, missing R)
        let bad = "UUUUUUUUUUUUUUUUUUFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB";
        let err = solver.solve(bad, 21, 100_000, 0, 0).unwrap_err();
        assert_eq!(err, SolverError::FaceletParse);
    }

    #[test]
    fn solves_java_fixture_100() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("..");
        path.push("..");
        path.push("fixtures");
        path.push("java_100.tsv");
        let txt = fs::read_to_string(&path).expect("read java_100.tsv");
        let mut solver = Solver::new();

        let mut total_us: u128 = 0;
        let mut total_len: usize = 0;
        let mut count = 0usize;

        for (lineno, line) in txt.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            assert!(cols.len() >= 2, "bad TSV line {}", lineno + 1);
            let facelets = cols[0];

            let t0 = Instant::now();
            let sol = match solver.solve(facelets, 21, 100_000, 0, INVERSE_SOLUTION) {
                Ok(s) => s,
                Err(e) => panic!(
                    "case {}: solve error {:?} on facelets {}",
                    lineno + 1,
                    e,
                    facelets
                ),
            };
            let elapsed = t0.elapsed();
            total_us += elapsed.as_micros();

            // Round-trip: applying the returned (inverse) solution as a scramble
            // must reproduce the input facelets.
            let regen = tools::from_scramble(&sol, &solver.tables);
            if regen != facelets {
                panic!(
                    "case {}: roundtrip mismatch\n  input:  {}\n  sol:    {}\n  regen:  {}",
                    lineno + 1,
                    facelets,
                    sol,
                    regen
                );
            }

            // Length check (count moves: each move is followed by a space).
            let move_count = sol.split_whitespace().count();
            assert!(move_count <= 21, "case {}: solution too long: {} moves", lineno + 1, move_count);
            total_len += move_count;
            count += 1;
        }

        let avg_us = total_us as f64 / count as f64;
        let avg_len = total_len as f64 / count as f64;
        eprintln!(
            "solves_java_fixture_100: {} cases, avg {:.0}us/solve, avg {:.2} moves",
            count, avg_us, avg_len
        );
        assert_eq!(count, 100);
    }
}
