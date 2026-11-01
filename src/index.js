// post_ambient_vst - VST Plugin Prototype (JavaScript)
// Концепция: Пост-панк 2000-х, Юго-Запад СПб, Панельные дома, Шум, Басс, Атмосфера
//
// v1.1.0: прототип теперь считает РЕАЛЬНЫЙ звук и рендерит WAV-файлы.
// Пайплайн: шум + басс -> «бетонный» фильтр -> дисторшн -> эхо -> реверб -> стерео -> мастер.

const fs = require('fs');
const path = require('path');

const { StateVariableFilter } = require('./dsp/filters');
const { renderNoise } = require('./dsp/noise');
const { renderBass } = require('./dsp/bass');
const { applyEcho } = require('./dsp/echo');
const { renderReverb } = require('./dsp/reverb');
const { applyStereoWidth } = require('./dsp/stereo');
const { applyDistortion } = require('./dsp/distortion');
const { writeWav } = require('./dsp/wav');

const TWO_PI = Math.PI * 2;

class PostAmbientVST {
  constructor(overrides = {}) {
    this.name = "Post Ambient VST (Southwest St. Petersburg)";
    this.version = "1.1.0";
    this.style = "Post-Punk 2000s / Ambient / Noise / Bass";
    this.inspiration = "Panel houses and small three-story houses of Southwest St. Petersburg";

    // Настройки рендера
    this.sampleRate = 44100;
    this.duration = 16;   // секунд
    this.seed = 42;       // «посев» шума - воспроизводимый рендер

    // Параметры звука
    this.noiseLevel = 0.5;     // Шум (радиопомехи, ветер между домами)
    this.bassFrequency = 60;   // Басс (холодный пост-панк бас), Гц
    this.reverbSize = 0.8;     // Атмосфера (панельные дома, эхо дворов), 0..1
    this.filterCutoff = 1000;  // Фильтры (бетонные стены), Гц

    // Новые функции (v1.0.1)
    this.distortion = 0.3;     // Дисторшн (пост-панк 2000-х), 0..1
    this.modulation = 0.2;     // Модуляция (ветер между домами), 0..1
    this.granularity = 0.7;    // Гранулярность шума (панельные дома), 0..1

    // Новые параметры (v1.0.2)
    this.tempo = 100;          // Темп (пост-панк ритм), BPM
    this.resonance = 0.4;      // Резонанс (холодный, бетонные стены), 0..1
    this.stereoEffect = 0.6;   // Стерео-эффект (дворы), 0..1
    this.echoDelay = 1.2;      // Эхо (панельные дома), сек

    // Применяем пользовательские настройки
    Object.assign(this, overrides);

    // Состояние
    this.isPlaying = false;
  }

  // Основной рендер: собирает все слои и обрабатывает их цепочкой эффектов
  render() {
    const { sampleRate, duration } = this;
    const length = Math.round(sampleRate * duration);

    // 1. Шум («ветер между домами», грануляция, радиопомехи)
    const noise = renderNoise({
      length,
      sampleRate,
      level: this.noiseLevel,
      granularity: this.granularity,
      modulation: this.modulation,
      seed: this.seed,
    });

    // 2. Басс («холодный» пост-панк рифф восьмыми)
    const bass = renderBass({
      length,
      sampleRate,
      rootHz: this.bassFrequency,
      bpm: this.tempo,
      resonance: this.resonance,
      drive: this.distortion,
    });

    // Микс шума и баса (левый и правый каналы слегка различаются - ранний стерео-эффект)
    const left = new Float32Array(length);
    const right = new Float32Array(length);
    const width = 0.15 + this.stereoEffect * 0.35;
    for (let i = 0; i < length; i++) {
      const delaySamples = Math.round(width * sampleRate * 0.004); // микро-задержка Хааса
      const nL = noise[i];
      const nR = noise[Math.max(0, i - delaySamples)];
      left[i] = nL * 0.9 + bass[i];
      right[i] = nR * 0.9 + bass[i];
    }

    // 3. Эффекты цепочкой
    this.applyConcreteFilter(left, right);
    this.applyDistortionPair(left, right);
    applyEcho(left, right, sampleRate, this.echoDelay, 0.4 + this.reverbSize * 0.2, 0.25 + this.distortion * 0.15);
    renderReverb(left, right, sampleRate, this.reverbSize);
    applyStereoWidth(left, right, this.stereoEffect);

    // 4. Мастер: плавное появление/затухание и нормировка пика
    this.applyFade(left, right, sampleRate, 0.3);
    this.normalize(left, right, 0.85);

    return { left, right, sampleRate };
  }

