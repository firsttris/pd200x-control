import { describe, expect, it } from "vitest";
import { type EqBand, PRESETS } from "./bindings";
import { curvePath, freqToX, responseAt, roundFreq, xToFreq } from "./eq";

const peak = (frequency: number, gainDb: number, q = 1): EqBand => ({
  enabled: true,
  filter: "peaking",
  frequency,
  gainDb,
  q,
});
const bands = (id: string) => PRESETS.find((p) => p.id === id)!.eq as unknown as EqBand[];

describe("responseAt", () => {
  it("a peaking band reaches its gain at its frequency", () => {
    expect(responseAt([peak(1000, 6)], 1000)).toBeCloseTo(6, 1);
    expect(responseAt([peak(1000, -4)], 1000)).toBeCloseTo(-4, 1);
    expect(Math.abs(responseAt([peak(1000, 6)], 50))).toBeLessThan(0.2);
  });

  it("disabled bands and flat do nothing", () => {
    expect(responseAt([{ ...peak(1000, 6), enabled: false }], 1000)).toBe(0);
    expect(responseAt(bands("flat"), 300)).toBe(0);
  });

  it("the high-pass cuts below its frequency (Q 1 like the firmware)", () => {
    const lowCut = bands("low_cut");
    expect(responseAt(lowCut, 30)).toBeLessThan(-12);
    expect(Math.abs(responseAt(lowCut, 1000))).toBeLessThan(0.1);
  });

  it("shelves lift or cut one side", () => {
    const low: EqBand = { enabled: true, filter: "lowShelf", frequency: 200, gainDb: -5, q: 0.8 };
    expect(responseAt([low], 30)).toBeCloseTo(-5, 0);
    expect(Math.abs(responseAt([low], 5000))).toBeLessThan(0.2);
    const high: EqBand = { enabled: true, filter: "highShelf", frequency: 5000, gainDb: 4, q: 0.7 };
    expect(responseAt([high], 18000)).toBeCloseTo(4, 0);
  });

  it("the deep preset rolls off the highs with its low-pass", () => {
    expect(responseAt(bands("deep"), 15000)).toBeLessThan(-6);
  });
});

describe("axis", () => {
  it("is logarithmic and invertible", () => {
    expect(freqToX(20)).toBe(0);
    expect(freqToX(22000)).toBe(1);
    expect(xToFreq(freqToX(1000))).toBeCloseTo(1000, 6);
    expect(freqToX(5)).toBe(0);
  });

  it("rounds dragged frequencies to readable steps", () => {
    expect(roundFreq(123.4)).toBe(123);
    expect(roundFreq(1234)).toBe(1230);
    expect(roundFreq(7777)).toBe(7800);
    expect(roundFreq(5)).toBe(20);
  });

  it("draws a path across the width", () => {
    const d = curvePath(bands("natural"), 100, 40, 15, 10);
    expect(d.startsWith("M0.0 ")).toBe(true);
    expect(d.split("L")).toHaveLength(11);
  });
});
