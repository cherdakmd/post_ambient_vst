// src/dsp/noise.js - Генераторы шума «Юго-Запад СПб».
//
// Несколько типов шума (noiseType):
//   wind    - «ветер между домами»: фильтрованный шум с медленной модуляцией яркости.
//   radio   - «радиопомехи»: полосовой шип + быстрая тряска + вспышки помех.
//   rain    - «дождь по бетону»: шип дождя + отдельные капли.
//   hum     - «гул панельного дома»: сеть 50 Гц с гармониками и медленным дыханием.
//   crackle - «скрип и треск»: редкие щелчки-осколки на тихом поле.
//   mixed   - «смесь Юго-Запада»: ветер + редкий треск + вспышки радио (по умолчанию).
//
// Любой тип зависит от `seed` (детерминированно) и от параметров
// granularity / modulation - так возможны «рандомайзером» вариации.

const { StateVariableFilter } = require('./filters');
const { mulberry32 } = require('./rng');

const TWO_PI = Math.PI * 2;

const NOISE_TYPES = ['wind', 'radio', 'rain', 'hum', 'crackle', 'mixed'];

// Амплитудное зерно: шум «рвётся» на панельные кусочки (без щелчков)
function makeGate(granularity, grainRate, sampleRate, rand) {
  let phase = rand();
  let env = 1;
  const step = grainRate / sampleRate;
  return function gate(s) {
    phase += step;
    if (phase >= 1) {
      phase -= 1;
      env = 0.15 + rand() * 0.85;
    }
    const w = 0.5 - 0.5 * Math.cos(TWO_PI * phase);
    return s * (1 - granularity * (1 - env) * w);
  };
}

// 1. Ветер между домами
function renderWind(p) {
  const { length, sampleRate, level, granularity, modulation, rand } = p;
  const out = new Float32Array(length);
  const filter = new StateVariableFilter(sampleRate);
  const gate = makeGate(granularity, 3 + granularity * 25, sampleRate, rand);

  for (let i = 0; i < length; i++) {
    const t = i / sampleRate;
    // Два LFO с разными периодами - «ветер» не повторяется по кругу
    const wind =
      0.5 +
      0.32 * Math.sin(TWO_PI * 0.061 * t) +
      0.18 * Math.sin(TWO_PI * 0.017 * t + 2.1);

    const white = rand() * 2 - 1;
    const cutoff = 220 + wind * (900 + modulation * 4500);
    filter.tick(white, cutoff, 0.2 + 0.5 * modulation);
    let s = filter.low * 1.2 + filter.band * 0.7;

    out[i] = gate(s) * level * 0.32;
  }
  return out;
}

// 2. Радиопомехи
function renderRadio(p) {
  const { length, sampleRate, level, granularity, modulation, rand } = p;
  const out = new Float32Array(length);
  const filter = new StateVariableFilter(sampleRate);
  const gate = makeGate(granularity * 0.5, 14 + granularity * 12, sampleRate, rand);
  let burst = 0;

  for (let i = 0; i < length; i++) {
    const t = i / sampleRate;
    const white = rand() * 2 - 1;

    const cutoff = 700 + modulation * 5000 + 1500 * Math.sin(TWO_PI * 0.23 * t);
    filter.tick(white, cutoff, 0.35 + 0.5 * modulation);
    let s = filter.band * 1.4 + filter.high * 0.5;

    // Быстрая «тряска» сигнала - нестабильный приём
    s *= 1 - modulation * 0.6 * (0.5 + 0.5 * Math.sin(TWO_PI * 9.7 * t));

    // Вспышки помех
    if (burst <= 0.001 && rand() < 0.0009 * (0.5 + granularity)) {
      burst = 0.3 + rand() * 0.7;
    }
    if (burst > 0) {
      s += white * burst * 0.5;
      burst *= 0.9998;
    }

    out[i] = gate(s) * level * 0.3;
  }
  return out;
}

