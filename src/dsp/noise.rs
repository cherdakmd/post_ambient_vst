// Генераторы шума «Юго-Запад СПб».
//
//   Wind    - «ветер между домами»: фильтрованный шум с медленной модуляцией.
//   Radio   - «радиопомехи»: полосовой шип + быстрая тряска + вспышки.
//   Rain    - «дождь по бетону»: шип дождя + отдельные капли.
//   Hum     - «гул панельного дома»: сеть 50 Гц с гармониками.
//   Crackle - «скрип и треск»: редкие щелчки на тихом поле.
//   Mixed   - «смесь Юго-Запада»: ветер + треск + радио (по умолчанию).

use super::filters::StateVariableFilter;
use super::rng::Mulberry32;
use nih_plug::prelude::*;
use std::f32::consts::TAU;

#[derive(Enum, PartialEq, Clone, Copy, Debug)]
pub enum NoiseType {
    Wind,
    Radio,
    Rain,
    Hum,
    Crackle,
    Mixed,
}

pub struct NoiseState {
    rng: Mulberry32,
    filter: StateVariableFilter,
    // Амплитудное зерно (гранулярность)
    gate_phase: f32,
    gate_env: f32,
    // Вспышки помех / треск
    burst: f32,
    drop_env: f32,
    crk_env: f32,
    // Общее время для LFO (секунды)
    time: f64,
}

impl NoiseState {
    pub fn new(sample_rate: f32, seed: u32) -> Self {
        let mut state = Self {
            rng: Mulberry32::new(seed),
            filter: StateVariableFilter::new(sample_rate),
            gate_phase: 0.0,
            gate_env: 1.0,
            burst: 0.0,
            drop_env: 0.0,
            crk_env: 0.0,
            time: 0.0,
        };
        // Первый вызов - «посев» фазы зерна (как в JS-прототипе)
        state.gate_phase = state.rng.next_f32();
        state
    }

    /// Смена seed: новый паттерн шума (вызывается при изменении параметра).
    pub fn reseed(&mut self, sample_rate: f32, seed: u32) {
        self.rng = Mulberry32::new(seed);
        self.filter = StateVariableFilter::new(sample_rate);
        self.gate_phase = self.rng.next_f32();
        self.gate_env = 1.0;
        self.burst = 0.0;
        self.drop_env = 0.0;
        self.crk_env = 0.0;
    }

    /// Амплитудное зерно: шум «рвётся» на панельные кусочки (без щелчков).
    fn gate(&mut self, s: f32, granularity: f32, grain_rate: f32, sample_rate: f32) -> f32 {
        self.gate_phase += grain_rate / sample_rate;
        if self.gate_phase >= 1.0 {
            self.gate_phase -= 1.0;
            self.gate_env = 0.15 + self.rng.next_f32() * 0.85;
        }
        let w = 0.5 - 0.5 * (TAU * self.gate_phase).cos();
        s * (1.0 - granularity * (1.0 - self.gate_env) * w)
    }

    pub fn next_sample(
        &mut self,
        sample_rate: f32,
        kind: NoiseType,
        level: f32,
        granularity: f32,
        modulation: f32,
    ) -> f32 {
        let out = match kind {
            NoiseType::Wind => self.wind(sample_rate, level, granularity, modulation),
            NoiseType::Radio => self.radio(sample_rate, level, granularity, modulation),
            NoiseType::Rain => self.rain(sample_rate, level, granularity, modulation),
            NoiseType::Hum => self.hum(sample_rate, level, granularity, modulation),
            NoiseType::Crackle => self.crackle(sample_rate, level, granularity, modulation),
            NoiseType::Mixed => {
                let mut out = self.wind(sample_rate, level, granularity, modulation);
                // Редкий треск панелей
                if self.crk_env <= 0.001 && self.rng.next_f32() < 0.0004 * (0.4 + granularity) {
                    self.crk_env = 0.35 + self.rng.next_f32() * 0.55;
                }
                if self.crk_env > 0.0 {
                    out += (self.rng.next_f32() * 2.0 - 1.0) * self.crk_env * 0.3;
                    self.crk_env *= 0.9995;
                }
                // Редкие вспышки радио
                if self.burst <= 0.001 && self.rng.next_f32() < 0.0003 * (0.5 + granularity) {
                    self.burst = 0.25 + self.rng.next_f32() * 0.45;
                }
                if self.burst > 0.0 {
                    out += (self.rng.next_f32() * 2.0 - 1.0) * self.burst * 0.25;
                    self.burst *= 0.99985;
                }
                out
            }
        };
        self.time += 1.0 / sample_rate as f64;
        out
    }

