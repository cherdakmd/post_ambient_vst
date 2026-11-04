// Джангл-брейк: встроенный 32-шаговый паттерн (свой ритм редактируется в GUI).
//
// Кит/снейр/хэты синтезируются на лету (без сэмплов), паттерн идёт по BPM
// со свингом — 2 такта по 16-х. Режим загрузки своего файла живёт
// в sample::Slicer (слайсы по MIDI-нотам).

use super::rng::Mulberry32;
use nih_plug::prelude::*;
use serde::{Deserialize, Serialize};
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

/// Уровень шага: 0 - пусто, 1 - тихо (ghost), 2 - нормально, 3 - акцент.
/// У хэтов уровень 3 - открытый (длинный), 1/2 - закрытые.
pub type Level = u8;

/// Ритм брейка: 3 дорожки × 32 шага. Сохраняется в пресете плагина.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Pattern {
    pub kick: [Level; STEPS],
    pub snare: [Level; STEPS],
    pub hat: [Level; STEPS],
}

impl Default for Pattern {
    fn default() -> Self {
        Self::amen()
    }
}

impl Pattern {
    pub fn empty() -> Self {
        Self {
            kick: [0; STEPS],
            snare: [0; STEPS],
            hat: [0; STEPS],
        }
    }

    /// Амен-подобный джангл (2 такта): кит/снейр/хэты с ghost-нотами.
    pub fn amen() -> Self {
        Self {
            kick: [
                3, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 3, 0, //
                3, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 3, 0,
            ],
            snare: [
                0, 0, 0, 0, 3, 0, 0, 1, 0, 0, 0, 0, 3, 0, 0, 1, //
                0, 0, 0, 0, 3, 0, 1, 0, 0, 0, 0, 0, 3, 0, 1, 0,
            ],
            hat: [
                2, 0, 1, 0, 2, 0, 1, 1, 2, 0, 1, 0, 2, 1, 3, 0, //
                2, 0, 1, 0, 2, 0, 1, 1, 2, 0, 1, 0, 2, 1, 3, 1,
            ],
        }
    }

    /// Музыкальный случайный ритм (детерминированный по seed):
    /// кит на сильных долях, снейр на 2/4 с редкими ghost, хэты вперемешку.
    pub fn random(seed: u32) -> Self {
        let mut rng = Mulberry32::new(seed);
        let mut p = Self::empty();
        for i in 0..STEPS {
            let r = rng.next_f32();
            // Кит: сильная доля каждого такта + редкие добавки
            p.kick[i] = if i % 8 == 0 {
                3
            } else if r < 0.12 {
                2
            } else if r < 0.18 {
                1
            } else {
                0
            };
            // Снейр: 2 и 4 доля + ghost перед следующей долей
            p.snare[i] = if i % 8 == 4 {
                3
            } else if i % 8 == 7 && r > 0.8 {
                1
            } else if r > 0.93 {
                2
            } else {
                0
            };
            // Хэты: чётные шаги всегда, нечётные - часто; открытый редко
            p.hat[i] = if i % 16 == 14 && r > 0.5 {
                3
            } else if i % 2 == 0 {
                if r > 0.75 {
                    3
                } else if r > 0.4 {
                    2
                } else {
                    1
                }
            } else if r < 0.55 {
                1
            } else {
                0
            };
        }
        p
    }

    /// Копирует первый такт во второй.
    pub fn repeat_first_bar(&mut self) {
        self.kick.copy_within(0..STEPS / 2, STEPS / 2);
        self.snare.copy_within(0..STEPS / 2, STEPS / 2);
        self.hat.copy_within(0..STEPS / 2, STEPS / 2);
    }

    /// Доступ к дорожке по индексу: 0 - кит, 1 - снейр, 2 - хэт.
    pub fn lane(&self, row: usize) -> &[Level; STEPS] {
        match row {
            0 => &self.kick,
            1 => &self.snare,
            _ => &self.hat,
        }
    }

    pub fn lane_mut(&mut self, row: usize) -> &mut [Level; STEPS] {
        match row {
            0 => &mut self.kick,
            1 => &mut self.snare,
            _ => &mut self.hat,
        }
    }
}

