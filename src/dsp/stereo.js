// src/dsp/stereo.js - Стерео-эффект «дворы».
//
// Расширение стерео-картины в пространстве Mid/Side: середина (Mid)
// остаётся на месте, бока (Side) усиливаются - звук «расходится» между домами.

function applyStereoWidth(left, right, amount) {
  const gain = 1 + amount * 1.3;
  for (let i = 0; i < left.length; i++) {
    const mid = (left[i] + right[i]) * 0.5;
    const side = (left[i] - right[i]) * 0.5 * gain;
    left[i] = mid + side;
    right[i] = mid - side;
  }
}

module.exports = { applyStereoWidth };
