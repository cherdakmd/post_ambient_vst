// «Холодный» пост-панк бас со слайдом нот (портаменто).
//
// Секвенсор восьмыми нотами играет рифф в минорной пентатонике:
// пила + суб-октава, фильтр с огибающей и драйвом.
// `glide` (0..1) заставляет ноты «плыть» друг в друга.

use super::filters::StateVariableFilter;
use std::f32::consts::TAU;

// Стабильный пост-панк рифф (полутонов относительно корня)
const PATTERN: [f32; 8] = [0.0, 0.0, 7.0, 0.0, 5.0, 3.0, 7.0, 3.0];

pub struct BassState {
    phase: f32,
    sub_phase: f32,
    note_index: u32,
    t_in_note: f32,
    cur_freq: f32,
    target_freq: f32,
    filter: StateVariableFilter,
    boost_filter: StateVariableFilter,
    sample_rate: f32,
}

impl BassState {
    pub fn new(sample_rate: f32, root_hz: f32) -> Self {
        Self {
            phase: 0.0,
            sub_phase: 0.0,
            note_index: 0,
            t_in_note: 0.0,
            cur_freq: root_hz,
            target_freq: root_hz,
            filter: StateVariableFilter::new(sample_rate),
            boost_filter: StateVariableFilter::new(sample_rate),
            sample_rate,
        }
    }

    pub fn reset(&mut self, root_hz: f32) {
        self.phase = 0.0;
        self.sub_phase = 0.0;
        self.note_index = 0;
        self.t_in_note = 0.0;
        self.cur_freq = root_hz;
        self.target_freq = root_hz;
        self.filter.reset();
        self.boost_filter.reset();
    }

    pub fn next_sample(
        &mut self,
        sample_rate: f32,
        root_hz: f32,
        bpm: f32,
        resonance: f32,
        drive: f32,
        glide: f32,
        bass_boost: f32,
    ) -> f32 {
        self.sample_rate = sample_rate;
        let step = 60.0 / bpm / 2.0; // длительность восьмой ноты (с)
        let attack = 0.004;
        let decay = 0.09;

        // Переход к следующей ноте
        if self.t_in_note >= step {
            self.t_in_note -= step;
            self.note_index = self.note_index.wrapping_add(1);
            let semitone = PATTERN[(self.note_index as usize) % PATTERN.len()];
            self.target_freq = root_hz * 2.0_f32.powf(semitone / 12.0);
        }

        // Слайд: частота плавно «плывёт» к новой ноте
        let glide_time = (glide * step * 0.9).max(0.005);
        let glide_coef = if glide > 0.0 {
            1.0 - (-1.0 / (glide_time * sample_rate)).exp()
        } else {
            1.0
        };
        self.cur_freq += (self.target_freq - self.cur_freq) * glide_coef;

        // Огибающая: атак -> спад -> сустейн -> релиз перед следующей нотой
        let mut env = if self.t_in_note < attack {
            self.t_in_note / attack
        } else if self.t_in_note < decay {
            1.0 - 0.35 * ((self.t_in_note - attack) / (decay - attack))
        } else {
            0.65
        };
        let release_start = step * 0.85;
        if self.t_in_note > release_start {
            env *= 1.0 - (self.t_in_note - release_start) / (step - release_start);
        }

        // Осцилляторы
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
        let mut s = saw * 0.5 + sub * 0.8;

        // Фильтр с огибающей и резонансом («бетонные стены»)
        let cutoff = 90.0 + env * (450.0 + resonance * 1600.0);
        self.filter.tick(s, cutoff, 0.25 + resonance * 0.55);
        s = self.filter.low;

        // Лёгкий драйв - «сырость» пост-панка
        let drive_gain = 1.0 + drive * 2.5;
        s = (s * drive_gain).tanh() / drive_gain.tanh();

        s *= env * 0.55;
        self.t_in_note += 1.0 / sample_rate;

        // Бас-буст: подчёркивание низа + мягкий ограничитель
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
