// src/dsp/distortion.js - Дисторшн «бетонные стены».
//
// Мягкое волновое искажение (tanh) с автоматической нормировкой,
// смешанное с чистым сигналом. Именно этот «сырой» слой даёт
// характерный пост-панковый гранж 2000-х.

function applyDistortion(buffer, amount) {
  if (amount <= 0) return;
  const drive = 1 + amount * 12;
  const norm = Math.tanh(drive);
  for (let i = 0; i < buffer.length; i++) {
    const dry = buffer[i];
    const wet = Math.tanh(dry * drive) / norm;
    buffer[i] = dry * (1 - amount) + wet * amount;
  }
}

module.exports = { applyDistortion };
