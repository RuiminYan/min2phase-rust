// from Java: cs.min2phase.CoordCube
//
// Phase 2b — coordinate cube + move/pruning table init. Mirrors Java structure verbatim;
// all signed Java `int` bit math is reproduced on `u32` (same mod-2^32 semantics).

use crate::cubie::{CubieCube, SYM_E2C_MAGIC};
use crate::util;
use crate::{
    Tables, N_COMB, N_FLIP, N_FLIP_SYM, N_MOVES, N_MOVES2, N_MPERM, N_PERM_SYM, N_SLICE,
    N_TWIST_SYM, P2_PARITY_MOVE, USE_CONJ_PRUN, USE_TWIST_FLIP_PRUN,
};

// ===== bit-packed pruning helpers =====

/// from Java: `table[index >> 3] >> (index << 2) & 0xf` — Java `int` shift is mod-32,
/// so `(index << 2)` is `(index & 7) << 2`. 4 bits per entry, 8 entries per u32.
#[inline]
pub fn get_pruning(table: &[u32], index: usize) -> u32 {
    (table[index >> 3] >> ((index & 7) << 2)) & 0xf
}

/// from Java: `table[index >> 3] ^= value << (index << 2)` — XOR-write 4 bits.
#[inline]
pub fn set_pruning(table: &mut [u32], index: usize, value: u32) {
    table[index >> 3] ^= value << ((index & 7) << 2);
}

/// from Java: `((val - 0x11111111) & ~val & 0x88888888) != 0`
#[inline]
pub fn has_zero(val: u32) -> bool {
    (val.wrapping_sub(0x11111111) & !val & 0x88888888) != 0
}

// ===== move/conj table init (operate on Tables fields) =====

fn init_ud_slice_move_conj(t: &mut Tables) {
    let mut c = CubieCube::solved();
    let mut d = CubieCube::empty();
    for i in 0..N_SLICE {
        c.set_ud_slice(i as i32, &t.cnk);
        let mut j = 0usize;
        while j < N_MOVES {
            CubieCube::edge_mult(&c, &t.move_cube[j], &mut d);
            t.ud_slice_move[i * N_MOVES + j] = d.get_ud_slice(&t.cnk) as u16;
            j += 3;
        }
        let mut j = 0usize;
        while j < 16 {
            let inv = t.sym_mult_inv[0][j] as i32;
            CubieCube::edge_conjugate(&c, inv, &mut d, t);
            t.ud_slice_conj[i * 8 + (j >> 1)] = d.get_ud_slice(&t.cnk) as u16;
            j += 2;
        }
    }
    // Fill in non-power moves (j+1, j+2) via chaining.
    for i in 0..N_SLICE {
        let mut j = 0usize;
        while j < N_MOVES {
            let mut udslice = t.ud_slice_move[i * N_MOVES + j] as usize;
            for k in 1..3 {
                udslice = t.ud_slice_move[udslice * N_MOVES + j] as usize;
                t.ud_slice_move[i * N_MOVES + j + k] = udslice as u16;
            }
            j += 3;
        }
    }
}

fn init_flip_move(t: &mut Tables) {
    let mut c = CubieCube::solved();
    let mut d = CubieCube::empty();
    for i in 0..N_FLIP_SYM {
        c.set_flip(t.flip_s2r[i] as i32);
        for j in 0..N_MOVES {
            CubieCube::edge_mult(&c, &t.move_cube[j], &mut d);
            t.flip_move[i * N_MOVES + j] = d.get_flip_sym(t) as u16;
        }
    }
}

fn init_twist_move(t: &mut Tables) {
    let mut c = CubieCube::solved();
    let mut d = CubieCube::empty();
    for i in 0..N_TWIST_SYM {
        c.set_twist(t.twist_s2r[i] as i32);
        for j in 0..N_MOVES {
            CubieCube::corn_mult(&c, &t.move_cube[j], &mut d);
            t.twist_move[i * N_MOVES + j] = d.get_twist_sym(t) as u16;
        }
    }
}

fn init_cperm_move(t: &mut Tables) {
    let mut c = CubieCube::solved();
    let mut d = CubieCube::empty();
    for i in 0..N_PERM_SYM {
        c.set_cperm(t.e_perm_s2r[i] as i32);
        for j in 0..N_MOVES2 {
            let mv = util::UD2STD[j] as usize;
            CubieCube::corn_mult(&c, &t.move_cube[mv], &mut d);
            t.c_perm_move[i * N_MOVES2 + j] = d.get_cperm_sym(t) as u16;
        }
    }
}

