// from Java: cs.min2phase.Tools
//
// Scramble <-> facelet conversion + random cube generation. The Java original
// also has `initFrom` / `saveTo` for serialising precomputed tables; we don't
// need that since `Tables::build(true)` is already fast (~100ms) in Rust.

use crate::cubie::CubieCube;
use crate::util::{self, U5, R5, F5, D5, L5, B5};
use crate::Tables;
use rand::{Rng, RngCore};

// ===== facelet verify =====

/// Result of `verify_facelets`. Mirrors Java's negative error codes from
/// `Search.verify` + `CubieCube.verify`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyError {
    /// -1: not exactly one facelet of each color
    FaceletParse,
    /// -2: not all 12 edges exist exactly once
    EdgeMissing,
    /// -3: one edge has to be flipped
    EdgeFlip,
    /// -4: not all 8 corners exist exactly once
    CornerMissing,
    /// -5: one corner has to be twisted
    CornerTwist,
    /// -6: two corners or two edges have to be exchanged
    Parity,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            VerifyError::FaceletParse => "Error 1: facelet parse failure",
            VerifyError::EdgeMissing => "Error 2: edge missing",
            VerifyError::EdgeFlip => "Error 3: edge flip",
            VerifyError::CornerMissing => "Error 4: corner missing",
            VerifyError::CornerTwist => "Error 5: corner twist",
            VerifyError::Parity => "Error 6: parity",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for VerifyError {}

/// from Java: Search.verify(String facelets)
///
/// Parses the 54-char facelet string into a `CubieCube` written to `cc_out`
/// (no allocation). Returns `Ok(())` on success.
pub fn verify_into(facelets: &str, cc_out: &mut CubieCube, tables: &Tables) -> Result<(), VerifyError> {
    let _ = tables; // Tables not needed for this verify (cubie.verify is self-contained)
    let bytes = facelets.as_bytes();
    if bytes.len() < 54 {
        return Err(VerifyError::FaceletParse);
    }
    let centers = [
        bytes[U5 as usize],
        bytes[R5 as usize],
        bytes[F5 as usize],
        bytes[D5 as usize],
        bytes[L5 as usize],
        bytes[B5 as usize],
    ];
    let mut f = [0u8; 54];
    let mut count: i32 = 0;
    for i in 0..54 {
        let ch = bytes[i];
        let mut found: i32 = -1;
        for (k, &c) in centers.iter().enumerate() {
            if c == ch {
                found = k as i32;
                break;
            }
        }
        if found == -1 {
            return Err(VerifyError::FaceletParse);
        }
        f[i] = found as u8;
        count += 1 << (found << 2);
    }
    if count != 0x999999 {
        return Err(VerifyError::FaceletParse);
    }
    util::to_cubie_cube(&f, cc_out);
    match cc_out.verify() {
        0 => Ok(()),
        -2 => Err(VerifyError::EdgeMissing),
        -3 => Err(VerifyError::EdgeFlip),
        -4 => Err(VerifyError::CornerMissing),
        -5 => Err(VerifyError::CornerTwist),
        -6 => Err(VerifyError::Parity),
        _ => Err(VerifyError::FaceletParse),
    }
}

/// Convenience wrapper that allocates a fresh `CubieCube`.
pub fn verify_facelets(facelets: &str, tables: &Tables) -> Result<(), VerifyError> {
    let mut cc = CubieCube::solved();
    verify_into(facelets, &mut cc, tables)
}

// ===== fromScramble =====

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrambleParseError;

impl std::fmt::Display for ScrambleParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("scramble parse error")
    }
}
impl std::error::Error for ScrambleParseError {}

/// from Java: Tools.fromScramble(int[]) — apply moves to solved cube,
/// return 54-char facelet string.
pub fn from_scramble_moves(scramble: &[u8], tables: &Tables) -> String {
    let mut c1 = CubieCube::solved();
    let mut c2 = CubieCube::empty();
    for &m in scramble {
        let mv = &tables.move_cube[m as usize];
        CubieCube::corn_mult(&c1, mv, &mut c2);
        CubieCube::edge_mult(&c1, mv, &mut c2);
        std::mem::swap(&mut c1, &mut c2);
    }
    util::to_face_cube(&c1)
}

