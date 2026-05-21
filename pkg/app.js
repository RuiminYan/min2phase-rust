// min2phase-rust GUI — single-file front end over m2p_wasm.
// State flow:
//   currentFacelets  — the cube state shown on screen
//   lastSolution     — last move sequence produced by Solve / Next; null otherwise

import init, { Min2Phase } from './m2p_wasm.js';

const SOLVED = 'UUUUUUUUURRRRRRRRRFFFFFFFFFDDDDDDDDDLLLLLLLLLBBBBBBBBB';
const FACES = ['U', 'R', 'F', 'D', 'L', 'B'];
// 12x9 grid placement (col_start, row_start), 1-indexed:
//   U at (4,1), L at (1,4), F at (4,4), R at (7,4), B at (10,4), D at (4,7)
const FACE_POS = { U: [4, 1], L: [1, 4], F: [4, 4], R: [7, 4], B: [10, 4], D: [4, 7] };

const $ = id => document.getElementById(id);
const cubeEl = $('cube');
const solEl  = $('solution');
const heroEl = $('solve-time-hero');

let m = null;
let currentFacelets = SOLVED;
let lastSolution = null;
let wasmInitMs = 0;
let tablesInitMs = 0;

function buildCubeNet() {
  cubeEl.innerHTML = '';
  for (const face of FACES) {
    const [c0, r0] = FACE_POS[face];
    for (let i = 0; i < 9; i++) {
      const r = Math.floor(i / 3);
      const c = i % 3;
      const d = document.createElement('div');
      d.className = 'sticker';
      d.dataset.face = face;
      d.dataset.idx = String(i);
      d.dataset.test = `sticker-${face}-${i}`;
      d.style.gridColumn = `${c0 + c} / span 1`;
      d.style.gridRow    = `${r0 + r} / span 1`;
      cubeEl.appendChild(d);
    }
  }
}

function paint(facelets) {
  if (typeof facelets !== 'string' || facelets.length !== 54) {
    setSolution('Invalid cube state (length != 54)', 'error');
    return;
  }
  currentFacelets = facelets;
  const stickers = cubeEl.children;
  for (let s = 0; s < stickers.length; s++) {
    const node = stickers[s];
    const face = node.dataset.face;
    const idx = parseInt(node.dataset.idx, 10);
    const offset = FACES.indexOf(face) * 9 + idx;
    const ch = facelets[offset];
    node.className = `sticker ${ch}`;
  }
  cubeEl.dataset.state = facelets;
}

function setSolution(text, kind) {
  solEl.textContent = text;
  solEl.classList.remove('muted', 'error');
  if (kind === 'muted') solEl.classList.add('muted');
  if (kind === 'error') solEl.classList.add('error');
}

function setStats({ solveMs, len, probes } = {}) {
  $('stat-wasm').textContent = wasmInitMs ? `${wasmInitMs.toFixed(0)} ms` : '—';
  $('stat-init').textContent = tablesInitMs ? `${tablesInitMs.toFixed(0)} ms` : '—';
  if (typeof solveMs === 'number') {
    $('stat-solve').textContent = `${solveMs.toFixed(2)} ms`;
    heroEl.textContent = `${solveMs.toFixed(2)} ms`;
  }
  if (typeof len === 'number')    $('stat-len').textContent    = String(len);
  if (typeof probes === 'number') $('stat-probes').textContent = String(probes);
}

function safeCall(fn, errorPrefix) {
  try {
    return fn();
  } catch (e) {
    const msg = (e && (e.message || e.toString())) || 'unknown error';
    setSolution(`${errorPrefix}: ${msg}`, 'error');
    return null;
  }
}

function clearSolution() {
  lastSolution = null;
  $('btn-next').disabled = true;
  $('btn-applysol').disabled = true;
  $('stat-solve').textContent = '—';
  $('stat-len').textContent = '—';
  $('stat-probes').textContent = '—';
}

function recordSolution(sol, solveMs) {
  lastSolution = sol;
  setSolution(sol.trim() || '(empty — already solved)', null);
  $('btn-next').disabled = false;
  $('btn-applysol').disabled = sol.trim().length === 0;
  setStats({ solveMs, len: m.lastLength(), probes: m.lastProbes() });
}

function bindHandlers() {
  $('btn-apply').addEventListener('click', () => {
    const s = $('scramble').value.trim();
    if (!s) { $('scramble').focus(); return; }
    const f = safeCall(() => m.fromScramble(s), 'Apply scramble failed');
    if (f) {
      paint(f);
      clearSolution();
      setSolution('Scrambled. Click Solve.', 'muted');
    }
  });

  $('btn-random').addEventListener('click', () => {
    const f = safeCall(() => m.randomCube(), 'Random failed');
    if (f) {
      paint(f);
      $('scramble').value = '';
      clearSolution();
      setSolution('Random cube. Click Solve.', 'muted');
    }
  });

  $('btn-reset').addEventListener('click', () => {
    paint(SOLVED);
    $('scramble').value = '';
    clearSolution();
    setSolution('Solved. Try Random or paste a scramble.', 'muted');
  });

  $('btn-solve').addEventListener('click', () => {
    const t = performance.now();
    const sol = safeCall(() => m.solve(currentFacelets), 'Solve failed');
    if (sol !== null) {
      const ms = performance.now() - t;
      recordSolution(sol, ms);
    }
  });

  $('btn-next').addEventListener('click', () => {
    const t = performance.now();
    const sol = safeCall(() => m.next(100000, 0), 'Next solution failed');
    if (sol !== null) {
      const ms = performance.now() - t;
      recordSolution(sol, ms);
    }
  });

  $('btn-applysol').addEventListener('click', () => {
    if (!lastSolution) return;
    const f = safeCall(() => m.applyMoves(currentFacelets, lastSolution), 'Apply solution failed');
    if (f) {
      paint(f);
      const isSolved = f === SOLVED;
      setSolution(isSolved
        ? 'Solved. The solution above transforms the previous state to a solved cube.'
        : 'State changed but not solved — the solver may have used INVERSE_SOLUTION mode.', isSolved ? 'muted' : null);
      clearSolution();
    }
  });

  $('scramble').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') $('btn-apply').click();
  });
}

(async () => {
  buildCubeNet();
  paint(SOLVED);
  setSolution('Loading WASM…', 'muted');

  try {
    const t0 = performance.now();
    await init();
    wasmInitMs = performance.now() - t0;

    const t1 = performance.now();
    m = new Min2Phase();
    tablesInitMs = performance.now() - t1;

    setStats({});
    setSolution('Ready. Try Random or paste a scramble.', 'muted');
    document.body.dataset.status = 'ready';
    bindHandlers();
  } catch (e) {
    setSolution(`WASM init failed: ${e.message || e}`, 'error');
    document.body.dataset.status = 'error';
  }
})();
