// src/dsp/filters.js - Фильтры «бетонные стены»
//
// Основной фильтр проекта - State-Variable Filter (Chamberlin SVF).
// Главное свойство: частота среза и резонанс можно менять КАЖДЫЙ сэмпл,
// что позволяет делать «ветровые» модуляции без щелчков.

class StateVariableFilter {
  constructor(sampleRate) {
    this.sampleRate = sampleRate;
    this.reset();
  }

  reset() {
    this.low = 0;   // низкочастотный выход («холодный» бетонный низ)
    this.band = 0;  // полосовой выход (громче с резонансом)
    this.high = 0;  // высокочастотный выход
  }

  // Продвинуть один сэмпл через фильтр.
  // cutoffHz   - частота среза (Гц), resonance - 0..1
  tick(input, cutoffHz, resonance) {
    // Ограничиваем частоту среза для устойчивости схемы
    const nyquist = this.sampleRate * 0.45;
    const fc = Math.max(20, Math.min(cutoffHz, nyquist));
    const f = 2 * Math.sin((Math.PI * fc) / this.sampleRate);
    const q = 1 - resonance * 0.98;

    const high = input - this.low - q * this.band;
    this.band += f * high;
    this.low += f * this.band;

    // Страховка от «взрыва» фильтра на резонансных частотах
    if (!isFinite(this.low) || !isFinite(this.band) || Math.abs(this.low) > 1e6) {
      this.reset();
    }
  }
}

module.exports = { StateVariableFilter };
