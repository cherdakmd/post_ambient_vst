// src/dsp/bass.js - «Холодный» пост-панк бас со слайдом нот.
//
// Секвенсор восьмыми нотами (темп BPM) играет рифф в минорной
// пентатонике: пила + суб-октава, фильтр с огибающей и драйвом.
// Параметр `glide` (0..1) добавляет портаменто - ноты «плывут»
// друг в друга, как на струне.

const { StateVariableFilter } = require('./filters');

const TWO_PI = Math.PI * 2;

// Стабильный пост-панк рифф (полутонов относительно корня)
const PATTERN = [0, 0, 7, 0, 5, 3, 7, 3];

function renderBass({ length, sampleRate, rootHz, bpm, resonance, drive, glide = 0 }) {
  const out = new Float32Array(length);
  const filter = new StateVariableFilter(sampleRate);

  const step = 60 / bpm / 2; // длительность восьмой ноты (с)
  const attack = 0.004;      // быстрый атак (щипок струны)
  const decay = 0.09;        // спад к сустейну

  let phase = 0;
  let subPhase = 0;
  let noteIndex = 0;
  let tInNote = 0;
  let curFreq = rootHz;
  let targetFreq = rootHz;

  const driveGain = 1 + drive * 2.5;
  // Коэффициент плавного «плыния» частоты к цели (экспоненциальный подход)
  const glideTime = Math.max(0.005, glide * step * 0.9);
  const glideCoef = glide > 0 ? 1 - Math.exp(-1 / (glideTime * sampleRate)) : 1;

  for (let i = 0; i < length; i++) {
    // Переход к следующей ноте
    if (tInNote >= step) {
      tInNote -= step;
      noteIndex++;
      targetFreq = rootHz * Math.pow(2, (PATTERN[noteIndex % PATTERN.length] % 12) / 12);
    }

    // Слайд: частота плавно «плывёт» к новой ноте
    curFreq += (targetFreq - curFreq) * glideCoef;

    // Огибающая: атак -> спад -> сустейн -> короткий релиз перед следующей нотой
    let env;
    if (tInNote < attack) {
      env = tInNote / attack;
    } else if (tInNote < decay) {
      env = 1 - 0.35 * ((tInNote - attack) / (decay - attack));
    } else {
      env = 0.65;
    }
    const releaseStart = step * 0.85;
    if (tInNote > releaseStart) {
      env *= 1 - (tInNote - releaseStart) / (step - releaseStart);
    }

    // Осцилляторы (частота берётся из «плывущей» curFreq - отсюда и слайд саба)
    phase += curFreq / sampleRate;
    if (phase >= 1) phase -= 1;
    subPhase += (curFreq * 0.5) / sampleRate; // суб-октава вниз
    if (subPhase >= 1) subPhase -= 1;

    const saw = 2 * phase - 1;
    const sub = Math.sin(TWO_PI * subPhase);
    let s = saw * 0.5 + sub * 0.8;

    // Фильтр с огибающей и резонансом («бетонные стены»)
    const cutoff = 90 + env * (450 + resonance * 1600);
    filter.tick(s, cutoff, 0.25 + resonance * 0.55);
    s = filter.low;

    // Лёгкий драйв - «сырость» пост-панка
    s = Math.tanh(s * driveGain) / Math.tanh(driveGain);

    out[i] = s * env * 0.55;
    tInNote += 1 / sampleRate;
  }

  return out;
}

module.exports = { renderBass };
