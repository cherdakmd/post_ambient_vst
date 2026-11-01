// src/render.js - Командная строка для рендера трека.
//
// Примеры:
//   npm run dev
//   node src/render.js --random                      # случайная сцена
//   node src/render.js --stems 1 --out output/mix.wav  # микс + отдельные дорожки
//   node src/render.js --tracks noise,bass --noise-type rain --glide 0.8 --bass-boost 0.9
//   node src/render.js --tempo 110 --echo 1.8 --duration 30

const fs = require('fs');
const PostAmbientVST = require('./index');
const { NOISE_TYPES } = require('./dsp/noise');

// Разбор простых аргументов вида --ключ значение
function parseArgs(argv) {
  const args = {};
  for (let i = 2; i < argv.length; i++) {
    const m = argv[i].match(/^--([\w-]+)$/);
    if (m && i + 1 < argv.length && !argv[i + 1].startsWith('--')) {
      args[m[1]] = argv[i + 1];
      i++;
    } else if (m) {
      args[m[1]] = '1'; // флаг без значения
    }
  }
  return args;
}

const args = parseArgs(process.argv);

const num = (v, fallback) => (v !== undefined && v !== '' && isFinite(Number(v)) ? Number(v) : fallback);

const overrides = {
  duration: num(args.duration, undefined),
  seed: num(args.seed, undefined),
  noiseLevel: num(args.noise, undefined),
  noiseType: args['noise-type'] !== undefined ? String(args['noise-type']) : undefined,
  bassFrequency: num(args.bass, undefined),
  bassBoost: num(args['bass-boost'], undefined),
  glide: num(args.glide, undefined),
  reverbSize: num(args.reverb, undefined),
  filterCutoff: num(args.filter, undefined),
  distortion: num(args.distortion, undefined),
  modulation: num(args.modulation, undefined),
  granularity: num(args.granularity, undefined),
  tempo: num(args.tempo, undefined),
  resonance: num(args.resonance, undefined),
  stereoEffect: num(args.stereo, undefined),
  echoDelay: num(args.echo, undefined),
};
// Убираем пустые значения, чтобы не затереть дефолты
Object.keys(overrides).forEach((k) => overrides[k] === undefined && delete overrides[k]);

// Включение/выключение дорожек: --tracks noise,bass (по умолчанию все включены)
if (args.tracks !== undefined) {
  const wanted = String(args.tracks)
    .split(',')
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
  const known = ['noise', 'bass', 'pad'];
  const unknown = wanted.filter((t) => !known.includes(t));
  if (unknown.length) {
    console.error(`[Error] Неизвестные дорожки: ${unknown.join(', ')}. Доступные: ${known.join(', ')}`);
    process.exit(1);
  }
  overrides.tracks = Object.fromEntries(known.map((t) => [t, wanted.includes(t)]));
}

const plugin = new PostAmbientVST(overrides);

// Рандомайзер: случайная сцена в жанре (перетирает большинство параметров)
if (args.random) {
  plugin.randomize();
}

if (plugin.noiseType && !NOISE_TYPES.includes(plugin.noiseType)) {
  console.error(`[Error] Неизвестный тип шума: ${plugin.noiseType}. Доступные: ${NOISE_TYPES.join(', ')}`);
  process.exit(1);
}

plugin.renderGUI();

const outPath = args.out || 'output/post_ambient.wav';
const t0 = Date.now();

let files;
if (args.stems) {
  // Отдельные дорожки + микс
  files = plugin.renderStemsToFiles(outPath);
} else {
  // Только готовый микс
  files = { mix: plugin.renderToFile(outPath) };
}

const ms = Date.now() - t0;

for (const [name, p] of Object.entries(files)) {
  const size = (fs.statSync(p).size / 1024 / 1024).toFixed(1);
  console.log(`[Render] ${name}: ${p} (${size} МБ)`);
}
console.log(`[Render] Готово за ${ms} мс (${plugin.duration} с аудио)`);
console.log(`[Hint] Стемы можно загрузить в разные каналы FL Studio и миксовать со своими эффектами.`);
