// src/dsp/noise.js - Генератор шума «Юго-Запад СПб».
//
// Три слоя шума:
//   1. «Ветер между домами» - фильтрованный белый шум с медленной модуляцией яркости.
//   2. Гранулярность - шум «рвётся» на панельные кусочки (амплитудные зерна).
//   3. «Радиопомехи» - редкие щелчки-вспышки с быстрым затуханием.

const { StateVariableFilter } = require('./filters');
const { mulberry32 } = require('./rng');

const TWO_PI = Math.PI * 2;

function renderNoise({ length, sampleRate, level, granularity, modulation, seed }) {
  const out = new Float32Array(length);
  const rand = mulberry32(seed);
  const filter = new StateVariableFilter(sampleRate);

  // Частота зерен: чем выше гранулярность, тем мельче «панельные» кусочки
  const grainRate = 3 + granularity * 25; // Гц
  const grainStep = grainRate / sampleRate;

  let grainPhase = rand();
  let grainEnv = 1; // «размер панели» текущего зерна

  let staticEnv = 0; // уровень текущей радиопомехи

  for (let i = 0; i < length; i++) {
    const t = i / sampleRate;

    // Слои 1: ветер - медленная модуляция яркости шума (два LFO, чтобы не «качало» по кругу)
    const wind =
      0.5 +
      0.32 * Math.sin(TWO_PI * 0.061 * t) +
      0.18 * Math.sin(TWO_PI * 0.017 * t + 2.1);

    const white = rand() * 2 - 1;

    const cutoff = 220 + wind * (900 + modulation * 4500);
    filter.tick(white, cutoff, 0.2 + 0.5 * modulation);
    let s = filter.low * 1.2 + filter.band * 0.7;

    // Слой 2: гранулярность - окно зерна (cos-кривая, без щелчков)
    grainPhase += grainStep;
    if (grainPhase >= 1) {
      grainPhase -= 1;
      grainEnv = 0.15 + rand() * 0.85;
    }
    const grainWindow = 0.5 - 0.5 * Math.cos(TWO_PI * grainPhase);
    const gate = 1 - granularity * (1 - grainEnv) * grainWindow;
    s *= gate;

    // Слой 3: радиопомехи - редкие вспышки с быстрым затуханием (~0.15 с)
    if (staticEnv <= 0.001 && rand() < 0.0006 * (0.5 + granularity)) {
      staticEnv = 0.4 + rand() * 0.6;
    }
    if (staticEnv > 0) {
      s += white * staticEnv * 0.35;
      staticEnv *= 0.99985;
    }

    out[i] = s * level * 0.32;
  }

  return out;
}

module.exports = { renderNoise };
