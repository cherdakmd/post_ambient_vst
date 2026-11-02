// src/dsp/noise.rs - 1. SW Wind Gate & 6. Granular Noise Storm (Innovation)
// 4. Granularity & 8. Wind Dynamics Modulation (Improvement)

use super::params::SuperAtmosphericParams;

const TAU: f32 = std::f32::consts::TAU;

/// Размер кольцевого буфера «ветрового» сигнала, из которого читаются зёрна.
const WIND_BUFFER: usize = 4096;

/// «Ветер Юго-Запада»: гранулярный шум дворов с медленным ветровым гейтом.
///
/// * 1. SW Wind Gate         — медленная случайная огибающая порывов (0.7–2.3 с).
/// * 6. Granular Noise Storm — два зёрна с перекрытием Ханна 50% (сумма = 1.0,
///   без тремоло), читаются из кольцевого буфера со случайного смещения:
///   спектр источника сохраняется, в отличие от sample-and-hold.
/// * 8. Wind Dynamics Modulation — чем сильнее ветер, тем более сглаженный
///   (дальний) гул и тем громче ВЧ-«воздух».
#[derive(Clone, Debug)]
pub struct WindNoise {
    rng: u32,
    wind_lp: f32,
    buffer: Vec<f32>,
    write: usize,
    /// (позиция чтения, позиция внутри зерна) для двух голосов
    voices: [(usize, f32); 2],
    gate_env: f32,
    gate_target: f32,
    gate_countdown: u32,
}

impl WindNoise {
    pub fn new(seed: u32) -> Self {
        Self {
            rng: seed | 1,
            wind_lp: 0.0,
            buffer: vec![0.0; WIND_BUFFER],
            write: 0,
            voices: [(0, 0.0), (WIND_BUFFER / 2, WIND_BUFFER as f32 / 4.0)],
            gate_env: 0.5,
            gate_target: 0.5,
            gate_countdown: 0,
        }
    }

