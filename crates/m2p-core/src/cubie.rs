// from Java: cs.min2phase.CubieCube

use crate::util;
use crate::{Tables, N_FLIP_SYM, USE_TWIST_FLIP_PRUN};

// from Java: CubieCube.urfMove[6][18]
pub const URF_MOVE: [[u8; 18]; 6] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
    [6, 7, 8, 0, 1, 2, 3, 4, 5, 15, 16, 17, 9, 10, 11, 12, 13, 14],
    [3, 4, 5, 6, 7, 8, 0, 1, 2, 12, 13, 14, 15, 16, 17, 9, 10, 11],
    [2, 1, 0, 5, 4, 3, 8, 7, 6, 11, 10, 9, 14, 13, 12, 17, 16, 15],
    [8, 7, 6, 2, 1, 0, 5, 4, 3, 17, 16, 15, 11, 10, 9, 14, 13, 12],
    [5, 4, 3, 8, 7, 6, 2, 1, 0, 14, 13, 12, 17, 16, 15, 11, 10, 9],
];

pub const SYM_E2C_MAGIC: u32 = 0x00DDDD00;

#[inline]
pub fn e_sym_2_c_sym(idx: i32) -> i32 {
    idx ^ ((SYM_E2C_MAGIC >> ((idx & 0xf) << 1)) & 3) as i32
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubieCube {
    pub ca: [u8; 8],
    pub ea: [u8; 12],
}

impl Default for CubieCube {
    fn default() -> Self {
        Self::solved()
    }
}

impl CubieCube {
    pub fn solved() -> Self {
        Self {
            ca: [0, 1, 2, 3, 4, 5, 6, 7],
            ea: [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22],
        }
    }

    pub fn empty() -> Self {
        Self {
            ca: [0; 8],
            ea: [0; 12],
        }
    }

    /// from Java: new CubieCube(cperm, twist, eperm, flip)
    pub fn from_coords(cperm: i32, twist: i32, eperm: i32, flip: i32) -> Self {
        let mut c = Self::solved();
        c.set_cperm(cperm);
        c.set_twist(twist);
        util::set_n_perm(&mut c.ea, eperm, 12, true);
        c.set_flip(flip);
        c
    }

    pub fn copy_from(&mut self, c: &CubieCube) {
        self.ca = c.ca;
        self.ea = c.ea;
    }

    /// from Java: invCubieCube
    pub fn inv_cubie_cube(&mut self) {
        let mut temps = CubieCube::empty();
        for edge in 0..12usize {
            temps.ea[(self.ea[edge] >> 1) as usize] = ((edge as u8) << 1) | (self.ea[edge] & 1);
        }
        for corn in 0..8usize {
            let cur = self.ca[corn];
            // (corn | 0x20 >> (cur >> 3) & 0x18) — Java precedence: >> then & then |
            let shift = (cur >> 3) as u32;
            let bits = (0x20u32 >> shift) & 0x18;
            temps.ca[(cur & 0x7) as usize] = (corn as u8) | (bits as u8);
        }
        self.copy_from(&temps);
    }

    /// from Java: CornMult(a, b, prod)
    pub fn corn_mult(a: &CubieCube, b: &CubieCube, prod: &mut CubieCube) {
        for corn in 0..8usize {
            let b_idx = (b.ca[corn] & 7) as usize;
            let ori_a = a.ca[b_idx] >> 3;
            let ori_b = b.ca[corn] >> 3;
            prod.ca[corn] = (a.ca[b_idx] & 7) | (((ori_a + ori_b) % 3) << 3);
        }
    }

    /// from Java: CornMultFull (mirrored cases considered)
    pub fn corn_mult_full(a: &CubieCube, b: &CubieCube, prod: &mut CubieCube) {
        for corn in 0..8usize {
            let b_idx = (b.ca[corn] & 7) as usize;
            let ori_a = (a.ca[b_idx] >> 3) as i32;
            let ori_b = (b.ca[corn] >> 3) as i32;
            let mut ori = ori_a + if ori_a < 3 { ori_b } else { 6 - ori_b };
            ori = ori % 3 + if (ori_a < 3) == (ori_b < 3) { 0 } else { 3 };
            prod.ca[corn] = (a.ca[b_idx] & 7) | ((ori as u8) << 3);
        }
    }

    /// from Java: EdgeMult(a, b, prod)
    pub fn edge_mult(a: &CubieCube, b: &CubieCube, prod: &mut CubieCube) {
        for ed in 0..12usize {
            let b_idx = (b.ea[ed] >> 1) as usize;
            prod.ea[ed] = a.ea[b_idx] ^ (b.ea[ed] & 1);
        }
    }

    /// from Java: CornConjugate(a, idx, b) — uses CubeSym[SymMultInv[0][idx]] / CubeSym[idx]
    pub fn corn_conjugate(a: &CubieCube, idx: i32, b: &mut CubieCube, tables: &Tables) {
        let sinv = &tables.cube_sym[tables.sym_mult_inv[0][idx as usize] as usize];
        let s = &tables.cube_sym[idx as usize];
        for corn in 0..8usize {
            let s_idx = (s.ca[corn] & 7) as usize;
            let a_idx = (a.ca[s_idx] & 7) as usize;
            let ori_a = (sinv.ca[a_idx] >> 3) as i32;
            let ori_b = (a.ca[s_idx] >> 3) as i32;
            let ori = if ori_a < 3 { ori_b } else { (3 - ori_b) % 3 };
            b.ca[corn] = (sinv.ca[a_idx] & 7) | ((ori as u8) << 3);
        }
    }

    /// from Java: EdgeConjugate(a, idx, b)
    pub fn edge_conjugate(a: &CubieCube, idx: i32, b: &mut CubieCube, tables: &Tables) {
        let sinv = &tables.cube_sym[tables.sym_mult_inv[0][idx as usize] as usize];
        let s = &tables.cube_sym[idx as usize];
        for ed in 0..12usize {
            let s_idx = (s.ea[ed] >> 1) as usize;
            let a_idx = (a.ea[s_idx] >> 1) as usize;
            b.ea[ed] = sinv.ea[a_idx] ^ (a.ea[s_idx] & 1) ^ (s.ea[ed] & 1);
        }
    }

    /// from Java: URFConjugate
    pub fn urf_conjugate(&mut self, tables: &Tables) {
        let mut temps = CubieCube::empty();
        let mut c = self.clone();
        Self::corn_mult(&tables.urf2, &c, &mut temps);
        Self::corn_mult(&temps, &tables.urf1, &mut c);
        Self::edge_mult(&tables.urf2, &c, &mut temps);
        Self::edge_mult(&temps, &tables.urf1, &mut c);
        *self = c;
    }

    // ===== coordinate getters/setters =====

    /// from Java: getFlip
    pub fn get_flip(&self) -> i32 {
        let mut idx = 0i32;
        for i in 0..11usize {
            idx = (idx << 1) | (self.ea[i] & 1) as i32;
        }
        idx
    }

    /// from Java: setFlip
    pub fn set_flip(&mut self, idx: i32) {
        let mut parity = 0i32;
        let mut idx = idx;
        for i in (0..=10usize).rev() {
            let val = idx & 1;
            parity ^= val;
            self.ea[i] = (self.ea[i] & !1u8) | (val as u8);
            idx >>= 1;
        }
        self.ea[11] = (self.ea[11] & !1u8) | (parity as u8);
    }

    /// from Java: getTwist
    pub fn get_twist(&self) -> i32 {
        let mut idx = 0i32;
        for i in 0..7usize {
            idx += (idx << 1) + (self.ca[i] >> 3) as i32;
        }
        idx
    }

    /// from Java: setTwist
    pub fn set_twist(&mut self, idx: i32) {
        let mut twst = 15i32;
        let mut idx = idx;
        for i in (0..=6usize).rev() {
            let val = idx % 3;
            twst -= val;
            self.ca[i] = (self.ca[i] & 0x7) | ((val as u8) << 3);
            idx /= 3;
        }
        self.ca[7] = (self.ca[7] & 0x7) | (((twst % 3) as u8) << 3);
    }

    pub fn get_ud_slice(&self, cnk: &[[u32; 13]; 13]) -> i32 {
        494 - util::get_comb(&self.ea, 8, true, cnk)
    }

    pub fn set_ud_slice(&mut self, idx: i32, cnk: &[[u32; 13]; 13]) {
        util::set_comb(&mut self.ea, 494 - idx, 8, true, cnk);
    }

    pub fn get_cperm(&self) -> i32 {
        util::get_n_perm(&self.ca, 8, false)
    }

    pub fn set_cperm(&mut self, idx: i32) {
        util::set_n_perm(&mut self.ca, idx, 8, false);
    }

    pub fn get_eperm(&self) -> i32 {
        util::get_n_perm(&self.ea, 8, true)
    }

    pub fn set_eperm(&mut self, idx: i32) {
        util::set_n_perm(&mut self.ea, idx, 8, true);
    }

    pub fn get_mperm(&self) -> i32 {
        util::get_n_perm(&self.ea, 12, true) % 24
    }

    pub fn set_mperm(&mut self, idx: i32) {
        util::set_n_perm(&mut self.ea, idx, 12, true);
    }

    pub fn get_ccomb(&self, cnk: &[[u32; 13]; 13]) -> i32 {
        util::get_comb(&self.ca, 0, false, cnk)
    }

    pub fn set_ccomb(&mut self, idx: i32, cnk: &[[u32; 13]; 13]) {
        util::set_comb(&mut self.ca, idx, 0, false, cnk);
    }

    // Sym-coord getters need lookup tables populated.
    pub fn get_flip_sym(&self, tables: &Tables) -> i32 {
        tables.flip_r2s[self.get_flip() as usize] as i32
    }

    pub fn get_twist_sym(&self, tables: &Tables) -> i32 {
        tables.twist_r2s[self.get_twist() as usize] as i32
    }

    pub fn get_cperm_sym(&self, tables: &Tables) -> i32 {
        e_sym_2_c_sym(tables.e_perm_r2s[self.get_cperm() as usize] as i32)
    }

    pub fn get_eperm_sym(&self, tables: &Tables) -> i32 {
        tables.e_perm_r2s[self.get_eperm() as usize] as i32
    }

    /// from Java: verify
    pub fn verify(&self) -> i32 {
        let mut sum = 0i32;
        let mut edge_mask = 0i32;
        for e in 0..12usize {
            edge_mask |= 1 << (self.ea[e] >> 1);
            sum ^= (self.ea[e] & 1) as i32;
        }
        if edge_mask != 0xfff {
            return -2;
        }
        if sum != 0 {
            return -3;
        }
        let mut corn_mask = 0i32;
        sum = 0;
        for c in 0..8usize {
            corn_mask |= 1 << (self.ca[c] & 7);
            sum += (self.ca[c] >> 3) as i32;
        }
        if corn_mask != 0xff {
            return -4;
        }
        if sum % 3 != 0 {
            return -5;
        }
        if util::get_n_parity(util::get_n_perm(&self.ea, 12, true), 12)
            ^ util::get_n_parity(self.get_cperm(), 8)
            != 0
        {
            return -6;
        }
        0
    }

    /// from Java: selfSymmetry — uses CornConjugate / EdgeConjugate / URFConjugate / invCubieCube.
    pub fn self_symmetry(&self, tables: &Tables) -> u64 {
        let mut c = self.clone();
        let mut d = CubieCube::empty();
        let cperm = c.get_cperm_sym(tables) >> 4;
        let mut sym: u64 = 0;
        for urf_inv in 0..6 {
            let cpermx = c.get_cperm_sym(tables) >> 4;
            if cperm == cpermx {
                for i in 0..16 {
                    let inv = tables.sym_mult_inv[0][i] as i32;
                    Self::corn_conjugate(&c, inv, &mut d, tables);
                    if d.ca == self.ca {
                        Self::edge_conjugate(&c, inv, &mut d, tables);
                        if d.ea == self.ea {
                            let shift = ((urf_inv << 4) | i).min(48);
                            sym |= 1u64 << shift;
                        }
                    }
                }
            }
            c.urf_conjugate(tables);
            if urf_inv % 3 == 2 {
                c.inv_cubie_cube();
            }
        }
        sym
    }
}

impl std::fmt::Display for CubieCube {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in 0..8 {
            write!(f, "|{} {}", self.ca[i] & 7, self.ca[i] >> 3)?;
        }
        writeln!(f)?;
        for i in 0..12 {
            write!(f, "|{} {}", self.ea[i] >> 1, self.ea[i] & 1)?;
        }
        Ok(())
    }
}

