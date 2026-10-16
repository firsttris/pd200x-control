/**
 * The EQ curve as the microphone's DSP shapes it: the magnitude response of
 * each enabled band as a biquad (Robert Bristow-Johnson's "Audio EQ
 * Cookbook"), summed in dB. Pure; drawn by the EQ graph and the preset tiles.
 */
import type { EqBand } from "./bindings";
import { clamp, num } from "./utils";

const SAMPLE_RATE = 48_000;
export const F_MIN = 20;
export const F_MAX = 22_000;
export const DB_RANGE = 15;
export const HPF_INDEX = 5;
export const LPF_INDEX = 6;

type Coeffs = { b0: number; b1: number; b2: number; a0: number; a1: number; a2: number };

function coefficients(band: EqBand, index: number): Coeffs | null {
  const f0 = clamp(band.frequency, F_MIN, SAMPLE_RATE / 2 - 1);
  // The firmware gets gain 0 and Q 1 for the high- and low-pass filters.
  const fixed = index === HPF_INDEX || index === LPF_INDEX;
  const q = fixed ? 1 : Math.max(0.1, num(band.q, 1));
  const gain = fixed ? 0 : num(band.gainDb);
  const w0 = (2 * Math.PI * f0) / SAMPLE_RATE;
  const cos = Math.cos(w0);
  const alpha = Math.sin(w0) / (2 * q);
  const A = 10 ** (gain / 40);
  switch (band.filter) {
    case "peaking":
      return { b0: 1 + alpha * A, b1: -2 * cos, b2: 1 - alpha * A, a0: 1 + alpha / A, a1: -2 * cos, a2: 1 - alpha / A };
    case "lowPass":
      return { b0: (1 - cos) / 2, b1: 1 - cos, b2: (1 - cos) / 2, a0: 1 + alpha, a1: -2 * cos, a2: 1 - alpha };
    case "highPass":
      return { b0: (1 + cos) / 2, b1: -(1 + cos), b2: (1 + cos) / 2, a0: 1 + alpha, a1: -2 * cos, a2: 1 - alpha };
    case "lowShelf":
    case "highShelf": {
      const s = band.filter === "lowShelf" ? 1 : -1;
      const sq = 2 * Math.sqrt(A) * alpha;
      return {
        b0: A * (A + 1 - s * (A - 1) * cos + sq),
        b1: s * 2 * A * (A - 1 - s * (A + 1) * cos),
        b2: A * (A + 1 - s * (A - 1) * cos - sq),
        a0: A + 1 + s * (A - 1) * cos + sq,
        a1: -s * 2 * (A - 1 + s * (A + 1) * cos),
        a2: A + 1 + s * (A - 1) * cos - sq,
      };
    }
    default:
      return null;
  }
}

function magnitudeDb(c: Coeffs, f: number): number {
  const w = (2 * Math.PI * f) / SAMPLE_RATE;
  const re = (x0: number, x1: number, x2: number) => x0 + x1 * Math.cos(w) + x2 * Math.cos(2 * w);
  const im = (x1: number, x2: number) => -(x1 * Math.sin(w) + x2 * Math.sin(2 * w));
  const num2 = re(c.b0, c.b1, c.b2) ** 2 + im(c.b1, c.b2) ** 2;
  const den2 = re(c.a0, c.a1, c.a2) ** 2 + im(c.a1, c.a2) ** 2;
  return 10 * Math.log10(num2 / den2);
}

/** The summed response in dB at `f` Hz. */
export function responseAt(bands: readonly EqBand[], f: number): number {
  return bands.reduce((sum, band, i) => {
    if (!band.enabled) return sum;
    const c = coefficients(band, i);
    return c ? sum + magnitudeDb(c, f) : sum;
  }, 0);
}

/** 0 … 1 across the logarithmic frequency axis. */
export function freqToX(f: number): number {
  return (Math.log10(clamp(f, F_MIN, F_MAX)) - Math.log10(F_MIN)) / (Math.log10(F_MAX) - Math.log10(F_MIN));
}

export function xToFreq(x: number): number {
  return 10 ** (Math.log10(F_MIN) + clamp(x, 0, 1) * (Math.log10(F_MAX) - Math.log10(F_MIN)));
}

/** An SVG path through the response, `width`×`height`, ±`range` dB. */
export function curvePath(
  bands: readonly EqBand[],
  width: number,
  height: number,
  range = DB_RANGE,
  points = 160,
): string {
  const parts: string[] = [];
  for (let i = 0; i <= points; i++) {
    const x = i / points;
    const db = clamp(responseAt(bands, xToFreq(x)), -range, range);
    const y = height / 2 - (db / range) * (height / 2);
    parts.push(`${i === 0 ? "M" : "L"}${(x * width).toFixed(1)} ${y.toFixed(1)}`);
  }
  return parts.join(" ");
}

/** Rounds a dragged frequency to steps that read well (10 Hz, 50 Hz, 100 Hz …). */
export function roundFreq(f: number): number {
  const step = f < 200 ? 1 : f < 1000 ? 5 : f < 5000 ? 10 : 50;
  return clamp(Math.round(f / step) * step, F_MIN, 22_000);
}
