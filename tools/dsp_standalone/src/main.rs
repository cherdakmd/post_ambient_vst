// tools/dsp_standalone/src/main.rs
// Автономная (без внешних зависимостей) проверка DSP-ядра плагина:
//   cd tools/dsp_standalone && cargo test            // юнит-тесты DSP
//   cd tools/dsp_standalone && cargo run --release   // рендер WAV + отчёт
//   cd tools/dsp_standalone && cargo run --release -- 12  // 12 секунд
//
// Ядро подключается прямо из ../../../src/dsp, поэтому никакого дублирования
// кода нет: собирается тот же самый Rust-код, что и в самом плагине.

#[path = "../../../src/dsp/mod.rs"]
mod dsp;

use dsp::{
    filter::ConcreteWallFilter, SuperAtmosphericEngine, SuperAtmosphericParams, DEFAULT_SAMPLE_RATE,
};
use std::io::Write;

/// Запись 16-битного стерео WAV (только std, без внешних крейтов).
fn write_wav(path: &str, samples: &[(f32, f32)], sample_rate: u32) -> std::io::Result<()> {
    let channels: u16 = 2;
    let bits: u16 = 16;
    let data_bytes = samples.len() * channels as usize * (bits as usize / 8);

    let mut buf: Vec<u8> = Vec::with_capacity(44 + data_bytes);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&channels.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(
        &(sample_rate * channels as u32 * (bits as u32 / 8)).to_le_bytes(), // byte rate
    );
    buf.extend_from_slice(&(channels * bits / 8).to_le_bytes()); // block align
    buf.extend_from_slice(&bits.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&(data_bytes as u32).to_le_bytes());

    for (l, r) in samples {
        for s in [*l, *r] {
            let v = (s.clamp(-1.0, 1.0) * 32_767.0) as i16;
            buf.extend_from_slice(&v.to_le_bytes());
        }
    }

    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::File::create(path)?;
    file.write_all(&buf)
}

fn main() {
    let duration_secs: f32 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(8.0);
    let sample_rate = DEFAULT_SAMPLE_RATE;
    let total = (duration_secs * sample_rate) as usize;

    let params = SuperAtmosphericParams::default();

    println!("=== Post Ambient VST (Southwest St. Petersburg - Super Atmospheric) ===");
    println!("Style: Post-Punk 2000s / Super Ambient / Noise / Bass / Brutalist");
    println!("Theme: brutalist_grey_sky (Grey Sky / Concrete / Fonts: Display, Mono-Bold, Mono-Regular)");
    println!("Sample rate: {} Hz | Duration: {:.1} s", sample_rate, duration_secs);
    println!();

    // Отчёт по параметрам (12 инноваций + 12 улучшений)
    dsp::apply_super_atmospheric_logic_at(&params, sample_rate);

    println!();
    println!("--- Timing (4. Post-Punk Sync / 3. Panel Echo Resonance) ---");
    println!("Beat: {:.6} s", params.beat_seconds());
    println!("Echo delay: {} samples", params.echo_delay_samples(sample_rate));
    println!(
        "Panel echo depth: {} samples",
        params.panel_echo_depth_samples(sample_rate)
    );

    println!();
    println!("--- 7. Concrete Wall Filter (smoke test) ---");
    let mut wall = ConcreteWallFilter::new(&params, sample_rate);
    let impulse = [1.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let tail: Vec<f32> = impulse.iter().map(|s| wall.process(*s)).collect();
    println!(
        "Concrete wall tail -> [{}]",
        tail.iter()
            .map(|s| format!("{:.5}", s))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Реальный рендер: движок сам генерирует атмосферу (вход — тишина)
    println!();
    println!("--- Rendering {} samples of SUPER ATMOSPHERIC ambience ---", total);
    let mut engine = SuperAtmosphericEngine::new(params, sample_rate);
    let mut rendered: Vec<(f32, f32)> = Vec::with_capacity(total);
    let mut peak = 0.0f32;
    let mut sum = 0.0f64;
    let mut clipped = 0usize;

    for _ in 0..total {
        let (l, r) = engine.process(0.0, 0.0);
        peak = peak.max(l.abs().max(r.abs()));
        sum += (l as f64) * (l as f64) + (r as f64) * (r as f64);
        if l.abs() >= 0.999 || r.abs() >= 0.999 {
            clipped += 1;
        }
        rendered.push((l, r));
    }

    let rms = (sum / (total as f64 * 2.0)).sqrt() as f32;
    let out_path = "../../output/post_ambient_super_atmospheric.wav";
    match write_wav(out_path, &rendered, sample_rate as u32) {
        Ok(()) => println!("WAV written: {} ({:.1} s)", out_path, duration_secs),
        Err(e) => println!("WAV not written: {}", e),
    }

    println!("Peak: {:.4} | RMS: {:.4} | Clipped samples: {}", peak, rms, clipped);

    // Профиль громкости ( RMS по блокам )
    let blocks = 64;
    let block_len = (total / blocks).max(1);
    let levels: Vec<f32> = (0..blocks)
        .map(|b| {
            let start = b * block_len;
            let end = ((b + 1) * block_len).min(total);
            if end <= start {
                return 0.0;
            }
            let acc: f64 = rendered[start..end]
                .iter()
                .map(|(l, r)| (*l as f64) * (*l as f64) + (*r as f64) * (*r as f64))
                .sum();
            (acc / ((end - start) as f64 * 2.0)).sqrt() as f32
        })
        .collect();
    let max_level = levels.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let bars = "▁▂▃▄▅▆▇█";
    let spark: String = levels
        .iter()
        .map(|v| {
            let idx = ((v / max_level) * (bars.chars().count() as f32 - 1.0)).round() as usize;
            bars.chars().nth(idx).unwrap_or('▁')
        })
        .collect();
    println!("Level profile: {}", spark);

    println!();
    println!("--- 12 Innovations ---");
    println!("1. SW Wind Gate | 2. Brutalist Texture Mapping | 3. Panel Echo Resonance | 4. Post-Punk Sync | 5. Courtyard Stereo Expansion | 6. Granular Noise Storm | 7. Concrete Wall Filter | 8. Wind Dynamics Modulation | 9. Post-Punk Distortion Engine | 10. Atmospheric Reverb Landscape | 11. Grey Sky GUI | 12. FL Studio Preset Bridge");
    println!("--- 12 Improvements ---");
    println!("1. GUI Palette | 2. Bass Optimization | 3. Panel Echo | 4. Granularity | 5. Stereo Effect | 6. Resonance | 7. Distortion | 8. Modulation | 9. Spectral Shift | 10. Reverb | 11. FL Studio Integration | 12. Documentation");
    println!();
    println!(
        "DSP core compiled and executed successfully (Rust >= {}).",
        env!("CARGO_PKG_RUST_VERSION")
    );
}