fn init_eperm_move(t: &mut Tables) {
    let mut c = CubieCube::solved();
    let mut d = CubieCube::empty();
    for i in 0..N_PERM_SYM {
        c.set_eperm(t.e_perm_s2r[i] as i32);
        for j in 0..N_MOVES2 {
            let mv = util::UD2STD[j] as usize;
            CubieCube::edge_mult(&c, &t.move_cube[mv], &mut d);
            t.e_perm_move[i * N_MOVES2 + j] = d.get_eperm_sym(t) as u16;
        }
    }
}

fn init_mperm_move_conj(t: &mut Tables) {
    let mut c = CubieCube::solved();
    let mut d = CubieCube::empty();
    for i in 0..N_MPERM {
        c.set_mperm(i as i32);
        for j in 0..N_MOVES2 {
            let mv = util::UD2STD[j] as usize;
            CubieCube::edge_mult(&c, &t.move_cube[mv], &mut d);
            t.m_perm_move[i * N_MOVES2 + j] = d.get_mperm() as u16;
        }
        for j in 0..16 {
            let inv = t.sym_mult_inv[0][j] as i32;
            CubieCube::edge_conjugate(&c, inv, &mut d, t);
            t.m_perm_conj[i * 16 + j] = d.get_mperm() as u16;
        }
    }
}

fn init_comb_p_move_conj(t: &mut Tables) {
    let mut c = CubieCube::solved();
    let mut d = CubieCube::empty();
    let p2pm = P2_PARITY_MOVE; // u32
    for i in 0..N_COMB {
        c.set_ccomb((i % 70) as i32, &t.cnk);
        for j in 0..N_MOVES2 {
            let mv = util::UD2STD[j] as usize;
            CubieCube::corn_mult(&c, &t.move_cube[mv], &mut d);
            let combo = d.get_ccomb(&t.cnk) as u32;
            let bonus = 70u32 * (((p2pm >> j) & 1) ^ ((i as u32) / 70));
            t.c_comb_p_move[i * N_MOVES2 + j] = (combo + bonus) as u16;
        }
        for j in 0..16 {
            let inv = t.sym_mult_inv[0][j] as i32;
            CubieCube::corn_conjugate(&c, inv, &mut d, t);
            let combo = d.get_ccomb(&t.cnk) as u32;
            let bonus = 70u32 * ((i as u32) / 70);
            t.c_comb_p_conj[i * 16 + j] = (combo + bonus) as u16;
        }
    }
}

// ===== generic pruning table BFS =====
//
// PrunFlag layout (from Java comment):
//   bits  0..3:  SYM_SHIFT
//   bit   4:     use SYM_E2C_MAGIC for sym-state branch conjugation
//   bit   5:     is_phase2 (selects N_MOVES = 10 vs 18, and NEXT_AXIS_MAGIC)
//   bits  8..11: INV_DEPTH
//   bits 12..15: MAX_DEPTH
//   bits 16..19: MIN_DEPTH

/// Source of move/conj data for the BFS step. Variant `Tfp` is the special
/// `RawMove == null` branch in Java (Twist-Flip pruning) which reads through
/// `FlipMove` / `Sym8Move` / `FlipS2RF` instead of a raw move table.
///
/// For `RawConj`, raw_move is `[n_raw][raw_move_stride]` flat and
/// raw_conj is `[n_raw][raw_conj_stride]` flat.
enum BfsSource<'a> {
    RawConj {
        raw_move: &'a [u16],
        raw_move_stride: usize,
        raw_conj: &'a [u16],
        raw_conj_stride: usize,
        n_raw: usize,
    },
    Tfp,
}

