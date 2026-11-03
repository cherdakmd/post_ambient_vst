// src/dsp/tempo.rs - 4. Post-Punk Sync (Innovation)
// 3. Panel Echo (Improvement)

use super::params::SuperAtmosphericParams;

const TAU: f32 = std::f32::consts::TAU;

/// Задержка, синхронная темпу: `division` — доля бита
/// (1.0 = четверть, 0.5 = восьмая, 1.5 = четверть с точкой).
pub fn synced_delay_seconds(params: &SuperAtmosphericParams, division: f32) -> f32 {
    params.beat_seconds() * division.max(0.0)
}

/// Задержка в сэмплах, синхронная темпу (3. Panel Echo).
pub fn synced_delay_samples(params: &SuperAtmosphericParams, division: f32, sample_rate: f32) -> usize {
    (synced_delay_seconds(params, division) * sample_rate.max(1.0)).max(0.0) as usize
}

/// Пульс в такте (4. Post-Punk Sync) — огибающая 0..1 в начале каждой доли.
#[derive(Clone, Debug)]
pub struct TempoPulse {
    phase: f32,
}

impl TempoPulse {
    pub fn new() -> Self {
        Self { phase: 0.0 }
    }

    pub fn process(&mut self, params: &SuperAtmosphericParams, sample_rate: f32) -> f32 {
        let sr = sample_rate.max(1.0);
        let beat = params.beat_seconds().max(0.01);
        self.phase = (self.phase + 1.0 / (sr * beat)) % 1.0;
        0.5 + 0.5 * (TAU * self.phase).cos()
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    pub fn phase(&self) -> f32 {
        self.phase
    }
}

impl Default for TempoPulse {
    fn default() -> Self {
        Self::new()
    }
}

/// 4. Post-Punk Sync | 3. Panel Echo
pub fn apply_super_atmospheric_tempo(params: &SuperAtmosphericParams) {
    println!(
        "[DSP Tempo] SUPER APPLYING post-punk rhythm... BPM: {} | [Innovation] Post-Punk Sync | [Improvement] Panel Echo Sync",
        params.tempo
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_note_at_90_bpm() {
        let p = SuperAtmosphericParams::default();
        assert!((synced_delay_seconds(&p, 1.0) - 0.666_666_7).abs() < 1e-5);
    }

    #[test]
    fn dotted_quarter_is_one_second() {
        let p = SuperAtmosphericParams::default();
        assert!((synced_delay_seconds(&p, 1.5) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn samples_at_48k() {
        let p = SuperAtmosphericParams::default();
        assert_eq!(synced_delay_samples(&p, 0.5, 48_000.0), 16_000);
    }

    #[test]
    fn pulse_starts_at_full_and_stays_in_range() {
        let p = SuperAtmosphericParams::default();
        let mut pulse = TempoPulse::new();
        let first = pulse.process(&p, 48_000.0);
        assert!(first > 0.99, "first = {}", first);
        for _ in 0..100_000 {
            let v = pulse.process(&p, 48_000.0);
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
