// src/dsp/wav.js - Запись WAV-файлов (16-bit PCM, без зависимостей).

function floatToInt16(sample) {
  const s = Math.max(-1, Math.min(1, sample));
  return Math.round(s * 32767);
}

function writeWav(channels, sampleRate) {
  const numChannels = channels.length;
  const numFrames = channels[0].length;
  const blockAlign = numChannels * 2; // 2 байта на сэмпл (16-bit)
  const dataSize = numFrames * blockAlign;

  const buffer = Buffer.alloc(44 + dataSize);
  buffer.write('RIFF', 0);
  buffer.writeUInt32LE(36 + dataSize, 4);
  buffer.write('WAVE', 8);
  buffer.write('fmt ', 12);
  buffer.writeUInt32LE(16, 16);            // размер fmt-блока
  buffer.writeUInt16LE(1, 20);             // PCM
  buffer.writeUInt16LE(numChannels, 22);
  buffer.writeUInt32LE(sampleRate, 24);
  buffer.writeUInt32LE(sampleRate * blockAlign, 28); // байт/сек
  buffer.writeUInt16LE(blockAlign, 32);
  buffer.writeUInt16LE(16, 34);            // битность
  buffer.write('data', 36);
  buffer.writeUInt32LE(dataSize, 40);

  let offset = 44;
  for (let i = 0; i < numFrames; i++) {
    for (let ch = 0; ch < numChannels; ch++) {
      buffer.writeInt16LE(floatToInt16(channels[ch][i]), offset);
      offset += 2;
    }
  }
  return buffer;
}

module.exports = { writeWav };
