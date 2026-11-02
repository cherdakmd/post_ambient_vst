// src/dsp/params.rs - «Супер атмосферные» параметры (Rust & JavaScript)
// Единое определение параметров, чтобы DSP-ядро можно было собирать
// без внешних зависимостей (nih-plug / egui) — см. tools/dsp_standalone.

/// 12 Крутых Фишек (Innovations v1.1.1)
/// 1. SW Wind Gate | 2. Brutalist Texture Mapping | 3. Panel Echo Resonance
/// 4. Post-Punk Sync | 5. Courtyard Stereo Expansion | 6. Granular Noise Storm
/// 7. Concrete Wall Filter | 8. Wind Dynamics Modulation
/// 9. Post-Punk Distortion Engine | 10. Atmospheric Reverb Landscape
/// 11. Grey Sky GUI | 12. FL Studio Preset Bridge
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SuperAtmosphericParams {
    // Базовые (v1.1.0)
    pub noise_level: f32,       // 0.8
    pub bass_frequency: f32,    // 45Hz
    pub reverb_size: f32,       // 0.95
    pub filter_cutoff: f32,     // 800Hz
    // v1.0.1
    pub distortion: f32,        // 0.6
    pub modulation: f32,        // 0.5
    pub granularity: f32,       // 0.9
    // v1.0.2
    pub tempo: f32,             // 90 BPM
    pub resonance: f32,         // 0.7
    pub stereo_effect: f32,     // 0.9
    pub echo_delay: f32,        // 2.5s
    // v1.1.0 — «супер атмосферные»
    pub spectral_shift: f32,    // 0.85
    pub brutalist_texture: f32, // 0.95
    pub southwest_wind: f32,    // 0.75
    pub panel_echo_depth: f32,  // 3.0s
}

impl Default for SuperAtmosphericParams {
    fn default() -> Self {
        Self {
            noise_level: 0.8,
            bass_frequency: 45.0,
            reverb_size: 0.95,
            filter_cutoff: 800.0,
            distortion: 0.6,
            modulation: 0.5,
            granularity: 0.9,
            tempo: 90.0,
            resonance: 0.7,
            stereo_effect: 0.9,
            echo_delay: 2.5,
            spectral_shift: 0.85,
            brutalist_texture: 0.95,
            southwest_wind: 0.75,
            panel_echo_depth: 3.0,
        }
    }
}

impl SuperAtmosphericParams {
    /// Длина одного бита в секундах (4. Post-Punk Sync, 90 BPM).
    pub fn beat_seconds(&self) -> f32 {
        if self.tempo <= 0.0 {
            0.0
        } else {
            60.0 / self.tempo
        }
    }

    /// Задержка эха в сэмплах (3. Panel Echo Resonance).
    pub fn echo_delay_samples(&self, sample_rate: f32) -> usize {
        (self.echo_delay * sample_rate).max(0.0) as usize
    }

    /// Глубина эха панелей в сэмплах (10. Atmospheric Reverb Landscape).
    pub fn panel_echo_depth_samples(&self, sample_rate: f32) -> usize {
        (self.panel_echo_depth * sample_rate).max(0.0) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::SuperAtmosphericParams;

    #[test]
    fn default_is_super_atmospheric() {
        let p = SuperAtmosphericParams::default();
        assert_eq!(p.noise_level, 0.8);
        assert_eq!(p.bass_frequency, 45.0);
        assert_eq!(p.reverb_size, 0.95);
        assert_eq!(p.filter_cutoff, 800.0);
        assert_eq!(p.distortion, 0.6);
        assert_eq!(p.modulation, 0.5);
        assert_eq!(p.granularity, 0.9);
        assert_eq!(p.tempo, 90.0);
        assert_eq!(p.resonance, 0.7);
        assert_eq!(p.stereo_effect, 0.9);
        assert_eq!(p.echo_delay, 2.5);
        assert_eq!(p.spectral_shift, 0.85);
        assert_eq!(p.brutalist_texture, 0.95);
        assert_eq!(p.southwest_wind, 0.75);
        assert_eq!(p.panel_echo_depth, 3.0);
    }

    #[test]
    fn post_punk_sync_90_bpm() {
        let p = SuperAtmosphericParams::default();
        assert!((p.beat_seconds() - 0.666_666_7).abs() < 1e-6);
    }

    #[test]
    fn echo_timing_in_samples() {
        let p = SuperAtmosphericParams::default();
        assert_eq!(p.echo_delay_samples(48_000.0), 120_000);
        assert_eq!(p.panel_echo_depth_samples(48_000.0), 144_000);
    }
}