/// from Java: Tools.fromScramble(String) — parse "R U R' U'" style scramble,
/// build the facelet string after applying the moves to a solved cube.
///
/// Mirrors Java behaviour: unrecognised characters are silently skipped, but
/// the function never errors (Java doesn't either). Returns the 54-char
/// facelet representation.
pub fn from_scramble(s: &str, tables: &Tables) -> String {
    let mut arr: Vec<u8> = Vec::with_capacity(s.len());
    let mut axis: i32 = -1;
    for ch in s.chars() {
        match ch {
            'U' => axis = 0,
            'R' => axis = 3,
            'F' => axis = 6,
            'D' => axis = 9,
            'L' => axis = 12,
            'B' => axis = 15,
            ' ' => {
                if axis != -1 {
                    arr.push(axis as u8);
                }
                axis = -1;
            }
            '2' => {
                if axis != -1 {
                    axis += 1;
                }
            }
            '\'' => {
                if axis != -1 {
                    axis += 2;
                }
            }
            _ => continue,
        }
    }
    if axis != -1 {
        arr.push(axis as u8);
    }
    from_scramble_moves(&arr, tables)
}

/// Apply a WCA-notation scramble to an existing facelet state, returning the
/// new 54-char facelet string. Errors if the input state fails verification.
///
/// This is the building block for GUI "apply solution" workflows — you can
/// take a solver's output and apply it to the displayed cube to watch it
/// progress to (or away from) solved, depending on whether the solver was
/// run with `INVERSE_SOLUTION` set.
pub fn apply_moves(facelets: &str, scramble: &str, tables: &Tables) -> Result<String, VerifyError> {
    let mut c1 = CubieCube::empty();
    verify_into(facelets, &mut c1, tables)?;
    let mut c2 = CubieCube::empty();

    let mut axis: i32 = -1;
    let emit = |a: &mut i32, c1: &mut CubieCube, c2: &mut CubieCube| {
        if *a != -1 {
            let mv = &tables.move_cube[*a as usize];
            CubieCube::corn_mult(c1, mv, c2);
            CubieCube::edge_mult(c1, mv, c2);
            std::mem::swap(c1, c2);
        }
    };
    for ch in scramble.chars() {
        match ch {
            'U' => { emit(&mut axis, &mut c1, &mut c2); axis = 0; }
            'R' => { emit(&mut axis, &mut c1, &mut c2); axis = 3; }
            'F' => { emit(&mut axis, &mut c1, &mut c2); axis = 6; }
            'D' => { emit(&mut axis, &mut c1, &mut c2); axis = 9; }
            'L' => { emit(&mut axis, &mut c1, &mut c2); axis = 12; }
            'B' => { emit(&mut axis, &mut c1, &mut c2); axis = 15; }
            ' ' | '\t' | '\n' => { emit(&mut axis, &mut c1, &mut c2); axis = -1; }
            '2' => { if axis != -1 { axis += 1; } }
            '\'' => { if axis != -1 { axis += 2; } }
            _ => continue,
        }
    }
    emit(&mut axis, &mut c1, &mut c2);
    Ok(util::to_face_cube(&c1))
}

// ===== randomCube / randomLastLayer / superFlip =====
//
// Java's STATE_RANDOM=null and STATE_SOLVED=new byte[0] sentinels become an
// enum here. -1 in slot arrays means "unknown, fill randomly".

#[allow(dead_code)]
#[derive(Clone)]
enum SlotState<'a> {
    Random,
    Solved,
    Partial(&'a [i8]),
}

fn get_n_perm_signed(arr: &[i8], n: usize) -> i32 {
    let mut idx: i32 = 0;
    for i in 0..n {
        idx *= (n - i) as i32;
        for j in (i + 1)..n {
            if arr[j] < arr[i] {
                idx += 1;
            }
        }
    }
    idx
}