#[allow(clippy::too_many_arguments)]
fn init_raw_sym_prun(
    prun_table: &mut [u32],
    src: BfsSource,
    sym_move: &[u16],
    sym_move_stride: usize,
    n_sym: usize,
    sym_state: &[u16],
    prun_flag: u32,
    full_init: bool,
    // extras used only by the Tfp variant
    flip_r2s: &[u16],
    flip_s2rf: &[u16],
    sym_8_move: &[u8; 8 * 18],
    flip_move: &[u16],
    flip_move_stride: usize,
) {
    let sym_shift: u32 = prun_flag & 0xf;
    let sym_e2c_magic: u32 = if ((prun_flag >> 4) & 1) == 1 { SYM_E2C_MAGIC } else { 0 };
    let is_phase2 = ((prun_flag >> 5) & 1) == 1;
    let inv_depth: u32 = (prun_flag >> 8) & 0xf;
    let max_depth: u32 = (prun_flag >> 12) & 0xf;
    let min_depth: u32 = (prun_flag >> 16) & 0xf;
    let search_depth: u32 = if full_init { max_depth } else { min_depth };

    let sym_mask: u32 = (1u32 << sym_shift) - 1;
    let istfp = matches!(src, BfsSource::Tfp);
    let n_raw: usize = match &src {
        BfsSource::Tfp => N_FLIP,
        BfsSource::RawConj { n_raw, .. } => *n_raw,
    };
    let n_size: usize = n_raw * n_sym;
    let n_moves_local: usize = if is_phase2 { 10 } else { 18 };
    let next_axis_magic: u32 = if n_moves_local == 10 { 0x42 } else { 0x92492 };

    // depth = getPruning(table, N_SIZE) - 1; on uninitialised (all zero) table this is -1.
    let probe = get_pruning(prun_table, n_size) as i32;
    let mut depth: i32 = probe - 1;

    if depth == -1 {
        for slot in prun_table.iter_mut() {
            *slot = 0x11111111;
        }
        // setPruning(table, 0, 0 ^ 1) — flips the low nibble from 1 to 0.
        set_pruning(prun_table, 0, 0 ^ 1);
        depth = 0;
        // `done = 1` in Java is debug-only counter; skip.
    }

    while (depth as u32) < search_depth {
        // Increment all entries whose value == depth+1 by tagging a 0 nibble.
        // mask = (depth+1) * 0x11111111 XOR 0xffffffff
        let mask: u32 = (depth as u32 + 1).wrapping_mul(0x11111111) ^ 0xffffffff;
        for slot in prun_table.iter_mut() {
            // Java: val = PrunTable[i] ^ mask; val &= val >> 1; PrunTable[i] += val & (val >> 2) & 0x11111111;
            // The `val >> 2` reads the POST-assignment val (= val & (val>>1)), not the original.
            let v0 = *slot ^ mask;
            let val = v0 & (v0 >> 1);
            *slot = slot.wrapping_add(val & (val >> 2) & 0x11111111);
        }

        let inv = depth > inv_depth as i32;
        let select: u32 = if inv { depth as u32 + 2 } else { depth as u32 };
        let sel_arr_mask: u32 = select.wrapping_mul(0x11111111);
        let check: u32 = if inv { depth as u32 } else { depth as u32 + 2 };
        depth += 1;
        let xor_val: u32 = (depth as u32) ^ (depth as u32 + 1);

        let mut val: u32 = 0;
        let mut i: usize = 0;
        while i < n_size {
            if (i & 7) == 0 {
                val = prun_table[i >> 3];
                if !has_zero(val ^ sel_arr_mask) {
                    i += 8;
                    continue;
                }
            }
            // Fall-through: examine low nibble
            if (val & 0xf) != select {
                val >>= 4;
                i += 1;
                continue;
            }
            let raw = i % n_raw;
            let sym = i / n_raw;
            // For ISTFP: flip = FlipR2S[raw]; fsym = flip & 7; flip >>= 3
            let (flip, fsym) = if istfp {
                let f = flip_r2s[raw] as u32;
                (f >> 3, f & 7)
            } else {
                (0, 0)
            };

            let mut m: usize = 0;
            while m < n_moves_local {
                let mut symx: u32 = sym_move[sym * sym_move_stride + m] as u32;
                let rawx: u32 = if istfp {
                    // FlipS2RF[FlipMove[flip][Sym8Move[m<<3 | fsym]] ^ fsym ^ (symx & SYM_MASK)]
                    let s8 = sym_8_move[(m << 3) | (fsym as usize)] as usize;
                    let fm = flip_move[(flip as usize) * flip_move_stride + s8] as u32;
                    let arg = fm ^ fsym ^ (symx & sym_mask);
                    flip_s2rf[arg as usize] as u32
                } else {
                    match &src {
                        BfsSource::RawConj { raw_move, raw_move_stride, raw_conj, raw_conj_stride, .. } => {
                            let rm = raw_move[raw * (*raw_move_stride) + m] as usize;
                            raw_conj[rm * (*raw_conj_stride) + (symx & sym_mask) as usize] as u32
                        }
                        BfsSource::Tfp => unreachable!(),
                    }
                };
                symx >>= sym_shift;
                let idx = (symx as usize) * n_raw + rawx as usize;
                let prun = get_pruning(prun_table, idx);
                if prun != check {
                    if (prun as i32) < depth - 1 {
                        m += ((next_axis_magic >> m) & 3) as usize;
                    }
                    m += 1;
                    continue;
                }
                if inv {
                    set_pruning(prun_table, i, xor_val);
                    break;
                }
                set_pruning(prun_table, idx, xor_val);

                // Sym-state self-orbit propagation. j starts at 1; symState >>= 1 each step.
                let mut sym_state_v: u32 = sym_state[symx as usize] as u32;
                let mut j: u32 = 1;
                loop {
                    sym_state_v >>= 1;
                    if sym_state_v == 0 {
                        break;
                    }
                    if (sym_state_v & 1) == 1 {
                        let mut idxx = (symx as usize) * n_raw;
                        if istfp {
                            // FlipS2RF[FlipR2S[rawx] ^ j]
                            let arg = (flip_r2s[rawx as usize] as u32) ^ j;
                            idxx += flip_s2rf[arg as usize] as usize;
                        } else {
                            match &src {
                                BfsSource::RawConj { raw_conj, raw_conj_stride, .. } => {
                                    let conj_idx = (j ^ ((sym_e2c_magic >> (j << 1)) & 3)) as usize;
                                    idxx += raw_conj[(rawx as usize) * (*raw_conj_stride) + conj_idx] as usize;
                                }
                                BfsSource::Tfp => unreachable!(),
                            }
                        }
                        if get_pruning(prun_table, idxx) == check {
                            set_pruning(prun_table, idxx, xor_val);
                        }
                    }
                    j += 1;
                }
                m += 1;
            }
            val >>= 4;
            i += 1;
        }
    }
}

