// src/dsp/mod.rs - «Супер атмосферная» логика (Rust & JavaScript)
// 12 Крутых Фишек (Innovations v1.1.1)
// 12 Улучшений (Improvements v1.1.1)

pub mod params;     // Единое определение SuperAtmosphericParams (без внешних зависимостей)
pub mod noise;      // 1. SW Wind Gate | 6. Granular Noise Storm | 4. Granularity | 8. Wind Dynamics Modulation
pub mod bass;       // 2. Brutalist Texture Mapping | 4. Post-Punk Sync | 2. Bass Optimization | 7. Distortion
pub mod reverb;     // 3. Panel Echo Resonance | 10. Atmospheric Reverb Landscape | 3. Panel Echo | 10. Reverb
pub mod filter;     // 7. Concrete Wall Filter | 9. Spectral Shift | 9. Spectral Shift
pub mod distortion; // 9. Post-Punk Distortion Engine | 7. Distortion
pub mod modulation; // 8. Wind Dynamics Modulation | 8. Modulation
pub mod tempo;      // 4. Post-Punk Sync | 3. Panel Echo
pub mod resonance;  // 3. Panel Echo Resonance | 6. Resonance
pub mod stereo;     // 5. Courtyard Stereo Expansion | 5. Stereo Effect
pub mod echo;       // 3. Panel Echo Resonance | 10. Atmospheric Reverb Landscape | 3. Panel Echo | 10. Reverb

pub use self::params::SuperAtmosphericParams;

use self::bass::SubBass;
use self::distortion::{post_punk_shape, soft_clip};
use self::echo::PanelEcho;
use self::filter::ResonantWallFilter;
use self::modulation::WindLfo;
use self::noise::WindNoise;
use self::resonance::PanelResonator;
use self::reverb::AtmosphericReverb;
use self::stereo::widen;

/// Стандартная частота дискретизации FL Studio.
pub const DEFAULT_SAMPLE_RATE: f32 = 48_000.0;

/// Полная «супер атмосферная» цепочка (12 инноваций + 12 улучшений).
///
/// Сигнал: вход (или тишина) → шум/басс → дисторшн → фильтр «бетонной стены»
/// → резонансы панелей → эхо панельного дома → реверб-ландшафт → стерео → лимитер.
#[derive(Clone, Debug)]
pub struct SuperAtmosphericEngine {
    sample_rate: f32,
    params: SuperAtmosphericParams,
    noise: WindNoise,
    bass: SubBass,
    wall: ResonantWallFilter,
    resonator: PanelResonator,
    lfo: WindLfo,
    echo: PanelEcho,
    reverb: AtmosphericReverb,
}

impl SuperAtmosphericEngine {
    pub fn new(params: SuperAtmosphericParams, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        Self {
            sample_rate: sr,
            params,
            noise: WindNoise::new(0x5A17),
            bass: SubBass::new(),
            wall: ResonantWallFilter::new(&params, sr),
            resonator: PanelResonator::new(&params, sr),
            lfo: WindLfo::new(),
            echo: PanelEcho::new(&params, sr),
            reverb: AtmosphericReverb::new(&params, sr),
        }
    }

    /// Обновить параметры и перестроить все зависящие от частоты дискретизации узлы
    /// (используется при смене пресета: хвосты эха/реверба сбрасываются).
    pub fn set_params(&mut self, params: SuperAtmosphericParams) {
        self.params = params;
        self.retune();
    }

    /// «Горячая» автоматизация параметров без перестройки линий задержки.
    pub fn update_params(&mut self, params: SuperAtmosphericParams) {
        self.params = params;
    }

    /// Сменить частоту дискретизации хоста (вызывать из `initialize`).
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.retune();
    }

    /// Пересчитать коэффициенты фильтров и размеры линий задержки.
    pub fn retune(&mut self) {
        self.wall = ResonantWallFilter::new(&self.params, self.sample_rate);
        self.resonator = PanelResonator::new(&self.params, self.sample_rate);
        self.echo = PanelEcho::new(&self.params, self.sample_rate);
        self.reverb = AtmosphericReverb::new(&self.params, self.sample_rate);
    }

    pub fn params(&self) -> &SuperAtmosphericParams {
        &self.params
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Один сэмпл стерео.
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let p = self.params;
        let dry = (left + right) * 0.5;

        // 1. SW Wind Gate | 6. Granular Noise Storm
        let wind = self.noise.process(&p, self.sample_rate);
        // 2. Brutalist Texture Mapping | 4. Post-Punk Sync
        let bass = self.bass.process(&p, self.sample_rate);

        let mut mono = dry * 0.5 + wind * 0.7 + bass * 0.32;

        // 9. Post-Punk Distortion Engine
        mono = post_punk_shape(mono, p.distortion, p.brutalist_texture);
        // 7. Concrete Wall Filter (75% «бетон» + 25% исходника, чтобы остался воздух)
        let walled = self.wall.process(mono);
        mono = walled * 0.75 + mono * 0.25;
        // 3. Panel Echo Resonance
        mono = mono * 0.8 + self.resonator.process(mono) * 0.35;
        // 8. Wind Dynamics Modulation
        mono *= self.lfo.process(&p, self.sample_rate);
        // 3. Panel Echo
        mono = self.echo.process(mono);
        // 10. Atmospheric Reverb Landscape
        let (l, r) = self.reverb.process(mono, mono);
        // 5. Courtyard Stereo Expansion
        let (l, r) = widen(l, r, p.stereo_effect);

        (soft_clip(l * 0.75), soft_clip(r * 0.75))
    }

    /// Обработка буфера (удобно для `process()` плагина).
    pub fn process_buffer(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        for i in 0..n {
            let (l, r) = self.process(left[i], right[i]);
            left[i] = l;
            right[i] = r;
        }
    }

    pub fn reset(&mut self) {
        self.noise.reset();
        self.bass.reset();
        self.wall.reset();
        self.resonator.reset();
        self.lfo.reset();
        self.echo.reset();
        self.reverb.reset();
    }
}

