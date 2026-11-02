// post_ambient_vst - VST Plugin Prototype (JavaScript)
// Концепция: Пост-панк 2000-х, Юго-Запад СПб, Панельные дома, Шум, Басс, Атмосфера
//
// v1.2.0: мультитрековость (стемы), бас-буст, слайд нот, рандомайзер,
//         несколько типов шума (wind/radio/rain/hum/crackle/mixed).
//
// Пайплайн: дорожки (шум + басс + пад) -> «бетонный» фильтр -> дисторшн
//           -> эхо -> реверб -> стерео -> мастер.
// Каждую дорожку можно выключить и отрендерить отдельным файлом (стемом).

const fs = require('fs');
const path = require('path');

const { StateVariableFilter } = require('./dsp/filters');
const { renderNoise, NOISE_TYPES } = require('./dsp/noise');
const { renderBass } = require('./dsp/bass');
const { renderPad } = require('./dsp/pad');
const { applyBassBoost } = require('./dsp/eq');
const { applyEcho } = require('./dsp/echo');
const { renderReverb } = require('./dsp/reverb');
const { applyStereoWidth } = require('./dsp/stereo');
const { applyDistortion } = require('./dsp/distortion');
const { writeWav } = require('./dsp/wav');

const TWO_PI = Math.PI * 2;

class PostAmbientVST {
  constructor(overrides = {}) {
    this.name = "Post Ambient VST (Southwest St. Petersburg)";
    this.version = "1.2.0";
    this.style = "Post-Punk 2000s / Ambient / Noise / Bass";
    this.inspiration = "Panel houses and small three-story houses of Southwest St. Petersburg";

    // Настройки рендера
    this.sampleRate = 44100;
    this.duration = 16;   // секунд
    this.seed = 42;       // «посев» шума - воспроизводимый рендер

    // Включённые дорожки (стемы) - каждую можно выключить и рендерить отдельно
    this.tracks = {
      noise: true, // дорожка 1: шум (ветер/радио/дождь/гул/треск)
      bass: true,  // дорожка 2: холодный пост-панк бас
      pad: true,   // дорожка 3: холодный эмбиент-пад
    };

    // Параметры звука
    this.noiseLevel = 0.5;     // Шум (радиопомехи, ветер между домами)
    this.noiseType = 'mixed';  // Тип шума: wind | radio | rain | hum | crackle | mixed
    this.bassFrequency = 60;   // Басс (холодный пост-панк бас), Гц
    this.bassBoost = 0.5;      // Бас-буст (подчёркивание низа), 0..1
    this.glide = 0.35;         // Слайд нот (портаменто), 0..1
    this.reverbSize = 0.8;     // Атмосфера (панельные дома, эхо дворов), 0..1
    this.filterCutoff = 1000;  // Фильтры (бетонные стены), Гц

    // Дисторшн / модуляция / гранулярность
    this.distortion = 0.3;     // 0..1
    this.modulation = 0.2;     // 0..1
    this.granularity = 0.7;    // 0..1

    // Темп / резонанс / стерео / эхо
    this.tempo = 100;          // BPM
    this.resonance = 0.4;      // 0..1
    this.stereoEffect = 0.6;   // 0..1
    this.echoDelay = 1.2;      // сек

    // Применяем пользовательские настройки
    Object.assign(this, overrides);

    // Состояние
    this.isPlaying = false;
  }

  // Случайная переброска параметров («рандомайзер»).
  // Звук остаётся в жанре (пост-панк/эмбиент), но каждый раз - новая сцена.
  randomize() {
    const r = Math.random;
    this.seed = Math.floor(r() * 1000000);
    this.noiseType = NOISE_TYPES[Math.floor(r() * NOISE_TYPES.length)];
    this.noiseLevel = 0.3 + r() * 0.6;
    this.granularity = 0.2 + r() * 0.75;
    this.modulation = 0.1 + r() * 0.8;
    this.filterCutoff = 400 + r() * 1600;
    this.reverbSize = 0.4 + r() * 0.55;
    this.echoDelay = 0.5 + r() * 1.5;
    this.tempo = 80 + Math.floor(r() * 41);     // 80..120
    this.resonance = 0.2 + r() * 0.6;
    this.distortion = 0.1 + r() * 0.5;
    this.stereoEffect = 0.3 + r() * 0.65;
    this.bassBoost = 0.3 + r() * 0.6;
    this.glide = r() * 0.7;
    this.bassFrequency = [41.2, 43.65, 49, 55, 60, 65.4][Math.floor(r() * 6)]; // E1..C2
    return this;
  }

  // Рендер отдельных дорожек (стемов)
  renderStems() {
    const { sampleRate, duration } = this;
    const length = Math.round(sampleRate * duration);
    const stems = {};

    if (this.tracks.noise) {
      stems.noise = renderNoise({
        length,
        sampleRate,
        level: this.noiseLevel,
        granularity: this.granularity,
        modulation: this.modulation,
        seed: this.seed,
        type: this.noiseType,
      });
    }

    if (this.tracks.bass) {
      const bass = renderBass({
        length,
        sampleRate,
        rootHz: this.bassFrequency,
        bpm: this.tempo,
        resonance: this.resonance,
        drive: this.distortion,
        glide: this.glide,
      });
      applyBassBoost(bass, sampleRate, this.bassBoost);
      stems.bass = bass;
    }

    if (this.tracks.pad) {
      stems.pad = renderPad({
        length,
        sampleRate,
        rootHz: this.bassFrequency,
        bpm: this.tempo,
        seed: this.seed,
        modulation: this.modulation,
      });
    }

    return stems;
  }

