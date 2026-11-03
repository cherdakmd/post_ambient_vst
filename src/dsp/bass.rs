// Монофонический пост-панк бас — настоящий инструмент.
//
// Играет реальную MIDI-ноту (никакого секвенсора): пила + суб-октава,
// ADSR-огибающая, резонансный фильтр по огибающей, дисторшн, глайд.
// Ориентир: глубокий моно-бас Molchat Doma / «Кино».

use super::filters::StateVariableFilter;
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct BassSynth {
    phase: f32,
    sub_phase: f32,
    cur_freq: f32,
    target_freq: f32,
    env: f32,
    stage: Stage,
    filter: StateVariableFilter,
    boost_filter: StateVariableFilter,
}

impl BassSynth {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sub_phase: 0.0,
            cur_freq: 55.0,
            target_freq: 55.0,
            env: 0.0,
            stage: Stage::Idle,
            filter: StateVariableFilter::new(sample_rate),
            boost_filter: StateVariableFilter::new(sample_rate),
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.sub_phase = 0.0;
        self.env = 0.0;
        self.stage = Stage::Idle;
        self.filter.reset();
        self.boost_filter.reset();
    }

    /// Целевая нота в Гц. Во время удержания частота «плывёт» к ней (глайд).
    pub fn set_note(&mut self, freq: f32) {
        self.target_freq = freq.clamp(15.0, 4_000.0);
    }

    /// Нажатие (true) / отпускание (false) клавиши.
    pub fn set_gate(&mut self, on: bool) {
        if on {
            // Повторное нажатие во время удержания не перезапускает огибающую
            if matches!(self.stage, Stage::Idle | Stage::Release) {
                self.stage = Stage::Attack;
            }
        } else if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    pub fn next_sample(
        &mut self,
        sample_rate: f32,
        glide: f32,
        resonance: f32,
        drive: f32,
        bass_boost: f32,
    ) -> f32 {
        let dt = 1.0 / sample_rate;

        // --- ADSR ---
        match self.stage {
            Stage::Idle => self.env = 0.0,
            Stage::Attack => {
                self.env += dt / 0.006; // быстрый атак ~6 мс
                if self.env >= 1.0 {
                    self.env = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                // Спад к сустейну 0.72 (~120 мс)
                let k = 1.0 - (-1.0 / (0.12 * sample_rate)).exp();
                self.env += (0.72 - self.env) * k;
                if (self.env - 0.72).abs() < 0.003 {
                    self.env = 0.72;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.env = 0.72,
            Stage::Release => {
                // Релиз ~70 мс
                let k = 1.0 - (-1.0 / (0.07 * sample_rate)).exp();
                self.env -= self.env * k;
                if self.env < 1e-4 {
                    self.env = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }

        // --- Глайд: частота плавно «плывёт» к новой ноте ---
        let glide_time = 0.005 + glide * 0.28; // до ~280 мс
        let coef = 1.0 - (-1.0 / (glide_time * sample_rate)).exp();
        self.cur_freq += (self.target_freq - self.cur_freq) * coef;

        // --- Осцилляторы: пила + суб-октава ---
        self.phase += self.cur_freq / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        self.sub_phase += (self.cur_freq * 0.5) / sample_rate;
        if self.sub_phase >= 1.0 {
            self.sub_phase -= 1.0;
        }
        let saw = 2.0 * self.phase - 1.0;
        let sub = (TAU * self.sub_phase).sin();
        let mut s = saw * 0.5 + sub * 0.85;

        // --- Фильтр с огибающей («бетонные стены») ---
        let cutoff = 90.0 + self.env * (450.0 + resonance * 1600.0);
        self.filter.tick(s, cutoff, 0.25 + resonance * 0.55);
        s = self.filter.low;

        // --- Дисторшн: «сырость» пост-панка ---
        if drive > 0.0 {
            let drive_gain = 1.0 + drive * 2.5;
            s = (s * drive_gain).tanh() / drive_gain.tanh();
        }

        s *= self.env * 0.55;

        // --- Бас-буст: подчёркивание низа + мягкий ограничитель ---
        if bass_boost > 0.0 {
            self.boost_filter.tick(s, 110.0, 0.1);
            s += self.boost_filter.low * bass_boost * 1.8;
            s = s.tanh();
        } else {
            self.boost_filter.reset();
        }

        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bass_is_silent_without_note() {
        let mut b = BassSynth::new(44_100.0);
        let mut peak = 0.0f32;
        for _ in 0..44_100 {
            peak = peak.max(b.next_sample(44_100.0, 0.0, 0.4, 0.3, 0.5).abs());
        }
        assert!(peak < 1e-3, "бас молчит без ноты, peak={peak}");
    }

    #[test]
    fn bass_plays_note_on_gate() {
        let mut b = BassSynth::new(44_100.0);
        b.set_note(55.0); // A1
        b.set_gate(true);
        let mut peak = 0.0f32;
        for _ in 0..22_050 {
            peak = peak.max(b.next_sample(44_100.0, 0.0, 0.4, 0.3, 0.5).abs());
        }
        assert!(peak > 0.05, "бас должен звучать после NoteOn, peak={peak}");

        // Отпустили ноту - через релиз затихает
        b.set_gate(false);
        for _ in 0..44_100 {
            b.next_sample(44_100.0, 0.0, 0.4, 0.3, 0.5);
        }
        let tail = b.next_sample(44_100.0, 0.0, 0.4, 0.3, 0.5).abs();
        assert!(tail < 1e-3, "после релиза бас должен замолчать, tail={tail}");
    }

    #[test]
    fn bass_pitch_follows_note() {
        // Выше нота - выше частота основного тона (проверка через нулевые
        // пересечения сглаженного сигнала)
        fn estimate(freq: f32) -> f32 {
            let mut b = BassSynth::new(44_100.0);
            b.set_note(freq);
            b.set_gate(true);
            // Прогрев фильтра и огибающей
            for _ in 0..8820 {
                b.next_sample(44_100.0, 0.0, 0.4, 0.0, 0.0);
            }
            let mut prev = 0.0f32;
            let mut crossings = 0u32;
            let n = 22_050u32;
            for i in 0..n {
                let s = b.next_sample(44_100.0, 0.0, 0.4, 0.0, 0.0);
                if prev < 0.0 && s >= 0.0 {
                    crossings += 1;
                }
                prev = s;
                let _ = i;
            }
            crossings as f32 / (n as f32 / 44_100.0)
        }
        let low = estimate(55.0);
        let high = estimate(110.0);
        assert!(
            high > low * 1.6,
            "удвоение ноты должно почти удвоить частоту: {low} -> {high}"
        );
    }
}
