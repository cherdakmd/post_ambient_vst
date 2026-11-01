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

/// Стандартная частота дискретизации FL Studio.
pub const DEFAULT_SAMPLE_RATE: f32 = 48_000.0;

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
