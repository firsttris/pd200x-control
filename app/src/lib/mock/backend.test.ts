import { describe, expect, it } from "vitest";
import { createMockBackend, propose, uniqueId } from "./backend";

const backend = () => createMockBackend();

describe("mock backend", () => {
  it("applies changes and reports them in the snapshot", async () => {
    const b = backend();
    b.state.latency = 0;
    await b.invoke("apply_change", { change: { kind: "gain", value: 50 } });
    const s = (await b.invoke("snapshot", {})) as { live: { gain: number } };
    expect(s.live.gain).toBe(50);
  });

  it("presets and own profiles", async () => {
    const b = backend();
    b.state.latency = 0;
    await b.invoke("apply_eq", { id: "natural" });
    expect(b.state.snapshot.settings.tone?.source).toBe("natural");
    const p = (await b.invoke("save_profile", { name: "Podcast" })) as { id: string };
    expect(p.id).toBe("podcast");
    expect(b.state.snapshot.settings.tone?.source).toBe("podcast");
    await b.invoke("rename_profile", { id: "podcast", name: "Talk" });
    expect(b.state.snapshot.settings.profiles?.[0].name).toBe("Talk");
    await b.invoke("delete_profile", { id: "podcast" });
    expect(b.state.snapshot.settings.profiles).toHaveLength(0);
    expect(b.state.snapshot.settings.tone?.source).toBeNull();
    await expect(b.invoke("apply_eq", { id: "nope" })).rejects.toMatchObject({ kind: "invalidValue" });
  });

  it("without a microphone, device commands fail and the snapshot says why", async () => {
    const b = backend();
    b.state.latency = 0;
    b.state.problem = "permissionDenied";
    await expect(b.invoke("apply_change", { change: { kind: "mute", value: true } })).rejects.toMatchObject({
      kind: "permissionDenied",
    });
    expect(await b.invoke("snapshot", {})).toMatchObject({ live: null, problem: "permissionDenied" });
    expect(await b.invoke("level", {})).toMatchObject({ peakDb: -60 });
  });

  it("boost and calibration", async () => {
    const b = backend();
    b.state.latency = 0;
    await b.invoke("set_boost", { db: 12 });
    expect(b.state.snapshot.settings.boostDb).toBe(12);
    await expect(b.invoke("set_boost", { db: 40 })).rejects.toMatchObject({ kind: "invalidValue" });
    await b.invoke("set_boost", { db: 0 });
    const p = await b.invoke("calibrate", { seconds: 5 });
    expect(p).toMatchObject({ gain: 100, boostDb: 12.5, expectedPeakDb: -9 });
    await b.invoke("apply_calibration", { gain: 90, boostDb: 3 });
    expect(b.state.snapshot.live?.gain).toBe(90);
    b.state.voiceDb = -58;
    await expect(b.invoke("calibrate", { seconds: 5 })).rejects.toMatchObject({ kind: "noVoice" });
  });

  it("unknown commands fail", async () => {
    await expect(backend().invoke("nope", {})).rejects.toMatchObject({ kind: "other" });
  });
});

describe("rules shared with the core", () => {
  it("propose matches calibrate.rs", () => {
    expect(propose(-25, 80, 0)).toMatchObject({ gain: 100, boostDb: 7.6 });
    expect(propose(-12, 100, 10)).toMatchObject({ gain: 100, boostDb: 3 });
    expect(propose(-2, 100, 2)).toMatchObject({ gain: 83, boostDb: 0 });
  });

  it("uniqueId matches hub.rs", () => {
    const p = (id: string) => ({ id, name: id, eq: [] });
    expect(uniqueId("Podcast (eigen)", [])).toBe("podcast-eigen");
    expect(uniqueId("Podcast", [p("podcast")])).toBe("podcast-2");
    expect(uniqueId("Natural", [])).toBe("profile-natural");
  });
});
