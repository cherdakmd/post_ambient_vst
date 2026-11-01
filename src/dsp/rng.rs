// Детерминированный генератор случайных чисел (mulberry32).
// Одинаковый seed всегда даёт одинаковый шум - воспроизводимый рендер.

pub struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    pub fn next_f32(&mut self) -> f32 {
        self.state = self.state.wrapping_add(0x6d2b79f5);
        let mut t = self.state;
        t = t.wrapping_mul(t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add(t ^ (t >> 7)).wrapping_mul(t | 61);
        // ^ (t >> 14) и нормировка к [0, 1)
        ((t ^ (t >> 14)) as f64 / 4294967296.0) as f32
    }
}
