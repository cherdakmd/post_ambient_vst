// src/render.js - Командная строка для рендера трека.
//
// Примеры:
//   npm run dev
//   node src/render.js --out output/sw_spb.wav --duration 30 --tempo 110 --echo 1.8

const PostAmbientVST = require('./index');

// Разбор простых аргументов вида --ключ значение
function parseArgs(argv) {
  const args = {};
  for (let i = 2; i < argv.length; i++) {
    const m = argv[i].match(/^--([\w-]+)$/);
    if (m && i + 1 < argv.length) {
      args[m[1]] = argv[i + 1];
      i++;
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
  bassFrequency: num(args.bass, undefined),
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

const plugin = new PostAmbientVST(overrides);
plugin.renderGUI();

const outPath = args.out || 'output/post_ambient.wav';
const t0 = Date.now();
plugin.renderToFile(outPath);
const ms = Date.now() - t0;

const size = (require('fs').statSync(outPath).size / 1024 / 1024).toFixed(1);
console.log(`[Render] Готово: ${outPath} (${plugin.duration} c, ${size} МБ) за ${ms} мс`);
console.log(`[Hint] Откройте файл в FL Studio: перетащите WAV в плейлист или используйте как сэмпл-источник.`);