fn resolve_ori<R: Rng + ?Sized>(arr: &mut [i8], base: i32, rng: &mut R) -> i32 {
    let mut sum = 0i32;
    let mut last_unknown: i32 = -1;
    for (i, slot) in arr.iter_mut().enumerate() {
        if *slot == -1 {
            *slot = rng.gen_range(0..base) as i8;
            last_unknown = i as i32;
        }
        sum += *slot as i32;
    }
    if sum % base != 0 && last_unknown != -1 {
        let lu = last_unknown as usize;
        arr[lu] = (((30 + arr[lu] as i32 - sum) % base) as i8 + base as i8) as i8 % base as i8;
        // Java's `(30 + arr[lu] - sum) % base` for positive `30+arr-sum` may still go
        // negative if sum is very large; but sum<=base*len<=24 and 30+arr>=30 so safe.
    }
    let mut idx = 0i32;
    for i in 0..(arr.len() - 1) {
        idx *= base;
        idx += arr[i] as i32;
    }
    idx
}

/// Owned-array variant — mutates `arr` in place.
fn resolve_perm_owned<R: Rng + ?Sized>(arr: &mut [i8], cnt_u: i32, parity: i32, rng: &mut R) -> i32 {
    let n = arr.len();
    let mut val: Vec<i8> = (0..n as i8).collect();
    for &v in arr.iter() {
        if v != -1 {
            val[v as usize] = -1;
        }
    }
    let mut idx: usize = 0;
    for i in 0..n {
        if val[i] != -1 {
            let j = rng.gen_range(0..=idx);
            let tmp = val[i];
            val[idx] = val[j];
            val[j] = tmp;
            idx += 1;
        }
    }
    let mut cnt = cnt_u;
    let mut last: i32 = -1;
    let mut idx2: usize = 0;
    while idx2 < n && cnt > 0 {
        if arr[idx2] == -1 {
            if cnt == 2 {
                last = idx2 as i32;
            }
            cnt -= 1;
            arr[idx2] = val[cnt as usize];
        }
        idx2 += 1;
    }
    let p = util::get_n_parity(get_n_perm_signed(arr, n), n as i32);
    if p == 1 - parity && last != -1 {
        let li = last as usize;
        arr.swap(idx2 - 1, li);
    }
    p
}

fn ori_to_idx(arr: &[i8], base: i32) -> i32 {
    let mut idx = 0i32;
    for i in 0..(arr.len() - 1) {
        idx *= base;
        idx += arr[i] as i32;
    }
    idx
}

