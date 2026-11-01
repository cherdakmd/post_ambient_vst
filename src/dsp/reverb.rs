// Реверберация «эхо дворов» (Schroeder reverb).
//
// 6 параллельных комб-фильтров с демпфированием (длинные плоские стены
// панельных домов) + 2 последовательных all-pass'а для густоты.
// Размер двора (scale) управляет временем затухания.
//
// Буферы выделены под максимальный масштаб один раз - в аудио-потоке
// никаких аллокаций: при изменении параметра меняется только индекс чтения.

// Задержки комб-фильтров в сэмплах (при 44.1 кГц)
const COMB_DELAYS: [f32; 6] = [1116.0, 1188.0, 1277.0, 1356.0, 1422.0, 1491.0];
const ALLPASS_DELAYS: [f32; 2] = [225.0, 556.0];
// Стерео-смещение правого канала (сэмплы) - разные «дворы» слева и справа
const STEREO_SPREAD: f32 = 23.0;
const MAX_SCALE: f32 = 1.3; // максимум: 0.7 + 0.6 * 1.0

struct Comb {
    buf: Vec<f32>,
    base: f32,
    idx: usize,
    store: f32,
    damping: f32,
}

impl Comb {
    fn new(base: f32, spread: f32) -> Self {
        let cap = (((base + spread) * MAX_SCALE) as usize + 4).max(8);
        Self {
            buf: vec![0.0; cap],
            base,
            idx: 0,
            store: 0.0,
            damping: 0.35,
        }
    }

    fn process(&mut self, input: f32, scale: f32, feedback: f32, spread: f32) -> f32 {
        let cap = self.buf.len();
        let delay = (((self.base + spread) * scale) as usize + 1).clamp(1, cap - 1);
        let read_idx = (self.idx + cap - delay) % cap;
        let out = self.buf[read_idx];
        // Демпфирование: высокие частоты гаснут быстрее низких
        self.store = out * (1.0 - self.damping) + self.store * self.damping;
        self.buf[self.idx] = input + self.store * feedback;
        self.idx = (self.idx + 1) % cap;
        out
    }
}

struct AllPass {
    buf: Vec<f32>,
    base: f32,
    idx: usize,
    feedback: f32,
}

impl AllPass {
    fn new(base: f32, spread: f32) -> Self {
        let cap = (((base + spread) * MAX_SCALE) as usize + 4).max(8);
        Self {
            buf: vec![0.0; cap],
            base,
            idx: 0,
            feedback: 0.5,
        }
    }

    fn process(&mut self, input: f32, scale: f32, spread: f32) -> f32 {
        let cap = self.buf.len();
        let delay = (((self.base + spread) * scale) as usize + 1).clamp(1, cap - 1);
        let read_idx = (self.idx + cap - delay) % cap;
        let buf_out = self.buf[read_idx];
        let output = -input + buf_out;
        self.buf[self.idx] = input + buf_out * self.feedback;
        self.idx = (self.idx + 1) % cap;
        output
    }
}

pub struct Reverb {
    combs_l: Vec<Comb>,
    combs_r: Vec<Comb>,
    aps_l: Vec<AllPass>,
    aps_r: Vec<AllPass>,
}

impl Reverb {
    pub fn new() -> Self {
        Self {
            combs_l: COMB_DELAYS.iter().map(|&d| Comb::new(d, 0.0)).collect(),
            combs_r: COMB_DELAYS.iter().map(|&d| Comb::new(d, STEREO_SPREAD)).collect(),
            aps_l: ALLPASS_DELAYS.iter().map(|&d| AllPass::new(d, 0.0)).collect(),
            aps_r: ALLPASS_DELAYS
                .iter()
                .map(|&d| AllPass::new(d, STEREO_SPREAD))
                .collect(),
        }
    }

    pub fn reset(&mut self) {
        for c in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            c.buf.iter_mut().for_each(|s| *s = 0.0);
            c.idx = 0;
            c.store = 0.0;
        }
        for a in self.aps_l.iter_mut().chain(self.aps_r.iter_mut()) {
            a.buf.iter_mut().for_each(|s| *s = 0.0);
            a.idx = 0;
        }
    }

    pub fn process_sample(&mut self, l: f32, r: f32, size: f32, wet: f32) -> (f32, f32) {
        let scale = 0.7 + size * 0.6;
        let feedback = 0.7 + size * 0.17;

        let mut acc_l = 0.0f32;
        let mut acc_r = 0.0f32;
        for c in 0..self.combs_l.len() {
            acc_l += self.combs_l[c].process(l, scale, feedback, 0.0);
            acc_r += self.combs_r[c].process(r, scale, feedback, STEREO_SPREAD);
        }
        for a in 0..self.aps_l.len() {
            acc_l = self.aps_l[a].process(acc_l, scale, 0.0);
            acc_r = self.aps_r[a].process(acc_r, scale, STEREO_SPREAD);
        }

        (
            l * (1.0 - wet) + acc_l * wet * 0.3,
            r * (1.0 - wet) + acc_r * wet * 0.3,
        )
    }
}
