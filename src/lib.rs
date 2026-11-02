#![crate_type = "cdylib"]

use nih_plug::{self, AudioPlugin, AudioProcessor, Controller, Processor, ControllerValue, ProcessorValue};
use egui::{CentralPanel, Ui};
use std::sync::{Arc, Mutex};

// 12 Крутых Фишек (Innovations v1.1.1) - Rust Implementation
// 1. SW Wind Gate
// 2. Brutalist Texture Mapping
// 3. Panel Echo Resonance
// 4. Post-Punk Sync
// 5. Courtyard Stereo Expansion
// 6. Granular Noise Storm
// 7. Concrete Wall Filter
// 8. Wind Dynamics Modulation
// 9. Post-Punk Distortion Engine
// 10. Atmospheric Reverb Landscape
// 11. Grey Sky GUI (Brutalist / Fonts: Display, Mono-Bold, Mono-Regular)
// 12. FL Studio Preset Bridge (JSON presets)

// 12 Улучшений (Improvements v1.1.1)
// 1. GUI Palette | 2. Bass Optimization | 3. Panel Echo | 4. Granularity | 5. Stereo Effect
// 6. Resonance | 7. Distortion | 8. Modulation | 9. Spectral Shift | 10. Reverb
// 11. FL Studio Integration | 12. Documentation

pub mod dsp;
pub mod editor;

// Параметры живут в DSP-ядре (src/dsp/params.rs), чтобы ядро собиралось
// без внешних зависимостей (nih-plug / egui) — см. tools/dsp_standalone.
pub use crate::dsp::{params::SuperAtmosphericParams, SuperAtmosphericEngine};

pub struct PostAmbientProcessor {
    params: Arc<Mutex<SuperAtmosphericParams>>,
    // DSP-ядро: вся «супер атмосферная» цепочка (12 инноваций + 12 улучшений)
    engine: SuperAtmosphericEngine,
    // Состояние GUI (11. Grey Sky GUI)
    gui_theme: String,
    gui_config: String,
    // 12. FL Studio Preset Bridge
    presets_path: String,
}

impl PostAmbientProcessor {
    pub fn new() -> Self {
        Self {
            params: Arc::new(Mutex::new(SuperAtmosphericParams::default())),
            engine: SuperAtmosphericEngine::new(
                SuperAtmosphericParams::default(),
                dsp::DEFAULT_SAMPLE_RATE,
            ),
            gui_theme: "brutalist_grey_sky".to_string(),
            gui_config: "./gui/config_brutalist_grey_sky.json".to_string(),
            presets_path: "./presets/super_atmospheric_v1_1_1.json".to_string(),
        }
    }
}

impl PostAmbientProcessor {
    /// Вызывать из `Plugin::initialize`, когда хост сообщил частоту дискретизации.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.engine.set_sample_rate(sample_rate);
    }
}

impl Processor for PostAmbientProcessor {
    fn process(&mut self, context: &mut nih_plug::ProcessContext) {
        // Параметры берём под замком и сразу отдаём движку (без перестройки линий задержки,
        // чтобы автоматизация не сбрасывала хвосты эха и реверба).
        let params = *self.params.lock().unwrap();
        self.engine.update_params(params);
        let buffer = context.buffer();
        
        // Логика обработки (аналог JS process v1.1.1)
        // 1. SW Wind Gate & Granular Noise Storm (Innovation 1, 6 / Improvement 4, 8)
        // 2. Brutalist Texture Mapping & Post-Punk Distortion Engine (Innovation 2, 9 / Improvement 7)
        // 3. Panel Echo Resonance & Panel Echo (Innovation 3 / Improvement 3)
        // 4. Post-Punk Sync (Innovation 4 / Improvement 3)
        // 5. Courtyard Stereo Expansion (Innovation 5 / Improvement 5)
        // 6. Concrete Wall Filter (Innovation 7 / Improvement 9)
        // 7. Wind Dynamics Modulation (Innovation 8 / Improvement 8)
        // 8. Atmospheric Reverb Landscape (Innovation 10 / Improvement 10)
        
        // В реальном коде здесь будет DSP-обработка (с использованием nih-plug)
        // Для прототипа в ветке arena/139069db-post-ambient-vst:
        println!(
            "[Rust Process] SUPER ATMOSPHERIC audio buffer processed (v1.1.1) | Theme: {} | Config: {} | Presets: {} | Innovations: 12 | Improvements: 12",
            self.gui_theme, self.gui_config, self.presets_path
        );
        
        // Обработка буфера: реальная DSP-цепочка (src/dsp)
        // noise_level, bass_frequency, reverb_size, filter_cutoff, distortion, modulation,
        // granularity, tempo, resonance, stereo_effect, echo_delay, spectral_shift,
        // brutalist_texture, southwest_wind, panel_echo_depth
        let channels = buffer.num_channels();
        let samples = buffer.num_samples();
        if channels == 0 {
            return;
        }
        for j in 0..samples {
            let input_l = buffer.channel_mut(0)[j];
            let input_r = if channels > 1 {
                buffer.channel_mut(1)[j]
            } else {
                input_l
            };
            let (out_l, out_r) = self.engine.process(input_l, input_r);
            buffer.channel_mut(0)[j] = out_l;
            if channels > 1 {
                buffer.channel_mut(1)[j] = out_r;
            }
        }
    }
}

