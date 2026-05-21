// from Java: cs.min2phase.Util

use crate::cubie::CubieCube;

// Moves
pub const UX1: u8 = 0;
pub const UX2: u8 = 1;
pub const UX3: u8 = 2;
pub const RX1: u8 = 3;
pub const RX2: u8 = 4;
pub const RX3: u8 = 5;
pub const FX1: u8 = 6;
pub const FX2: u8 = 7;
pub const FX3: u8 = 8;
pub const DX1: u8 = 9;
pub const DX2: u8 = 10;
pub const DX3: u8 = 11;
pub const LX1: u8 = 12;
pub const LX2: u8 = 13;
pub const LX3: u8 = 14;
pub const BX1: u8 = 15;
pub const BX2: u8 = 16;
pub const BX3: u8 = 17;

// Facelet indices 0..54
// Layout: U1..U9, R1..R9, F1..F9, D1..D9, L1..L9, B1..B9
pub const U1: u8 = 0;
pub const U2: u8 = 1;
pub const U3: u8 = 2;
pub const U4: u8 = 3;
pub const U5: u8 = 4;
pub const U6: u8 = 5;
pub const U7: u8 = 6;
pub const U8: u8 = 7;
pub const U9: u8 = 8;
pub const R1: u8 = 9;
pub const R2: u8 = 10;
pub const R3: u8 = 11;
pub const R4: u8 = 12;
pub const R5: u8 = 13;
pub const R6: u8 = 14;
pub const R7: u8 = 15;
pub const R8: u8 = 16;
pub const R9: u8 = 17;
pub const F1: u8 = 18;
pub const F2: u8 = 19;
pub const F3: u8 = 20;
pub const F4: u8 = 21;
pub const F5: u8 = 22;
pub const F6: u8 = 23;
pub const F7: u8 = 24;
pub const F8: u8 = 25;
pub const F9: u8 = 26;
pub const D1: u8 = 27;
pub const D2: u8 = 28;
pub const D3: u8 = 29;
pub const D4: u8 = 30;
pub const D5: u8 = 31;
pub const D6: u8 = 32;
pub const D7: u8 = 33;
pub const D8: u8 = 34;
pub const D9: u8 = 35;
pub const L1: u8 = 36;
pub const L2: u8 = 37;
pub const L3: u8 = 38;
pub const L4: u8 = 39;
pub const L5: u8 = 40;
pub const L6: u8 = 41;
pub const L7: u8 = 42;
pub const L8: u8 = 43;
pub const L9: u8 = 44;
pub const B1: u8 = 45;
pub const B2: u8 = 46;
pub const B3: u8 = 47;
pub const B4: u8 = 48;
pub const B5: u8 = 49;
pub const B6: u8 = 50;
pub const B7: u8 = 51;
pub const B8: u8 = 52;
pub const B9: u8 = 53;

// Colors
pub const U: u8 = 0;
pub const R: u8 = 1;
pub const F: u8 = 2;
pub const D: u8 = 3;
pub const L: u8 = 4;
pub const B: u8 = 5;

pub const CORNER_FACELET: [[u8; 3]; 8] = [
    [U9, R1, F3], [U7, F1, L3], [U1, L1, B3], [U3, B1, R3],
    [D3, F9, R7], [D1, L9, F7], [D7, B9, L7], [D9, R9, B7],
];

pub const EDGE_FACELET: [[u8; 2]; 12] = [
    [U6, R2], [U8, F2], [U4, L2], [U2, B2], [D6, R8], [D2, F8],
    [D4, L8], [D8, B8], [F6, R4], [F4, L6], [B6, L4], [B4, R6],
];

pub const MOVE2STR: [&str; 18] = [
    "U ", "U2", "U'", "R ", "R2", "R'", "F ", "F2", "F'",
    "D ", "D2", "D'", "L ", "L2", "L'", "B ", "B2", "B'",
];

pub const UD2STD: [u8; 18] = [
    UX1, UX2, UX3, RX2, FX2, DX1, DX2, DX3, LX2, BX2,
    RX1, RX3, FX1, FX3, LX1, LX3, BX1, BX3,
];

// Verbose flags (mirrors Search.* flags)
pub const USE_SEPARATOR: u32 = 0x1;
pub const INVERSE_SOLUTION: u32 = 0x2;
pub const APPEND_LENGTH: u32 = 0x4;
pub const OPTIMAL_SOLUTION: u32 = 0x8;

/// from Java: Util.Solution
#[derive(Debug, Clone)]
pub struct Solution {
    pub length: usize,
    pub depth1: i32,
    pub verbose: u32,
    pub urf_idx: i32,
    pub moves: [i32; 31],
}

impl Default for Solution {
    fn default() -> Self {
        Self::new()
    }
}

impl Solution {
    pub fn new() -> Self {
        Self {
            length: 0,
            depth1: 0,
            verbose: 0,
            urf_idx: 0,
            moves: [0; 31],
        }
    }

    pub fn set_args(&mut self, verbose: u32, urf_idx: i32, depth1: i32) {
        self.verbose = verbose;
        self.urf_idx = urf_idx;
        self.depth1 = depth1;
    }