// ===== public entry: extend Tables init with CoordCube tables =====

impl Tables {
    /// Build all tables. `full_init=true` extends BFS depth to MAX_DEPTH (matches Java's
    /// `init(true)` which is what the solver actually wants).
    pub fn build(full_init: bool) -> Self {
        let mut t = Self::new();
        t.init_coord_tables(full_init);
        t
    }

    fn init_coord_tables(&mut self, full_init: bool) {
        // Allocate flat row-major tables (single contiguous Vec per table).
        self.ud_slice_move = vec![0u16; N_SLICE * N_MOVES];
        self.twist_move = vec![0u16; N_TWIST_SYM * N_MOVES];
        self.flip_move = vec![0u16; N_FLIP_SYM * N_MOVES];
        self.ud_slice_conj = vec![0u16; N_SLICE * 8];
        self.c_perm_move = vec![0u16; N_PERM_SYM * N_MOVES2];
        self.e_perm_move = vec![0u16; N_PERM_SYM * N_MOVES2];
        self.m_perm_move = vec![0u16; N_MPERM * N_MOVES2];
        self.m_perm_conj = vec![0u16; N_MPERM * 16];
        self.c_comb_p_move = vec![0u16; N_COMB * N_MOVES2];
        self.c_comb_p_conj = vec![0u16; N_COMB * 16];

        // Java order in CoordCube.init():
        //   phase2: CPerm, EPerm, MPermConj, CombPConj
        //   phase1: Flip, Twist, UDSliceConj
        init_cperm_move(self);
        init_eperm_move(self);
        init_mperm_move_conj(self);
        init_comb_p_move_conj(self);
        init_flip_move(self);
        init_twist_move(self);
        init_ud_slice_move_conj(self);

        // Allocate pruning tables (+1 sentinel int beyond data, per Java).
        self.ud_slice_twist_prun = vec![0u32; N_SLICE * N_TWIST_SYM / 8 + 1];
        self.ud_slice_flip_prun = vec![0u32; N_SLICE * N_FLIP_SYM / 8 + 1];
        self.mc_perm_prun = vec![0u32; N_MPERM * N_PERM_SYM / 8 + 1];
        self.e_perm_c_comb_p_prun = vec![0u32; N_COMB * N_PERM_SYM / 8 + 1];
        self.twist_flip_prun = vec![0u32; N_FLIP * N_TWIST_SYM / 8 + 1];

        // Run BFS. Move the prun-table fields out via std::mem::take to avoid
        // overlapping borrows with the rest of `self` (which we pass for read).
        // initMCPermPrun: MPermMove / MPermConj, CPermMove (phase2), SymStatePerm, flag 0x8ea34
        let mut mc = std::mem::take(&mut self.mc_perm_prun);
        init_raw_sym_prun(
            &mut mc,
            BfsSource::RawConj {
                raw_move: &self.m_perm_move,
                raw_move_stride: N_MOVES2,
                raw_conj: &self.m_perm_conj,
                raw_conj_stride: 16,
                n_raw: N_MPERM,
            },
            &self.c_perm_move,
            N_MOVES2,
            N_PERM_SYM,
            &self.sym_state_perm,
            0x8ea34,
            full_init,
            &self.flip_r2s,
            &self.flip_s2rf,
            &self.sym_8_move,
            &self.flip_move,
            N_MOVES,
        );
        self.mc_perm_prun = mc;

        // initPermCombPPrun: CCombPMove / CCombPConj, EPermMove (phase2), SymStatePerm, flag 0x7d824
        let mut ec = std::mem::take(&mut self.e_perm_c_comb_p_prun);
        init_raw_sym_prun(
            &mut ec,
            BfsSource::RawConj {
                raw_move: &self.c_comb_p_move,
                raw_move_stride: N_MOVES2,
                raw_conj: &self.c_comb_p_conj,
                raw_conj_stride: 16,
                n_raw: N_COMB,
            },
            &self.e_perm_move,
            N_MOVES2,
            N_PERM_SYM,
            &self.sym_state_perm,
            0x7d824,
            full_init,
            &self.flip_r2s,
            &self.flip_s2rf,
            &self.sym_8_move,
            &self.flip_move,
            N_MOVES,
        );
        self.e_perm_c_comb_p_prun = ec;

        // initSliceTwistPrun: UDSliceMove / UDSliceConj, TwistMove (phase1), SymStateTwist, 0x69603
        let mut st = std::mem::take(&mut self.ud_slice_twist_prun);
        init_raw_sym_prun(
            &mut st,
            BfsSource::RawConj {
                raw_move: &self.ud_slice_move,
                raw_move_stride: N_MOVES,
                raw_conj: &self.ud_slice_conj,
                raw_conj_stride: 8,
                n_raw: N_SLICE,
            },
            &self.twist_move,
            N_MOVES,
            N_TWIST_SYM,
            &self.sym_state_twist,
            0x69603,
            full_init,
            &self.flip_r2s,
            &self.flip_s2rf,
            &self.sym_8_move,
            &self.flip_move,
            N_MOVES,
        );
        self.ud_slice_twist_prun = st;

        // initSliceFlipPrun: UDSliceMove / UDSliceConj, FlipMove (phase1), SymStateFlip, 0x69603
        let mut sf = std::mem::take(&mut self.ud_slice_flip_prun);
        init_raw_sym_prun(
            &mut sf,
            BfsSource::RawConj {
                raw_move: &self.ud_slice_move,
                raw_move_stride: N_MOVES,
                raw_conj: &self.ud_slice_conj,
                raw_conj_stride: 8,
                n_raw: N_SLICE,
            },
            &self.flip_move,
            N_MOVES,
            N_FLIP_SYM,
            &self.sym_state_flip,
            0x69603,
            full_init,
            &self.flip_r2s,
            &self.flip_s2rf,
            &self.sym_8_move,
            &self.flip_move,
            N_MOVES,
        );
        self.ud_slice_flip_prun = sf;

        // initTwistFlipPrun: RawMove=null (ISTFP), TwistMove (phase1), SymStateTwist, 0x19603
        if USE_TWIST_FLIP_PRUN {
            let mut tfp = std::mem::take(&mut self.twist_flip_prun);
            init_raw_sym_prun(
                &mut tfp,
                BfsSource::Tfp,
                &self.twist_move,
                N_MOVES,
                N_TWIST_SYM,
                &self.sym_state_twist,
                0x19603,
                full_init,
                &self.flip_r2s,
                &self.flip_s2rf,
                &self.sym_8_move,
                &self.flip_move,
                N_MOVES,
            );
            self.twist_flip_prun = tfp;
        }
    }
}

