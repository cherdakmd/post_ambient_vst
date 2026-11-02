// src/dsp/rng.js - Детерминированный генератор случайных чисел.
//
// Нужен, чтобы рендер был воспроизводимым: одинаковые настройки
// всегда дают одинаковый WAV-файл (важно для отладки и для «посева»).

function mulberry32(seed) {
  let s = seed | 0;
  return function () {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

module.exports = { mulberry32 };
