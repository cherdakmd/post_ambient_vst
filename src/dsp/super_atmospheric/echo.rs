// src/dsp/echo.rs - 3. Panel Echo Resonance (Innovation)
// 3. Panel Echo & 10. Reverb (Improvement)

use super::params::SuperAtmosphericParams;

/// Эхо панельного дома: линия задержки с тембрально окрашенной обратной связью.
/// Время — `echo_delay` (2.5 с), «глубина панелей» управляет обратной связью и подмесом.
#[derive(Clone, Debug)]
pub struct PanelEcho {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    wet: f32,
    damp: f32,
    damp_coef: f32,
}

impl PanelEcho {
    pub fn new(params: &SuperAtmosphericParams, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        // 2.5 с эха — это и есть «двор между домами», ограничиваем 8 с
        let delay = params.echo_delay_samples(sr).clamp(1, (8.0 * sr) as usize);
        let depth_norm = (params.panel_echo_depth.clamp(0.0, 8.0) / 3.0).clamp(0.0, 1.0);

        let feedback = (0.15 + 0.30 * params.resonance.clamp(0.0, 1.0)) * (0.5 + 0.5 * depth_norm);
        let wet = 0.25 + 0.35 * depth_norm;
        // бруталистская текстура = более тёмные, «бетонные» повторы
        let damp_coef = 0.15 + 0.35 * (1.0 - params.brutalist_texture.clamp(0.0, 1.0));

        Self {
            buffer: vec![0.0; delay],
            index: 0,
            feedback,
            wet,
            damp: 0.0,
            damp_coef,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let delayed = self.buffer[self.index];
        self.damp += (delayed - self.damp) * self.damp_coef;
        self.buffer[self.index] = input + self.damp * self.feedback;
        self.index = (self.index + 1) % self.buffer.len();
        (input + self.wet * delayed).clamp(-4.0, 4.0)
    }

    pub fn reset(&mut self) {
        for s in self.buffer.iter_mut() {
            *s = 0.0;
        }
        self.index = 0;
        self.damp = 0.0;
    }

    /// Длина линии задержки в сэмплах.
    pub fn delay_samples(&self) -> usize {
        self.buffer.len()
    }
}

/// 3. Panel Echo Resonance | 10. Atmospheric Reverb Landscape
pub fn apply_super_atmospheric_echo(params: &SuperAtmosphericParams) {
    println!(
        "[DSP Echo] SUPER APPLYING panel house echo... Delay: {}s, Panel Echo Depth: {}s | [Innovation] Panel Echo Resonance | [Improvement] Panel Echo Depth",
        params.echo_delay, params.panel_echo_depth
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params_with_delay(seconds: f32) -> SuperAtmosphericParams {
        let mut p = SuperAtmosphericParams::default();
        p.echo_delay = seconds;
        p
    }

    #[test]
    fn echo_arrives_after_delay_time() {
        let p = params_with_delay(0.002); // 96 сэмплов при 48 кГц
        let mut echo = PanelEcho::new(&p, 48_000.0);
        assert_eq!(echo.delay_samples(), 96);

        // тишина до прихода первого повтора
        for _ in 0..95 {
            assert_eq!(echo.process(0.0), 0.0);
        }
        let tail = echo.process(0.0);
        assert_eq!(tail, 0.0, "сигнал ещё не дошёл");

        // теперь подаём импульс и ждём повтор через 96 сэмплов
        let mut echo = PanelEcho::new(&p, 48_000.0);
        echo.process(1.0);
        for i in 0..95 {
            let s = echo.process(0.0);
            assert_eq!(s, 0.0, "повтор появился раньше времени на сэмпле {}", i);
        }
        let repeat = echo.process(0.0);
        assert!(repeat > 0.05, "повтор слишком тихий: {}", repeat);
    }

    #[test]
    fn echo_decays_over_time() {
        let p = params_with_delay(0.01);
        let mut echo = PanelEcho::new(&p, 48_000.0);
        echo.process(1.0);
        let mut peak_early = 0.0f32;
        for _ in 0..480 {
            peak_early = peak_early.max(echo.process(0.0).abs());
        }
        let mut peak_late = 0.0f32;
        for _ in 0..48_000 {
            peak_late = peak_late.max(echo.process(0.0).abs());
        }
        assert!(peak_late < peak_early, "early = {}, late = {}", peak_early, peak_late);
    }

    #[test]
    fn silence_stays_silent() {
        let p = SuperAtmosphericParams::default();
        let mut echo = PanelEcho::new(&p, 48_000.0);
        for _ in 0..1_000 {
            assert_eq!(echo.process(0.0), 0.0);
        }
    }
}