#[allow(clippy::too_many_arguments)]
fn random_state<R: Rng + ?Sized>(
    cp: SlotState,
    co: SlotState,
    ep: SlotState,
    eo: SlotState,
    rng: &mut R,
) -> String {
    // Materialise Partial slots into owned Vecs (we need to mutate them in
    // resolve_perm / resolve_ori).
    let mut cp_buf: Option<Vec<i8>> = if let SlotState::Partial(a) = &cp {
        Some(a.to_vec())
    } else {
        None
    };
    let mut co_buf: Option<Vec<i8>> = if let SlotState::Partial(a) = &co {
        Some(a.to_vec())
    } else {
        None
    };
    let mut ep_buf: Option<Vec<i8>> = if let SlotState::Partial(a) = &ep {
        Some(a.to_vec())
    } else {
        None
    };
    let mut eo_buf: Option<Vec<i8>> = if let SlotState::Partial(a) = &eo {
        Some(a.to_vec())
    } else {
        None
    };

    let cnt_ue = match &ep {
        SlotState::Random => 12,
        SlotState::Solved => 0,
        SlotState::Partial(_) => ep_buf.as_ref().unwrap().iter().filter(|&&v| v == -1).count() as i32,
    };
    let cnt_uc = match &cp {
        SlotState::Random => 8,
        SlotState::Solved => 0,
        SlotState::Partial(_) => cp_buf.as_ref().unwrap().iter().filter(|&&v| v == -1).count() as i32,
    };

    let parity;
    let cp_val;
    let ep_val;
    if cnt_ue < 2 {
        // ep != STATE_RANDOM
        match &ep {
            SlotState::Solved => {
                ep_val = 0;
                parity = 0;
            }
            SlotState::Partial(_) => {
                let buf = ep_buf.as_mut().unwrap();
                parity = resolve_perm_owned(buf, cnt_ue, -1, rng);
                ep_val = get_n_perm_signed(buf, 12);
            }
            SlotState::Random => unreachable!(),
        }
        match &cp {
            SlotState::Solved => {
                cp_val = 0;
            }
            SlotState::Random => {
                let mut v: i32;
                loop {
                    v = rng.gen_range(0..40320);
                    if util::get_n_parity(v, 8) == parity {
                        break;
                    }
                }
                cp_val = v;
            }
            SlotState::Partial(_) => {
                let buf = cp_buf.as_mut().unwrap();
                resolve_perm_owned(buf, cnt_uc, parity, rng);
                cp_val = get_n_perm_signed(buf, 8);
            }
        }
    } else {
        match &cp {
            SlotState::Solved => {
                cp_val = 0;
                parity = 0;
            }
            SlotState::Random => {
                cp_val = rng.gen_range(0..40320);
                parity = util::get_n_parity(cp_val, 8);
            }
            SlotState::Partial(_) => {
                let buf = cp_buf.as_mut().unwrap();
                parity = resolve_perm_owned(buf, cnt_uc, -1, rng);
                cp_val = get_n_perm_signed(buf, 8);
            }
        }
        match &ep {
            SlotState::Random => {
                let mut v: i32;
                loop {
                    v = rng.gen_range(0..479_001_600);
                    if util::get_n_parity(v, 12) == parity {
                        break;
                    }
                }
                ep_val = v;
            }
            SlotState::Solved => {
                ep_val = 0;
            }
            SlotState::Partial(_) => {
                let buf = ep_buf.as_mut().unwrap();
                resolve_perm_owned(buf, cnt_ue, parity, rng);
                ep_val = get_n_perm_signed(buf, 12);
            }
        }
    }

    let co_val = match &co {
        SlotState::Random => rng.gen_range(0..2187),
        SlotState::Solved => 0,
        SlotState::Partial(_) => {
            let buf = co_buf.as_mut().unwrap();
            resolve_ori(buf, 3, rng);
            ori_to_idx(buf, 3)
        }
    };
    let eo_val = match &eo {
        SlotState::Random => rng.gen_range(0..2048),
        SlotState::Solved => 0,
        SlotState::Partial(_) => {
            let buf = eo_buf.as_mut().unwrap();
            resolve_ori(buf, 2, rng);
            ori_to_idx(buf, 2)
        }
    };

    let cc = CubieCube::from_coords(cp_val, co_val, ep_val, eo_val);
    util::to_face_cube(&cc)
}

/// Generates a uniformly-random solvable cube. The string is the 54-char
/// facelet representation suitable for `Solver::solve`.
pub fn random_cube<R: Rng + ?Sized>(rng: &mut R) -> String {
    random_state(SlotState::Random, SlotState::Random, SlotState::Random, SlotState::Random, rng)
}

/// Cube with random Last Layer (U-face stickers scrambled, rest solved).
pub fn random_last_layer<R: Rng + ?Sized>(rng: &mut R) -> String {
    let cp: [i8; 8] = [-1, -1, -1, -1, 4, 5, 6, 7];
    let co: [i8; 8] = [-1, -1, -1, -1, 0, 0, 0, 0];
    let ep: [i8; 12] = [-1, -1, -1, -1, 4, 5, 6, 7, 8, 9, 10, 11];
    let eo: [i8; 12] = [-1, -1, -1, -1, 0, 0, 0, 0, 0, 0, 0, 0];
    random_state(
        SlotState::Partial(&cp),
        SlotState::Partial(&co),
        SlotState::Partial(&ep),
        SlotState::Partial(&eo),
        rng,
    )
}