// ===== Table init =====

/// from Java: getPermSymInv
pub fn get_perm_sym_inv(idx: i32, sym: i32, is_corner: bool, tables: &Tables) -> i32 {
    let mut idxi = tables.perm_inv_edge_sym[idx as usize] as i32;
    if is_corner {
        idxi = e_sym_2_c_sym(idxi);
    }
    (idxi & 0xfff0) | tables.sym_mult[(idxi & 0xf) as usize][sym as usize] as i32
}

/// from Java: getSkipMoves
pub fn get_skip_moves(ssym: u64, tables: &Tables) -> i32 {
    let mut ret = 0i32;
    let mut s = ssym >> 1;
    let mut i = 1usize;
    while s != 0 {
        if s & 1 == 1 {
            ret |= tables.first_move_sym[i];
        }
        s >>= 1;
        i += 1;
    }
    ret
}

/// from Java: initMove — populate moveCube[0..18]
pub fn init_move() -> Vec<CubieCube> {
    let mut mc: Vec<CubieCube> = (0..18).map(|_| CubieCube::solved()).collect();
    mc[0] = CubieCube::from_coords(15120, 0, 119750400, 0);
    mc[3] = CubieCube::from_coords(21021, 1494, 323403417, 0);
    mc[6] = CubieCube::from_coords(8064, 1236, 29441808, 550);
    mc[9] = CubieCube::from_coords(9, 0, 5880, 0);
    mc[12] = CubieCube::from_coords(1230, 412, 2949660, 0);
    mc[15] = CubieCube::from_coords(224, 137, 328552, 137);
    for a in (0..18).step_by(3) {
        for p in 0..2 {
            // moveCube[a+p+1] = moveCube[a+p] * moveCube[a]
            // Java does EdgeMult then CornMult on the same target which overwrites ca? No — EdgeMult writes ea only, CornMult writes ca only.
            let mut prod = CubieCube::empty();
            let base = mc[a + p].clone();
            let mover = mc[a].clone();
            CubieCube::edge_mult(&base, &mover, &mut prod);
            CubieCube::corn_mult(&base, &mover, &mut prod);
            mc[a + p + 1] = prod;
        }
    }
    mc
}

