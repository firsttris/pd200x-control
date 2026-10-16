import { describe, expect, it } from "vitest";
import { formatDb, formatHz, formatNumber } from "./format";

describe("format (German)", () => {
  it("dB with comma and real minus", () => {
    expect(formatDb(-6.4)).toBe("−6,4 dB");
    expect(formatDb(14.6, { signed: true })).toBe("+14,6 dB");
    expect(formatDb(0, { signed: true })).toBe("0,0 dB");
    expect(formatDb(-20, { digits: 0 })).toBe("−20 dB");
  });

  it("numbers without grouping, so fields read back right", () => {
    expect(formatNumber(1000)).toBe("1000");
    expect(formatNumber(-12)).toBe("−12");
  });

  it("frequencies", () => {
    expect(formatHz(80)).toBe("80 Hz");
    expect(formatHz(1320)).toBe("1,3 kHz");
    expect(formatHz(5000)).toBe("5 kHz");
    expect(formatHz(22000)).toBe("22 kHz");
  });
});