    /// xorshift32 → белый шум в диапазоне [-1, 1].
    fn white(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Случайный индекс внутри кольцевого буфера.
    fn random_offset(&mut self) -> usize {
        let r = self.white() * 0.5 + 0.5;
        (r * (self.buffer.len() - 1) as f32) as usize
    }

    /// Один сэмпл «супер атмосферного» шума.
    pub fn process(&mut self, params: &SuperAtmosphericParams, sample_rate: f32) -> f32 {
        let sr = sample_rate.max(1.0);
        let wind = params.southwest_wind.clamp(0.0, 1.0);
        let granularity = params.granularity.clamp(0.0, 1.0);

        // 8. Ветер: сильный ветер = низкочастотный, «дальний» гул панельных дворов
        let wind_alpha = 0.02 + 0.18 * (1.0 - wind);
        let w = self.white();
        self.wind_lp += (w - self.wind_lp) * wind_alpha;
        // «Воздух»: ВЧ-остаток после сглаживания — свист ветра между домами
        let air = w - self.wind_lp;
        // Нормировка: сглаживание съедает энергию, компенсируем её
        let rumble_gain = 0.30 / (0.12 + wind_alpha.sqrt());
        let air_gain = 1.1 * (0.3 + 0.7 * wind);
        let winded = self.wind_lp * rumble_gain + air * air_gain;

        // Источник для зёрен — кольцевой буфер ветрового сигнала
        self.buffer[self.write] = winded;
        self.write = (self.write + 1) % self.buffer.len();

        // 6. Granular Noise Storm: зерно 4–64 мс (granularity 0.9 → ~10 мс)
        let grain_len = ((0.004 + 0.06 * (1.0 - granularity)) * sr).max(4.0);
        let mut out = 0.0;
        for i in 0..2 {
            if self.voices[i].1 >= grain_len {
                self.voices[i].0 = self.random_offset();
                self.voices[i].1 = 0.0;
            }
            let (read, pos) = self.voices[i];
            let idx = (read + pos as usize) % self.buffer.len();
            // Окно Ханна; два голоса со сдвигом в половину зерна дают сумму = 1.0
            let env = 0.5 - 0.5 * (TAU * pos / grain_len).cos();
            out += self.buffer[idx] * env;
            self.voices[i].1 = pos + 1.0;
        }

        // 1. SW Wind Gate: порывы ветра между домами
        if self.gate_countdown == 0 {
            self.gate_target = 0.25 + 0.75 * (self.white() * 0.5 + 0.5);
            self.gate_countdown = ((0.7 + 1.6 * (1.0 - wind)) * sr) as u32;
        }
        self.gate_countdown = self.gate_countdown.saturating_sub(1);
        self.gate_env += (self.gate_target - self.gate_env) * 0.0008;

        let gust = (0.35 + 0.65 * self.gate_env) * (0.4 + 0.6 * wind);
        let level = params.noise_level.clamp(0.0, 1.0);
        (out * gust * level * 0.7).clamp(-1.0, 1.0)
    }

    pub fn reset(&mut self) {
        for s in self.buffer.iter_mut() {
            *s = 0.0;
        }
        self.write = 0;
        self.wind_lp = 0.0;
        self.voices = [(0, 0.0), (WIND_BUFFER / 2, WIND_BUFFER as f32 / 4.0)];
        self.gate_env = 0.5;
        self.gate_target = 0.5;
        self.gate_countdown = 0;
    }

    /// Текущее раскрытие ветрового гейта (для визуализации в GUI).
    pub fn gate(&self) -> f32 {
        self.gate_env
    }
}

/// Отчёт параметров (используется плагином и автономной проверкой).
pub fn generate_super_atmospheric_noise(params: &SuperAtmosphericParams) {
    println!(
        "[DSP Noise] SUPER GENERATING noise... Level: {}, Granularity: {}, Southwest Wind: {} | [Innovation] SW Wind Gate & Granular Storm | [Improvement] Granularity & Wind Dynamics",
        params.noise_level, params.granularity, params.southwest_wind
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_with_same_seed() {
        let p = SuperAtmosphericParams::default();
        let mut a = WindNoise::new(42);
        let mut b = WindNoise::new(42);
        for _ in 0..128 {
            assert_eq!(a.process(&p, 48_000.0), b.process(&p, 48_000.0));
        }
    }

    #[test]
    fn output_is_bounded() {
        let mut p = SuperAtmosphericParams::default();
        p.noise_level = 1.0;
        p.granularity = 1.0;
        let mut n = WindNoise::new(7);
        for i in 0..20_000 {
            let s = n.process(&p, 48_000.0);
            assert!(s.is_finite(), "sample {} is not finite", i);
            assert!(s.abs() <= 1.0, "sample {} = {} out of range", i, s);
        }
    }

    #[test]
    fn silence_when_noise_level_is_zero() {
        let mut p = SuperAtmosphericParams::default();
        p.noise_level = 0.0;
        let mut n = WindNoise::new(1);
        for _ in 0..1_000 {
            assert_eq!(n.process(&p, 48_000.0), 0.0);
        }
    }

    #[test]
    fn granular_preserves_high_frequencies() {
        // Зёрна читаются из буфера, поэтому спектр источника сохраняется:
        // энергии в полосе 1–6 кГц должно быть заметно больше нуля.
        let p = SuperAtmosphericParams::default();
        let mut n = WindNoise::new(11);
        let mut hp = 0.0f32;
        let mut energy = 0.0;
        for _ in 0..48_000 {
            let s = n.process(&p, 48_000.0);
            hp += 0.6 * (s - hp); // ФВЧ: быстрая составляющая
            energy += (s - hp) * (s - hp);
        }
        let rms_hf = (energy / 48_000.0).sqrt();
        assert!(rms_hf > 0.02, "высоких почти нет: rms_hf = {}", rms_hf);
    }
}
