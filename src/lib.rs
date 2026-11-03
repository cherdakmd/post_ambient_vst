// post_ambient_vst - VST3/CLAP плагин для FL Studio (Rust + nih-plug).
//
// «Звук Юго-Запада Санкт-Петербурга в стиле пост-панк 2000-х».
//
// Пайплайн (в реальном времени, без аллокаций в audio-потоке):
//   дорожки (шум + басс + пад) -> «бетонный» фильтр -> дисторшн
//   -> ping-pong эхо -> Schroeder-реверб -> стерео -> мастер.
//
// Каждую дорожку можно выключить параметром (Noise/Bass/Pad) - так один
// инстанс даёт шум, другой - бас, третий - пад, и дальше микшируются в FL.

use nih_plug::prelude::*;
use std::f32::consts::TAU;
use std::sync::Arc;

mod dsp;
mod editor;
mod sample;
use dsp::bass::BassState;
use dsp::echo::Echo;
use dsp::filters::StateVariableFilter;
use dsp::noise::{NoiseState, NoiseType};
use dsp::pad::PadState;
use dsp::reverb::Reverb;
use nih_plug_egui::EguiState;
use sample::{SamplerVoice, SampleShared};

/// Стек зажатых нот (монофонный «последняя нота сверху», без аллокаций).
struct NoteStack {
    notes: [u8; 16],
    len: usize,
}

impl NoteStack {
    const fn new() -> Self {
        Self {
            notes: [0; 16],
            len: 0,
        }
    }

    fn push(&mut self, note: u8) {
        if self.len < self.notes.len() && !self.notes[..self.len].contains(&note) {
            self.notes[self.len] = note;
            self.len += 1;
        }
    }

    fn remove(&mut self, note: u8) {
        if let Some(i) = self.notes[..self.len].iter().position(|&n| n == note) {
            self.notes.copy_within(i + 1..self.len, i);
            self.len -= 1;
        }
    }

    fn last(&self) -> Option<u8> {
        if self.len > 0 {
            Some(self.notes[self.len - 1])
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.len = 0;
    }
}

pub struct PostAmbient {
    params: Arc<PostAmbientParams>,
    sample_rate: f32,

    // Состояние DSP
    noise: NoiseState,
    bass: BassState,
    pad: PadState,
    concrete_l: StateVariableFilter,
    concrete_r: StateVariableFilter,
    haas: Vec<f32>,
    haas_idx: usize,
    echo: Echo,
    reverb: Reverb,
    // Время для LFO «ветра» в цепочке эффектов (секунды)
    fx_time: f64,
    last_seed: i32,

    // --- MIDI-гейт: звук только пока зажаты ноты ---
    held: NoteStack,
    current_note: u8,
    gate_env: f32,
    gate_target: f32,
    /// Уровень гейта для MIDI-LED в GUI (безопасно для audio-потока).
    gate_level: Arc<AtomicF32>,

    // --- Сэмплер: свой аудиофайл через всю FX-цепочку ---
    sampler: SamplerVoice,
    pub sample_shared: Arc<SampleShared>,
}

#[derive(Params)]
pub struct PostAmbientParams {
    /// Состояние GUI - сохраняется вместе с пресетами.
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    // --- Дорожки (стемы) ---
    #[id = "noise_on"]
    pub noise_on: BoolParam,
    #[id = "bass_on"]
    pub bass_on: BoolParam,
    #[id = "pad_on"]
    pub pad_on: BoolParam,

    // --- Шум ---
    #[id = "noise_type"]
    pub noise_type: EnumParam<NoiseType>,
    #[id = "noise_lvl"]
    pub noise_level: FloatParam,
    #[id = "granularity"]
    pub granularity: FloatParam,
    #[id = "modulation"]
    pub modulation: FloatParam,
    #[id = "seed"]
    pub seed: IntParam,

    // --- Бас ---
    #[id = "bass_freq"]
    pub bass_freq: FloatParam,
    #[id = "bass_boost"]
    pub bass_boost: FloatParam,
    #[id = "glide"]
    pub glide: FloatParam,
    #[id = "distortion"]
    pub distortion: FloatParam,

    // --- Атмосфера ---
    #[id = "filter"]
    pub filter_cutoff: FloatParam,
    #[id = "resonance"]
    pub resonance: FloatParam,
    #[id = "echo"]
    pub echo_delay: FloatParam,
    #[id = "reverb"]
    pub reverb_size: FloatParam,
    #[id = "stereo"]
    pub stereo_width: FloatParam,

    // --- Темп ---
    #[id = "tempo_sync"]
    pub tempo_sync: BoolParam,
    #[id = "tempo"]
    pub tempo: FloatParam,