    /// Ветер между домами
    fn wind(&mut self, sample_rate: f32, level: f32, granularity: f32, modulation: f32) -> f32 {
        let t = self.time;
        let wind = 0.5 + 0.32 * (TAU as f64 * 0.061 * t).sin() as f32
            + 0.18 * (TAU as f64 * 0.017 * t + 2.1).sin() as f32;

        let white = self.rng.next_f32() * 2.0 - 1.0;
        let cutoff = 220.0 + wind * (900.0 + modulation * 4500.0);
        self.filter.tick(white, cutoff, 0.2 + 0.5 * modulation);
        let s = self.filter.low * 1.2 + self.filter.band * 0.7;

        self.gate(s, granularity, 3.0 + granularity * 25.0, sample_rate) * level * 0.32
    }

    /// Радиопомехи
    fn radio(&mut self, sample_rate: f32, level: f32, granularity: f32, modulation: f32) -> f32 {
        let t = self.time;
        let white = self.rng.next_f32() * 2.0 - 1.0;

        let cutoff = 700.0 + modulation * 5000.0 + 1500.0 * (TAU as f64 * 0.23 * t).sin() as f32;
        self.filter.tick(white, cutoff, 0.35 + 0.5 * modulation);
        let mut s = self.filter.band * 1.4 + self.filter.high * 0.5;

        // Быстрая «тряска» сигнала - нестабильный приём
        s *= 1.0 - modulation * 0.6 * (0.5 + 0.5 * (TAU as f64 * 9.7 * t).sin() as f32);

        // Вспышки помех
        if self.burst <= 0.001 && self.rng.next_f32() < 0.0009 * (0.5 + granularity) {
            self.burst = 0.3 + self.rng.next_f32() * 0.7;
        }
        if self.burst > 0.0 {
            s += white * self.burst * 0.5;
            self.burst *= 0.9998;
        }

        self.gate(s, granularity * 0.5, 14.0 + granularity * 12.0, sample_rate) * level * 0.3
    }

    /// Дождь по бетону
    fn rain(&mut self, sample_rate: f32, level: f32, granularity: f32, modulation: f32) -> f32 {
        let white = self.rng.next_f32() * 2.0 - 1.0;
        let cutoff = 3000.0 + modulation * 4500.0;
        self.filter.tick(white, cutoff, 0.15);
        let mut s = self.filter.high * 0.6 + self.filter.band * 0.4;

        // Отдельные капли по подоконникам
        if self.rng.next_f32() < 0.01 + 0.01 * granularity {
            self.drop_env = 0.3 + self.rng.next_f32() * 0.7;
        }
        if self.drop_env > 0.0 {
            s += white * self.drop_env * 0.6;
            self.drop_env *= 0.9988; // ~10 мс
        }

        self.gate(s, granularity * 0.4, 6.0 + granularity * 10.0, sample_rate) * level * 0.28
    }

    /// Гул панельного дома (50 Гц)
    fn hum(&mut self, sample_rate: f32, level: f32, granularity: f32, modulation: f32) -> f32 {
        let t = self.time;
        let mut s = 1.0 * (TAU as f64 * 50.0 * t + 0.7 * (TAU as f64 * 0.05 * t).sin()).sin() as f32
            + 0.5 * (TAU as f64 * 100.0 * t + 1.2).sin() as f32
            + 0.25 * (TAU as f64 * 150.0 * t).sin() as f32;
        // Медленное дыхание трансформатора
        s *= 1.0 + 0.25 * (TAU as f64 * 0.13 * t).sin() as f32;

        let white = self.rng.next_f32() * 2.0 - 1.0;
        self.filter.tick(white, 200.0 + modulation * 1500.0, 0.3);
        s = s * 0.5 + self.filter.low * 0.8;

        self.gate(s, granularity * 0.6, 5.0, sample_rate) * level * 0.25
    }

    /// Скрип и треск
    fn crackle(&mut self, sample_rate: f32, level: f32, granularity: f32, modulation: f32) -> f32 {
        let t = self.time;
        let white = self.rng.next_f32() * 2.0 - 1.0;
        self.filter.tick(white, 1000.0 + modulation * 2500.0, 0.4);
        let mut s = self.filter.low * 0.12; // тихий бетонный пол

        // Редкие острые щелчки (скрип панелей, стуки в трубах)
        if self.crk_env <= 0.001 && self.rng.next_f32() < 0.002 * (0.3 + granularity) {
            self.crk_env = 0.4 + self.rng.next_f32() * 0.6;
        }
        if self.crk_env > 0.0 {
            s += white * (0.5 + 0.5 * (TAU as f64 * 90.0 * t).sin() as f32) * self.crk_env * 0.7;
            self.crk_env *= 0.9994; // ~60 мс
        }

        self.gate(s, granularity * 0.5, 8.0, sample_rate) * level * 0.35
    }
}