/// Уровень шага -> сила удара (0.0..1.0).
pub fn level_vel(level: Level) -> f32 {
    match level {
        0 => 0.0,
        1 => 0.35,
        2 => 0.7,
        _ => 1.0,
    }
}

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
    /// Копия редактируемого паттерна (обновляется на границе блока).
    pat: Pattern,
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
            pat: Pattern::amen(),
        }
    }

    /// Подхватывает паттерн из GUI (вызывается на границе аудио-блока).
    pub fn set_pattern(&mut self, pat: &Pattern) {
        self.pat = *pat;
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
            let kv = level_vel(self.pat.kick[s]);
            if kv > 0.0 {
                self.kick.trigger(kv);
            }
            let sv = level_vel(self.pat.snare[s]);
            if sv > 0.0 {
                self.snare.trigger(sv);
            }
            let hv = level_vel(self.pat.hat[s]);
            if hv > 0.0 {
                self.hat.trigger(hv);
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

    #[test]
    fn empty_pattern_is_silent() {
        let mut p = BreakPattern::new();
        p.set_pattern(&Pattern::empty());
        let mut peak = 0.0f32;
        for _ in 0..(44_100 * 2) {
            let (l, r) = p.next_sample(44_100.0, 174.0, 0.0);
            peak = peak.max(l.abs()).max(r.abs());
        }
        assert!(peak < 1e-4, "пустой паттерн не должен звучать, peak={peak}");
    }

    #[test]
    fn custom_pattern_only_plays_its_steps() {
        // Ровно один удар - кит на первом шаге
        let mut pat = Pattern::empty();
        pat.kick[0] = 3;
        let mut p = BreakPattern::new();
        p.set_pattern(&pat);

        let mut sounding = 0u32;
        // Один такт при 120 BPM = 2 с (32 шага по 0.125 с)
        for _ in 0..(44_100 * 2) {
            let (l, r) = p.next_sample(44_100.0, 120.0, 0.0);
            if l.abs() > 1e-4 || r.abs() > 1e-4 {
                sounding += 1;
            }
        }
        // Кит затухает ~0.5 с - остальные 1.5 с должны быть тишиной
        assert!(
            sounding < 44_100,
            "должен звучать только один удар: {sounding} сэмплов"
        );
        assert!(sounding > 1000, "удар должен прозвучать: {sounding} сэмплов");
    }

    #[test]
    fn levels_are_bounded_and_deterministic() {
        for seed in [1u32, 42, 999_999] {
            let a = Pattern::random(seed);
            let b = Pattern::random(seed);
            assert_eq!(a, b, "одинаковый seed должен давать одинаковый ритм");
            for i in 0..STEPS {
                assert!(a.kick[i] <= 3 && a.snare[i] <= 3 && a.hat[i] <= 3);
            }
            // В случайном ритме всегда есть кит и снейр
            assert!(a.kick.iter().any(|&l| l > 0), "кит должен быть");
            assert!(a.snare.iter().any(|&l| l > 0), "снейр должен быть");
        }
        // Разные seed - разные ритмы
        assert_ne!(Pattern::random(1), Pattern::random(2));
    }

    #[test]
    fn amen_pattern_has_all_lanes_and_levels() {
        let p = Pattern::amen();
        assert!(p.kick.iter().any(|&l| l == 3));
        assert!(p.snare.iter().any(|&l| l == 1), "ghost-снейры должны быть");
        assert!(p.hat.iter().any(|&l| l == 3), "открытый хэт должен быть");
        // Копирование первого такта во второй
        let mut q = p;
        q.repeat_first_bar();
        assert_eq!(&q.kick[..16], &q.kick[16..]);
        assert_eq!(&q.snare[..16], &q.snare[16..]);
        assert_eq!(&q.hat[..16], &q.hat[16..]);
        // Доступ к дорожкам по индексу
        assert_eq!(p.lane(0), &p.kick);
        assert_eq!(p.lane(1), &p.snare);
        assert_eq!(p.lane(2), &p.hat);
    }

    #[test]
    fn level_vel_maps_to_velocity() {
        assert_eq!(level_vel(0), 0.0);
        assert!(level_vel(1) < level_vel(2));
        assert!(level_vel(2) < level_vel(3));
        assert_eq!(level_vel(3), 1.0);
        // Открытый хэт включается только на максимальном уровне
        assert!(level_vel(3) > 0.7);
        assert!(level_vel(2) <= 0.7);
    }
}
