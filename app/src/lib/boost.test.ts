import { describe, expect, it } from "vitest";
import { dbToPercent, percentToDb } from "./format";

describe("boost in percent like KDE", () => {
  it("175 % is +14,6 dB", () => {
    expect(percentToDb(175)).toBe(14.6);
    expect(dbToPercent(14.6)).toBe(175);
    expect(percentToDb(100)).toBe(0);
    expect(dbToPercent(18)).toBe(200);
  });
});
