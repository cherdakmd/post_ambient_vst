// src/dsp/modulation.rs - 8. Wind Dynamics Modulation (Innovation)
// 8. Modulation (Improvement)

use super::params::SuperAtmosphericParams;

const TAU: f32 = std::f32::consts::TAU;

/// LFO «ветровой динамики»: медленное дыхание громкости,
/// синхронное темпу (4. Post-Punk Sync) и зависящее от ветра Юго-Запада.
#[derive(Clone, Debug)]
pub struct WindLfo {
    phase: f32,
}

impl WindLfo {
    pub fn new() -> Self {
        Self { phase: 0.0 }
    }

    /// Частота LFO в Гц: 90 BPM + ветер 0.75 → ~0.375 Гц (цикл ~2.7 с).
    pub fn rate_hz(params: &SuperAtmosphericParams) -> f32 {
        let bps = params.tempo.max(1.0) / 60.0;
        let wind = params.southwest_wind.clamp(0.0, 1.0);
        (bps * (0.5 + 2.0 * wind)) / 8.0
    }

    /// Множитель громкости в диапазоне [1 - modulation, 1].
    pub fn process(&mut self, params: &SuperAtmosphericParams, sample_rate: f32) -> f32 {
        let sr = sample_rate.max(1.0);
        self.phase = (self.phase + Self::rate_hz(params) / sr) % 1.0;

        let s = 0.5 + 0.5 * (TAU * self.phase).sin();
        let depth = params.modulation.clamp(0.0, 1.0);
        1.0 - depth * (1.0 - s)
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    pub fn phase(&self) -> f32 {
        self.phase
    }
}

impl Default for WindLfo {
    fn default() -> Self {
        Self::new()
    }
}

/// 8. Wind Dynamics Modulation | 8. Modulation
pub fn apply_super_atmospheric_modulation(params: &SuperAtmosphericParams) {
    println!(
        "[DSP Modulation] SUPER APPLYING wind modulation... Level: {}, Southwest Wind: {} | [Innovation] Wind Dynamics Modulation | [Improvement] Modulation",
        params.modulation, params.southwest_wind
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_within_modulation_range() {
        let p = SuperAtmosphericParams::default();
        let mut lfo = WindLfo::new();
        let lo = 1.0 - p.modulation;
        for _ in 0..100_000 {
            let f = lfo.process(&p, 48_000.0);
            assert!(f >= lo - 1e-5 && f <= 1.0 + 1e-5, "factor = {}", f);
        }
    }

    #[test]
    fn rate_is_slow_and_tempo_synced() {
        let p = SuperAtmosphericParams::default();
        let rate = WindLfo::rate_hz(&p);
        assert!((rate - 0.375).abs() < 1e-3, "rate = {}", rate);
    }

    #[test]
    fn phase_wraps() {
        let p = SuperAtmosphericParams::default();
        let mut lfo = WindLfo::new();
        for _ in 0..500_000 {
            lfo.process(&p, 48_000.0);
            assert!((0.0..1.0).contains(&lfo.phase()));
        }
    }
}
