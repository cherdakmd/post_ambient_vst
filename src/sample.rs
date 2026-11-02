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

// ---------------------------------------------------------------- голос сэмплера

/// Монофонический голос сэмплера: линейная интерполяция, транспонирование
/// по MIDI-ноте, петля с кроссфейдом, перезапуск по NoteOn.
pub struct SamplerVoice {
    current: Option<Arc<LoadedSample>>,
    pos: f64,
    playing: bool,
    note: u8,
    ratio: f64,
}

impl SamplerVoice {
    pub fn new() -> Self {
        Self {
            current: None,
            pos: 0.0,
            playing: false,
            note: 60,
            ratio: 1.0,
        }
    }

    /// Подхватывает новый сэмпл (сброс позиции только если файл реально сменился).
    pub fn set_sample(&mut self, sample: Option<Arc<LoadedSample>>) {
        let changed = match (&self.current, &sample) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        };
        if changed {
            self.current = sample;
            self.pos = 0.0;
            self.playing = self.current.is_some();
            self.update_ratio(48_000.0);
        }
    }

    /// Перезапуск с новой нотой (вызывается по NoteOn).
    pub fn retrigger(&mut self, note: u8, engine_sr: f32) {
        self.note = note;
        if self.current.is_some() {
            self.pos = 0.0;
            self.playing = true;
        }
        self.update_ratio(engine_sr);
    }

    fn update_ratio(&mut self, engine_sr: f32) {
        if let Some(s) = &self.current {
            // Нота 60 (до) = исходная высота; выше - быстрее и выше
            let pitch = 2.0f64.powf((self.note as f64 - 60.0) / 12.0);
            self.ratio = pitch * (s.sample_rate as f64 / engine_sr as f64);
        }
    }

    /// Следующая стереопара сэмплов.
    pub fn next_sample(&mut self, looped: bool) -> (f32, f32) {
        let Some(s) = self.current.as_ref() else {
            return (0.0, 0.0);
        };
        if !self.playing || s.frames == 0 {
            return (0.0, 0.0);
        }
        let n = s.frames as f64;
        if self.pos >= n {
            if looped {
                self.pos %= n;
            } else {
                self.playing = false;
                return (0.0, 0.0);
            }
        }
        let out = read_interp(s, self.pos, looped);
        self.pos += self.ratio;
        out
    }
}

/// Чтение сэмпла с линейной интерполяцией; при петле - кроссфейд хвоста в голову.
fn read_interp(s: &LoadedSample, pos: f64, looped: bool) -> (f32, f32) {
    let n = s.frames;
    let ch = s.channels;
    let get = |i: usize| -> (f32, f32) {
        let i = if i >= n {
            if looped {
                i % n
            } else {
                n - 1
            }
        } else {
            i
        };
        let base = i * ch;
        let a = s.data[base];
        let b = if ch > 1 { s.data[base + 1] } else { a };
        (a, b)
    };

    let fi = pos.floor();
    let idx = fi as usize;
    let frac = (pos - fi) as f32;
    let (a0, b0) = get(idx);
    let (a1, b1) = get(idx + 1);
    let mut l = a0 + (a1 - a0) * frac;
    let mut r = b0 + (b1 - b0) * frac;

    // Кроссфейд петли: последние ~10 мс хвоста растворяются в начале файла
    if looped && n > 8 {
        let xf = ((n / 8).max(64)).min((s.sample_rate as usize) / 100).min(n);
        if xf > 1 && idx + xf >= n && idx < n {
            let head = idx + xf - n; // индекс в голове файла
            let (h0l, h0r) = get(head);
            let (h1l, h1r) = get(head + 1);
            let hl = h0l + (h1l - h0l) * frac;
            let hr = h0r + (h1r - h0r) * frac;
            let w = ((idx + xf - n) as f32 / xf as f32).clamp(0.0, 1.0);
            l = l * (1.0 - w) + hl * w;
            r = r * (1.0 - w) + hr * w;
        }
    }

    (l, r)
}