  // «Бетонные стены»: резонансный фильтр с ветровой модуляцией частоты среза
  applyConcreteFilter(left, right) {
    const fL = new StateVariableFilter(this.sampleRate);
    const fR = new StateVariableFilter(this.sampleRate);
    const mod = this.modulation;
    for (let i = 0; i < left.length; i++) {
      const t = i / this.sampleRate;
      // Два LFO с разными периодами - «ветер» не повторяется по кругу
      const lfo =
        1 +
        mod * 0.45 * Math.sin(TWO_PI * 0.11 * t) +
        mod * 0.2 * Math.sin(TWO_PI * 0.037 * t + 1.7);
      const cutoff = Math.min(Math.max(this.filterCutoff * lfo, 60), this.sampleRate * 0.45);

      fL.tick(left[i], cutoff, this.resonance);
      fR.tick(right[i], cutoff, this.resonance);
      left[i] = fL.low + fL.band * 0.15;
      right[i] = fR.low + fR.band * 0.15;
    }
  }

  // Дисторшн применяется к обоим каналам одинаково (моно-искажение, как ламповый усилитель)
  applyDistortionPair(left, right) {
    // Сначала общая моно-составляющая, чтобы искажение было «коллективным»
    const mono = new Float32Array(left.length);
    for (let i = 0; i < left.length; i++) mono[i] = (left[i] + right[i]) * 0.5;
    applyDistortion(mono, this.distortion);
    const mix = 0.6;
    for (let i = 0; i < left.length; i++) {
      left[i] = left[i] * (1 - mix) + mono[i] * mix;
      right[i] = right[i] * (1 - mix) + mono[i] * mix;
    }
  }

  // Плавные начала и концы - без щелчков в DAW
  applyFade(left, right, sampleRate, fadeSec) {
    const n = Math.min(Math.round(sampleRate * fadeSec), Math.floor(left.length / 2));
    for (let i = 0; i < n; i++) {
      const g = 0.5 - 0.5 * Math.cos((Math.PI * i) / n);
      left[i] *= g;
      right[i] *= g;
      const j = left.length - 1 - i;
      left[j] *= g;
      right[j] *= g;
    }
  }

  // Нормировка по пику - громкость без клиппинга
  normalize(left, right, peak) {
    let max = 0;
    for (let i = 0; i < left.length; i++) {
      const a = Math.abs(left[i]);
      const b = Math.abs(right[i]);
      if (a > max) max = a;
      if (b > max) max = b;
    }
    if (max === 0) return;
    const gain = peak / max;
    for (let i = 0; i < left.length; i++) {
      left[i] *= gain;
      right[i] *= gain;
    }
  }

  // Рендер в WAV-файл
  renderToFile(filePath) {
    const { left, right, sampleRate } = this.render();
    const dir = path.dirname(filePath);
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(filePath, writeWav([left, right], sampleRate));
    return filePath;
  }

  // Интерфейс (прототип)
  renderGUI() {
    console.log(`--- ${this.name} v${this.version} ---`);
    console.log(`Style: ${this.style}`);
    console.log(`Inspiration: ${this.inspiration}`);
    console.log(`Noise Level: ${this.noiseLevel} | Granularity: ${this.granularity}`);
    console.log(`Bass Frequency: ${this.bassFrequency}Hz | Distortion: ${this.distortion}`);
    console.log(`Reverb Size: ${this.reverbSize} | Filter: ${this.filterCutoff}Hz | Modulation: ${this.modulation}`);
    console.log(`Tempo: ${this.tempo} BPM | Resonance: ${this.resonance} | Stereo: ${this.stereoEffect} | Echo: ${this.echoDelay}s`);
    console.log(`Render: ${this.duration}s @ ${this.sampleRate}Hz | Seed: ${this.seed}`);
    console.log(`-----------------------------------------------------------------------------------`);
  }
}

// API для будущего VST: обработка готового буфера той же цепочкой эффектов
PostAmbientVST.prototype.process = function (left, right) {
  this.applyConcreteFilter(left, right);
  this.applyDistortionPair(left, right);
  applyEcho(left, right, this.sampleRate, this.echoDelay, 0.4 + this.reverbSize * 0.2, 0.25 + this.distortion * 0.15);
  renderReverb(left, right, this.sampleRate, this.reverbSize);
  applyStereoWidth(left, right, this.stereoEffect);
  return { left, right };
};

module.exports = PostAmbientVST;
