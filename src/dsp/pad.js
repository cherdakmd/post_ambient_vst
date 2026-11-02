// src/dsp/pad.js - «Холодный» эмбиент-пад (третья дорожка).
//
// Медленная минорная прогрессия из расстроенных пил, «утопленных»
// в низкочастотный фильтр с ветровой модуляцией. Даёт ту самую
// «серую небо-панелька» подложку под шум и бас.

const { StateVariableFilter } = require('./filters');
const { mulberry32 } = require('./rng');

const TWO_PI = Math.PI * 2;

// Прогрессия холодных аккордов (полутонов относительно корня): i - VI - III - VII
const PROGRESSION = [
  [0, 3, 7, 14],
  [8, 12, 15, 19],
  [3, 7, 10, 17],
  [10, 14, 17, 24],
];

const VOICES = 4;

function renderPad({ length, sampleRate, rootHz, bpm, seed, modulation }) {
  const rand = mulberry32(seed + 101);
  const chordDur = (60 / bpm) * 4;     // 4 удара на аккорд
  const xfade = Math.min(2.5, chordDur * 0.4); // кроссфейд аккордов

  const left = new Float32Array(length);
  const right = new Float32Array(length);
  const fL = new StateVariableFilter(sampleRate);
  const fR = new StateVariableFilter(sampleRate);

  const phase = new Float64Array(VOICES);
  const freq = new Float64Array(VOICES);
  // Расстройка голосов: левый и правый каналы чуть расходятся - живое «дыхание»
  const detuneL = new Float64Array(VOICES);
  const detuneR = new Float64Array(VOICES);
  for (let v = 0; v < VOICES; v++) {
    const cents = 4 + rand() * 10; // 4..10 центов
    detuneL[v] = Math.pow(2, -cents / 1200);
    detuneR[v] = Math.pow(2, cents / 1200);
    phase[v] = rand();
  }

  const baseHz = rootHz * 4; // пад играет на две октавы выше баса
  let chordIdx = -1;

  for (let i = 0; i < length; i++) {
    const t = i / sampleRate;

    // Смена аккорда
    const idx = Math.floor(t / chordDur) % PROGRESSION.length;
    if (idx !== chordIdx) {
      chordIdx = idx;
      for (let v = 0; v < VOICES; v++) {
        freq[v] = baseHz * Math.pow(2, PROGRESSION[idx][v] / 12);
      }
    }

    // Огибающая аккорда: медленное появление и растворение
    const tChord = t % chordDur;
    let env = Math.min(tChord, chordDur - tChord) / xfade;
    if (env > 1) env = 1;
    env = env * env * (3 - 2 * env); // smoothstep

    // Синтез голосов (расстроенные пилы)
    let sL = 0;
    let sR = 0;
    for (let v = 0; v < VOICES; v++) {
      phase[v] += freq[v] / sampleRate;
      if (phase[v] >= 1) phase[v] -= 1;
      const saw = 2 * phase[v] - 1;
      sL += saw * detuneL[v];
      sR += saw * detuneR[v];
    }

    // «Ветровой» срез: медленный LFO двигает частоту фильтра
    const lfo = 1 + 0.35 * Math.sin(TWO_PI * 0.05 * t) * (0.5 + modulation);
    const cutoff = Math.min((350 + 250 * Math.sin(TWO_PI * 0.023 * t)) * lfo, sampleRate * 0.45);

    fL.tick(sL * 0.2, cutoff, 0.3);
    fR.tick(sR * 0.2, cutoff, 0.3);

    left[i] = fL.low * env * 0.32;
    right[i] = fR.low * env * 0.32;
  }

  return { left, right };
}

module.exports = { renderPad };