    // --- Сэмплер (свой аудиофайл) ---
    #[id = "sample_on"]
    pub sample_on: BoolParam,
    #[id = "sample_loop"]
    pub sample_loop: BoolParam,
    #[id = "sample_lvl"]
    pub sample_level: FloatParam,

    // --- Режим гейта ---
    #[id = "note_gate"]
    pub note_gate: BoolParam,

    // --- Выход ---
    #[id = "output"]
    pub output: FloatParam,
}

impl Default for PostAmbient {
    fn default() -> Self {
        Self {
            params: Arc::new(PostAmbientParams::default()),
            sample_rate: 1.0,
            noise: NoiseState::new(1.0, 42),
            bass: BassState::new(1.0, 60.0),
            pad: PadState::new(1.0, 42),
            concrete_l: StateVariableFilter::new(1.0),
            concrete_r: StateVariableFilter::new(1.0),
            haas: vec![0.0; 8192],
            haas_idx: 0,
            echo: Echo::new(8192),
            reverb: Reverb::new(),
            fx_time: 0.0,
            last_seed: 42,

            held: NoteStack::new(),
            current_note: 60,
            gate_env: 0.0,
            gate_target: 0.0,
            gate_level: Arc::new(AtomicF32::new(0.0)),

            sampler: SamplerVoice::new(),
            sample_shared: SampleShared::new(),
        }
    }
}

/// Хелпер для параметров в диапазоне 0..1 (с глайдером и малым шагом).
fn p01(name: &'static str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_smoother(SmoothingStyle::Linear(40.0))
        .with_step_size(0.001)
}

impl Default for PostAmbientParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(470, 700),

            noise_on: BoolParam::new("Noise (wind/radio)", true),
            bass_on: BoolParam::new("Bass", true),
            pad_on: BoolParam::new("Pad", true),

            noise_type: EnumParam::new("Noise Type", NoiseType::Mixed),
            noise_level: p01("Noise Level", 0.5),
            granularity: p01("Granularity", 0.7),
            modulation: p01("Modulation", 0.2),
            seed: IntParam::new(
                "Seed (randomizer)",
                42,
                IntRange::Linear { min: 0, max: 999_999 },
            ),

            bass_freq: FloatParam::new(
                "Bass Freq",
                60.0,
                FloatRange::Skewed {
                    min: 30.0,
                    max: 120.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(30.0))
            .with_step_size(0.1)
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),
            bass_boost: p01("Bass Boost", 0.5),
            glide: p01("Glide (slide)", 0.35),
            distortion: p01("Distortion", 0.3),

            filter_cutoff: FloatParam::new(
                "Filter Cutoff",
                1000.0,
                FloatRange::Skewed {
                    min: 100.0,
                    max: 12_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0))
            .with_string_to_value(formatters::s2v_f32_hz_then_khz()),
            resonance: p01("Resonance", 0.4),
            echo_delay: FloatParam::new(
                "Echo Delay",
                1.2,
                FloatRange::Linear { min: 0.5, max: 3.0 },
            )
            .with_smoother(SmoothingStyle::Linear(100.0))
            .with_step_size(0.01)
            .with_unit(" s"),
            reverb_size: p01("Reverb Size", 0.8),
            stereo_width: p01("Stereo (courtyards)", 0.6),

            tempo_sync: BoolParam::new("Sync Tempo to Host", false),
            tempo: FloatParam::new(
                "Tempo",
                100.0,
                FloatRange::Linear { min: 60.0, max: 160.0 },
            )
            .with_step_size(1.0)
            .with_unit(" BPM"),

            sample_on: BoolParam::new("Sample Track", true),
            sample_loop: BoolParam::new("Sample Loop", true),
            sample_level: p01("Sample Level", 0.7),

            note_gate: BoolParam::new("Gate By Notes", true),

