#!/usr/bin/env python3
"""Анализ спектрального баланса отрендеренного WAV (только стандартная библиотека).

Использование:
    python3 tools/analyze_bands.py output/post_ambient_super_atmospheric.wav

Показывает RMS в трёх полосах (низ/середина/верх), пик, наличие клиппинга
и стерео-базу. Полосы выделяются однополюсными ФНЧ (разностный метод),
поэтому точность «грубая, но достаточная», чтобы увидеть перекос в миксе.
"""

import array
import math
import sys
import wave


def load_mono(path):
    with wave.open(path) as w:
        frames = w.readframes(w.getnframes())
        data = array.array("h")
        data.frombytes(frames)
        channels = w.getnchannels()
        rate = w.getframerate()
    mono = [data[i] / 32768.0 for i in range(0, len(data), channels)]
    stereo = None
    if channels >= 2:
        left = [data[i] / 32768.0 for i in range(0, len(data), channels)]
        right = [data[i + 1] / 32768.0 for i in range(0, len(data), channels)]
        stereo = (left, right)
    return mono, stereo, rate


def onepole(x, alpha):
    y = 0.0
    out = []
    for s in x:
        y += alpha * (s - y)
        out.append(y)
    return out


def rms(x):
    return math.sqrt(sum(s * s for s in x) / max(1, len(x)))


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "output/post_ambient_super_atmospheric.wav"
    mono, stereo, rate = load_mono(path)

    a_low = 1 - math.exp(-2 * math.pi * 120 / rate)
    a_mid = 1 - math.exp(-2 * math.pi * 1500 / rate)
    lp_low = onepole(mono, a_low)
    lp_mid = onepole(mono, a_mid)
    low = lp_low
    mid = [b - a for a, b in zip(lp_low, lp_mid)]
    high = [s - b for s, b in zip(mono, lp_mid)]

    peak = max(abs(min(mono)), abs(max(mono)))
    clipped = sum(1 for s in mono if abs(s) >= 0.999)

    print(f"file      : {path}")
    print(f"rate      : {rate} Hz | {len(mono)} samples | {len(mono)/rate:.2f} s")
    print(f"peak      : {peak:.4f} | clipped: {clipped}")
    print(f"total RMS : {rms(mono):.4f}")
    print(f"low  <120 Hz   : {rms(low):.4f}")
    print(f"mid  120-1500  : {rms(mid):.4f}")
    print(f"high >1500 Hz  : {rms(high):.4f}")
    if stereo:
        left, right = stereo
        side = [(l - r) * 0.5 for l, r in zip(left, right)]
        print(f"side (stereo)  : {rms(side):.4f}")


if __name__ == "__main__":
    main()
