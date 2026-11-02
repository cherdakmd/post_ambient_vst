// src/dsp/filter.rs - 7. Concrete Wall Filter (Innovation)
// 9. Spectral Shift (Improvement)
// «Бетонная стена»: однополюсный ФНЧ панельного дома + резонансный SVF + спектральный сдвиг.

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
    let cutoff = spectral_shifted_cutoff(params);
    let alpha = concrete_wall_alpha(cutoff, sample_rate);
    println!(
        "[DSP Filter] SUPER APPLYING concrete wall filter... Cutoff: {}Hz -> {}Hz (shift {}), Resonance: {}, Alpha: {:.6} | [Innovation] Concrete Wall Filter | [Improvement] Spectral Shift",
        params.filter_cutoff, cutoff, params.spectral_shift, params.resonance, alpha
    );
}

/// Однополюсный ФНЧ «бетонная стена».
#[derive(Clone, Debug)]
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

/// Резонансный фильтр «бетонной стены» (Chamberlin SVF, низкий выход).
/// Резонанс управляет добротностью, спектральный сдвиг — частотой среза.
#[derive(Clone, Debug)]
pub struct ResonantWallFilter {
    low: f32,
    band: f32,
    f: f32,
    q: f32,
}

impl ResonantWallFilter {
    pub fn new(params: &SuperAtmosphericParams, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let cutoff = spectral_shifted_cutoff(params).clamp(20.0, sr * 0.45);
        let f = (2.0 * (std::f32::consts::PI * cutoff / sr).sin()).clamp(0.001, 1.2);
        // q = 1/Q: больше резонанс — меньше затухание
        let q = (1.0 - 0.85 * params.resonance.clamp(0.0, 1.0)).clamp(0.05, 0.999);
        Self {
            low: 0.0,
            band: 0.0,
            f,
            q,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let high = input - self.low - self.q * self.band;
        self.band = self.f * high + self.band;
        self.low = self.f * self.band + self.low;
        self.low.clamp(-4.0, 4.0)
    }

    pub fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
    }

    /// Текущая частота среза (для GUI).
    pub fn cutoff_hz(&self, sample_rate: f32) -> f32 {
        let s = (self.f * 0.5).clamp(-1.0, 1.0);
        (s.asin() / std::f32::consts::PI) * sample_rate.max(1.0)
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

    #[test]
    fn resonant_filter_passes_dc_and_kills_nyquist() {
        let p = SuperAtmosphericParams::default();
        let mut lp = ResonantWallFilter::new(&p, 48_000.0);
        let mut dc = 0.0;
        for _ in 0..20_000 {
            dc = lp.process(1.0);
        }
        assert!((0.8..1.3).contains(&dc), "DC gain = {}", dc);

        let mut hp = ResonantWallFilter::new(&p, 48_000.0);
        let mut out = 0.0;
        for i in 0..20_000 {
            out = hp.process(if i % 2 == 0 { 1.0 } else { -1.0 });
        }
        assert!(out.abs() < 0.3, "Nyquist leak = {}", out);
    }

    #[test]
    fn cutoff_is_reported_back() {
        let p = SuperAtmosphericParams::default();
        let f = ResonantWallFilter::new(&p, 48_000.0);
        let reported = f.cutoff_hz(48_000.0);
        assert!((reported - 1480.0).abs() < 15.0, "reported = {}", reported);
    }
}