            output: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-24.0),
                    max: util::db_to_gain(6.0),
                    factor: FloatRange::gain_skew_factor(-24.0, 6.0),
                },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Plugin for PostAmbient {
    const NAME: &'static str = "Post Ambient VST";
    const VENDOR: &'static str = "cherdakmd";
    const URL: &'static str = "https://github.com/cherdakmd/post_ambient_vst";
    const EMAIL: &'static str = "cherdakmd@users.noreply.github.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.gate_level.clone(),
            self.sample_shared.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        let sr = self.sample_rate;
        let seed = self.params.seed.value() as u32;
        let root = self.params.bass_freq.value();

        self.noise = NoiseState::new(sr, seed);
        self.bass = BassState::new(sr, root);
        self.pad = PadState::new(sr, seed);
        self.concrete_l = StateVariableFilter::new(sr);
        self.concrete_r = StateVariableFilter::new(sr);

        // Эхо: ёмкость под максимум параметра (3.0 с) + запас
        self.echo = Echo::new((3.1 * sr) as usize);
        self.haas = vec![0.0; 8192];
        self.haas_idx = 0;
        self.reverb = Reverb::new();

        self.fx_time = 0.0;
        self.last_seed = self.params.seed.value();

        true
    }

    fn reset(&mut self) {
        let seed = self.params.seed.value() as u32;
        let root = self.params.bass_freq.value();
        self.noise.reseed(self.sample_rate, seed);
        self.bass.reset(root);
        self.concrete_l.reset();
        self.concrete_r.reset();
        self.echo.reset();
        self.reverb.reset();
        self.haas.iter_mut().for_each(|s| *s = 0.0);
        self.haas_idx = 0;
        self.fx_time = 0.0;
        self.held.clear();
        self.current_note = 60;
        self.gate_env = 0.0;
        self.gate_target = 0.0;
    }

    #[allow(clippy::too_many_arguments)]
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let sr = self.sample_rate;

        // --- Настройки на границе блока ---
        let noise_on = self.params.noise_on.value();
        let bass_on = self.params.bass_on.value();
        let pad_on = self.params.pad_on.value();
        let sample_on = self.params.sample_on.value();
        let sample_loop = self.params.sample_loop.value();
        let sample_level = self.params.sample_level.value();
        let note_gate = self.params.note_gate.value();
        let kind = self.params.noise_type.value();

        // --- Подхватываем новый сэмпл (try_lock - без блокировки audio-потока) ---
        if let Ok(guard) = self.sample_shared.loaded.try_lock() {
            let new = guard.as_ref().map(|a| a.clone());
            self.sampler.set_sample(new);
        }

        // --- MIDI: монофонический гейт «последняя нота сверху» ---
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn { note, .. } => {
                    self.held.push(note);
                    self.current_note = note;
                    self.gate_target = 1.0;
                    if sample_on {
                        self.sampler.retrigger(note, sr);
                    }
                }
                NoteEvent::NoteOff { note, .. } => {
                    self.held.remove(note);
                    match self.held.last() {
                        // Остались другие зажатые ноты - гейт не закрывается
                        Some(n) => self.current_note = n,
                        None => self.gate_target = 0.0,
                    }
                }
                NoteEvent::Choke { .. } => {
                    self.held.clear();
                    self.gate_target = 0.0;
                }
                _ => (),
            }
        }

        // Коэффициенты сглаживания гейта: быстрая атака, медленный «хвост»
        let gate_on_coef = 1.0 - (-1.0 / (0.008 * sr)).exp();
        let gate_off_coef = 1.0 - (-1.0 / (0.12 * sr)).exp();

        // Транспонирование баса и пада по зажатой ноте (нота 60 = исходный лад)
        let note_factor = 2.0f32.powf((self.current_note as f32 - 60.0) / 12.0);

        // Рандомайзер: новый seed = новый паттерн шума
        let seed = self.params.seed.value();
        if seed != self.last_seed {
            self.noise.reseed(sr, seed as u32);
            self.last_seed = seed;
        }

        // Темп: от хоста (BPM из FL) или ручной
        let bpm = if self.params.tempo_sync.value() {
            context
                .transport()
                .tempo
                .map(|t| t as f32)
                .filter(|t| *t > 0.0)
                .unwrap_or(self.params.tempo.value())
        } else {
            self.params.tempo.value()
        };

        for channel_samples in buffer.iter_samples() {
            // Сглаживание параметров (обязательно вызывать каждый сэмпл)
            let noise_level = self.params.noise_level.smoothed.next();
            let granularity = self.params.granularity.smoothed.next();
            let modulation = self.params.modulation.smoothed.next();
            let bass_freq = self.params.bass_freq.smoothed.next();
            let bass_boost = self.params.bass_boost.smoothed.next();
            let glide = self.params.glide.smoothed.next();
            let distortion = self.params.distortion.smoothed.next();
            let filter_cutoff = self.params.filter_cutoff.smoothed.next();
            let resonance = self.params.resonance.smoothed.next();
            let echo_delay = self.params.echo_delay.smoothed.next();
            let reverb_size = self.params.reverb_size.smoothed.next();
            let stereo_width = self.params.stereo_width.smoothed.next();
            let output_gain = self.params.output.smoothed.next();

            // --- Микс дорожек ---
            let mut l = 0.0f32;
            let mut r = 0.0f32;

            if noise_on {
                let n = self
                    .noise
                    .next_sample(sr, kind, noise_level, granularity, modulation);
                // Ранняя стерео-картина: микро-задержка Хааса на правом канале
                let cap = self.haas.len();
                let d = (((0.15 + stereo_width * 0.35) * sr * 0.004) as usize).clamp(1, cap - 1);
                let n_r = self.haas[(self.haas_idx + cap - d) % cap];
                self.haas[self.haas_idx] = n;
                self.haas_idx = (self.haas_idx + 1) % cap;
                l += n;
                r += n_r;
            }

            if bass_on {
                let b = self.bass.next_sample(
                    sr,
                    bass_freq * note_factor,
                    bpm,
                    resonance,
                    distortion,
                    glide,
                    bass_boost,
                );
                l += b * 0.8;
                r += b * 0.8;
            }

            if pad_on {
                let (pl, pr) = self.pad.next_sample(sr, bass_freq * note_factor, bpm, modulation);
                l += pl * 1.1;
                r += pr * 1.1;
            }

            // --- Сэмплер: свой файл через всю FX-цепочку ---
            if sample_on {
                let (sl, sr_) = self.sampler.next_sample(sample_loop);
                l += sl * sample_level;
                r += sr_ * sample_level;
            }

            // --- MIDI-гейт: звук только пока зажаты ноты ---
            // Применяется до FX - хвосты эхо/реверба свободно «дозваниваются».
            let target = if note_gate { self.gate_target } else { 1.0 };
            let coef = if target > self.gate_env {
                gate_on_coef
            } else {
                gate_off_coef
            };
            self.gate_env += (target - self.gate_env) * coef;
            l *= self.gate_env;
            r *= self.gate_env;

            // --- «Бетонные стены»: резонансный фильтр с ветровой модуляцией ---
            let t = self.fx_time;
            let lfo = 1.0
                + modulation * 0.45 * (TAU as f64 * 0.11 * t).sin() as f32
                + modulation * 0.2 * (TAU as f64 * 0.037 * t + 1.7).sin() as f32;
            let cutoff = (filter_cutoff * lfo).clamp(60.0, sr * 0.45);
            self.concrete_l.tick(l, cutoff, resonance);
            l = self.concrete_l.low + self.concrete_l.band * 0.15;
            self.concrete_r.tick(r, cutoff, resonance);
            r = self.concrete_r.low + self.concrete_r.band * 0.15;
            self.fx_time += 1.0 / sr as f64;

            // --- Дисторшн (моно-искажение, как ламповый усилитель) ---
            if distortion > 0.0 {
                let drive = 1.0 + distortion * 12.0;
                let mono = (l + r) * 0.5;
                let wet = (mono * drive).tanh() / drive.tanh();
                let mixed = mono * (1.0 - distortion) + wet * distortion;
                let mix = 0.6;
                l = l * (1.0 - mix) + mixed * mix;
                r = r * (1.0 - mix) + mixed * mix;
            }

            // --- Эхо «панельные дома» (ping-pong) ---
            let delay_samples = (echo_delay * sr) as usize;
            let (el, er) = self.echo.process_sample(
                l,
                r,
                delay_samples,
                0.4 + reverb_size * 0.2,
                0.25 + distortion * 0.15,
            );
            l = el;
            r = er;

            // --- Реверберация «эхо дворов» ---
            let wet = 0.22 + reverb_size * 0.4;
            let (rl, rr) = self.reverb.process_sample(l, r, reverb_size, wet);
            l = rl;
            r = rr;

            // --- Стерео «дворы» (Mid/Side) ---
            let g = 1.0 + stereo_width * 1.3;
            let mid = (l + r) * 0.5;
            let side = (l - r) * 0.5 * g;
            l = mid + side;
            r = mid - side;

            // --- Мастер: выход + мягкий ограничитель ---
            l = (l * output_gain).tanh();
            r = (r * output_gain).tanh();

            let mut ch_iter = channel_samples.into_iter();
            if let Some(first) = ch_iter.next() {
                if let Some(second) = ch_iter.next() {
                    *first = l;
                    *second = r;
                } else {
                    *first = (l + r) * 0.5;
                }
            }
        }

        // Уровень гейта для MIDI-LED в GUI
        self.gate_level
            .store(self.gate_env, std::sync::atomic::Ordering::Relaxed);

        ProcessStatus::Normal
    }
}

impl ClapPlugin for PostAmbient {
    const CLAP_ID: &'static str = "com.cherdakmd.post-ambient-vst";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Post-punk ambient soundscape generator (Southwest St. Petersburg)");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for PostAmbient {
    const VST3_CLASS_ID: [u8; 16] = *b"PostAmbientSWSPb";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
    ];
}

nih_export_clap!(PostAmbient);
nih_export_vst3!(PostAmbient);
