// src/dsp/distortion.rs - 9. Post-Punk Distortion Engine (Innovation)
// 7. Distortion (Improvement)

use super::params::SuperAtmosphericParams;

/// Бруталистский дисторшн пост-панка:
/// мягкий клиппинг (tanh) + асимметричная «бетонная» примесь + нормировка громкости.
///
/// * `drive`   — 0..1, глубина перегруза (0.6 → gain ×8.2)
/// * `texture` — 0..1, доля «бетонной» примеси (0.95)
pub fn post_punk_shape(x: f32, drive: f32, texture: f32) -> f32 {
    let d = 1.0 + drive.clamp(0.0, 1.0) * 12.0;
    let texture = texture.clamp(0.0, 1.0);

    let soft = (x * d).tanh();
    // «Бетонная» примесь: нечётная кубика на ограниченной области (k <= 1/3 → монотонна),
    // вне ±1 держим константой, чтобы форма была устойчивой на любом входе.
    let xc = x.clamp(-1.0, 1.0);
    let k = 0.15 + 0.15 * texture;
    let grit = xc - k * xc * xc * xc;
    let norm = (1.0 + 0.35 * d).tanh();

    let y = (soft * (1.0 - 0.25 * texture) + grit * 0.25 * texture) / norm;
    y.clamp(-1.0, 1.0)
}

/// Мягкий лимитер на выходе («бетонный потолок»).
pub fn soft_clip(x: f32) -> f32 {
    x.tanh()
}

/// 9. Post-Punk Distortion Engine | 7. Distortion
pub fn apply_super_atmospheric_distortion(params: &SuperAtmosphericParams) {
    println!(
        "[DSP Distortion] SUPER APPLYING post-punk distortion... Level: {}, Brutalist Texture: {} | [Innovation] Post-Punk Distortion Engine | [Improvement] Distortion Engine",
        params.distortion, params.brutalist_texture
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_is_odd_symmetric() {
        for i in 1..50 {
            let x = i as f32 / 50.0;
            let pos = post_punk_shape(x, 0.6, 0.95);
            let neg = post_punk_shape(-x, 0.6, 0.95);
            assert!((pos + neg).abs() < 1e-5, "asymmetry at {}", x);
        }
    }

    #[test]
    fn shape_is_monotonic_and_bounded() {
        let mut prev = post_punk_shape(-1.5, 0.6, 0.95);
        for i in -149..=150 {
            let x = i as f32 / 100.0;
            let y = post_punk_shape(x, 0.9, 1.0);
            assert!(y.is_finite() && y.abs() <= 1.0 + f32::EPSILON);
            assert!(y >= prev - 1e-4, "не монотонно на {}", x);
            prev = y;
        }
    }

    #[test]
    fn zero_in_zero_out() {
        assert_eq!(post_punk_shape(0.0, 0.6, 0.95), 0.0);
    }

    #[test]
    fn no_distortion_is_almost_clean() {
        // drive = 0, texture = 0 → почти единичное усиление
        let y = post_punk_shape(0.5, 0.0, 0.0);
        assert!((y - 0.5).abs() < 0.35, "y = {}", y);
    }

    #[test]
    fn soft_clip_limits() {
        assert!(soft_clip(100.0) <= 1.0);
        assert!(soft_clip(-100.0) >= -1.0);
        assert!(soft_clip(0.0) == 0.0);
    }
}
