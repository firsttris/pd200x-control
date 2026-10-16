import { describe, expect, it } from "vitest";
import { DEFAULT_TONE, PRESETS } from "./bindings";
import { initialState } from "./mock/backend";
import { lightingOf, toneOf, withChange } from "./snapshot";

const base = () => initialState().snapshot;

describe("withChange", () => {
  it("updates live registers", () => {
    expect(withChange(base(), { kind: "gain", value: 40 }).live?.gain).toBe(40);
    expect(withChange(base(), { kind: "mute", value: true }).live?.mute).toBe(true);
    expect(withChange(base(), { kind: "ncStrength", value: 30 }).live?.ncStrength).toBe(30);
  });

  it("remembers tone and marks a hand-edited EQ", () => {
    const band = PRESETS[5].eq[2];
    const s = withChange(base(), { kind: "eqBand", value: { index: 2, band } });
    expect(s.settings.tone?.eq[2]).toEqual(band);
    expect(s.settings.tone?.source).toBeNull();
    const c = withChange(s, { kind: "compressor", value: { enabled: true, thresholdDb: -9 } });
    expect(c.settings.tone?.compressor.thresholdDb).toBe(-9);
  });

  it("remembers lighting", () => {
    const s = withChange(base(), { kind: "rgbColor", value: "cyan" });
    expect(s.settings.lighting?.color).toBe("cyan");
    expect(withChange(s, { kind: "linkage", value: true }).settings.lighting?.linkage).toBe(true);
  });

  it("leaves the input alone", () => {
    const before = base();
    withChange(before, { kind: "rgbBrightness", value: 10 });
    expect(before.settings.lighting).toBeNull();
  });

  it("defaults come from the backend constants", () => {
    expect(toneOf(undefined).eq).toHaveLength(DEFAULT_TONE.eq.length);
    expect(lightingOf(undefined).color).toBe("rainbow");
  });
});

describe("device state", () => {
  it("the microphone's tone and lighting win over the saved ones and follow changes", () => {
    const s = base();
    s.settings.lighting = { enabled: true, brightness: 10, color: "red", linkage: false };
    expect(lightingOf(s).color).toBe("cyan");
    const next = withChange(s, { kind: "rgbColor", value: "green" });
    expect(next.lighting?.color).toBe("green");
    expect(next.settings.lighting?.color).toBe("green");
    expect(toneOf(s).source).toBe("flat");
  });
});
