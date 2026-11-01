// src/dsp/filter.rs - 7. Concrete Wall Filter (Innovation)
// 9. Spectral Shift (Improvement)
// «Бетонная стена»: однополюсный ФНЧ панельного дома + спектральный сдвиг.

use super::params::SuperAtmosphericParams;

/// Коэффициент однополюсного ФНЧ «бетонная стена» (в диапазоне 0..1).
/// cut-off 800 Hz при 48 kHz даёт очень глухой, «панельный» звук.
pub fn concrete_wall_alpha(cutoff_hz: f32, sample_rate: f32) -> f32 {
    if sample_rate <= 0.0 || cutoff_hz <= 0.0 {
        return 1.0;
    }
    let dt = 1.0 / sample_rate;
    let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
    let alpha = dt / (rc + dt);
    alpha.clamp(0.0, 1.0)
}

/// Спектральный сдвиг (9. Spectral Shift) — множитель частоты среза.
pub fn spectral_shifted_cutoff(params: &SuperAtmosphericParams) -> f32 {
    (params.filter_cutoff * (1.0 + params.spectral_shift)).max(20.0)
}

/// 7. Concrete Wall Filter | 9. Spectral Shift
pub fn apply_super_atmospheric_filter(params: &SuperAtmosphericParams, sample_rate: f32) {
    // Логика «Фильтра» (срез 800Hz, резонанс 0.7, спектральный сдвиг 0.85)
    // Нововведение 7: Concrete Wall Filter (фильтр бетонной стены)
    // Улучшение 9: Спектральный Сдвиг (холодный спектр Юго-Запада)
    let cutoff = spectral_shifted_cutoff(params);
    let alpha = concrete_wall_alpha(cutoff, sample_rate);
    println!(
        "[DSP Filter] SUPER APPLYING concrete wall filter... Cutoff: {}Hz -> {}Hz (shift {}), Resonance: {}, Alpha: {:.6} | [Innovation] Concrete Wall Filter | [Improvement] Spectral Shift",
        params.filter_cutoff, cutoff, params.spectral_shift, params.resonance, alpha
    );
}

/// Пропустить один сэмпл через фильтр «бетонная стена».
pub struct ConcreteWallFilter {
    alpha: f32,
    state: f32,
}

impl ConcreteWallFilter {
    pub fn new(params: &SuperAtmosphericParams, sample_rate: f32) -> Self {
        Self {
            alpha: concrete_wall_alpha(spectral_shifted_cutoff(params), sample_rate),
            state: 0.0,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.state += self.alpha * (input - self.state);
        self.state
    }

    #[allow(dead_code)] // публичное API ядра, вызывается из плагина при смене пресета
    pub fn reset(&mut self) {
        self.state = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_stays_in_range() {
        let p = SuperAtmosphericParams::default();
        let a = concrete_wall_alpha(spectral_shifted_cutoff(&p), 48_000.0);
        assert!(a > 0.0 && a < 1.0);
    }

    #[test]
    fn filter_smooths_impulse() {
        let p = SuperAtmosphericParams::default();
        let mut f = ConcreteWallFilter::new(&p, 48_000.0);
        let first = f.process(1.0);
        let later = f.process(0.0);
        assert!(first > 0.0 && later < first);
    }
}