// ===== CoordCube — per-search state =====

/// from Java: CoordCube (instance fields + setWithPrun/doMovePrun/doMovePrunConj/calcPruning)
#[derive(Debug, Clone, Copy, Default)]
pub struct CoordCube {
    pub twist: i32,
    pub tsym: i32,
    pub flip: i32,
    pub fsym: i32,
    pub slice: i32,
    pub prun: i32,
    pub twistc: i32,
    pub flipc: i32,
}

impl CoordCube {
    pub fn new() -> Self {
        Self::default()
    }

    /// from Java: set(CoordCube node)
    pub fn set(&mut self, node: &CoordCube) {
        self.twist = node.twist;
        self.tsym = node.tsym;
        self.flip = node.flip;
        self.fsym = node.fsym;
        self.slice = node.slice;
        self.prun = node.prun;
        if USE_CONJ_PRUN {
            self.twistc = node.twistc;
            self.flipc = node.flipc;
        }
    }

    /// from Java: calcPruning(boolean isPhase1)
    #[inline]
    pub fn calc_pruning(&mut self, tables: &Tables, _is_phase1: bool) {
        let a = get_pruning(
            &tables.ud_slice_twist_prun,
            (self.twist as usize) * N_SLICE
                + tables.ud_slice_conj[(self.slice as usize) * 8 + self.tsym as usize] as usize,
        ) as i32;
        let b = get_pruning(
            &tables.ud_slice_flip_prun,
            (self.flip as usize) * N_SLICE
                + tables.ud_slice_conj[(self.slice as usize) * 8 + self.fsym as usize] as usize,
        ) as i32;
        let tfp = &tables.twist_flip_prun;
        let s2rf = &tables.flip_s2rf;
        let c = if USE_CONJ_PRUN {
            let idx = ((self.twistc >> 3) as usize) << 11
                | s2rf[(self.flipc ^ (self.twistc & 7)) as usize] as usize;
            get_pruning(tfp, idx) as i32
        } else {
            0
        };
        let d = if USE_TWIST_FLIP_PRUN {
            let idx = (self.twist as usize) << 11
                | s2rf[((self.flip << 3) | (self.fsym ^ self.tsym)) as usize] as usize;
            get_pruning(tfp, idx) as i32
        } else {
            0
        };
        self.prun = a.max(b).max(c.max(d));
    }