    pub fn append_sol_move(&mut self, cur_move: i32) {
        if self.length == 0 {
            self.moves[self.length] = cur_move;
            self.length += 1;
            return;
        }
        let axis_cur = cur_move / 3;
        let axis_last = self.moves[self.length - 1] / 3;
        if axis_cur == axis_last {
            let pow = (cur_move % 3 + self.moves[self.length - 1] % 3 + 1) % 4;
            if pow == 3 {
                self.length -= 1;
            } else {
                self.moves[self.length - 1] = axis_cur * 3 + pow;
            }
            return;
        }
        if self.length > 1
            && axis_cur % 3 == axis_last % 3
            && axis_cur == self.moves[self.length - 2] / 3
        {
            let pow = (cur_move % 3 + self.moves[self.length - 2] % 3 + 1) % 4;
            if pow == 3 {
                self.moves[self.length - 2] = self.moves[self.length - 1];
                self.length -= 1;
            } else {
                self.moves[self.length - 2] = axis_cur * 3 + pow;
            }
            return;
        }
        self.moves[self.length] = cur_move;
        self.length += 1;
    }

    /// Render solution using urfMove from Tables.
    pub fn render(&self, urf_move: &[[u8; 18]; 6]) -> String {
        use std::fmt::Write;
        let mut sb = String::new();
        let urf = if self.verbose & INVERSE_SOLUTION != 0 {
            (self.urf_idx + 3) % 6
        } else {
            self.urf_idx
        };
        if urf < 3 {
            for s in 0..self.length {
                if self.verbose & USE_SEPARATOR != 0 && s as i32 == self.depth1 {
                    sb.push_str(".  ");
                }
                let mv = urf_move[urf as usize][self.moves[s] as usize] as usize;
                sb.push_str(MOVE2STR[mv]);
                sb.push(' ');
            }
        } else {
            for s in (0..self.length).rev() {
                let mv = urf_move[urf as usize][self.moves[s] as usize] as usize;
                sb.push_str(MOVE2STR[mv]);
                sb.push(' ');
                if self.verbose & USE_SEPARATOR != 0 && s as i32 == self.depth1 {
                    sb.push_str(".  ");
                }
            }
        }
        if self.verbose & APPEND_LENGTH != 0 {
            let _ = write!(sb, "({}f)", self.length);
        }
        sb
    }
}

/// from Java: Util.toCubieCube(byte[] f, CubieCube ccRet)
pub fn to_cubie_cube(f: &[u8], cc_ret: &mut CubieCube) {
    for i in 0..8 {
        cc_ret.ca[i] = 0;
    }
    for i in 0..12 {
        cc_ret.ea[i] = 0;
    }
    for i in 0u8..8 {
        let mut ori: u8 = 0;
        while ori < 3 {
            let fc = f[CORNER_FACELET[i as usize][ori as usize] as usize];
            if fc == U || fc == D {
                break;
            }
            ori += 1;
        }
        let col1 = f[CORNER_FACELET[i as usize][((ori + 1) % 3) as usize] as usize];
        let col2 = f[CORNER_FACELET[i as usize][((ori + 2) % 3) as usize] as usize];
        for j in 0u8..8 {
            if col1 == CORNER_FACELET[j as usize][1] / 9 && col2 == CORNER_FACELET[j as usize][2] / 9 {
                cc_ret.ca[i as usize] = ((ori % 3) << 3) | j;
                break;
            }
        }
    }
    for i in 0u8..12 {
        for j in 0u8..12 {
            let fi0 = f[EDGE_FACELET[i as usize][0] as usize];
            let fi1 = f[EDGE_FACELET[i as usize][1] as usize];
            let fj0 = EDGE_FACELET[j as usize][0] / 9;
            let fj1 = EDGE_FACELET[j as usize][1] / 9;
            if fi0 == fj0 && fi1 == fj1 {
                cc_ret.ea[i as usize] = j << 1;
                break;
            }
            if fi0 == fj1 && fi1 == fj0 {
                cc_ret.ea[i as usize] = (j << 1) | 1;
                break;
            }
        }
    }
}

/// from Java: Util.toFaceCube
pub fn to_face_cube(cc: &CubieCube) -> String {
    let ts = ['U', 'R', 'F', 'D', 'L', 'B'];
    let mut f = [0u8; 54];
    for i in 0..54 {
        f[i] = (i / 9) as u8;
    }
    for c in 0u8..8 {
        let j = (cc.ca[c as usize] & 0x7) as usize;
        let ori = (cc.ca[c as usize] >> 3) as usize;
        for n in 0u8..3 {
            let dst = CORNER_FACELET[c as usize][(n as usize + ori) % 3] as usize;
            f[dst] = CORNER_FACELET[j][n as usize] / 9;
        }
    }
    for e in 0u8..12 {
        let j = (cc.ea[e as usize] >> 1) as usize;
        let ori = (cc.ea[e as usize] & 1) as usize;
        for n in 0u8..2 {
            let dst = EDGE_FACELET[e as usize][(n as usize + ori) % 2] as usize;
            f[dst] = EDGE_FACELET[j][n as usize] / 9;
        }
    }
    f.iter().map(|&b| ts[b as usize]).collect()
}