impl Controller for PostAmbientProcessor {
    fn render(&mut self, ui: &mut Ui) {
        CentralPanel::default().show(ui, |ui| {
            ui.heading(&format!("{} v1.1.1", "Post Ambient VST (Southwest St. Petersburg - Super Atmospheric)"));
            ui.label(&format!("Style: Post-Punk 2000s / Super Ambient / Noise / Bass / Brutalist"));
            ui.label(&format!("Inspiration: Panel houses and small three-story houses of Southwest St. Petersburg"));
            ui.label(&format!("Theme: {} (Brutalist / Grey Sky / Fonts: Display, Mono-Bold, Mono-Regular)", self.gui_theme));
            ui.label(&format!("Config: {}", self.gui_config));
            ui.label(&format!("Presets: {}", self.presets_path));
            
            let mut params = self.params.lock().unwrap();
            ui.label(&format!("Noise Level: {} | Granularity: {} | Southwest Wind: {}", params.noise_level, params.granularity, params.southwest_wind));
            ui.label(&format!("Bass Frequency: {}Hz | Distortion: {} | Brutalist Texture: {} | Tempo: {} BPM", params.bass_frequency, params.distortion, params.brutalist_texture, params.tempo));
            ui.label(&format!("Reverb Size: {} | Filter: {}Hz | Modulation: {} | Spectral Shift: {}", params.reverb_size, params.filter_cutoff, params.modulation, params.spectral_shift));
            ui.label(&format!("Resonance: {} | Stereo: {} | Echo: {}s | Panel Echo Depth: {}s", params.resonance, params.stereo_effect, params.echo_delay, params.panel_echo_depth));
            
            ui.label("--- 12 Innovations ---");
            ui.label("1. SW Wind Gate | 2. Brutalist Texture Mapping | 3. Panel Echo Resonance | 4. Post-Punk Sync | 5. Courtyard Stereo Expansion | 6. Granular Noise Storm | 7. Concrete Wall Filter | 8. Wind Dynamics Modulation | 9. Post-Punk Distortion Engine | 10. Atmospheric Reverb Landscape | 11. Grey Sky GUI | 12. FL Studio Preset Bridge");
            
            ui.label("--- 12 Improvements ---");
            ui.label("1. GUI Palette | 2. Bass Optimization | 3. Panel Echo | 4. Granularity | 5. Stereo Effect | 6. Resonance | 7. Distortion | 8. Modulation | 9. Spectral Shift | 10. Reverb | 11. FL Studio Integration | 12. Documentation");
            
            ui.label("--- Brutalist Grey Sky Theme (Fonts: Display, Mono-Bold, Mono-Regular) ---");
        });
    }
}

nih_plug::plugin!(
    PostAmbientPlugin,
    name = "Post Ambient VST (Southwest St. Petersburg - Super Atmospheric)",
    version = "1.1.1",
    category = "Instrument",
    manufacturer = "cherdakmd",
    controller = PostAmbientProcessor,
    processor = PostAmbientProcessor,
);
