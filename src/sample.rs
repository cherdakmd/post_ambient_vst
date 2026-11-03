// src/sample.rs - сэмплер: загрузка пользовательского аудиофайла
// и его воспроизведение внутри плагина через всю FX-цепочку.
//
// Декодирование (symphonia: WAV/MP3/FLAC/OGG/AIFF/M4A) идёт в фоновом
// потоке; audio-поток забирает готовый буфер через try_lock - без ожиданий
// и блокировок в реальном времени.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Декодированный сэмпл: interleaved f32.
pub struct LoadedSample {
    pub name: String,
    pub channels: usize,
    pub frames: usize,
    pub sample_rate: f32,
    pub data: Vec<f32>,
}

/// Статус загрузки для GUI.
pub enum UiStatus {
    Idle,
    Loading,
    Error(String),
}

/// Общее состояние сэмпла: фоновый декодер пишет, audio-поток читает.
pub struct SampleShared {
    pub loaded: Mutex<Option<Arc<LoadedSample>>>,
    pub status: Mutex<UiStatus>,
}

impl SampleShared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            loaded: Mutex::new(None),
            status: Mutex::new(UiStatus::Idle),
        })
    }
}

/// Захват мьютекса без паники на отравленном замке.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Декодирует аудиофайл в память (вызывать только в фоновом потоке).
pub fn decode_file(path: &Path) -> Result<LoadedSample, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("не открыть файл: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Подсказка по расширению - помогает пробе выбрать контейнер
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("формат не распознан: {e}"))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| "в файле нет аудиодорожки".to_string())?;
    let dec_opts = AudioDecoderOptions::default();
    let codec_params = track
        .codec_params
        .as_ref()
        .ok_or_else(|| "нет параметров кодека".to_string())?;
    let audio_params = codec_params
        .audio()
        .ok_or_else(|| "дорожка не является аудио".to_string())?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio_params, &dec_opts)
        .map_err(|e| format!("кодек не поддерживается: {e}"))?;
    let track_id = track.id;

    let mut data: Vec<f32> = Vec::new();
    let mut sr = 44100u32;
    let mut channels = 1usize;
    // Потолок размера: ~10 минут стерео 48 кГц
    let limit: usize = 10 * 60 * 48_000 * 2;

    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(Error::ResetRequired) => break,
            Err(Error::IoError(_)) => break,
            Err(e) => return Err(format!("ошибка чтения: {e}")),
        };
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => {
                let spec = buf.spec();
                sr = spec.rate();
                channels = spec.channels().count().max(1);
                let n = buf.samples_interleaved();
                let start = data.len();
                if start + n > limit {
                    // Слишком длинный файл - обрезаем и хватит
                    break;
                }
                data.resize(start + n, 0.0);
                buf.copy_to_slice_interleaved(&mut data[start..]);
            }
            Err(Error::DecodeError(_)) => continue,
            Err(Error::IoError(_)) => break,
            Err(e) => return Err(format!("ошибка декодирования: {e}")),
        }
    }

    if data.is_empty() {
        return Err("не удалось декодировать ни одного сэмпла".to_string());
    }
    let frames = data.len() / channels;
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "сэмпл".to_string());

    Ok(LoadedSample {
        name,
        channels,
        frames,
        sample_rate: sr as f32,
        data,
    })
}

/// Запускает фоновую загрузку файла; GUI сразу получает UiStatus::Loading.
pub fn spawn_load(path: PathBuf, shared: Arc<SampleShared>) {
    *lock(&shared.status) = UiStatus::Loading;
    std::thread::spawn(move || {
        match decode_file(&path) {
            Ok(s) => {
                *lock(&shared.loaded) = Some(Arc::new(s));
                *lock(&shared.status) = UiStatus::Idle;
            }
            Err(e) => *lock(&shared.status) = UiStatus::Error(e),
        }
    });
}

// ---------------------------------------------------------------- слайсер брейка

/// 16 равных слайсов загруженного файла.
/// Нота 36 (C2) = слайс 0, дальше по полутонам; октавы повторяют набор.
pub const SLICES: usize = 16;
const MAX_SLICE_VOICES: usize = 8;

struct SliceVoice {
    sample: Option<Arc<LoadedSample>>,
    slice: usize,
    pos: f64,
    start: f64,
    end: f64,
    playing: bool,
}

/// Слайсер: NoteOn -> проигрывание слайса до его границы (одноразово),
/// фейды 3 мс на границах против щелчков, до 8 голосов.
pub struct Slicer {
    voices: [SliceVoice; MAX_SLICE_VOICES],
    sample: Option<Arc<LoadedSample>>,
    /// file_sr / engine_sr - сохраняем высоту файла независимо от SR хоста.
    ratio: f64,
    engine_sr: f32,
}

impl Slicer {
    pub fn new() -> Self {
        Self {
            voices: std::array::from_fn(|_| SliceVoice {
                sample: None,
                slice: 0,
                pos: 0.0,
                start: 0.0,
                end: 0.0,
                playing: false,
            }),
            sample: None,
            ratio: 1.0,
            engine_sr: 44_100.0,
        }
    }

