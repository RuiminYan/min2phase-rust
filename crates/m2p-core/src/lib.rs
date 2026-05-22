// from Java: cs.min2phase package — Rust port (Util + CubieCube + CoordCube + Search + Tools)

pub mod coord;
pub mod cubie;
pub mod search;
pub mod tools;
pub mod util;

pub use coord::CoordCube;
pub use cubie::CubieCube;
pub use search::{Solver, SolverError, SearchState, VerboseFlag};
pub use tools::{
    from_scramble, from_scramble_moves, random_cube, random_last_layer, super_flip,
    verify_facelets, verify_into, ScrambleParseError, VerifyError,
};

/// Verbose flag bits for `Solver::solve` / `Solver::next`.
pub mod verbose {
    pub const USE_SEPARATOR: u32 = 0x1;
    pub const INVERSE_SOLUTION: u32 = 0x2;
    pub const APPEND_LENGTH: u32 = 0x4;
    pub const OPTIMAL_SOLUTION: u32 = 0x8;
}

pub const N_MOVES: usize = 18;
pub const N_MOVES2: usize = 10;
pub const N_SLICE: usize = 495;
pub const N_TWIST: usize = 2187;
pub const N_TWIST_SYM: usize = 324;
pub const N_FLIP: usize = 2048;
pub const N_FLIP_SYM: usize = 336;
pub const N_PERM: usize = 40320;
pub const N_PERM_SYM: usize = 2768;
pub const N_MPERM: usize = 24;

pub const USE_TWIST_FLIP_PRUN: bool = true;
pub const USE_COMBP_PRUN: bool = USE_TWIST_FLIP_PRUN;
pub const USE_CONJ_PRUN: bool = USE_TWIST_FLIP_PRUN;
pub const N_COMB: usize = if USE_COMBP_PRUN { 140 } else { 70 };
pub const P2_PARITY_MOVE: u32 = if USE_COMBP_PRUN { 0xA5 } else { 0 };

/// All precomputed tables. Built once via `Tables::new()`.
pub struct Tables {
    // CubieCube tables
    pub cube_sym: Vec<CubieCube>,         // 16
    pub move_cube: Vec<CubieCube>,        // 18
    pub move_cube_sym: [u64; 18],
    pub first_move_sym: [i32; 48],
    pub sym_mult: [[u8; 16]; 16],
    pub sym_mult_inv: [[u8; 16]; 16],
    pub sym_move: [[u8; 18]; 16],
    pub sym_8_move: [u8; 8 * 18],
    pub sym_move_ud: [[u8; 18]; 16],

    // Sym2Raw / Raw2Sym lookup tables
    pub flip_s2r: Vec<u16>,        // N_FLIP_SYM
    pub twist_s2r: Vec<u16>,       // N_TWIST_SYM
    pub e_perm_s2r: Vec<u16>,      // N_PERM_SYM
    pub perm2_comb_p: Vec<u8>,     // N_PERM_SYM
    pub perm_inv_edge_sym: Vec<u16>, // N_PERM_SYM
    pub m_perm_inv: Vec<u8>,       // N_MPERM
    pub flip_r2s: Vec<u16>,        // N_FLIP
    pub twist_r2s: Vec<u16>,       // N_TWIST
    pub e_perm_r2s: Vec<u16>,      // N_PERM
    pub flip_s2rf: Vec<u16>,       // N_FLIP_SYM*8 (always present; USE_TWIST_FLIP_PRUN is const true)

    pub sym_state_twist: Vec<u16>, // N_TWIST_SYM
    pub sym_state_flip: Vec<u16>,  // N_FLIP_SYM
    pub sym_state_perm: Vec<u16>,  // N_PERM_SYM

    // Bracket constants from Java: urf1 / urf2 / Cnk
    pub urf1: CubieCube,
    pub urf2: CubieCube,
    pub cnk: [[u32; 13]; 13],
    pub std2ud: [u8; 18],
    pub ckmv2bit: [i32; 11],

