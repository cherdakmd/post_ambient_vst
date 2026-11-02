# post_ambient_vst - VST Plugin for FL Studio 🎸🏢🌫️

> **«Звук Юго-Запада Санкт-Петербурга в стиле пост-панк 2000-х»**

## 📸 Вдохновение
- **Фотографии дома:** [image-1.png](https://github.com/cherdakmd/post_ambient_vst/blob/arena/e1b75971-post-ambient-vst/image-1.png) | [image-2.png](https://github.com/cherdakmd/post_ambient_vst/blob/arena/e1b75971-post-ambient-vst/image-2.png)
- **Видео с Юго-Запада СПб:** [YouTube](https://www.youtube.com/watch?v=x6oa72m8kmg&list=RDx6oa72m8kmg&start_radio=1) (ветер между панельками, эхо дворов, холодный бас).
- **Музыкальный стиль:** Пост-панк 2000-х (Кино, Molchat Doma, Сплин), эмбиент, шум, басс.

## 🎧 Концепция
VST-плагин для **FL Studio**, генерирующий эмбиент-звуковые ландшафты и атмосферу.

**Основа звука:**
- **Шум:** гранулированный шум, имитация «радиопомех» и «ветра между домами» (Юго-Запад СПб).
- **Басс:** глубокий, «холодный» пост-панк бас.
- **Атмосфера:** реверберация (панельные дома, эхо дворов), фильтры (бетонные стены).

**Новые параметры (v1.0.1+):**
- **Дисторшн:** пост-панк 2000-х (искажение звука, «бетонные стены»).
- **Модуляция:** ветер между домами (изменение частоты, «радиопомехи»).
- **Гранулярность:** панельные дома (разбиение шума на части).
- **Темп (BPM):** 80–120 (пост-панк ритм).
- **Резонанс:** «холодный» (имитация бетонных стен).
- **Стерео-эффект:** «дворы» (расширение звука).
- **Эхо:** «панельные дома» (задержка, 0.5–2 сек).

**Мультитрековость и новые фичи (v1.2.0+):**
- **Дорожки (стемы):** три независимые дорожки — шум, бас, пад. Каждую можно выключить (`--tracks`) и отрендерить отдельным WAV-файлом (`--stems 1`) для своего микширования в FL Studio.
- **Типы шума:** `wind` (ветер), `radio` (помехи), `rain` (дождь по бетону), `hum` (гул дома 50 Гц), `crackle` (скрип и треск), `mixed` (смесь Юго-Запада).
- **Бас-буст:** подчёркивание глубокого низа (0–1).
- **Слайд нот (glide):** портаменто баса — ноты «плывут» друг в друга.
- **Рандомайзер (`--random`):** каждый запуск — новая случайная сцена в жанре (случайный шум, темп, эффекты, seed).

**Визуализация:**
- Темный дизайн, «брутализм», минимализм, шрифты в стиле плакатов 2000-х, визуализация «бетона» и «серого неба».

## 🛠️ Технологии
- **Язык:** **JavaScript/Node.js** (прототип DSP и рендера WAV), **Rust** (VST3/CLAP плагин).
- **Фреймворк:** [`nih-plug`](https://github.com/robbert-vdh/nih-plug) (Rust, VST3 + CLAP), `egui` (интерфейс, планы).
- **Прототип (v1.2.0):** считает **реальный звук** (DSP) и рендерит стерео WAV-файлы без сторонних зависимостей.
- **Плагин:** собирается в `post_ambient_vst.vst3` для FL Studio.

## 🧩 VST3 для FL Studio (Rust)

### Сборка (один раз)
Нужно: [Rust (rustup)](https://rustup.rs/), Visual Studio Build Tools 2022 (workload «C++ build tools»), CMake.

```bash
cargo xtask bundle post_ambient_vst --release
```

Готовый плагин: `target/bundled/post_ambient_vst.vst3`

### Установка в FL Studio
1. Скопируйте `post_ambient_vst.vst3` в `C:\Program Files\Common Files\VST3\`
2. FL Studio → `Options → File → Plugin search list → Add plugin folder` (или Manage plugins)
3. Нажмите `Fast scan` — плагин появится как **Post Ambient VST** (инструмент)
4. Добавьте его в Channel rack и отправляйте на любой mixer-канал

### Параметры плагина (автоматизация в FL)
- **Дорожки:** Noise (вкл/выкл), Bass, Pad — гейты отдельных слоёв
- **Шум:** Noise Type (Wind/Radio/Rain/Hum/Crackle/Mixed), Noise Level, Granularity, Modulation, Seed (рандомайзер)
- **Бас:** Bass Freq, Bass Boost, Glide (слайд нот), Distortion
- **Атмосфера:** Filter Cutoff, Resonance, Echo Delay, Reverb Size, Stereo
- **Темп:** Sync Tempo to Host (BPM из FL) или ручной Tempo 60–160
- **Выход:** Output (dB)

Совет для стемов: поставьте 3 инстанса плагина — на одном только Noise, на втором только Bass, на третьем только Pad — и микшируйте в разных mixer-каналах FL.

## 📂 Структура проекта
- `package.json` — конфигурация проекта.
- `src/index.js` — основной класс `PostAmbientVST`: параметры, дорожки (стемы), микс, цепочка эффектов, рендер в WAV.
- `src/render.js` — CLI: рендер трека с настройкой параметров из командной строки.
- `src/dsp/filters.js` — state-variable фильтр («бетонные стены») с плавной модуляцией.
- `src/dsp/noise.js` — 6 типов шума: wind, radio, rain, hum, crackle, mixed.
- `src/dsp/bass.js` — секвенсор «холодного» пост-панк баса со слайдом нот (glide).
- `src/dsp/pad.js` — холодный эмбиент-пад (минорная прогрессия расстроенных пил).
- `src/dsp/eq.js` — бас-буст (подчёркивание низа).
- `src/dsp/echo.js` — ping-pong эхо «панельные дома».
- `src/dsp/reverb.js` — Schroeder-реверберация «эхо дворов» (комб-фильтры + all-pass).
- `src/dsp/distortion.js` — мягкий волновой дисторшн (tanh).
- `src/dsp/stereo.js` — расширение стерео Mid/Side («дворы»).
- `src/dsp/wav.js` — запись 16-bit PCM WAV (без зависимостей).
- `src/dsp/rng.js` — детерминированный генератор шума (воспроизводимый рендер).
- `src/dsp/super_atmospheric/` — автономное «супер атмосферное» DSP-ядро на Rust
  (12 инноваций + 12 улучшений, без внешних крейтов).
- `src/dsp/*.rs` (bass/echo/filters/noise/pad/reverb/rng) — DSP плагина VST3 v3.0.0.
- `src/sample.rs` — сэмплер (WAV/MP3/FLAC/OGG) на `symphonia`.
- `xtask/` — бандлер VST3 (`cargo xtask bundle post_ambient_vst --release`).
- `tools/dsp_standalone/` — проверка «супер атмосферного» ядра без сети (`cargo test`, рендер WAV).
- `tools/analyze_bands.py` — спектральный анализ рендера (низ/середина/верх).
- `scripts/install_rust_offline.sh` — установка `rustc`/`cargo` без `static.rust-lang.org`.
- `presets/super_atmospheric_v1_1_1.json` — пресеты для FL Studio.
- `gui/config_brutalist_grey_sky.json` — бруталистская тема «серое небо».
- `docs/BUILD_OFFLINE.md` — сборка онлайн и офлайн (`cargo vendor`).
- `README.md` — документация (эта страница).

## 🚀 Разработка (Прототип на Node.js / JavaScript)
Прототип на Node.js реализует **полный DSP-пайплайн**: дорожки (шум + басс + пад) → «бетонный» фильтр → дисторшн → эхо → реверб → стерео-расширение → мастер (нормировка, фейды).

### Как запустить прототип
1. Установите зависимости: `npm install` (зависимостей нет, но так обновится lock-файл)
2. Отрендерите трек: `npm run dev` → `output/post_ambient.wav`

### Примеры рендера
```bash
node src/render.js --random                          # случайная сцена в жанре
node src/render.js --stems 1 --out output/mix.wav    # микс + отдельные дорожки (_noise, _bass, _pad)
node src/render.js --tracks noise,bass               # только шум и бас (без пада)
node src/render.js --noise-type rain --bass-boost 0.9 --glide 0.8
node src/render.js --tempo 110 --echo 1.8 --duration 30 --seed 7
```
Доступные флаги: `--out --duration --seed --random --tracks noise,bass,pad --stems 1 --noise-type wind|radio|rain|hum|crackle|mixed --noise --bass --bass-boost --glide --reverb --filter --distortion --modulation --granularity --tempo --resonance --stereo --echo`.

### Использование в FL Studio
1. Отрендерите WAV (см. выше).
2. Перетащите файл в плейлист FL Studio или загрузите в сэмплер (Fruity Slicer, Granulizer) для эмбиент-сцен.
3. В будущем (Rust): `cargo build --release` → `.vst3` → `Options -> Plugins -> Manage plugins`.

### Как использовать в FL Studio (в будущем)
1. Соберите плагин на Rust: `cargo build --release` (требуется установка Rust/vst-rs).
2. Найдите скомпилированный `.dll` (Windows) или `.vst3` (macOS/Linux) в `target/release`.
3. Добавьте плагин в FL Studio (`Options -> Plugins -> Manage plugins`).
4. Используйте плагин для генерации эмбиент-звуков и атмосферы.

## 🎛️ «Супер атмосферное» DSP-ядро на Rust (v1.2.0)

Рядом с JS-прототипом и VST3-плагином живёт автономное DSP-ядро
`src/dsp/super_atmospheric/` — 12 инноваций и 12 улучшений, реализованные
по-настоящему (обработка сэмплов, а не `println!`-заглушки) и **без единого внешнего крейта**:

| Модуль | Что делает |
|---|---|
| `noise.rs` | гранулярный шум: два зерна с перекрытием Ханна 50%, чтение из кольцевого буфера; гул + ВЧ-«воздух»; ветровой гейт с порывами 0.7–2.3 с |
| `bass.rs` | басс 45 Гц: тон + суб-октава + «бетонная» примесь, пульс в такт 90 BPM |
| `filter.rs` | однополюсный «бетонный» ФНЧ + резонансный SVF (Chamberlin); 800 Гц → 1480 Гц спектральным сдвигом |
| `distortion.rs` | `tanh`-клиппинг (драйв ×8.2) + нечётная «бетонная» примесь + лимитер |
| `modulation.rs` | LFO 0.375 Гц, синхронный темпу |
| `tempo.rs` | задержки, синхронные доле (четверть 0.667 с, с точкой 1.0 с) |
| `resonance.rs` | 3 моды панели (62/187/431 Гц) |
| `stereo.rs` | M/S-расширение «дворов» |
| `echo.rs` | линия задержки 2.5 с, окрашенная обратная связь |
| `reverb.rs` | Schroeder/Freeverb: 4 comb + 2 allpass на канал |

Цепочка: `вход → шум/басс → дисторшн → фильтр «бетонной стены» → резонансы панелей →
LFO ветра → эхо панельного дома → реверб-ландшафт → стерео → лимитер`,
точка входа — `SuperAtmosphericEngine::process(left, right)`.

Проверяется без зависимостей и без сети:

```bash
cd tools/dsp_standalone
cargo test             # 43 теста
cargo run --release    # 8 с стерео → output/post_ambient_super_atmospheric.wav
python3 ../../tools/analyze_bands.py ../../output/post_ambient_super_atmospheric.wav
# peak 0.88 | RMS 0.26 | clipped 0 | low 0.193 | mid 0.126 | high 0.093
```

## 🦀 Установка Rust и офлайн-сборка

Если `static.rust-lang.org` / `sh.rustup.rs` недоступны (закрытая сеть):

```bash
./scripts/install_rust_offline.sh     # rustc/cargo 1.88.0 из npm-пакетов @rustbin/*
```

Плагину нужен `crates.io` (`nih-plug` + `symphonia` + `rfd` — около 200 крейтов);
рецепт офлайн-сборки через `cargo vendor` — в `docs/BUILD_OFFLINE.md`.
«Супер атмосферное» ядро (`tools/dsp_standalone`) собирается и тестируется вообще без сети.

## 🌐 Публикация на GitHub
- **Репозиторий:** [https://github.com/cherdakmd/post_ambient_vst](https://github.com/cherdakmd/post_ambient_vst)
- **Ветка:** `arena/e1b75971-post-ambient-vst`
- **Pull Request:** [https://github.com/cherdakmd/post_ambient_vst/pull/1](https://github.com/cherdakmd/post_ambient_vst/pull/1) (влит)
- **Коммит:** `05b82d73ef5e40070ec2ddb2184bfab99bd35be7` (базовый), текущие изменения в ветке.
- **Ветка разработки:** `arena/139069db-post-ambient-vst` (PR #3) — «супер атмосферное» ядро v1.2.0.

## 📜 Лицензия
MIT (или любая другая, по выбору автора).

## 🛠️ Как добавить новые параметры (для разработчиков)
1. Откройте `src/index.js`.
2. Добавьте новый параметр в `constructor()` (например, `this.tempo = 100`).
3. Добавьте функцию обработки в `process()` (например, `this.applyTempo()`).
4. Обновите `renderGUI()`.
5. Запустите `npm run dev`.
6. Сделайте коммит и пуш в ветку `arena/e1b75971-post-ambient-vst`.

---
*Создано с любовью к Юго-Западу СПб и пост-панку 2000-х.*