/// from Java: initSym — populates cube_sym, sym_mult, sym_mult_inv, sym_move, sym_8_move,
/// sym_move_ud, move_cube_sym, first_move_sym.
#[allow(clippy::too_many_arguments)]
pub fn init_sym(
    move_cube: &[CubieCube],
    std2ud: &[u8; 18],
) -> (
    Vec<CubieCube>,         // cube_sym (16)
    [[u8; 16]; 16],         // sym_mult
    [[u8; 16]; 16],         // sym_mult_inv
    [[u8; 18]; 16],         // sym_move
    [u8; 8 * 18],           // sym_8_move
    [[u8; 18]; 16],         // sym_move_ud
) {
    let mut cube_sym: Vec<CubieCube> = (0..16).map(|_| CubieCube::empty()).collect();
    let mut sym_mult = [[0u8; 16]; 16];
    let mut sym_mult_inv = [[0u8; 16]; 16];
    let mut sym_move = [[0u8; 18]; 16];
    let mut sym_8_move = [0u8; 8 * 18];
    let mut sym_move_ud = [[0u8; 18]; 16];

    let mut c = CubieCube::solved();
    let mut d = CubieCube::empty();

    let f2 = CubieCube::from_coords(28783, 0, 259268407, 0);
    let u4 = CubieCube::from_coords(15138, 0, 119765538, 7);
    let mut lr2 = CubieCube::from_coords(5167, 0, 83473207, 0);
    for i in 0..8 {
        lr2.ca[i] |= 3 << 3;
    }

    for i in 0..16 {
        cube_sym[i] = c.clone();
        CubieCube::corn_mult_full(&c, &u4, &mut d);
        CubieCube::edge_mult(&c, &u4, &mut d);
        std::mem::swap(&mut c, &mut d);
        if i % 4 == 3 {
            CubieCube::corn_mult_full(&c, &lr2, &mut d);
            CubieCube::edge_mult(&c, &lr2, &mut d);
            std::mem::swap(&mut c, &mut d);
        }
        if i % 8 == 7 {
            CubieCube::corn_mult_full(&c, &f2, &mut d);
            CubieCube::edge_mult(&c, &f2, &mut d);
            std::mem::swap(&mut c, &mut d);
        }
    }

    // sym_mult / sym_mult_inv
    for i in 0..16 {
        for j in 0..16 {
            CubieCube::corn_mult_full(&cube_sym[i], &cube_sym[j], &mut c);
            for k in 0..16 {
                if cube_sym[k].ca == c.ca {
                    sym_mult[i][j] = k as u8;
                    sym_mult_inv[k][j] = i as u8;
                    break;
                }
            }
        }
    }

    // sym_move / sym_move_ud / sym_8_move. Mirrors Java exactly:
    //   CornConjugate(moveCube[j], SymMultInv[0][s], c)
    // Inside CornConjugate(a, idx, b): sinv = CubeSym[SymMultInv[0][idx]], s = CubeSym[idx].
    // So with idx = SymMultInv[0][s], sinv = CubeSym[(SymMultInv[0][s])^-1] = CubeSym[s] and
    // s_in = CubeSym[SymMultInv[0][s]].
    let mut tmp = CubieCube::empty();
    for j in 0..18usize {
        for s in 0..16usize {
            let idx = sym_mult_inv[0][s] as usize;
            let sinv = &cube_sym[sym_mult_inv[0][idx] as usize];
            let sx = &cube_sym[idx];
            for corn in 0..8usize {
                let s_idx = (sx.ca[corn] & 7) as usize;
                let a_idx = (move_cube[j].ca[s_idx] & 7) as usize;
                let ori_a = (sinv.ca[a_idx] >> 3) as i32;
                let ori_b = (move_cube[j].ca[s_idx] >> 3) as i32;
                let ori = if ori_a < 3 { ori_b } else { (3 - ori_b) % 3 };
                tmp.ca[corn] = (sinv.ca[a_idx] & 7) | ((ori as u8) << 3);
            }
            for m in 0..18usize {
                if move_cube[m].ca == tmp.ca {
                    sym_move[s][j] = m as u8;
                    sym_move_ud[s][std2ud[j] as usize] = std2ud[m];
                    break;
                }
            }
            if s % 2 == 0 {
                sym_8_move[(j << 3) | (s >> 1)] = sym_move[s][j];
            }
        }
    }

    (cube_sym, sym_mult, sym_mult_inv, sym_move, sym_8_move, sym_move_ud)
}

