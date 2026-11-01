// tools/dsp_standalone/src/main.rs
// Автономная (без внешних зависимостей) проверка DSP-ядра плагина:
//   cd tools/dsp_standalone && cargo run --release
//   cd tools/dsp_standalone && cargo test
//
// Ядро подключается прямо из ../../../src/dsp, поэтому никакого дублирования
// кода нет: собирается тот же самый Rust-код, что и в самом плагине.

#[path = "../../../src/dsp/mod.rs"]
mod dsp;

use dsp::{filter::ConcreteWallFilter, SuperAtmosphericParams, DEFAULT_SAMPLE_RATE};

fn main() {
    let params = SuperAtmosphericParams::default();

    println!("=== Post Ambient VST (Southwest St. Petersburg - Super Atmospheric) v1.1.1 ===");
    println!("Style: Post-Punk 2000s / Super Ambient / Noise / Bass / Brutalist");
    println!("Theme: brutalist_grey_sky (Grey Sky / Concrete / Fonts: Display, Mono-Bold, Mono-Regular)");
    println!("Sample rate: {} Hz", DEFAULT_SAMPLE_RATE);
    println!();

    // Вся цепочка: 12 инноваций + 12 улучшений
    dsp::apply_super_atmospheric_logic_at(&params, DEFAULT_SAMPLE_RATE);

    println!();
    println!("--- Timing (4. Post-Punk Sync / 3. Panel Echo Resonance) ---");
    println!("Beat: {:.6} s", params.beat_seconds());
    println!("Echo delay: {} samples", params.echo_delay_samples(DEFAULT_SAMPLE_RATE));
    println!(
        "Panel echo depth: {} samples",
        params.panel_echo_depth_samples(DEFAULT_SAMPLE_RATE)
    );

    println!();
    println!("--- 7. Concrete Wall Filter (real DSP smoke test) ---");
    let mut wall = ConcreteWallFilter::new(&params, DEFAULT_SAMPLE_RATE);
    let impulse = [1.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let tail: Vec<f32> = impulse.iter().map(|s| wall.process(*s)).collect();
    println!("Impulse -> {:?}", impulse);
    println!(
        "Concrete wall tail -> [{}]",
        tail.iter()
            .map(|s| format!("{:.5}", s))
            .collect::<Vec<_>>()
            .join(", ")
    );

    println!();
    println!("--- 12 Innovations ---");
    println!("1. SW Wind Gate | 2. Brutalist Texture Mapping | 3. Panel Echo Resonance | 4. Post-Punk Sync | 5. Courtyard Stereo Expansion | 6. Granular Noise Storm | 7. Concrete Wall Filter | 8. Wind Dynamics Modulation | 9. Post-Punk Distortion Engine | 10. Atmospheric Reverb Landscape | 11. Grey Sky GUI | 12. FL Studio Preset Bridge");
    println!("--- 12 Improvements ---");
    println!("1. GUI Palette | 2. Bass Optimization | 3. Panel Echo | 4. Granularity | 5. Stereo Effect | 6. Resonance | 7. Distortion | 8. Modulation | 9. Spectral Shift | 10. Reverb | 11. FL Studio Integration | 12. Documentation");
    println!();
    println!(
        "DSP core compiled and executed successfully (Rust >= {}, toolchain: see `rustc --version`).",
        env!("CARGO_PKG_RUST_VERSION")
    );
}
