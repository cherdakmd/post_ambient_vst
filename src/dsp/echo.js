// src/dsp/echo.js - Эхо «панельные дома» (ping-pong delay).
//
// Две линии задержки L и R, кросс-связанные обратными связями:
// сигнал «прыгает» между каналами, имитируя отражения между домами.

function applyEcho(left, right, sampleRate, delaySec, feedback = 0.45, mix = 0.35) {
  const delay = Math.max(1, Math.round(delaySec * sampleRate));
  const bufL = new Float32Array(delay);
  const bufR = new Float32Array(delay);
  let idx = 0;

  for (let i = 0; i < left.length; i++) {
    const tapL = bufL[idx];
    const tapR = bufR[idx];

    // Ping-pong: в линию L пишем исходный L + эхо из R, и наоборот
    bufL[idx] = left[i] + tapR * feedback;
    bufR[idx] = right[i] + tapL * feedback;

    left[i] = left[i] * (1 - mix) + tapL * mix;
    right[i] = right[i] * (1 - mix) + tapR * mix;

    idx++;
    if (idx >= delay) idx = 0;
  }
}

module.exports = { applyEcho };