/// from Java: Util.getNParity
pub fn get_n_parity(idx: i32, n: i32) -> i32 {
    let mut p = 0i32;
    let mut idx = idx;
    let mut i = n - 2;
    while i >= 0 {
        p ^= idx % (n - i);
        idx /= n - i;
        i -= 1;
    }
    p & 1
}

#[inline]
pub fn set_val(val0: u8, val: i32, is_edge: bool) -> u8 {
    if is_edge {
        ((val << 1) as u8) | (val0 & 1)
    } else {
        (val as u8) | (val0 & !7u8)
    }
}

#[inline]
pub fn get_val(val0: u8, is_edge: bool) -> i32 {
    if is_edge {
        (val0 >> 1) as i32
    } else {
        (val0 & 7) as i32
    }
}

/// from Java: Util.setNPerm
pub fn set_n_perm(arr: &mut [u8], idx: i32, n: i32, is_edge: bool) {
    let mut val: u64 = 0xFEDCBA9876543210u64;
    let mut extract: u64 = 0;
    let mut idx = idx;
    let mut p = 2i32;
    while p <= n {
        extract = (extract << 4) | (idx % p) as u64;
        idx /= p;
        p += 1;
    }
    for i in 0..(n - 1) as usize {
        let v = ((extract as u32 & 0xf) << 2) as u32;
        extract >>= 4;
        let new_val = ((val >> v) & 0xf) as i32;
        arr[i] = set_val(arr[i], new_val, is_edge);
        let m = (1u64 << v).wrapping_sub(1);
        val = (val & m) | ((val >> 4) & !m);
    }
    let last = (n - 1) as usize;
    arr[last] = set_val(arr[last], (val & 0xf) as i32, is_edge);
}

/// from Java: Util.getNPerm
pub fn get_n_perm(arr: &[u8], n: i32, is_edge: bool) -> i32 {
    let mut idx = 0i32;
    let mut val: u64 = 0xFEDCBA9876543210u64;
    for i in 0..(n - 1) as usize {
        let v = (get_val(arr[i], is_edge) as u32) << 2;
        idx = (n - i as i32) * idx + ((val >> v) & 0xf) as i32;
        val = val.wrapping_sub(0x1111111111111110u64 << v);
    }
    idx
}

/// from Java: Util.getComb
pub fn get_comb(arr: &[u8], mask: i32, is_edge: bool, cnk: &[[u32; 13]; 13]) -> i32 {
    let end = arr.len() as i32 - 1;
    let mut idx_c = 0i32;
    let mut r = 4i32;
    let mut i = end;
    while i >= 0 {
        let perm = get_val(arr[i as usize], is_edge);
        if (perm & 0xc) == mask {
            idx_c += cnk[i as usize][r as usize] as i32;
            r -= 1;
        }
        i -= 1;
    }
    idx_c
}

/// from Java: Util.setComb
pub fn set_comb(arr: &mut [u8], idx_c: i32, mask: i32, is_edge: bool, cnk: &[[u32; 13]; 13]) {
    let end = arr.len() as i32 - 1;
    let mut r = 4i32;
    let mut fill = end;
    let mut idx_c = idx_c;
    let mut i = end;
    while i >= 0 {
        if idx_c >= cnk[i as usize][r as usize] as i32 {
            idx_c -= cnk[i as usize][r as usize] as i32;
            r -= 1;
            arr[i as usize] = set_val(arr[i as usize], r | mask, is_edge);
        } else {
            if (fill & 0xc) == mask {
                fill -= 4;
            }
            arr[i as usize] = set_val(arr[i as usize], fill, is_edge);
            fill -= 1;
        }
        i -= 1;
    }
}

/// from Java: static block — initialize Cnk, std2ud, ckmv2bit.
pub fn init_cnk() -> [[u32; 13]; 13] {
    let mut cnk = [[0u32; 13]; 13];
    for i in 0..13 {
        cnk[i][0] = 1;
        cnk[i][i] = 1;
        for j in 1..i {
            cnk[i][j] = cnk[i - 1][j - 1] + cnk[i - 1][j];
        }
    }
    cnk
}

pub fn init_std2ud() -> [u8; 18] {
    let mut std2ud = [0u8; 18];
    for i in 0..18 {
        std2ud[UD2STD[i] as usize] = i as u8;
    }
    std2ud
}

pub fn init_ckmv2bit() -> [i32; 11] {
    let mut ckmv2bit = [0i32; 11];
    for i in 0..10 {
        let ix = (UD2STD[i] / 3) as i32;
        ckmv2bit[i] = 0;
        for j in 0..10 {
            let jx = (UD2STD[j] / 3) as i32;
            let bit = if ix == jx || (ix % 3 == jx % 3 && ix >= jx) { 1 } else { 0 };
            ckmv2bit[i] |= bit << j;
        }
    }
    ckmv2bit[10] = 0;
    ckmv2bit
}
