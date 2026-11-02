# Руководство по ручному слиянию PR #3 (v1.1.1 Super Atmospheric)
## Конфликт: JavaScript-прототип (v1.1.1) vs Rust-VST3 (v3.0.0)

### Цель
Синхронизировать «супер атмосферную» логику из `arena/139069db-post-ambient-vst` (JS) в `main` (Rust) с учетом 12 нововведений и 12 улучшений.

### 1. Параметры для переноса в Rust (`src/dsp/*.rs`)
В Rust-коде (`src/dsp/bass.rs`, `src/dsp/noise.rs`, `src/dsp/reverb.rs` и др.) необходимо обновить константы и структуры согласно `presets/super_atmospheric_v1_1_1.json`:

- **Шум:** `noise_level: 0.8`, `granularity: 0.9`, `southwest_wind: 0.75` (Нововведение: SW Wind Gate, Улучшение: Гранулярность).
- **Басс:** `frequency: 45.0`, `distortion: 0.6`, `brutalist_texture: 0.95`, `tempo: 90.0` (Нововведение: Post-Punk Sync, Улучшение: Оптимизация басса).
- **Атмосфера:** `reverb_size: 0.95`, `filter_cutoff: 800.0`, `modulation: 0.5`, `spectral_shift: 0.85` (Нововведение: Concrete Wall Filter, Улучшение: Реверберация).
- **Эхо/Резонанс:** `echo_delay: 2.5`, `panel_echo_depth: 3.0`, `resonance: 0.7` (Нововведение: Panel Echo Resonance, Улучшение: Эхо-Панелей).
- **Стерео:** `stereo_effect: 0.9` (Нововведение: Courtyard Stereo Expansion, Улучшение: Стерео-Эффект).

### 2. Интерфейс (GUI - `src/editor.rs` / `egui`)
- **Красивее интерфейс:** Применить «бруталистскую» палитру («бетон», «серое небо»).
- **Шрифты:** Использовать файлы из `fonts/` (Display.ttf, Mono-Bold.ttf, Mono-Regular.ttf) — вдохновение/GUI.
- **Визуализация:** Добавить элементы «Юго-Запад СПб» (фото/видео ссылки в README).
- **12 Нововведений в GUI:**
  1. SW Wind Gate (переключатель)
  2. Brutalist Texture Mapping (слайдер)
  3. Panel Echo Resonance (кнопка)
  4. Post-Punk Sync (BPM-синхро)
  5. Courtyard Stereo Expansion (регулятор)
  6. Granular Noise Storm (индикатор)
  7. Concrete Wall Filter (кривая)
  8. Wind Dynamics Modulation (LFO)
  9. Post-Punk Distortion Engine (драйв)
  10. Atmospheric Reverb Landscape (размер)
  11. Grey Sky GUI (тема)
  12. FL Studio Preset Bridge (импорт JSON)

### 3. Пресеты для FL Studio
Импортировать параметры из `presets/super_atmospheric_v1_1_1.json` в Rust-структуру `PluginPreset`.

### 4. Шаги ручного слияния (opencode)
1. В `main` (Rust) создать структуру `SuperAtmosphericParams` с полями из JS-прототипа v1.1.1.
2. Реализовать функции обработки (аналог `process` в `src/index.js`):
   - `generate_noise` (SW Wind Gate, Granular Storm)
   - `generate_bass` (Cold Post-Punk, Brutalist Texture)
   - `apply_atmosphere` (Concrete Wall, Spectral Shift)
   - `apply_distortion` (Post-Punk Engine)
   - `apply_modulation` (Wind Dynamics)
   - `apply_tempo` (Post-Punk Sync)
   - `apply_resonance` (Cold Resonance)
   - `apply_stereo` (Courtyard Expansion)
   - `apply_echo` (Panel Echo Depth)
3. Обновить `src/lib.rs` для экспорта VST3 с новыми параметрами.
4. Обновить `src/editor.rs` для «красивее интерфейса» (брутализм, «серое небо»).
5. Сохранить коммит в `main` как «Merge Super Atmospheric v1.1.1 (Manual/Opencode)».

### 5. Проверка
- Запуск прототипа JS: `npm run dev` (должен выводить «SUPER ATMOSPHERIC audio buffer processed» с новыми параметрами).
- Сборка Rust: `cargo build --release` (после ручного слияния).

---
*Документ создан для ручного разрешения конфликта PR #3 в ветке `arena/139069db-post-ambient-vst`.*
