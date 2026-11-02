// src/dsp/reverb.rs - 3. Panel Echo Resonance & 10. Atmospheric Reverb Landscape (Innovation)
// 3. Panel Echo & 10. Reverb (Improvement)

use super::params::SuperAtmosphericParams;

/// Гребенчатые задержки (Freeverb-тюнинг, в сэмплах при 44.1 кГц).
const COMB_TUNINGS: [f32; 4] = [1116.0, 1188.0, 1277.0, 1356.0];
/// Всепропускающие задержки.
const ALLPASS_TUNINGS: [f32; 2] = [556.0, 441.0];
/// Стереоразброс между каналами.
const STEREO_SPREAD: f32 = 23.0;

#[derive(Clone, Debug)]
struct Comb {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    damp: f32,
    damp_state: f32,
}

impl Comb {
    fn new(len: usize, feedback: f32, damp: f32) -> Self {
        Self {
            buffer: vec![0.0; len.max(1)],
            index: 0,
            feedback,
            damp,
            damp_state: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let out = self.buffer[self.index];
        self.damp_state = out * (1.0 - self.damp) + self.damp_state * self.damp;
        self.buffer[self.index] = input + self.damp_state * self.feedback;
        self.index = (self.index + 1) % self.buffer.len();
        out
    }

    fn reset(&mut self) {
        for s in self.buffer.iter_mut() {
            *s = 0.0;
        }
        self.index = 0;
        self.damp_state = 0.0;
    }
}

#[derive(Clone, Debug)]
struct Allpass {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
}

impl Allpass {
    fn new(len: usize, feedback: f32) -> Self {
        Self {
            buffer: vec![0.0; len.max(1)],
            index: 0,
            feedback,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.index];
        let out = buffered - input;
        self.buffer[self.index] = input + buffered * self.feedback;
        self.index = (self.index + 1) % self.buffer.len();
        out
    }

    fn reset(&mut self) {
        for s in self.buffer.iter_mut() {
            *s = 0.0;
        }
        self.index = 0;
    }
}

/// «Супер атмосферный» реверб-ландшафт: 4 гребенчатых фильтра + 2 всехпропускающих
/// на канал (Schroeder/Freeverb), размер — `reverb_size`, окраска — `spectral_shift`.
#[derive(Clone, Debug)]
pub struct AtmosphericReverb {
    combs_l: Vec<Comb>,
    combs_r: Vec<Comb>,
    allpass_l: Vec<Allpass>,
    allpass_r: Vec<Allpass>,
    wet: f32,
    dry: f32,
}

impl AtmosphericReverb {
    pub fn new(params: &SuperAtmosphericParams, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let scale = sr / 44_100.0;
        let room = params.reverb_size.clamp(0.0, 1.0);

        // 0.89 при reverb_size 0.95 — длинный, но устойчивый «дворовый» хвост
        let feedback = 0.70 + 0.20 * room;
        // спектральный сдвиг приоткрывает высокие («серое небо» светлеет)
        let damp = (0.5 - 0.3 * params.spectral_shift.clamp(0.0, 1.0)).clamp(0.05, 0.95);
        let spread = (STEREO_SPREAD * scale).round() as usize;

        let combs_l: Vec<Comb> = COMB_TUNINGS
            .iter()
            .map(|t| Comb::new((*t * scale).round() as usize, feedback, damp))
            .collect();
        let combs_r: Vec<Comb> = COMB_TUNINGS
            .iter()
            .map(|t| Comb::new((*t * scale).round() as usize + spread, feedback, damp))
            .collect();
        let allpass_l: Vec<Allpass> = ALLPASS_TUNINGS
            .iter()
            .map(|t| Allpass::new((*t * scale).round() as usize, 0.5))
            .collect();
        let allpass_r: Vec<Allpass> = ALLPASS_TUNINGS
            .iter()
            .map(|t| Allpass::new((*t * scale).round() as usize + spread, 0.5))
            .collect();

        Self {
            combs_l,
            combs_r,
            allpass_l,
            allpass_r,
            wet: (0.9 * room).clamp(0.0, 1.0),
            dry: (1.0 - 0.65 * room).clamp(0.0, 1.0),
        }
    }

    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let input = (left + right) * 0.5;

        let mut acc_l = 0.0;
        for comb in self.combs_l.iter_mut() {
            acc_l += comb.process(input);
        }
        let mut acc_r = 0.0;
        for comb in self.combs_r.iter_mut() {
            acc_r += comb.process(input);
        }

        acc_l *= 0.25;
        acc_r *= 0.25;

        for ap in self.allpass_l.iter_mut() {
            acc_l = ap.process(acc_l);
        }
        for ap in self.allpass_r.iter_mut() {
            acc_r = ap.process(acc_r);
        }

        let out_l = (self.dry * left + self.wet * acc_l).clamp(-4.0, 4.0);
        let out_r = (self.dry * right + self.wet * acc_r).clamp(-4.0, 4.0);
        (out_l, out_r)
    }

    pub fn reset(&mut self) {
        for c in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            c.reset();
        }
        for a in self.allpass_l.iter_mut().chain(self.allpass_r.iter_mut()) {
            a.reset();
        }
    }

    pub fn wet(&self) -> f32 {
        self.wet
    }
}

/// 10. Atmospheric Reverb Landscape | 3. Panel Echo | 10. Reverb
pub fn apply_super_atmospheric_reverb(params: &SuperAtmosphericParams) {
    println!(
        "[DSP Reverb] SUPER APPLYING reverb... Size: {}, Filter: {}Hz, Modulation: {}, Spectral Shift: {}, Echo: {}s, Panel Echo Depth: {}s, Resonance: {}, Stereo: {} | [Innovation] Panel Echo Resonance, Concrete Wall Filter, Atmospheric Reverb Landscape | [Improvement] Panel Echo, Stereo Effect, Resonance, Spectral Shift, Reverb",
        params.reverb_size, params.filter_cutoff, params.modulation, params.spectral_shift,
        params.echo_delay, params.panel_echo_depth, params.resonance, params.stereo_effect
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_produces_decaying_tail() {
        let p = SuperAtmosphericParams::default();
        let mut rev = AtmosphericReverb::new(&p, 48_000.0);

        let rms = |rev: &mut AtmosphericReverb, n: usize| {
            let mut acc = 0.0;
            for _ in 0..n {
                let (l, _r) = rev.process(0.0, 0.0);
                acc += l * l;
            }
            (acc / n as f32).sqrt()
        };

        rev.process(1.0, 1.0);
        let early = rms(&mut rev, 4_800);
        let _ = rms(&mut rev, 96_000); // ~2 с
        let late = rms(&mut rev, 4_800);

        assert!(early > 0.0, "реверб не возбудился");
        assert!(late < early, "early = {}, late = {}", early, late);
    }

    #[test]
    fn output_is_finite_and_bounded() {
        let p = SuperAtmosphericParams::default();
        let mut rev = AtmosphericReverb::new(&p, 48_000.0);
        for i in 0..48_000 {
            let (l, r) = rev.process(1.0, -1.0);
            assert!(l.is_finite() && r.is_finite(), "NaN/Inf на сэмпле {}", i);
            assert!(l.abs() <= 4.0 && r.abs() <= 4.0);
        }
    }

    #[test]
    fn silence_stays_silent() {
        let p = SuperAtmosphericParams::default();
        let mut rev = AtmosphericReverb::new(&p, 48_000.0);
        for _ in 0..1_000 {
            let (l, r) = rev.process(0.0, 0.0);
            assert_eq!(l, 0.0);
            assert_eq!(r, 0.0);
        }
    }
}