/// from Java: tail of initSym — moveCubeSym + firstMoveSym (needs corn_conjugate via Tables)
pub fn init_move_cube_sym(
    move_cube: &[CubieCube],
    tables: &Tables,
) -> ([u64; 18], [i32; 48]) {
    let mut move_cube_sym = [0u64; 18];
    let mut first_move_sym = [0i32; 48];
    for i in 0..18usize {
        move_cube_sym[i] = move_cube[i].self_symmetry(tables);
        let mut j = i;
        for s in 0..48usize {
            if (tables.sym_move[s % 16][j] as usize) < i {
                first_move_sym[s] |= 1 << i;
            }
            if s % 16 == 15 {
                j = URF_MOVE[2][j] as usize;
            }
        }
    }
    (move_cube_sym, first_move_sym)
}

/// from Java: initSym2Raw — coord: 0=flip, 1=twist, 2=eperm
/// Returns (count, sym2raw, raw2sym, sym_state, opt s2rf for flip+USE_TWIST_FLIP_PRUN)
pub fn init_sym_2_raw(
    n_raw: usize,
    n_sym: usize,
    coord: i32,
    tables: &Tables,
) -> (i32, Vec<u16>, Vec<u16>, Vec<u16>, Option<Vec<u16>>) {
    let mut sym2raw = vec![0u16; n_sym];
    let mut raw2sym = vec![0u16; n_raw];
    let mut sym_state = vec![0u16; n_sym];
    let mut s2rf: Option<Vec<u16>> = if coord == 0 && USE_TWIST_FLIP_PRUN {
        Some(vec![0u16; N_FLIP_SYM * 8])
    } else {
        None
    };

    // Java's `new CubieCube()` is solved-state; set_flip/set_twist/set_eperm only patch
    // their own bits, so starting from empty (all-zero) leaves invalid position bytes that
    // later corn/edge_conjugate would propagate. Mirror Java by starting solved.
    let mut c = CubieCube::solved();
    let mut d = CubieCube::empty();
    let mut count = 0i32;
    let sym_inc: usize = if coord >= 2 { 1 } else { 2 };
    let is_edge = coord != 1;

    for i in 0..n_raw {
        if raw2sym[i] != 0 {
            continue;
        }
        match coord {
            0 => c.set_flip(i as i32),
            1 => c.set_twist(i as i32),
            2 => c.set_eperm(i as i32),
            _ => unreachable!(),
        }
        let mut s = 0usize;
        while s < 16 {
            if is_edge {
                CubieCube::edge_conjugate(&c, s as i32, &mut d, tables);
            } else {
                CubieCube::corn_conjugate(&c, s as i32, &mut d, tables);
            }
            let idx = match coord {
                0 => d.get_flip(),
                1 => d.get_twist(),
                2 => d.get_eperm(),
                _ => unreachable!(),
            };
            if coord == 0 && USE_TWIST_FLIP_PRUN {
                if let Some(buf) = s2rf.as_mut() {
                    buf[((count as usize) << 3) | (s >> 1)] = idx as u16;
                }
            }
            if idx as usize == i {
                sym_state[count as usize] |= 1u16 << (s / sym_inc);
            }
            let sym_idx = ((count as usize) << 4 | s) / sym_inc;
            raw2sym[idx as usize] = sym_idx as u16;
            s += sym_inc;
        }
        sym2raw[count as usize] = i as u16;
        count += 1;
    }
    (count, sym2raw, raw2sym, sym_state, s2rf)
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util;
    use crate::{Tables, N_FLIP_SYM, N_PERM_SYM, N_TWIST_SYM};

    #[test]
    fn tables_new_runs() {
        let t = Tables::new();
        assert_eq!(t.cube_sym.len(), 16);
        assert_eq!(t.move_cube.len(), 18);
    }

    #[test]
    fn solved_cube_verifies() {
        let c = CubieCube::solved();
        assert_eq!(c.verify(), 0);
    }

    #[test]
    fn u_squared_equals_u_times_u_corners() {
        let t = Tables::new();
        // moveCube indices: U=0, U2=1
        let mut prod = CubieCube::empty();
        CubieCube::corn_mult(&t.move_cube[0], &t.move_cube[0], &mut prod);
        assert_eq!(prod.ca, t.move_cube[1].ca);
        CubieCube::edge_mult(&t.move_cube[0], &t.move_cube[0], &mut prod);
        assert_eq!(prod.ea, t.move_cube[1].ea);
    }

    #[test]
    fn r3_equals_r_times_r2() {
        let t = Tables::new();
        // R=3, R2=4, R'=5; R' = R * R2
        let mut prod = CubieCube::empty();
        CubieCube::corn_mult(&t.move_cube[3], &t.move_cube[4], &mut prod);
        assert_eq!(prod.ca, t.move_cube[5].ca);
        CubieCube::edge_mult(&t.move_cube[3], &t.move_cube[4], &mut prod);
        assert_eq!(prod.ea, t.move_cube[5].ea);
    }

    #[test]
    fn sym_mult_identity() {
        let t = Tables::new();
        // CubeSym[0] is identity; SymMult[0][j] == j and SymMult[i][0] == i
        for j in 0..16 {
            assert_eq!(t.sym_mult[0][j], j as u8);
            assert_eq!(t.sym_mult[j][0], j as u8);
        }
    }

    #[test]
    fn sym_mult_inv_consistent() {
        let t = Tables::new();
        // SymMultInv[k][j] = i where i*j=k, so SymMult[SymMultInv[k][j]][j] == k
        for k in 0..16 {
            for j in 0..16 {
                let i = t.sym_mult_inv[k][j];
                assert_eq!(t.sym_mult[i as usize][j], k as u8);
            }
        }
    }

    #[test]
    fn flip_sym_lookup_count() {
        let t = Tables::new();
        assert_eq!(t.flip_s2r.len(), N_FLIP_SYM);
        assert_eq!(t.twist_s2r.len(), N_TWIST_SYM);
        assert_eq!(t.e_perm_s2r.len(), N_PERM_SYM);
    }

    #[test]
    fn cperm_roundtrip() {
        let mut c = CubieCube::solved();
        for idx in [0, 1, 100, 12345, 40319] {
            c.set_cperm(idx);
            assert_eq!(c.get_cperm(), idx);
        }
    }

    #[test]
    fn flip_roundtrip() {
        let mut c = CubieCube::solved();
        for idx in [0, 1, 100, 1000, 2047] {
            c.set_flip(idx);
            assert_eq!(c.get_flip(), idx);
        }
    }

    #[test]
    fn twist_roundtrip() {
        let mut c = CubieCube::solved();
        for idx in [0, 1, 100, 1000, 2186] {
            c.set_twist(idx);
            assert_eq!(c.get_twist(), idx);
        }
    }

    #[test]
    fn sym2raw_class_counts() {
        let t = Tables::new();
        // Class 0 represents raw 0 (identity flip/twist/perm).
        assert_eq!(t.flip_s2r[0], 0);
        assert_eq!(t.twist_s2r[0], 0);
        assert_eq!(t.e_perm_s2r[0], 0);
        // Sym-state bitmask for class 0 should mark all 8 (or 16) self-stabilizers.
        // Identity is fixed by every symmetry → all bits set within sym_inc grouping.
        // Flip uses sym_inc=2 so 8 bits; mask = 0xff.
        assert_eq!(t.sym_state_flip[0], 0xff);
        assert_eq!(t.sym_state_twist[0], 0xff);
        // Perm uses sym_inc=1 → 16 bits.
        assert_eq!(t.sym_state_perm[0], 0xffff);
        // Inverse of identity perm should have the same raw value (0). Sym index will be
        // the *last* sym writing raw=0 (in Java + here, that's sym_idx 15 for perm).
        // Validate via raw: e_perm_s2r[perm_inv_edge_sym[0] >> 4] == 0.
        let inv_class = (t.perm_inv_edge_sym[0] >> 4) as usize;
        assert_eq!(t.e_perm_s2r[inv_class], 0);
        // m_perm_inv[0]: inverse of identity slice perm is also identity → 0.
        assert_eq!(t.m_perm_inv[0], 0);
    }

    #[test]
    fn move_cube_sym_filled() {
        let t = Tables::new();
        // Identity-element selfSymmetry should yield non-zero (matches solved cube on multiple syms).
        // Just sanity-check the array is populated (Java values are non-trivial bitmasks).
        let any_nonzero = t.move_cube_sym.iter().any(|&v| v != 0);
        assert!(any_nonzero);
    }

    #[test]
    fn cnk_pascal() {
        let cnk = util::init_cnk();
        assert_eq!(cnk[12][4], 495);
        assert_eq!(cnk[8][4], 70);
        assert_eq!(cnk[5][2], 10);
    }
}
