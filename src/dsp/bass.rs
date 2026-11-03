// Суб-бас для джангла/эмбиента: чистый синус с 808-дропом.
//
// При NoteOn питч стартует выше и «падает» на ноту (dro p-огибающая),
// дальше — ADSR, глайд между нотами и мягкая сатурация.
// Играет реальную MIDI-ноту, монофонически (последняя нота сверху).

use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct SubBass {
    phase: f32,
    cur_freq: f32,
    target_freq: f32,
    /// Огибающая дропа: 1 сразу после NoteOn -> 0 (~90 мс).
    drop_env: f32,
    env: f32,
    stage: Stage,
}

impl SubBass {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            cur_freq: 55.0,
            target_freq: 55.0,
            drop_env: 0.0,
            env: 0.0,
            stage: Stage::Idle,
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.env = 0.0;
        self.drop_env = 0.0;
        self.stage = Stage::Idle;
    }

    /// Целевая нота в Гц; во время удержания частота плывёт к ней (глайд).
    pub fn set_note(&mut self, freq: f32) {
        self.target_freq = freq.clamp(15.0, 400.0);
    }

    pub fn set_gate(&mut self, on: bool) {
        if on {
            if matches!(self.stage, Stage::Idle | Stage::Release) {
                self.stage = Stage::Attack;
                self.drop_env = 1.0; // дроп стартует с каждого атака
            }
        } else if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    pub fn next_sample(
        &mut self,
        sample_rate: f32,
        glide: f32,
        drop: f32,
        drive: f32,
    ) -> f32 {
        let dt = 1.0 / sample_rate;

        // --- ADSR: быстрый атак, сустейн 0.9, релиз ~120 мс ---
        match self.stage {
            Stage::Idle => self.env = 0.0,
            Stage::Attack => {
                self.env += dt / 0.005;
                if self.env >= 1.0 {
                    self.env = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                let k = 1.0 - (-1.0 / (0.15 * sample_rate)).exp();
                self.env += (0.9 - self.env) * k;
                if (self.env - 0.9).abs() < 0.003 {
                    self.env = 0.9;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.env = 0.9,
            Stage::Release => {
                let k = 1.0 - (-1.0 / (0.12 * sample_rate)).exp();
                self.env -= self.env * k;
                if self.env < 1e-4 {
                    self.env = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }

        // --- 808-дроп: питч снижается с +drop*2 октавы за ~90 мс ---
        self.drop_env -= 1.0 / (0.09 * sample_rate);
        if self.drop_env < 0.0 {
            self.drop_env = 0.0;
        }
        let semis = drop * 24.0 * self.drop_env;
        let pitch = 2.0f32.powf(semis / 12.0);

        // --- Глайд ---
        let glide_time = 0.005 + glide * 0.3;
        let coef = 1.0 - (-1.0 / (glide_time * sample_rate)).exp();
        self.cur_freq += (self.target_freq - self.cur_freq) * coef;

        // --- Осциллятор: синус + слабая вторая гармоника (слышно на мониторах) ---
        let f = self.cur_freq * pitch;
        self.phase += f / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let mut s = (self.phase * TAU).sin() + 0.18 * (self.phase * TAU * 2.0).sin();

        s *= self.env;

        // --- Мягкая сатурация (драйв = параметр дисторшна) ---
        if drive > 0.0 {
            let g = 1.0 + drive * 3.0;
            s = (s * g).tanh() / g.tanh();
        }

        s * 0.85
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_is_silent_without_note() {
        let mut b = SubBass::new();
        let mut peak = 0.0f32;
        for _ in 0..44_100 {
            peak = peak.max(b.next_sample(44_100.0, 0.0, 0.4, 0.0).abs());
        }
        assert!(peak < 1e-3, "суб молчит без ноты, peak={peak}");
    }

    #[test]
    fn sub_plays_note_and_releases() {
        let mut b = SubBass::new();
        b.set_note(55.0);
        b.set_gate(true);
        let mut peak = 0.0f32;
        for _ in 0..22_050 {
            peak = peak.max(b.next_sample(44_100.0, 0.0, 0.4, 0.3).abs());
        }
        assert!(peak > 0.1, "суб должен звучать, peak={peak}");

        b.set_gate(false);
        for _ in 0..88_200 {
            b.next_sample(44_100.0, 0.0, 0.4, 0.3);
        }
        let tail = b.next_sample(44_100.0, 0.0, 0.4, 0.3).abs();
        assert!(tail < 1e-3, "после релиза тишина, tail={tail}");
    }

    #[test]
    fn drop_raises_initial_pitch() {
        // С дропом первый момент звучит заметно выше ноты
        fn first_freq(drop: f32) -> f32 {
            let mut b = SubBass::new();
            b.set_note(55.0);
            b.set_gate(true);
            // Прогрев без дропа не нужен - меряем сразу
            let mut crossings = 0u32;
            let mut prev = 0.0f32;
            let n = 4_410u32; // 100 мс
            for _ in 0..n {
                let s = b.next_sample(44_100.0, 0.0, drop, 0.0);
                if prev < 0.0 && s >= 0.0 {
                    crossings += 1;
                }
                prev = s;
            }
            crossings as f32 / (n as f32 / 44_100.0)
        }
        let with_drop = first_freq(1.0);
        let without = first_freq(0.0);
        assert!(
            with_drop > without * 1.5,
            "дроп должен поднять стартовую высоту: {without} -> {with_drop}"
        );
    }
}