    // CoordCube move/conj tables — flat row-major, contiguous in memory.
    // Stride is the second dimension; access via `t.x[i * STRIDE + j]`.
    pub ud_slice_move: Vec<u16>,    // [N_SLICE][N_MOVES]
    pub twist_move: Vec<u16>,       // [N_TWIST_SYM][N_MOVES]
    pub flip_move: Vec<u16>,        // [N_FLIP_SYM][N_MOVES]
    pub ud_slice_conj: Vec<u16>,    // [N_SLICE][8]
    pub c_perm_move: Vec<u16>,      // [N_PERM_SYM][N_MOVES2]
    pub e_perm_move: Vec<u16>,      // [N_PERM_SYM][N_MOVES2]
    pub m_perm_move: Vec<u16>,      // [N_MPERM][N_MOVES2]
    pub m_perm_conj: Vec<u16>,      // [N_MPERM][16]
    pub c_comb_p_move: Vec<u16>,    // [N_COMB][N_MOVES2]
    pub c_comb_p_conj: Vec<u16>,    // [N_COMB][16]

    // Pruning tables (bit-packed 4-bit/entry, Vec<u32> mirrors Java int[]).
    // `USE_TWIST_FLIP_PRUN` is a compile-time const true, so the TFP / S2RF
    // tables are always present and stored as plain Vec (not Option).
    pub ud_slice_twist_prun: Vec<u32>,         // N_SLICE * N_TWIST_SYM / 8 + 1
    pub ud_slice_flip_prun: Vec<u32>,          // N_SLICE * N_FLIP_SYM / 8 + 1
    pub twist_flip_prun: Vec<u32>,             // N_FLIP * N_TWIST_SYM / 8 + 1
    pub mc_perm_prun: Vec<u32>,                // N_MPERM * N_PERM_SYM / 8 + 1
    pub e_perm_c_comb_p_prun: Vec<u32>,        // N_COMB * N_PERM_SYM / 8 + 1
}

impl Default for Tables {
    fn default() -> Self {
        Self::new()
    }
}

