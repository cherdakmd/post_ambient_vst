// src/dsp/reverb.js - Реверберация «эхо дворов» (Schroeder reverb).
//
// Классическая схема: 6 параллельных комб-фильтров с демпфированием
// (имитация длинных плоских стен панельных домов) + 2 последовательных
// all-pass'а для густоты. Размер двора (size) управляет временем
// затухания и обратной связью.

class Comb {
  constructor(delaySamples, feedback, damping) {
    this.buf = new Float32Array(delaySamples);
    this.idx = 0;
    this.feedback = feedback;
    this.damping = damping;
    this.store = 0;
  }

  process(input) {
    const out = this.buf[this.idx];
    // Демпфирование: высокие частоты гаснут быстрее низких («бетон глушит»)
    this.store = out * (1 - this.damping) + this.store * this.damping;
    this.buf[this.idx] = input + this.store * this.feedback;
    this.idx++;
    if (this.idx >= this.buf.length) this.idx = 0;
    return out;
  }
}

class AllPass {
  constructor(delaySamples, feedback = 0.5) {
    this.buf = new Float32Array(delaySamples);
    this.idx = 0;
    this.feedback = feedback;
  }

  process(input) {
    const bufOut = this.buf[this.idx];
    const output = -input + bufOut;
    this.buf[this.idx] = input + bufOut * this.feedback;
    this.idx++;
    if (this.idx >= this.buf.length) this.idx = 0;
    return output;
  }
}

// Задержки комб-фильтров в сэмплах (при 44.1 кГц) - «дворообразные» промежутки
const COMB_DELAYS = [1116, 1188, 1277, 1356, 1422, 1491];
const ALLPASS_DELAYS = [225, 556];
// Стерео-смещение правого канала (сэмплы) - разные «дворы» слева и справа
const STEREO_SPREAD = 23;

function renderReverb(left, right, sampleRate, size) {
  const scale = 0.7 + size * 0.6;             // масштаб времени затухания
  const feedback = 0.7 + size * 0.17;         // чем больше «двор», тем дольше эхо
  const damping = 0.35;
  const wet = 0.22 + size * 0.4;              // доля реверберации

  const combsL = COMB_DELAYS.map((d) => new Comb(Math.round(d * scale), feedback, damping));
  const combsR = COMB_DELAYS.map((d) => new Comb(Math.round((d + STEREO_SPREAD) * scale), feedback, damping));
  const apsL = ALLPASS_DELAYS.map((d) => new AllPass(Math.round(d * scale)));
  const apsR = ALLPASS_DELAYS.map((d) => new AllPass(Math.round((d + STEREO_SPREAD) * scale)));

  for (let i = 0; i < left.length; i++) {
    const inL = left[i];
    const inR = right[i];

    let accL = 0;
    let accR = 0;
    for (let c = 0; c < combsL.length; c++) {
      accL += combsL[c].process(inL);
      accR += combsR[c].process(inR);
    }

    for (let a = 0; a < apsL.length; a++) {
      accL = apsL[a].process(accL);
      accR = apsR[a].process(accR);
    }

    left[i] = inL * (1 - wet) + accL * wet * 0.3;
    right[i] = inR * (1 - wet) + accR * wet * 0.3;
  }
}

module.exports = { renderReverb };