    /// Подхватывает новый файл (сброс голосов только если файл реально сменился).
    pub fn set_sample(&mut self, sample: Option<Arc<LoadedSample>>, engine_sr: f32) {
        let changed = match (&self.sample, &sample) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        if changed {
            self.sample = sample;
            for v in &mut self.voices {
                v.playing = false;
                v.sample = None;
            }
        }
        if changed || engine_sr != self.engine_sr {
            self.engine_sr = engine_sr;
            self.ratio = self
                .sample
                .as_ref()
                .map(|s| s.sample_rate as f64 / engine_sr as f64)
                .unwrap_or(1.0);
        }
    }

    /// NoteOn: нота -> слайс (C2 = 0, октавы повторяют набор из 16).
    pub fn trigger(&mut self, note: u8) {
        let Some(s) = self.sample.clone() else {
            return;
        };
        if s.frames < SLICES {
            return;
        }
        let idx = (note as i32 - 36).rem_euclid(SLICES as i32) as usize;
        let start = s.frames * idx / SLICES;
        let end = s.frames * (idx + 1) / SLICES;
        if end <= start {
            return;
        }

        // Перезапуск того же слайса, иначе свободный голос, иначе кража первого
        let voice = self
            .voices
            .iter()
            .position(|v| v.playing && v.slice == idx)
            .or_else(|| self.voices.iter().position(|v| !v.playing))
            .unwrap_or(0);

        let v = &mut self.voices[voice];
        v.sample = Some(s);
        v.slice = idx;
        v.start = start as f64;
        v.end = end as f64;
        v.pos = v.start;
        v.playing = true;
    }

    /// Останавливает все голосы (Choke / сброс).
    pub fn all_off(&mut self) {
        for v in &mut self.voices {
            v.playing = false;
        }
    }

    /// Стереопара активных слайсов.
    pub fn next_sample(&mut self) -> (f32, f32) {
        let mut l = 0.0f32;
        let mut r = 0.0f32;
        let fade = (self.engine_sr as f64 * 0.003).max(1.0); // фейд 3 мс

        for v in &mut self.voices {
            if !v.playing {
                continue;
            }
            let Some(s) = v.sample.clone() else {
                v.playing = false;
                continue;
            };
            if s.frames == 0 || v.pos >= v.end - 1.0 {
                v.playing = false;
                continue;
            }

            // Линейная интерполяция по позиции
            let fi = v.pos.floor();
            let idx = (fi as usize).min(s.frames - 1);
            let frac = (v.pos - fi) as f32;
            let get = |i: usize| -> (f32, f32) {
                let i = i.min(s.frames - 1);
                let base = i * s.channels;
                let a = s.data[base];
                let b = if s.channels > 1 { s.data[base + 1] } else { a };
                (a, b)
            };
            let (a0, b0) = get(idx);
            let (a1, b1) = get(idx + 1);
            let sl = a0 + (a1 - a0) * frac;
            let sr_ = b0 + (b1 - b0) * frac;

            // Анти-щелчковые фейды на границах слайса
            let in_f = ((v.pos - v.start) / fade).clamp(0.0, 1.0);
            let out_f = ((v.end - v.pos) / fade).clamp(0.0, 1.0);
            let g = (in_f * out_f) as f32;
            l += sl * g;
            r += sr_ * g;

            v.pos += self.ratio;
        }

        (l, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample(frames: usize) -> Arc<LoadedSample> {
        // Стерео-файл: левый =1, правый = -1 (для проверки каналов)
        let mut data = Vec::with_capacity(frames * 2);
        for _ in 0..frames {
            data.push(1.0);
            data.push(-1.0);
        }
        Arc::new(LoadedSample {
            name: "test.wav".into(),
            channels: 2,
            frames,
            sample_rate: 44_100.0,
            data,
        })
    }

    #[test]
    fn slicer_is_silent_without_file() {
        let mut s = Slicer::new();
        s.trigger(60);
        let (l, r) = s.next_sample();
        assert_eq!((l, r), (0.0, 0.0));
    }

    #[test]
    fn note_maps_to_slice_and_plays_to_its_end() {
        let mut s = Slicer::new();
        s.set_sample(Some(make_sample(16_000)), 44_100.0); // 16 слайсов по 1000 фреймов
        s.trigger(36); // слайс 0: [0, 1000)

        let mut peak = 0.0f32;
        let mut frames_played = 0;
        for _ in 0..4_000 {
            let (l, r) = s.next_sample();
            if l != 0.0 || r != 0.0 {
                frames_played += 1;
            }
            peak = peak.max(l.abs()).max(r.abs());
        }
        assert!(peak > 0.5, "слайс должен звучать, peak={peak}");
        // Слайс = 1000 фреймов + запас на фейды - голос должен замолчать
        assert!(frames_played < 1_200, "голос не должен играть дальше слайса: {frames_played}");
        let (l, r) = s.next_sample();
        assert_eq!((l, r), (0.0, 0.0), "после слайса - тишина");
    }

    #[test]
    fn higher_octave_repeats_slices() {
        let mut s = Slicer::new();
        s.set_sample(Some(make_sample(16_000)), 44_100.0);
        s.trigger(36); // слайс 0
        s.trigger(48); // та же нота октавой выше -> снова слайс 0
        // Пропускаем фейд-ин (первый сэмпл всегда 0 из-за 3 мс фейда)
        let mut peak = 0.0f32;
        for _ in 0..200 {
            let (l, _) = s.next_sample();
            peak = peak.max(l.abs());
        }
        assert!(peak > 0.5, "слайс должен звучать, peak={peak}");
    }
}
