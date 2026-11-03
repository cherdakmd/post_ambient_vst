// src/dsp/resonance.rs - 3. Panel Echo Resonance (Innovation)
// 6. Resonance (Improvement)

use super::params::SuperAtmosphericParams;

/// Моды «бетонной панели» (в Гц) — глухой гул панельного дома.
const PANEL_MODES: [f32; 3] = [62.0, 187.0, 431.0];
const PANEL_GAINS: [f32; 3] = [0.30, 0.36, 0.30];

/// Одна мода: полосовой фильтр Chamberlin (band-выход).
#[derive(Clone, Debug)]
struct PanelMode {
    low: f32,
    band: f32,
    f: f32,
    q: f32,
}

impl PanelMode {
    fn new(freq_hz: f32, q: f32, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let fc = freq_hz.clamp(20.0, sr * 0.45);
        let f = (2.0 * (std::f32::consts::PI * fc / sr).sin()).clamp(0.0005, 1.0);
        Self {
            low: 0.0,
            band: 0.0,
            f,
            q,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let high = x - self.low - self.q * self.band;
        self.band = self.f * high + self.band;
        self.low = self.f * self.band + self.low;
        self.band.clamp(-4.0, 4.0)
    }
}

/// Банк резонансов панельных стен (3. Panel Echo Resonance).
#[derive(Clone, Debug)]
pub struct PanelResonator {
    modes: [PanelMode; 3],
}

impl PanelResonator {
    pub fn new(params: &SuperAtmosphericParams, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let shift = params.spectral_shift.clamp(0.0, 1.0);
        // резонанс 0.7 → q = 0.44 (долгое «бетонное» дребезжание)
        let q = (1.0 - 0.8 * params.resonance.clamp(0.0, 1.0)).clamp(0.05, 0.98);
        let detune = 0.8 + 0.5 * shift;

        let mut modes: Vec<PanelMode> = Vec::with_capacity(PANEL_MODES.len());
        for m in PANEL_MODES.iter() {
            modes.push(PanelMode::new(m * detune, q, sr));
        }
        let modes: [PanelMode; 3] = modes.try_into().expect("ровно три моды");

        Self { modes }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let mut y = 0.0;
        for (mode, gain) in self.modes.iter_mut().zip(PANEL_GAINS.iter()) {
            y += mode.process(x) * gain;
        }
        (y * 0.6).clamp(-1.0, 1.0)
    }

    pub fn reset(&mut self) {
        for m in self.modes.iter_mut() {
            m.low = 0.0;
            m.band = 0.0;
        }
    }
}

/// 3. Panel Echo Resonance | 6. Resonance
pub fn apply_super_atmospheric_resonance(params: &SuperAtmosphericParams) {
    println!(
        "[DSP Resonance] SUPER APPLYING cold resonance... Level: {}, Panel Echo Depth: {}s | [Innovation] Panel Echo Resonance | [Improvement] Cold Resonance",
        params.resonance, params.panel_echo_depth
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_excites_panel_modes() {
        let p = SuperAtmosphericParams::default();
        let mut res = PanelResonator::new(&p, 48_000.0);
        let first = res.process(1.0);
        assert!(first > 0.0, "моды не возбудились: {}", first);

        let mut energy = 0.0;
        for _ in 0..4_800 {
            let s = res.process(0.0);
            energy += s * s;
            assert!(s.is_finite() && s.abs() <= 1.0);
        }
        assert!(energy > 0.0, "резонанс затух мгновенно");
    }

    #[test]
    fn low_resonance_decays_faster_than_high() {
        let sr = 48_000.0;
        let mut soft = SuperAtmosphericParams::default();
        soft.resonance = 0.0; // q = 0.98 → сильное затухание
        let mut hard = SuperAtmosphericParams::default();
        hard.resonance = 1.0; // q = 0.2 → долгое дребезжание

        let rms = |p: &SuperAtmosphericParams| {
            let mut res = PanelResonator::new(p, sr);
            res.process(1.0);
            let mut acc = 0.0;
            for _ in 0..9_600 {
                let s = res.process(0.0);
                acc += s * s;
            }
            (acc / 9_600.0).sqrt()
        };

        assert!(
            rms(&hard) > rms(&soft),
            "soft = {}, hard = {}",
            rms(&soft),
            rms(&hard)
        );
    }

    #[test]
    fn silence_stays_silent() {
        let p = SuperAtmosphericParams::default();
        let mut res = PanelResonator::new(&p, 48_000.0);
        for _ in 0..1_000 {
            assert_eq!(res.process(0.0), 0.0);
        }
    }
}
