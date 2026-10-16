import { describe, expect, it } from "vitest";
import { smooth } from "./queries";

describe("meter ballistics", () => {
  it("rises at once and falls slowly", () => {
    expect(smooth(-40, -10, 60)).toBe(-10);
    expect(smooth(-10, -50, 60)).toBeCloseTo(-11.08, 2);
    expect(smooth(-10, -50, 3000)).toBe(-50);
  });
});