    /// from Java: setWithPrun(CubieCube cc, int depth) -> bool
    #[inline]
    pub fn set_with_prun(&mut self, tables: &Tables, cc: &CubieCube, depth: i32) -> bool {
        let twist_full = cc.get_twist_sym(tables);
        let flip_full = cc.get_flip_sym(tables);
        self.tsym = twist_full & 7;
        self.twist = twist_full >> 3;

        let s2rf = &tables.flip_s2rf;
        let tfp = &tables.twist_flip_prun;
        self.prun = if USE_TWIST_FLIP_PRUN {
            let idx = (self.twist as usize) << 11
                | s2rf[(flip_full ^ self.tsym) as usize] as usize;
            get_pruning(tfp, idx) as i32
        } else {
            0
        };
        if self.prun > depth {
            return false;
        }

        self.fsym = flip_full & 7;
        self.flip = flip_full >> 3;

        self.slice = cc.get_ud_slice(&tables.cnk);
        let a = get_pruning(
            &tables.ud_slice_twist_prun,
            (self.twist as usize) * N_SLICE
                + tables.ud_slice_conj[(self.slice as usize) * 8 + self.tsym as usize] as usize,
        ) as i32;
        let b = get_pruning(
            &tables.ud_slice_flip_prun,
            (self.flip as usize) * N_SLICE
                + tables.ud_slice_conj[(self.slice as usize) * 8 + self.fsym as usize] as usize,
        ) as i32;
        self.prun = self.prun.max(a).max(b);
        if self.prun > depth {
            return false;
        }

        if USE_CONJ_PRUN {
            let mut pc = CubieCube::empty();
            CubieCube::corn_conjugate(cc, 1, &mut pc, tables);
            CubieCube::edge_conjugate(cc, 1, &mut pc, tables);
            let tc = pc.get_twist_sym(tables);
            let fc = pc.get_flip_sym(tables);
            self.twistc = tc;
            self.flipc = fc;
            let idx = ((self.twistc >> 3) as usize) << 11
                | s2rf[(self.flipc ^ (self.twistc & 7)) as usize] as usize;
            self.prun = self.prun.max(get_pruning(tfp, idx) as i32);
        }

        self.prun <= depth
    }

