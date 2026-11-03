// «Холодный» полифонический пад — настоящий инструмент.
//
// Играет зажатые ноты (до 8 голосов): расстроенные пилы + октава + квинта,
// медленный атак/релиз и «ветровой» LFO по фильтру. Никакой прогрессии
// аккордов — только то, что вы нажимаете. Ориентир: холодный расстроенный
// пад Molchat Doma / Сплин.

use super::filters::StateVariableFilter;
use std::f32::consts::TAU;

const VOICES: usize = 8;
/// Чистая квинта (7 полутонос) — «загадочность» пада.
const FIFTH: f32 = 1.498_307;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    Attack,
    Sustain,
    Release,
}

struct Voice {
    note: u8,
    freq: f32,
    phase_l: f32,
    phase_r: f32,
    phase_oct: f32,
    phase_5: f32,
    env: f32,
    stage: Stage,
}

impl Voice {
    const fn new() -> Self {
        Self {
            note: 0,
            freq: 0.0,
            phase_l: 0.0,
            phase_r: 0.0,
            phase_oct: 0.0,
            phase_5: 0.0,
            env: 0.0,
            stage: Stage::Idle,
        }
    }
}

pub struct PadSynth {
    voices: [Voice; VOICES],
    filter_l: StateVariableFilter,
    filter_r: StateVariableFilter,
    time: f64,
}

impl PadSynth {
    pub fn new(sample_rate: f32, _seed: u32) -> Self {
        Self {
            voices: std::array::from_fn(|_| Voice::new()),
            filter_l: StateVariableFilter::new(sample_rate),
            filter_r: StateVariableFilter::new(sample_rate),
            time: 0.0,
        }
    }

    pub fn reset(&mut self) {
        for v in &mut self.voices {
            v.env = 0.0;
            v.stage = Stage::Idle;
        }
        self.filter_l.reset();
        self.filter_r.reset();
        self.time = 0.0;
    }

    /// Нажатие ноты: новый голос (или перезапуск уже звучащей такой же ноты).
    pub fn note_on(&mut self, freq: f32, note: u8) {
        // Такая же нота уже звучит — перезапускаем её, не тратя голос
        if let Some(v) = self
            .voices
            .iter_mut()
            .find(|v| v.stage != Stage::Idle && v.note == note)
        {
            v.freq = freq;
            v.stage = Stage::Attack;
            return;
        }

        // Свободный голос; если все заняты — «крадём» самый тихий
        let idx = match self.voices.iter().position(|v| v.stage == Stage::Idle) {
            Some(i) => i,
            None => self
                .voices
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    a.1.env
                        .partial_cmp(&b.1.env)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0),
        };
        let v = &mut self.voices[idx];
        v.note = note;
        v.freq = freq;
        v.stage = Stage::Attack;
    }

    /// Отпускание ноты — плавный релиз.
    pub fn note_off(&mut self, note: u8) {
        for v in &mut self.voices {
            if v.note == note && v.stage != Stage::Idle && v.stage != Stage::Release {
                v.stage = Stage::Release;
            }
        }
    }

    /// Все голоса в релиз (Choke / новый проект).
    pub fn all_off(&mut self) {
        for v in &mut self.voices {
            if v.stage != Stage::Idle {
                v.stage = Stage::Release;
            }
        }
    }

    pub fn next_sample(&mut self, sample_rate: f32, modulation: f32) -> (f32, f32) {
        let mut l = 0.0f32;
        let mut r = 0.0f32;

        // Медленный атак ~450 мс, длинный релиз ~1.6 с («растворение»)
        let attack_k = 1.0 - (-1.0 / (0.45 * sample_rate)).exp();
        let release_k = 1.0 - (-1.0 / (1.6 * sample_rate)).exp();

        for v in &mut self.voices {
            match v.stage {
                Stage::Idle => continue,
                Stage::Attack => {
                    v.env += (1.0 - v.env) * attack_k;
                    if v.env > 0.995 {
                        v.env = 1.0;
                        v.stage = Stage::Sustain;
                    }
                }
                Stage::Sustain => {}
                Stage::Release => {
                    v.env -= v.env * release_k;
                    if v.env < 1e-4 {
                        v.env = 0.0;
                        v.stage = Stage::Idle;
                        continue;
                    }
                }
            }

            // Детюн: левый и правый каналы чуть расходятся
            let det = 0.0035; // ±~6 центов
            v.phase_l += v.freq * (1.0 - det) / sample_rate;
            v.phase_r += v.freq * (1.0 + det) / sample_rate;
            v.phase_oct += v.freq * 2.0 / sample_rate;
            v.phase_5 += v.freq * FIFTH / sample_rate;
            if v.phase_l >= 1.0 {
                v.phase_l -= 1.0;
            }
            if v.phase_r >= 1.0 {
                v.phase_r -= 1.0;
            }
            if v.phase_oct >= 1.0 {
                v.phase_oct -= 1.0;
            }
            if v.phase_5 >= 1.0 {
                v.phase_5 -= 1.0;
            }

            let saw_l = 2.0 * v.phase_l - 1.0;
            let saw_r = 2.0 * v.phase_r - 1.0;
            let oct = 2.0 * v.phase_oct - 1.0;
            let fifth = 2.0 * v.phase_5 - 1.0;
            let common = oct * 0.26 + fifth * 0.20;

            l += (saw_l * 0.55 + common) * v.env;
            r += (saw_r * 0.55 + common) * v.env;
        }

        // «Ветровой» срез: медленный LFO двигает частоту фильтра
        let t = self.time;
        let lfo = 1.0 + 0.35 * (TAU as f64 * 0.05 * t).sin() as f32 * (0.5 + modulation);
        let base_cutoff = 350.0 + 250.0 * (TAU as f64 * 0.023 * t).sin() as f32;
        let cutoff = (base_cutoff * lfo).min(sample_rate * 0.45);

        self.filter_l.tick(l * 0.2, cutoff, 0.3);
        self.filter_r.tick(r * 0.2, cutoff, 0.3);

        let out_l = self.filter_l.low * 0.75;
        let out_r = self.filter_r.low * 0.75;

        self.time += 1.0 / sample_rate as f64;
        (out_l, out_r)
    }
}