  // Смешивание дорожек в стерео-пару (с учётом их включения)
  mixStems(stems) {
    const { sampleRate } = this;
    const anyNoise = !!stems.noise;
    const length = anyNoise
      ? stems.noise.length
      : stems.bass
        ? stems.bass.length
        : stems.pad.left.length;

    const left = new Float32Array(length);
    const right = new Float32Array(length);

    // Шум: микро-задержка Хааса для ранней стерео-картины
    if (stems.noise) {
      const delaySamples = Math.round((0.15 + this.stereoEffect * 0.35) * sampleRate * 0.004);
      for (let i = 0; i < length; i++) {
        const nR = stems.noise[Math.max(0, i - delaySamples)];
        left[i] += stems.noise[i];
        right[i] += nR;
      }
    }

    // Бас: моно на оба канала (так тяжелее)
    if (stems.bass) {
      for (let i = 0; i < length; i++) {
        left[i] += stems.bass[i] * 0.8;
        right[i] += stems.bass[i] * 0.8;
      }
    }

    // Пад: уже стерео
    if (stems.pad) {
      for (let i = 0; i < length; i++) {
        left[i] += stems.pad.left[i] * 1.1;
        right[i] += stems.pad.right[i] * 1.1;
      }
    }

    return { left, right };
  }

  // Основной рендер: дорожки -> микс -> цепочка эффектов -> мастер
  render() {
    return this.mixAndProcess(this.renderStems());
  }

  // Цепочка эффектов над миксом
  mixAndProcess(stems) {
    const { left, right } = this.mixStems(stems);

    this.applyConcreteFilter(left, right);
    this.applyDistortionPair(left, right);
    applyEcho(left, right, this.sampleRate, this.echoDelay, 0.4 + this.reverbSize * 0.2, 0.25 + this.distortion * 0.15);
    renderReverb(left, right, this.sampleRate, this.reverbSize);
    applyStereoWidth(left, right, this.stereoEffect);

    this.applyFade(left, right, this.sampleRate, 0.3);
    this.normalize(left, right, 0.85);

    return { left, right };
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

  // Рендер в WAV-файл (микс со всеми эффектами)
  renderToFile(filePath) {
    const { left, right, sampleRate } = this.render();
    ensureDir(filePath);
    fs.writeFileSync(filePath, writeWav([left, right], sampleRate));
    return filePath;
  }

  // Рендер дорожек отдельными файлами + микс.
  // Из 'output/post_ambient.wav' получатся:
  //   output/post_ambient.wav       - готовый микс (эффекты наложены)
  //   output/post_ambient_noise.wav - чистая дорожка шума
  //   output/post_ambient_bass.wav  - чистая дорожка баса
  //   output/post_ambient_pad.wav   - чистая дорожка пада
  renderStemsToFiles(filePath) {
    const stems = this.renderStems();
    const { left, right } = this.mixAndProcess(stems);

    ensureDir(filePath);
    fs.writeFileSync(filePath, writeWav([left, right], this.sampleRate));

    const files = { mix: filePath };
    const base = filePath.replace(/\.wav$/i, '');
    for (const [name, buf] of Object.entries(stems)) {
      const p = `${base}_${name}.wav`;
      const channels = buf instanceof Float32Array ? [buf] : [buf.left, buf.right];
      fs.writeFileSync(p, writeWav(channels, this.sampleRate));
      files[name] = p;
    }
    return files;
  }

  // Интерфейс (прототип)
  renderGUI() {
    const tracks = Object.entries(this.tracks)
      .filter(([, on]) => on)
      .map(([name]) => name)
      .join(' + ');
    console.log(`--- ${this.name} v${this.version} ---`);
    console.log(`Style: ${this.style}`);
    console.log(`Inspiration: ${this.inspiration}`);
    console.log(`Tracks: ${tracks || '(все дорожки выключены!)'}`);
    console.log(`Noise: type=${this.noiseType} level=${this.noiseLevel} | Granularity: ${this.granularity}`);
    console.log(`Bass: ${this.bassFrequency}Hz | Boost: ${this.bassBoost} | Glide: ${this.glide} | Distortion: ${this.distortion}`);
    console.log(`Pad: on=${this.tracks.pad} | Reverb: ${this.reverbSize} | Filter: ${this.filterCutoff}Hz | Modulation: ${this.modulation}`);
    console.log(`Tempo: ${this.tempo} BPM | Resonance: ${this.resonance} | Stereo: ${this.stereoEffect} | Echo: ${this.echoDelay}s`);
    console.log(`Render: ${this.duration}s @ ${this.sampleRate}Hz | Seed: ${this.seed}`);
    console.log(`-----------------------------------------------------------------------------------`);
  }
}

function ensureDir(filePath) {
  const dir = path.dirname(filePath);
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
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