// 3. Дождь по бетону
function renderRain(p) {
  const { length, sampleRate, level, granularity, modulation, rand } = p;
  const out = new Float32Array(length);
  const filter = new StateVariableFilter(sampleRate);
  const gate = makeGate(granularity * 0.4, 6 + granularity * 10, sampleRate, rand);
  let dropEnv = 0;

  for (let i = 0; i < length; i++) {
    const white = rand() * 2 - 1;
    const cutoff = 3000 + modulation * 4500;
    filter.tick(white, cutoff, 0.15);
    let s = filter.high * 0.6 + filter.band * 0.4; // шип дождя

    // Отдельные капли по подоконникам
    if (rand() < 0.01 + 0.01 * granularity) {
      dropEnv = 0.3 + rand() * 0.7;
    }
    if (dropEnv > 0) {
      s += white * dropEnv * 0.6;
      dropEnv *= 0.9988; // ~10 мс
    }

    out[i] = gate(s) * level * 0.28;
  }
  return out;
}

// 4. Гул панельного дома (50 Гц)
function renderHum(p) {
  const { length, sampleRate, level, granularity, modulation, rand } = p;
  const out = new Float32Array(length);
  const filter = new StateVariableFilter(sampleRate);
  const gate = makeGate(granularity * 0.6, 5, sampleRate, rand);

  for (let i = 0; i < length; i++) {
    const t = i / sampleRate;
    // Сеть и её гармоники с лёгким «дрожанием» фазы
    let s =
      1.0 * Math.sin(TWO_PI * 50 * t + 0.7 * Math.sin(TWO_PI * 0.05 * t)) +
      0.5 * Math.sin(TWO_PI * 100 * t + 1.2) +
      0.25 * Math.sin(TWO_PI * 150 * t);
    // Медленное дыхание трансформатора
    s *= 1 + 0.25 * Math.sin(TWO_PI * 0.13 * t);

    const white = rand() * 2 - 1;
    filter.tick(white, 200 + modulation * 1500, 0.3);
    s = s * 0.5 + filter.low * 0.8;

    out[i] = gate(s) * level * 0.25;
  }
  return out;
}

// 5. Скрип и треск
function renderCrackle(p) {
  const { length, sampleRate, level, granularity, modulation, rand } = p;
  const out = new Float32Array(length);
  const filter = new StateVariableFilter(sampleRate);
  const gate = makeGate(granularity * 0.5, 8, sampleRate, rand);
  let env = 0;

  for (let i = 0; i < length; i++) {
    const t = i / sampleRate;
    const white = rand() * 2 - 1;
    filter.tick(white, 1000 + modulation * 2500, 0.4);
    let s = filter.low * 0.12; // тихий бетонный пол

    // Редкие острые щелчки (скрип панелей, стуки в трубах)
    if (env <= 0.001 && rand() < 0.002 * (0.3 + granularity)) {
      env = 0.4 + rand() * 0.6;
    }
    if (env > 0) {
      s += white * (0.5 + 0.5 * Math.sin(TWO_PI * 90 * t)) * env * 0.7;
      env *= 0.9994; // ~60 мс
    }

    out[i] = gate(s) * level * 0.35;
  }
  return out;
}

// 6. Смесь Юго-Запада: ветер + редкий треск + вспышки радио
function renderMixed(p) {
  const out = renderWind(p);
  const { length, sampleRate, granularity, rand } = p;
  let crackleEnv = 0;
  let burst = 0;

  for (let i = 0; i < length; i++) {
    if (crackleEnv <= 0.001 && rand() < 0.0004 * (0.4 + granularity)) {
      crackleEnv = 0.35 + rand() * 0.55;
    }
    if (crackleEnv > 0) {
      out[i] += (rand() * 2 - 1) * crackleEnv * 0.3;
      crackleEnv *= 0.9995;
    }
    if (burst <= 0.001 && rand() < 0.0003 * (0.5 + granularity)) {
      burst = 0.25 + rand() * 0.45;
    }
    if (burst > 0) {
      out[i] += (rand() * 2 - 1) * burst * 0.25;
      burst *= 0.99985;
    }
  }
  return out;
}

const RENDERERS = {
  wind: renderWind,
  radio: renderRadio,
  rain: renderRain,
  hum: renderHum,
  crackle: renderCrackle,
  mixed: renderMixed,
};

function renderNoise({ length, sampleRate, level, granularity, modulation, seed, type = 'mixed' }) {
  const rand = mulberry32(seed);
  const renderer = RENDERERS[type] || renderMixed;
  return renderer({ length, sampleRate, level, granularity, modulation, seed, rand });
}

module.exports = { renderNoise, NOISE_TYPES };
