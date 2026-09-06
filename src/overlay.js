import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const COLS = 18;
const ROWS = 7;
const CELL = 6;
const DOT_R = 1.6;

const pill = document.getElementById('pill');
const label = document.getElementById('label');
const stopBtn = document.getElementById('stop-btn');
const matrix = document.getElementById('matrix');

// Pre-create circles in a grid. Each column has ROWS circles vertically centered.
// Top and bottom rows mirror around the middle so amplitude grows symmetrically.
const dots = []; // dots[col][row] = element
const startX = 4;
const middleY = 11; // half of viewBox height
for (let c = 0; c < COLS; c++) {
  const colDots = [];
  for (let r = 0; r < ROWS; r++) {
    const offset = r - Math.floor(ROWS / 2);
    const cy = middleY + offset * CELL * 0.55;
    const cx = startX + c * CELL;
    const dot = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
    dot.setAttribute('cx', cx);
    dot.setAttribute('cy', cy);
    dot.setAttribute('r', DOT_R);
    dot.setAttribute('fill', 'currentColor');
    dot.setAttribute('opacity', '0.18');
    matrix.appendChild(dot);
    colDots.push(dot);
  }
  dots.push(colDots);
}

// Rolling buffer of recent levels, oldest at index 0, newest at last.
let levels = new Array(COLS).fill(0);

const setColumn = (col, level) => {
  // level: 0..1. Determine how many rows around center to light up.
  // ROWS=7, center=3, so radius can be 0..3.
  const center = Math.floor(ROWS / 2);
  const radius = Math.min(center, Math.round(level * center));
  for (let r = 0; r < ROWS; r++) {
    const distance = Math.abs(r - center);
    const lit = distance <= radius;
    const opacity = lit ? Math.max(0.55, 1 - distance * 0.18) : 0.18;
    dots[col][r].setAttribute('opacity', opacity.toFixed(2));
  }
};

const renderAll = () => {
  for (let c = 0; c < COLS; c++) setColumn(c, levels[c]);
};

// Smoothing so bars don't strobe.
let smoothed = 0;

const pushLevel = (raw) => {
  // RMS values from cpal are typically 0.001..0.3 for quiet/loud speech.
  // Map with a non-linear curve so quiet speech is visible.
  const normalized = Math.min(1, Math.pow(Math.min(raw * 6, 1), 0.6));
  smoothed = smoothed * 0.4 + normalized * 0.6;
  levels.shift();
  levels.push(smoothed);
  renderAll();
};

let errorTimer = null;

const showError = (title) => {
  if (errorTimer) {
    clearTimeout(errorTimer);
    errorTimer = null;
  }
  pill.dataset.state = 'error';
  label.textContent = title || 'Dictation failed';
  levels = levels.map(() => 0);
  renderAll();
  errorTimer = setTimeout(() => {
    errorTimer = null;
    setState('Ready');
  }, 4500);
};

const setState = (state) => {
  const map = { Ready: 'ready', Recording: 'recording', Transcribing: 'transcribing' };
  const s = map[state] || 'ready';
  if (s === 'ready' && pill.dataset.state === 'error' && errorTimer) {
    return;
  }
  if (errorTimer && s !== 'ready') {
    clearTimeout(errorTimer);
    errorTimer = null;
  }
  pill.dataset.state = s;
  if (s === 'recording') label.textContent = 'Listening...';
  else if (s === 'transcribing') label.textContent = 'Transcribing...';
  if (s !== 'recording') {
    // Decay bars to flat when not recording.
    levels = levels.map(() => 0);
    renderAll();
  }
};

const errorTitle = (payload) => {
  if (payload && typeof payload === 'object' && payload.title) return payload.title;
  const msg = typeof payload === 'string' ? payload : payload?.message || '';
  if (/mic|silence|microphone/i.test(msg)) return 'Mic access needed';
  if (/model|parakeet|whisper|downloaded|not linked/i.test(msg)) return 'Model not ready';
  if (/no audio|too short|captured/i.test(msg)) return 'No audio captured';
  if (/no text|recognized/i.test(msg)) return 'Nothing recognized';
  if (/paste|system events|accessibility/i.test(msg)) return 'Paste blocked';
  return 'Dictation failed';
};

renderAll();

console.log('[Mabel overlay] initialized');

listen('recording-state', (e) => {
  console.log('[Mabel overlay] recording-state:', e.payload);
  setState(e.payload);
});
listen('audio-level', (e) => {
  const lvl = Number(e.payload) || 0;
  pushLevel(lvl);
});
listen('transcription-error', (e) => {
  console.error('[Mabel overlay] transcription-error:', e.payload);
  showError(errorTitle(e.payload));
});
stopBtn.addEventListener('click', async () => {
  console.log('[Mabel overlay] stop button clicked -> toggle_recording');
  try {
    const result = await invoke('toggle_recording');
    console.log('[Mabel overlay] toggle_recording result:', result);
  } catch (err) {
    console.error('[Mabel overlay] toggle_recording failed:', err);
    showError(errorTitle(typeof err === 'string' ? err : err?.message || String(err)));
  }
});