/// Применить всю «супер атмосферную» цепочку (12 инноваций + 12 улучшений).
#[allow(dead_code)] // используется плагином (src/lib.rs), здесь — публичное API ядра
pub fn apply_super_atmospheric_logic(params: &SuperAtmosphericParams) {
    apply_super_atmospheric_logic_at(params, DEFAULT_SAMPLE_RATE);
}

/// То же самое с явной частотой дискретизации.
pub fn apply_super_atmospheric_logic_at(params: &SuperAtmosphericParams, sample_rate: f32) {
    println!("[DSP] Applying 12 Innovations & 12 Improvements: SW Wind Gate, Brutalist Texture Mapping, Panel Echo Resonance, Post-Punk Sync, Courtyard Stereo Expansion, Granular Noise Storm, Concrete Wall Filter, Wind Dynamics Modulation, Post-Punk Distortion Engine, Atmospheric Reverb Landscape, Grey Sky GUI, FL Studio Preset Bridge");
    println!(
        "[DSP] Parameters: Noise {} | Bass {}Hz | Reverb {} | Filter {}Hz | Distortion {} | Modulation {} | Granularity {} | Tempo {} | Resonance {} | Stereo {} | Echo {}s | Spectral Shift {} | Brutalist Texture {} | Southwest Wind {} | Panel Echo Depth {}s",
        params.noise_level, params.bass_frequency, params.reverb_size, params.filter_cutoff,
        params.distortion, params.modulation, params.granularity, params.tempo,
        params.resonance, params.stereo_effect, params.echo_delay, params.spectral_shift,
        params.brutalist_texture, params.southwest_wind, params.panel_echo_depth
    );

    self::noise::generate_super_atmospheric_noise(params);
    self::bass::generate_super_atmospheric_bass(params);
    self::filter::apply_super_atmospheric_filter(params, sample_rate);
    self::distortion::apply_super_atmospheric_distortion(params);
    self::modulation::apply_super_atmospheric_modulation(params);
    self::tempo::apply_super_atmospheric_tempo(params);
    self::resonance::apply_super_atmospheric_resonance(params);
    self::stereo::apply_super_atmospheric_stereo(params);
    self::echo::apply_super_atmospheric_echo(params);
    self::reverb::apply_super_atmospheric_reverb(params);

    println!(
        "[DSP] Buffer processed successfully with all post-punk, Southwest St. Petersburg parameters, 12 innovations, and 12 improvements (v1.1.1)."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_generates_sound_from_silence() {
        let p = SuperAtmosphericParams::default();
        let mut engine = SuperAtmosphericEngine::new(p, 48_000.0);

        let mut sum = 0.0;
        let mut peak = 0.0f32;
        for _ in 0..48_000 {
            let (l, r) = engine.process(0.0, 0.0);
            assert!(l.is_finite() && r.is_finite());
            assert!(l.abs() <= 1.0 && r.abs() <= 1.0);
            sum += l * l + r * r;
            peak = peak.max(l.abs().max(r.abs()));
        }
        let rms = (sum / 96_000.0).sqrt();
        assert!(rms > 0.01, "движок слишком тихий: rms = {}", rms);
        assert!(peak > 0.1, "пик слишком мал: {}", peak);
    }

    #[test]
    fn engine_is_stable_over_long_run() {
        let p = SuperAtmosphericParams::default();
        let mut engine = SuperAtmosphericEngine::new(p, 48_000.0);
        for i in 0..480_000 {
            let (l, r) = engine.process(0.5, -0.5);
            assert!(l.is_finite() && r.is_finite(), "нестабильность на сэмпле {}", i);
            assert!(l.abs() <= 1.0 && r.abs() <= 1.0, "перегруз на сэмпле {}", i);
        }
    }

    #[test]
    fn preset_change_retunes_without_panic() {
        let p = SuperAtmosphericParams::default();
        let mut engine = SuperAtmosphericEngine::new(p, 48_000.0);
        for _ in 0..4_800 {
            engine.process(0.0, 0.0);
        }
        let mut other = SuperAtmosphericParams::default();
        other.echo_delay = 1.25;
        other.reverb_size = 0.5;
        other.tempo = 120.0;
        engine.set_params(other);
        for _ in 0..4_800 {
            let (l, r) = engine.process(0.0, 0.0);
            assert!(l.is_finite() && r.is_finite());
        }
    }

    #[test]
    fn silence_input_still_returns_after_reset() {
        let p = SuperAtmosphericParams::default();
        let mut engine = SuperAtmosphericEngine::new(p, 48_000.0);
        for _ in 0..4_800 {
            engine.process(1.0, 1.0);
        }
        engine.reset();
        let (l, r) = engine.process(0.0, 0.0);
        assert!(l.abs() < 0.5 && r.abs() < 0.5, "l = {}, r = {}", l, r);
    }
}
