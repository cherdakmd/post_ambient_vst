// src/dsp/eq.js - Работа с частотным балансом дорожек.

const { StateVariableFilter } = require('./filters');

// Бас-буст: добавляем к сигналу его низкочастотную составляющую.
// «Подчёркивание» глубокого пост-панк баса (и общая «тяжеловесность» трека).
function applyBassBoost(buffer, sampleRate, amount, cutoffHz = 110) {
  if (amount <= 0) return;
  const filter = new StateVariableFilter(sampleRate);
  const gain = amount * 1.8;
  for (let i = 0; i < buffer.length; i++) {
    filter.tick(buffer[i], cutoffHz, 0.1);
    buffer[i] += filter.low * gain;
  }
  // Мягкий ограничитель - после буста не клиппим
  for (let i = 0; i < buffer.length; i++) {
    buffer[i] = Math.tanh(buffer[i]);
  }
}

module.exports = { applyBassBoost };