    /// from Java: doMovePrun(CoordCube cc, int m, boolean isPhase1) -> int
    #[inline]
    pub fn do_move_prun(
        &mut self,
        tables: &Tables,
        cc: &CoordCube,
        m: usize,
        _is_phase1: bool,
    ) -> i32 {
        self.slice = tables.ud_slice_move[(cc.slice as usize) * N_MOVES + m] as i32;

        let s8f = tables.sym_8_move[(m << 3) | (cc.fsym as usize)] as usize;
        let fmove = tables.flip_move[(cc.flip as usize) * N_MOVES + s8f] as i32;
        self.fsym = (fmove & 7) ^ cc.fsym;
        self.flip = fmove >> 3;

        let s8t = tables.sym_8_move[(m << 3) | (cc.tsym as usize)] as usize;
        let tmove = tables.twist_move[(cc.twist as usize) * N_MOVES + s8t] as i32;
        self.tsym = (tmove & 7) ^ cc.tsym;
        self.twist = tmove >> 3;

        let a = get_pruning(
            &tables.ud_slice_twist_prun,
            (self.twist as usize) * N_SLICE
                + tables.ud_slice_conj[(self.slice as usize) * 8 + self.tsym as usize] as usize,
        ) as i32;
        let b = get_pruning(
            &tables.ud_slice_flip_prun,
            (self.flip as usize) * N_SLICE
                + tables.ud_slice_conj[(self.slice as usize) * 8 + self.fsym as usize] as usize,
        ) as i32;
        let c = if USE_TWIST_FLIP_PRUN {
            let s2rf = &tables.flip_s2rf;
            let idx = (self.twist as usize) << 11
                | s2rf[((self.flip << 3) | (self.fsym ^ self.tsym)) as usize] as usize;
            get_pruning(&tables.twist_flip_prun, idx) as i32
        } else {
            0
        };
        self.prun = a.max(b).max(c);
        self.prun
    }