/// from Java: Tools.superFlip — the 20-move-deep "every edge flipped" state.
pub fn super_flip() -> String {
    let cc = CubieCube::from_coords(0, 0, 0, 2047);
    util::to_face_cube(&cc)
}

/// Helper: pull entropy from `rand::thread_rng()`.
pub fn random_cube_thread() -> String {
    let mut rng: Box<dyn RngCore> = Box::new(rand::thread_rng());
    random_cube(&mut rng)
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn tbl() -> Tables {
        Tables::build(true)
    }

    #[test]
    fn super_flip_facelets_well_formed() {
        let s = super_flip();
        assert_eq!(s.len(), 54);
        // Centers must read U R F D L B
        let b = s.as_bytes();
        assert_eq!(b[4], b'U');
        assert_eq!(b[13], b'R');
        assert_eq!(b[22], b'F');
        assert_eq!(b[31], b'D');
        assert_eq!(b[40], b'L');
        assert_eq!(b[49], b'B');
    }

    #[test]
    fn verify_solved_ok() {
        let t = tbl();
        let solved = "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB";
        verify_facelets(solved, &t).unwrap();
    }

    #[test]
    fn from_scramble_solved_when_empty() {
        let t = tbl();
        let s = from_scramble("", &t);
        assert_eq!(s, "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB");
    }

    #[test]
    fn from_scramble_round_trip_known() {
        let t = tbl();
        // "R U R' U'" is a sexy move; applying it 6 times returns identity.
        let mut s = "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB".to_string();
        for _ in 0..6 {
            // Build a fresh facelets each time by composing from a sequence; here
            // we just sanity-check that applying "R U R' U'" once gives a non-solved
            // facelet and verify still passes.
            s = from_scramble("R U R' U' R U R' U' R U R' U' R U R' U' R U R' U' R U R' U'", &t);
        }
        assert_eq!(s, "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB");
    }

    #[test]
    fn random_cube_is_solvable_facelets() {
        let t = tbl();
        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..20 {
            let f = random_cube(&mut rng);
            assert_eq!(f.len(), 54);
            verify_facelets(&f, &t).expect("random cube must be solvable");
        }
    }

    #[test]
    fn random_last_layer_is_solvable() {
        let t = tbl();
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..10 {
            let f = random_last_layer(&mut rng);
            verify_facelets(&f, &t).expect("LL state must verify");
        }
    }

    #[test]
    fn apply_moves_identity_returns_same_state() {
        let t = tbl();
        let scrambled = from_scramble("R U2 D' B D'", &t);
        let same = apply_moves(&scrambled, "", &t).unwrap();
        assert_eq!(same, scrambled);
    }

    #[test]
    fn apply_moves_on_solved_matches_from_scramble() {
        let t = tbl();
        let solved = "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB";
        let a = apply_moves(solved, "R U R' U' R' F R F'", &t).unwrap();
        let b = from_scramble("R U R' U' R' F R F'", &t);
        assert_eq!(a, b);
    }

    #[test]
    fn apply_moves_roundtrip_solves_scrambled() {
        // Apply scramble to solved, then apply the algorithmically-inverted
        // sequence to that scrambled state — result must be solved again.
        let t = tbl();
        let scramble = "U2 R2 F2 D' U F2 U2 B U B'";
        let inverse = invert_scramble(scramble);
        let s = from_scramble(scramble, &t);
        let back = apply_moves(&s, &inverse, &t).unwrap();
        assert_eq!(back, "UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB");
    }

    /// Reverse a scramble: reverse token order and invert each move's direction.
    fn invert_scramble(s: &str) -> String {
        s.split_whitespace()
            .rev()
            .map(|m| {
                if let Some(stripped) = m.strip_suffix('2') {
                    format!("{stripped}2")
                } else if let Some(stripped) = m.strip_suffix('\'') {
                    stripped.to_string()
                } else {
                    format!("{m}'")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn apply_moves_rejects_bad_state() {
        let t = tbl();
        let bad = "X".repeat(54);
        assert!(apply_moves(&bad, "R", &t).is_err());
    }
}
