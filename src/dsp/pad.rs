// «Холодный» эмбиент-пад.
//
// Медленная минорная прогрессия из расстроенных пил, «утопленных»
// в низкочастотный фильтр с ветровой модуляцией - «серое небо-панелька».

use super::filters::StateVariableFilter;
use super::rng::Mulberry32;
use std::f32::consts::TAU;

// Прогрессия холодных аккордов (полутонов): i - VI - III - VII
const PROGRESSION: [[f32; 4]; 4] = [
    [0.0, 3.0, 7.0, 14.0],
    [8.0, 12.0, 15.0, 19.0],
    [3.0, 7.0, 10.0, 17.0],
    [10.0, 14.0, 17.0, 24.0],
];

const VOICES: usize = 4;

pub struct PadState {
    phase: [f64; VOICES],
    freq: [f64; VOICES],
    detune_l: [f32; VOICES],
    detune_r: [f32; VOICES],
    chord_idx: usize,
    filter_l: StateVariableFilter,
    filter_r: StateVariableFilter,
    time: f64,
}

impl PadState {
    pub fn new(sample_rate: f32, seed: u32) -> Self {
        let mut rng = Mulberry32::new(seed.wrapping_add(101));
        let mut state = Self {
            phase: [0.0; VOICES],
            freq: [0.0; VOICES],
            detune_l: [1.0; VOICES],
            detune_r: [1.0; VOICES],
            chord_idx: usize::MAX,
            filter_l: StateVariableFilter::new(sample_rate),
            filter_r: StateVariableFilter::new(sample_rate),
            time: 0.0,
        };
        for v in 0..VOICES {
            // Расстройка голосов: левый и правый каналы чуть расходятся
            let cents = 4.0 + rng.next_f32() * 10.0;
            state.detune_l[v] = 2.0_f32.powf(-cents / 1200.0);
            state.detune_r[v] = 2.0_f32.powf(cents / 1200.0);
            state.phase[v] = rng.next_f32() as f64;
        }
        state
    }

    pub fn next_sample(
        &mut self,
        sample_rate: f32,
        root_hz: f32,
        bpm: f32,
        modulation: f32,
    ) -> (f32, f32) {
        let t = self.time;
        let chord_dur = (60.0 / bpm * 4.0) as f64; // 4 удара на аккорд
        let xfade = (chord_dur * 0.4).min(2.5);    // кроссфейд аккордов

        // Смена аккорда
        let idx = ((t / chord_dur) as usize) % PROGRESSION.len();
        if idx != self.chord_idx {
            self.chord_idx = idx;
            let base = root_hz as f64 * 4.0; // пад на две октавы выше баса
            for v in 0..VOICES {
                self.freq[v] = base * 2.0_f64.powf(PROGRESSION[idx][v] as f64 / 12.0);
            }
        }

        // Огибающая аккорда: медленное появление и растворение
        let t_chord = t % chord_dur;
        let mut env = (t_chord.min(chord_dur - t_chord) / xfade) as f32;
        if env > 1.0 {
            env = 1.0;
        }
        env = env * env * (3.0 - 2.0 * env); // smoothstep

        // Синтез голосов (расстроенные пилы)
        let mut s_l = 0.0f32;
        let mut s_r = 0.0f32;
        for v in 0..VOICES {
            self.phase[v] += self.freq[v] / sample_rate as f64;
            if self.phase[v] >= 1.0 {
                self.phase[v] -= 1.0;
            }
            let saw = 2.0 * self.phase[v] as f32 - 1.0;
            s_l += saw * self.detune_l[v];
            s_r += saw * self.detune_r[v];
        }

        // «Ветровой» срез: медленный LFO двигает частоту фильтра
        let lfo = 1.0 + 0.35 * (TAU as f64 * 0.05 * t).sin() as f32 * (0.5 + modulation);
        let base_cutoff = 350.0 + 250.0 * (TAU as f64 * 0.023 * t).sin() as f32;
        let cutoff = (base_cutoff * lfo).min(sample_rate * 0.45);

        self.filter_l.tick(s_l * 0.2, cutoff, 0.3);
        self.filter_r.tick(s_r * 0.2, cutoff, 0.3);

        let out_l = self.filter_l.low * env * 0.32;
        let out_r = self.filter_r.low * env * 0.32;

        self.time += 1.0 / sample_rate as f64;
        (out_l, out_r)
    }
}