    /// from Java: doMovePrunConj(CoordCube cc, int m) -> int
    #[inline]
    pub fn do_move_prun_conj(&mut self, tables: &Tables, cc: &CoordCube, m: usize) -> i32 {
        let m_conj = tables.sym_move[3][m] as usize;

        let s8f = tables.sym_8_move[(m_conj << 3) | ((cc.flipc & 7) as usize)] as usize;
        let fmove = tables.flip_move[((cc.flipc >> 3) as usize) * N_MOVES + s8f] as i32;
        self.flipc = fmove ^ (cc.flipc & 7);

        let s8t = tables.sym_8_move[(m_conj << 3) | ((cc.twistc & 7) as usize)] as usize;
        let tmove = tables.twist_move[((cc.twistc >> 3) as usize) * N_MOVES + s8t] as i32;
        self.twistc = tmove ^ (cc.twistc & 7);

        let s2rf = &tables.flip_s2rf;
        let idx = ((self.twistc >> 3) as usize) << 11
            | s2rf[(self.flipc ^ (self.twistc & 7)) as usize] as usize;
        get_pruning(&tables.twist_flip_prun, idx) as i32
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cubie::CubieCube;
    use std::time::Instant;

    fn solved_tables() -> Tables {
        Tables::build(true)
    }

    #[test]
    fn pruning_pack_roundtrip() {
        let mut t = vec![0u32; 4];
        // Set entries 0..32 with varying nibble values (low 4 bits of index).
        for i in 0..32usize {
            set_pruning(&mut t, i, (i as u32) & 0xf);
        }
        for i in 0..32usize {
            assert_eq!(get_pruning(&t, i), (i as u32) & 0xf, "idx {}", i);
        }
    }

    #[test]
    fn has_zero_smoke() {
        // 0x11111111: no zero nibble -> false
        assert!(!has_zero(0x11111111));
        // 0x11011111: has a zero nibble -> true
        assert!(has_zero(0x11011111));
        assert!(has_zero(0));
    }

    #[test]
    fn tables_build_full() {
        let start = Instant::now();
        let t = Tables::build(true);
        let elapsed = start.elapsed();
        eprintln!("Tables::build(true) took {:?}", elapsed);

        // Sanity: move/conj tables populated (flat row-major, length = rows * stride).
        assert_eq!(t.ud_slice_move.len(), N_SLICE * N_MOVES);
        assert_eq!(t.twist_move.len(), N_TWIST_SYM * N_MOVES);
        assert_eq!(t.flip_move.len(), N_FLIP_SYM * N_MOVES);
        assert_eq!(t.c_perm_move.len(), N_PERM_SYM * N_MOVES2);
        assert_eq!(t.e_perm_move.len(), N_PERM_SYM * N_MOVES2);

        // Pruning tables filled — not still all 0x11.
        assert!(t.ud_slice_twist_prun[0] != 0x11111111);
        assert!(t.ud_slice_flip_prun[0] != 0x11111111);
        assert!(t.mc_perm_prun[0] != 0x11111111);
        assert!(t.e_perm_c_comb_p_prun[0] != 0x11111111);
        if USE_TWIST_FLIP_PRUN {
            assert!(t.twist_flip_prun[0] != 0x11111111);
        }
    }

    #[test]
    fn solved_cube_prun_is_zero() {
        let t = solved_tables();
        let mut cc = CoordCube::new();
        let solved = CubieCube::solved();
        let ok = cc.set_with_prun(&t, &solved, 10);
        assert!(ok, "set_with_prun returned false for solved cube");
        assert_eq!(cc.prun, 0, "solved cube prun should be 0, got {}", cc.prun);

        // All pruning tables should hold 0 at entry 0 (solved state).
        assert_eq!(get_pruning(&t.ud_slice_twist_prun, 0), 0);
        assert_eq!(get_pruning(&t.ud_slice_flip_prun, 0), 0);
        assert_eq!(get_pruning(&t.mc_perm_prun, 0), 0);
        assert_eq!(get_pruning(&t.e_perm_c_comb_p_prun, 0), 0);
        if USE_TWIST_FLIP_PRUN {
            assert_eq!(get_pruning(&t.twist_flip_prun, 0), 0);
        }
    }

    #[test]
    fn one_move_increases_prun() {
        let t = solved_tables();
        let mut cc = CoordCube::new();
        // Apply one R move to a solved cube: cube = solved * moveCube[3]
        let mut scrambled = CubieCube::empty();
        CubieCube::edge_mult(&CubieCube::solved(), &t.move_cube[3], &mut scrambled);
        CubieCube::corn_mult(&CubieCube::solved(), &t.move_cube[3], &mut scrambled);

        let ok = cc.set_with_prun(&t, &scrambled, 20);
        assert!(ok);
        assert!(cc.prun >= 1, "one R away from solved, prun = {}", cc.prun);
    }

    fn count_depth(table: &[u32], n_size: usize, max_d: u32) -> Vec<u32> {
        let mut counts = vec![0u32; (max_d as usize) + 2];
        for i in 0..n_size {
            let v = get_pruning(table, i);
            counts[v as usize] += 1;
        }
        counts
    }

    #[test]
    fn prun_depth_distribution_matches_java() {
        // pruningValue.txt cumulative counts. Note: depth-N count is cumulative reached by depth N.
        let t = Tables::build(true);

        // MCPerm: cumulative count of entries with prun <= N, for N=1..14, matches pruningValue.txt.
        let counts_mc = count_depth(&t.mc_perm_prun, N_MPERM * N_PERM_SYM, 14);
        let expected_mc = [4u32, 14, 58, 272, 1118, 3531, 8653, 17292, 29420, 41991, 54976, 62730, 66096, 66432];
        let mut cum = 1u32; // entry 0 (solved)
        for (i, &exp) in expected_mc.iter().enumerate() {
            cum += counts_mc[i + 1];
            assert_eq!(cum, exp, "MCPerm cumulative @ depth {}: got {}, want {}", i + 1, cum, exp);
        }

        // SliceTwist max 9, table size 160380.
        let counts_st = count_depth(&t.ud_slice_twist_prun, N_SLICE * N_TWIST_SYM, 9);
        let expected_st = [3u32, 17, 126, 1050, 8761, 51261, 136795, 160231, 160380];
        let mut cum = 1u32;
        for (i, &exp) in expected_st.iter().enumerate() {
            cum += counts_st[i + 1];
            assert_eq!(cum, exp, "SliceTwist cumulative @ depth {}: got {}, want {}", i + 1, cum, exp);
        }
    }

    #[test]
    fn do_move_prun_roundtrip_solved() {
        let t = solved_tables();
        let mut start = CoordCube::new();
        let solved = CubieCube::solved();
        assert!(start.set_with_prun(&t, &solved, 20));

        // Apply move R (m=3) then R' (m=5) — should return to a state with prun=0.
        let mut a = CoordCube::new();
        a.do_move_prun(&t, &start, 3, true);
        let mut b = CoordCube::new();
        b.do_move_prun(&t, &a, 5, true);
        assert_eq!(b.prun, 0, "R then R' should give prun=0, got {}", b.prun);
    }
}
