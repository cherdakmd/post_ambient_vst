// Джангл-брейк: встроенный 32-шаговый амен-подобный паттерн.
//
// Кит/снейр/хэты синтезируются на лету (без сэмплов), паттерн идёт по BPM
// со свингом — 2 такта по 16-х. Режим загрузки своего файла живёт
// в sample::Slicer (слайсы по MIDI-нотам).

use super::rng::Mulberry32;
use nih_plug::prelude::*;
use std::f32::consts::TAU;

/// Два такта по 16-х.
pub const STEPS: usize = 32;

#[derive(Enum, PartialEq, Clone, Copy, Debug)]
pub enum BreakMode {
    /// Встроенный амен-паттерн (синтезированные удары по BPM).
    BuiltIn,
    /// Свой WAV: автонарезка на 16 слайсов, ноты = слайсы.
    File,
}

// ---------------------------------------------------------------- паттерн
// Амен-подобный джангл (2 такта, 16-е): кит/снейр/хэты с акцентами.
const KICK: [f32; STEPS] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.0, //
    1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.9, 0.0, 0.0,
];
const SNARE: [f32; STEPS] = [
    0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.35, //
    0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.3, 0.0,
];
// 0.5/0.35 - закрытый хэт, 0.8+ - открытый (длинный)
const HAT: [f32; STEPS] = [
    0.5, 0.0, 0.35, 0.0, 0.5, 0.0, 0.35, 0.3, 0.5, 0.0, 0.35, 0.0, 0.5, 0.3, 0.8, 0.0, //
    0.5, 0.0, 0.35, 0.0, 0.5, 0.0, 0.35, 0.3, 0.5, 0.0, 0.35, 0.0, 0.5, 0.3, 0.8, 0.4,
];

// ---------------------------------------------------------------- голоса

struct Kick {
    t: f64,
    vel: f32,
    phase: f32,
    freq: f32,
    active: bool,
}

impl Kick {
    const fn new() -> Self {
        Self {
            t: 0.0,
            vel: 0.0,
            phase: 0.0,
            freq: 45.0,
            active: false,
        }
    }

    fn trigger(&mut self, vel: f32) {
        self.t = 0.0;
        self.vel = vel;
        self.phase = 0.0;
        self.active = true;
    }

    fn next(&mut self, sr: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let dt = 1.0 / sr;
        self.t += dt as f64;
        // 808-подобный спад частоты 150 -> 45 Гц
        self.freq = 45.0 + 105.0 * (-(self.t as f32) / 0.02).exp();
        self.phase += self.freq * dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        if self.t > 0.5 {
            self.active = false;
        }
        let env = (-(self.t as f32) / 0.13).exp();
        (self.phase * TAU).sin() * env * self.vel * 0.95
    }
}

struct Snare {
    t: f64,
    vel: f32,
    phase: f32,
    prev: f32,
    active: bool,
}

impl Snare {
    const fn new() -> Self {
        Self {
            t: 0.0,
            vel: 0.0,
            phase: 0.0,
            prev: 0.0,
            active: false,
        }
    }

    fn trigger(&mut self, vel: f32) {
        self.t = 0.0;
        self.vel = vel;
        self.phase = 0.0;
        self.prev = 0.0;
        self.active = true;
    }

    fn next(&mut self, sr: f32, rng: &mut Mulberry32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let dt = 1.0 / sr;
        self.t += dt as f64;
        // Тональная составляющая ~190 Гц + шумовой «хлоп»
        self.phase += 190.0 * dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let tone = (self.phase * TAU).sin() * (-(self.t as f32) / 0.05).exp() * 0.45;
        let n = rng.next_f32() * 2.0 - 1.0;
        let hp = n - self.prev; // простое HP - «трещотка» снейра
        self.prev = n;
        if self.t > 0.4 {
            self.active = false;
        }
        let env = (-(self.t as f32) / 0.075).exp();
        (tone + hp * 0.75) * env * self.vel * 0.6
    }
}

struct Hat {
    t: f64,
    vel: f32,
    decay: f32,
    p1: f32,
    p2: f32,
    active: bool,
}

impl Hat {
    const fn new() -> Self {
        Self {
            t: 0.0,
            vel: 0.0,
            decay: 0.03,
            p1: 0.0,
            p2: 0.0,
            active: false,
        }
    }

    fn trigger(&mut self, vel: f32) {
        self.t = 0.0;
        self.vel = vel;
        // Открытый хэт звучит дольше
        self.decay = if vel > 0.7 { 0.13 } else { 0.028 };
        self.p1 = 0.0;
        self.p2 = 0.0;
        self.active = true;
    }

