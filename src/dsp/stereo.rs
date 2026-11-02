// src/dsp/stereo.rs - 5. Courtyard Stereo Expansion (Innovation)
// 5. Stereo Effect (Improvement)

use super::params::SuperAtmosphericParams;

/// M/S-расширение «дворов Юго-Запада»:
/// `width` 0 → моно, 1 → без изменений, >1 → расширение.
pub fn widen(left: f32, right: f32, width: f32) -> (f32, f32) {
    let w = width.max(0.0);
    let mid = (left + right) * 0.5;
    let side = (left - right) * 0.5 * w;
    (mid + side, mid - side)
}

/// Расширитель стереобазы с плавным сглаживанием коэффициента.
#[derive(Clone, Debug)]
pub struct CourtyardStereo {
    width: f32,
}

impl CourtyardStereo {
    pub fn new(params: &SuperAtmosphericParams) -> Self {
        Self {
            width: params.stereo_effect.clamp(0.0, 2.0),
        }
    }

    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(0.0, 2.0);
    }

    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        widen(left, right, self.width)
    }

    /// Инерция, чтобы автоматизация ширины не щелкала.
    pub fn process_smoothed(&mut self, left: f32, right: f32, target: f32, amount: f32) -> (f32, f32) {
        self.width += (target.clamp(0.0, 2.0) - self.width) * amount.clamp(0.0, 1.0);
        widen(left, right, self.width)
    }
}

/// 5. Courtyard Stereo Expansion | 5. Stereo Effect
pub fn apply_super_atmospheric_stereo(params: &SuperAtmosphericParams) {
    println!(
        "[DSP Stereo] SUPER APPLYING courtyard stereo effect... Level: {} | [Innovation] Courtyard Stereo Expansion | [Improvement] Stereo Effect",
        params.stereo_effect
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_width_is_transparent() {
        let (l, r) = widen(0.7, -0.3, 1.0);
        assert!((l - 0.7).abs() < 1e-6);
        assert!((r + 0.3).abs() < 1e-6);
    }

    #[test]
    fn zero_width_is_mono() {
        let (l, r) = widen(0.7, -0.3, 0.0);
        assert!((l - r).abs() < 1e-6);
        assert!((l - 0.2).abs() < 1e-6);
    }

    #[test]
    fn wide_width_expands_side() {
        let (l, r) = widen(0.8, -0.8, 1.5);
        assert!(l > 0.8 && r < -0.8);
    }

    #[test]
    fn smoother_converges_to_target() {
        let p = SuperAtmosphericParams::default();
        let mut st = CourtyardStereo::new(&p);
        for _ in 0..1_000 {
            st.process_smoothed(0.5, -0.5, 1.5, 0.05);
        }
        let (l, _) = st.process(1.0, -1.0);
        assert!((l - 1.5).abs() < 1e-3, "l = {}", l);
    }
}
