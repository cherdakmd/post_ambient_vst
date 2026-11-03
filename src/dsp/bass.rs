// Монофонический пост-панк бас — настоящий инструмент.
//
// Играет реальную MIDI-ноту (никакого секвенсора): пила + суб-октава,
// ADSR-огибающая, резонансный фильтр по огибающей, дисторшн, глайд.
// Ориентир: глубокий моно-бас Molchat Doma / «Кино».

use super::filters::StateVariableFilter;
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct BassSynth {
    phase: f32,
    sub_phase: f32,
    cur_freq: f32,
    target_freq: f32,
    env: f32,
    stage: Stage,
    filter: StateVariableFilter,
    boost_filter: StateVariableFilter,
}

impl BassSynth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sub_phase: 0.0,
            cur_freq: 55.0,
            target_freq: 55.0,
            env: 0.0,
            stage: Stage::Idle,
            filter: StateVariableFilter::new(sample_rate),
            boost_filter: StateVariableFilter::new(sample_rate),
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.sub_phase = 0.0;
        self.env = 0.0;
        self.stage = Stage::Idle;
        self.filter.reset();
        self.boost_filter.reset();
    }

    /// Целевая нота в Гц. Во время удержания частота «плывёт» к ней (глайд).
    pub fn set_note(&mut self, freq: f32) {
        self.target_freq = freq.clamp(15.0, 4_000.0);
    }

    /// Нажатие (true) / отпускание (false) клавиши.
    pub fn set_gate(&mut self, on: bool) {
        if on {
            // Повторное нажатие во время удержания не перезапускает огибающую
            if matches!(self.stage, Stage::Idle | Stage::Release) {
                self.stage = Stage::Attack;
            }
        } else if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    pub fn next_sample(
        &mut self,
        sample_rate: f32,
        glide: f32,
        resonance: f32,
        drive: f32,
        bass_boost: f32,
    ) -> f32 {
        let dt = 1.0 / sample_rate;

        // --- ADSR ---
        match self.stage {
            Stage::Idle => self.env = 0.0,
            Stage::Attack => {
                self.env += dt / 0.006; // быстрый атак ~6 мс
                if self.env >= 1.0 {
                    self.env = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                // Спад к сустейну 0.72 (~120 мс)
                let k = 1.0 - (-1.0 / (0.12 * sample_rate)).exp();
                self.env += (0.72 - self.env) * k;
                if (self.env - 0.72).abs() < 0.003 {
                    self.env = 0.72;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.env = 0.72,
            Stage::Release => {
                // Релиз ~70 мс
                let k = 1.0 - (-1.0 / (0.07 * sample_rate)).exp();
                self.env -= self.env * k;
                if self.env < 1e-4 {
                    self.env = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }

        // --- Глайд: частота плавно «плывёт» к новой ноте ---
        let glide_time = 0.005 + glide * 0.28; // до ~280 мс
        let coef = 1.0 - (-1.0 / (glide_time * sample_rate)).exp();
        self.cur_freq += (self.target_freq - self.cur_freq) * coef;

        // --- Осцилляторы: пила + суб-октава ---
        self.phase += self.cur_freq / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        self.sub_phase += (self.cur_freq * 0.5) / sample_rate;
        if self.sub_phase >= 1.0 {
            self.sub_phase -= 1.0;
        }
        let saw = 2.0 * self.phase - 1.0;
        let sub = (TAU * self.sub_phase).sin();
        let mut s = saw * 0.5 + sub * 0.85;

        // --- Фильтр с огибающей («бетонные стены») ---
        let cutoff = 90.0 + self.env * (450.0 + resonance * 1600.0);
        self.filter.tick(s, cutoff, 0.25 + resonance * 0.55);
        s = self.filter.low;

        // --- Дисторшн: «сырость» пост-панка ---
        if drive > 0.0 {
            let drive_gain = 1.0 + drive * 2.5;
            s = (s * drive_gain).tanh() / drive_gain.tanh();
        }

        s *= self.env * 0.55;

        // --- Бас-буст: подчёркивание низа + мягкий ограничитель ---
        if bass_boost > 0.0 {
            self.boost_filter.tick(s, 110.0, 0.1);
            s += self.boost_filter.low * bass_boost * 1.8;
            s = s.tanh();
        } else {
            self.boost_filter.reset();
        }

        s
    }
}
