// State-Variable Filter (Chamberlin SVF) - «бетонные стены».
// Частота среза и резонанс можно менять каждый сэмпл.

pub struct StateVariableFilter {
    pub low: f32,
    pub band: f32,
    pub high: f32,
    sample_rate: f32,
}

impl StateVariableFilter {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            low: 0.0,
            band: 0.0,
            high: 0.0,
            sample_rate,
        }
    }

    pub fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
        self.high = 0.0;
    }

    pub fn tick(&mut self, input: f32, cutoff_hz: f32, resonance: f32) {
        let nyquist = self.sample_rate * 0.45;
        let fc = cutoff_hz.clamp(20.0, nyquist);
        let f = 2.0 * (std::f32::consts::PI * fc / self.sample_rate).sin();
        let q = 1.0 - resonance * 0.98;

        let high = input - self.low - q * self.band;
        self.band += f * high;
        self.low += f * self.band;
        self.high = high;

        // Страховка от «взрыва» фильтра на резонансных частотах
        if !self.low.is_finite() || !self.band.is_finite() || self.low.abs() > 1e6 {
            self.reset();
        }
    }
}
