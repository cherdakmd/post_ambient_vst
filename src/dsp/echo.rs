// Эхо «панельные дома» - ping-pong delay.
//
// Две линии задержки L и R с кросс-связью: сигнал «прыгает» между
// каналами, имитируя отражения между домами.

pub struct Echo {
    buf_l: Vec<f32>,
    buf_r: Vec<f32>,
    idx: usize,
}

impl Echo {
    /// capacity - максимальная задержка в сэмплах (выделяется один раз).
    pub fn new(capacity: usize) -> Self {
        Self {
            buf_l: vec![0.0; capacity.max(2)],
            buf_r: vec![0.0; capacity.max(2)],
            idx: 0,
        }
    }

    pub fn reset(&mut self) {
        self.buf_l.iter_mut().for_each(|s| *s = 0.0);
        self.buf_r.iter_mut().for_each(|s| *s = 0.0);
        self.idx = 0;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_sample(&mut self, l: f32, r: f32, delay_samples: usize, feedback: f32, mix: f32) -> (f32, f32) {
        let cap = self.buf_l.len();
        let d = delay_samples.clamp(1, cap - 1);
        let tap_idx = (self.idx + cap - d) % cap;

        let tap_l = self.buf_l[tap_idx];
        let tap_r = self.buf_r[tap_idx];

        // Ping-pong: в линию L пишем исходный L + эхо из R, и наоборот
        self.buf_l[self.idx] = l + tap_r * feedback;
        self.buf_r[self.idx] = r + tap_l * feedback;

        let out_l = l * (1.0 - mix) + tap_l * mix;
        let out_r = r * (1.0 - mix) + tap_r * mix;

        self.idx = (self.idx + 1) % cap;
        (out_l, out_r)
    }
}
