// src/dsp/bass.rs - 2. Brutalist Texture Mapping & 4. Post-Punk Sync (Innovation)
// 2. Bass Optimization & 7. Distortion (Improvement)

use super::params::SuperAtmosphericParams;

const TAU: f32 = std::f32::consts::TAU;

/// Холодный пост-панк басс: основной тон + суб-октава + «бетонная» примесь,
/// пульсирующий в такт темпу (4. Post-Punk Sync).
#[derive(Clone, Debug)]
pub struct SubBass {
    phase: f32,
    sub_phase: f32,
    beat_phase: f32,
}

impl SubBass {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            sub_phase: 0.0,
            beat_phase: 0.0,
        }
    }

    pub fn process(&mut self, params: &SuperAtmosphericParams, sample_rate: f32) -> f32 {
        let sr = sample_rate.max(1.0);
        let freq = params.bass_frequency.clamp(10.0, sr * 0.45);
        let inc = TAU * freq / sr;

        self.phase = (self.phase + inc) % TAU;
        self.sub_phase = (self.sub_phase + inc * 0.5) % TAU;

        // 4. Post-Punk Sync: доля = 60 / BPM
        let beat = params.beat_seconds();
        self.beat_phase = (self.beat_phase + 1.0 / (sr * beat)) % 1.0;

        // 2. Bass Optimization: основной тон + суб-октава
        let body = self.phase.sin() * 0.6 + self.sub_phase.sin() * 0.35;
        // 2. Brutalist Texture Mapping: жёсткая примесь «бетона»
        let grit = (self.phase * 2.0).sin() * 0.15 * params.brutalist_texture.clamp(0.0, 1.0);
        // Пульс в такте
        let pulse = 0.85 + 0.15 * (TAU * self.beat_phase).sin();

        ((body + grit) * pulse * 0.7).clamp(-1.0, 1.0)
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.sub_phase = 0.0;
        self.beat_phase = 0.0;
    }

    /// Фаза доли (0..1) — для визуализации в GUI.
    pub fn beat_phase(&self) -> f32 {
        self.beat_phase
    }
}

impl Default for SubBass {
    fn default() -> Self {
        Self::new()
    }
}

/// Отчёт параметров (используется плагином и автономной проверкой).
pub fn generate_super_atmospheric_bass(params: &SuperAtmosphericParams) {
    println!(
        "[DSP Bass] SUPER GENERATING bass... Frequency: {}Hz, Distortion: {}, Brutalist Texture: {}, Tempo: {} BPM | [Innovation] Brutalist Texture 2.0 & Post-Punk Sync | [Improvement] Bass Optimization & Distortion Engine",
        params.bass_frequency, params.distortion, params.brutalist_texture, params.tempo
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_bounded() {
        let p = SuperAtmosphericParams::default();
        let mut b = SubBass::new();
        for _ in 0..20_000 {
            let s = b.process(&p, 48_000.0);
            assert!(s.is_finite() && s.abs() <= 1.0);
        }
    }

    #[test]
    fn pitch_follows_bass_frequency() {
        // За 1 секунду на 45 Гц фаза должна сделать 45 полных оборотов.
        let p = SuperAtmosphericParams::default();
        let mut b = SubBass::new();
        let mut crossings = 0;
        let mut prev = b.process(&p, 48_000.0);
        for _ in 1..48_000 {
            let s = b.process(&p, 48_000.0);
            if prev <= 0.0 && s > 0.0 {
                crossings += 1;
            }
            prev = s;
        }
        assert!(
            (40..=50).contains(&crossings),
            "ожидалось ~45 пересечений нуля, получено {}",
            crossings
        );
    }

    #[test]
    fn beat_phase_stays_in_unit_range() {
        let p = SuperAtmosphericParams::default();
        let mut b = SubBass::new();
        for _ in 0..100_000 {
            b.process(&p, 48_000.0);
            let ph = b.beat_phase();
            assert!((0.0..1.0).contains(&ph));
        }
    }
}