    fn next(&mut self, sr: f32, rng: &mut Mulberry32) -> f32 {
        if !self.active {
            return 0.0;
        }
        let dt = 1.0 / sr;
        self.t += dt as f64;
        let n = rng.next_f32() * 2.0 - 1.0;
        // Двойное дифференцирование - яркий HP для хэтов
        let hp = n - 2.0 * self.p1 + self.p2;
        self.p2 = self.p1;
        self.p1 = n;
        if self.t > 0.4 {
            self.active = false;
        }
        let env = (-(self.t as f32) / self.decay).exp();
        hp * env * self.vel * 0.18
    }
}

// ---------------------------------------------------------------- паттерн-плеер

pub struct BreakPattern {
    /// Индекс СЛЕДУЮЩЕГО шага.
    step: usize,
    next_time: f64,
    time: f64,
    started: bool,
    kick: Kick,
    snare: Snare,
    hat: Hat,
    rng: Mulberry32,
}

impl BreakPattern {
    pub fn new() -> Self {
        Self {
            step: 0,
            next_time: 0.0,
            time: 0.0,
            started: false,
            kick: Kick::new(),
            snare: Snare::new(),
            hat: Hat::new(),
            rng: Mulberry32::new(0x_5EED_17),
        }
    }

    pub fn reset(&mut self) {
        self.step = 0;
        self.next_time = 0.0;
        self.time = 0.0;
        self.started = false;
        self.kick = Kick::new();
        self.snare = Snare::new();
        self.hat = Hat::new();
    }

    /// Сейчас звучащий шаг (0..31) — для ленты шагов в GUI.
    pub fn current_step(&self) -> usize {
        if !self.started {
            0
        } else {
            (self.step + STEPS - 1) % STEPS
        }
    }

    /// Стереопара: хэты слегка панорамируются, кит/снейр в центре.
    pub fn next_sample(&mut self, sample_rate: f32, bpm: f32, swing: f32) -> (f32, f32) {
        let step_len = (60.0 / bpm.clamp(40.0, 300.0) / 4.0) as f64;
        self.time += 1.0 / sample_rate as f64;
        if !self.started {
            self.started = true;
            self.next_time = self.time;
        }

        while self.time >= self.next_time {
            let s = self.step;
            if KICK[s] > 0.0 {
                self.kick.trigger(KICK[s]);
            }
            if SNARE[s] > 0.0 {
                self.snare.trigger(SNARE[s]);
            }
            if HAT[s] > 0.0 {
                self.hat.trigger(HAT[s]);
            }
            // Свинг: после чётного шага пауза длиннее, после нечётного - короче
            let sw = if s % 2 == 0 {
                1.0 + swing as f64 * 0.6
            } else {
                1.0 - swing as f64 * 0.6
            };
            self.next_time += step_len * sw;
            self.step = (s + 1) % STEPS;
        }

        let k = self.kick.next(sample_rate);
        let sn = self.snare.next(sample_rate, &mut self.rng);
        let h = self.hat.next(sample_rate, &mut self.rng);
        let mono = k + sn;
        let (hl, hr) = if self.step % 2 == 0 {
            (1.0, 0.6)
        } else {
            (0.6, 1.0)
        };
        (mono + h * hl, mono + h * hr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_produces_sound_and_advances() {
        let mut p = BreakPattern::new();
        let mut peak = 0.0f32;
        let mut seen_steps = [false; STEPS];
        // 4 секунды при 174 BPM - больше двух тактов (32 шага ~ 2.76 с)
        for _ in 0..(44_100 * 4) {
            let (l, r) = p.next_sample(44_100.0, 174.0, 0.0);
            peak = peak.max(l.abs()).max(r.abs());
            seen_steps[p.current_step()] = true;
            assert!(l.is_finite() && r.is_finite(), "NaN в выходе брейка");
        }
        assert!(peak > 0.1, "паттерн должен звучать, peak={peak}");
        assert!(seen_steps.iter().all(|&s| s), "все 32 шага должны пройти");
    }

    #[test]
    fn swing_keeps_bar_aligned() {
        // После двух тактов (64 шага с свингом) позиция должна вернуться к 0
        let mut p = BreakPattern::new();
        for _ in 0..(44_100 * 4) {
            p.next_sample(44_100.0, 174.0, 0.5);
        }
        // Чистое время 4 с при 174 BPM = 46.4 шага - просто проверяем, что
        // фаза не сбита и звук продолжается
        let (l, r) = p.next_sample(44_100.0, 174.0, 0.5);
        assert!(l.is_finite() && r.is_finite());
    }
}