impl Tables {
    pub fn new() -> Self {
        let cnk = util::init_cnk();
        let std2ud = util::init_std2ud();
        let ckmv2bit = util::init_ckmv2bit();
        let urf1 = CubieCube::from_coords(2531, 1373, 67026819, 1367);
        let urf2 = CubieCube::from_coords(2089, 1906, 322752913, 2040);
        let move_cube = cubie::init_move();
        let (cube_sym, sym_mult, sym_mult_inv, sym_move, sym_8_move, sym_move_ud) =
            cubie::init_sym(&move_cube, &std2ud);

        // Partial Tables to feed self_symmetry / first_move_sym calc.
        let mut t = Tables {
            cube_sym,
            move_cube,
            move_cube_sym: [0u64; 18],
            first_move_sym: [0i32; 48],
            sym_mult,
            sym_mult_inv,
            sym_move,
            sym_8_move,
            sym_move_ud,

            // Java semantics: EPermR2S/FlipR2S/TwistR2S are pre-zeroed before initSym2Raw
            // populates them. selfSymmetry() reads EPermR2S during init_move_cube_sym; with
            // zeros it degenerates to comparing 0 == 0 which still produces the right syms
            // (cperm pre-filter is a no-op when the lookup is zero everywhere).
            flip_s2r: vec![0u16; N_FLIP_SYM],
            twist_s2r: vec![0u16; N_TWIST_SYM],
            e_perm_s2r: vec![0u16; N_PERM_SYM],
            perm2_comb_p: Vec::new(),
            perm_inv_edge_sym: Vec::new(),
            m_perm_inv: Vec::new(),
            flip_r2s: vec![0u16; N_FLIP],
            twist_r2s: vec![0u16; N_TWIST],
            e_perm_r2s: vec![0u16; N_PERM],
            flip_s2rf: Vec::new(),

            sym_state_twist: Vec::new(),
            sym_state_flip: Vec::new(),
            sym_state_perm: Vec::new(),

            urf1,
            urf2,
            cnk,
            std2ud,
            ckmv2bit,

            // CoordCube fields populated lazily by Tables::build / init_coord_tables.
            ud_slice_move: Vec::new(),
            twist_move: Vec::new(),
            flip_move: Vec::new(),
            ud_slice_conj: Vec::new(),
            c_perm_move: Vec::new(),
            e_perm_move: Vec::new(),
            m_perm_move: Vec::new(),
            m_perm_conj: Vec::new(),
            c_comb_p_move: Vec::new(),
            c_comb_p_conj: Vec::new(),
            ud_slice_twist_prun: Vec::new(),
            ud_slice_flip_prun: Vec::new(),
            twist_flip_prun: Vec::new(),
            mc_perm_prun: Vec::new(),
            e_perm_c_comb_p_prun: Vec::new(),
        };

        // move_cube_sym / first_move_sym — needs cube_sym + sym_mult_inv + sym_move already filled.
        let mc = t.move_cube.clone();
        let (move_cube_sym, first_move_sym) = cubie::init_move_cube_sym(&mc, &t);
        t.move_cube_sym = move_cube_sym;
        t.first_move_sym = first_move_sym;

        // Sym2Raw tables. These only depend on cube_sym + sym_mult_inv (via *_conjugate).
        // Java does Perm first (CoordCube.init does it first too), then Flip, then Twist.
        let (_pc, e_perm_s2r, e_perm_r2s, sym_state_perm, _) =
            cubie::init_sym_2_raw(N_PERM, N_PERM_SYM, 2, &t);
        t.e_perm_s2r = e_perm_s2r;
        t.e_perm_r2s = e_perm_r2s;
        t.sym_state_perm = sym_state_perm;

        let (_fc, flip_s2r, flip_r2s, sym_state_flip, flip_s2rf) =
            cubie::init_sym_2_raw(N_FLIP, N_FLIP_SYM, 0, &t);
        t.flip_s2r = flip_s2r;
        t.flip_r2s = flip_r2s;
        t.sym_state_flip = sym_state_flip;
        t.flip_s2rf = flip_s2rf;

        let (_tc, twist_s2r, twist_r2s, sym_state_twist, _) =
            cubie::init_sym_2_raw(N_TWIST, N_TWIST_SYM, 1, &t);
        t.twist_s2r = twist_s2r;
        t.twist_r2s = twist_r2s;
        t.sym_state_twist = sym_state_twist;

        // Perm2CombP / PermInvEdgeSym / MPermInv (tail of initPermSym2Raw in Java).
        let mut perm2_comb_p = vec![0u8; N_PERM_SYM];
        let mut perm_inv_edge_sym = vec![0u16; N_PERM_SYM];
        // Java's `new CubieCube()` is solved-state (ca={0..7}, ea={0,2,..,22}); using empty()
        // here would leave ea[8..12] at 0, polluting get_comb's slice-edge match against mask 0.
        let mut cc = CubieCube::solved();
        for i in 0..N_PERM_SYM {
            cc.set_eperm(t.e_perm_s2r[i] as i32);
            let comb = util::get_comb(&cc.ea, 0, true, &t.cnk);
            let parity_bonus = if USE_COMBP_PRUN {
                util::get_n_parity(t.e_perm_s2r[i] as i32, 8) * 70
            } else {
                0
            };
            perm2_comb_p[i] = (comb + parity_bonus) as u8;
            cc.inv_cubie_cube();
            perm_inv_edge_sym[i] = cc.get_eperm_sym(&t) as u16;
        }
        let mut m_perm_inv = vec![0u8; N_MPERM];
        for i in 0..N_MPERM {
            cc.set_mperm(i as i32);
            cc.inv_cubie_cube();
            m_perm_inv[i] = cc.get_mperm() as u8;
        }
        t.perm2_comb_p = perm2_comb_p;
        t.perm_inv_edge_sym = perm_inv_edge_sym;
        t.m_perm_inv = m_perm_inv;

        t
    }
}
