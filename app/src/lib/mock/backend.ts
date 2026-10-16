/**
 * In-memory implementation of every Tauri command, used when the frontend
 * runs in a plain browser (`pnpm dev:mock`), in unit tests and in the
 * Playwright tests. It behaves like src-tauri/src/hub.rs with a simulated
 * microphone, so every screen can be used without hardware.
 */
import {
  type Change,
  type CmdError,
  DEFAULT_TONE,
  type EqBand,
  type Level,
  MAX_BOOST_DB,
  PRESETS,
  type Profile,
  type Proposal,
  type Snapshot,
  TARGET_PEAK_DB,
  type Tone,
} from "../bindings";
import { toneOf, withChange } from "../snapshot";

type Args = Record<string, unknown>;

export interface MockState {
  snapshot: Snapshot;
  /** `null` = the microphone is plugged in; else the error kind. */
  problem: string | null;
  /** Peak of the simulated voice in dB, before the software boost. */
  voiceDb: number;
  /** Delay of every command in ms. */
  latency: number;
  /** Every command with its arguments, oldest first. */
  calls: { cmd: string; args: Args }[];
}

function fail(kind: string, message = kind): never {
  throw { kind, message } satisfies CmdError;
}

const round1 = (v: number) => Math.round(v * 10) / 10;
const DB_PER_GAIN_STEP = 0.42;

/** Same rules as `calibrate::propose` in the core. */
export function propose(peak: number, gain: number, boostDb: number): Proposal {
  let needed = TARGET_PEAK_DB - (peak + boostDb);
  let newGain = gain;
  let newBoost = boostDb;
  if (needed > 0) {
    const fromGain = Math.min(needed, (100 - newGain) * DB_PER_GAIN_STEP);
    newGain += fromGain / DB_PER_GAIN_STEP;
    needed -= fromGain;
    newBoost += needed;
  } else {
    const fromBoost = Math.min(-needed, newBoost);
    newBoost -= fromBoost;
    needed += fromBoost;
    newGain += needed / DB_PER_GAIN_STEP;
  }
  newGain = Math.round(Math.min(100, Math.max(0, newGain)));
  newBoost = round1(Math.min(MAX_BOOST_DB, Math.max(0, newBoost)));
  return {
    measuredPeakDb: round1(peak),
    gain: newGain,
    boostDb: newBoost,
    expectedPeakDb: round1(peak + (newGain - gain) * DB_PER_GAIN_STEP + newBoost),
  };
}

/** `podcast`, `podcast-2`, … like `unique_id` in hub.rs. */
export function uniqueId(name: string, profiles: Profile[]): string {
  let base = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  if (!base || PRESETS.some((p) => p.id === base)) base = `profile-${base}`;
  const taken = (id: string) => profiles.some((p) => p.id === id);
  if (!taken(base)) return base;
  let n = 2;
  while (taken(`${base}-${n}`)) n++;
  return `${base}-${n}`;
}

export function initialState(): MockState {
  return {
    snapshot: {
      live: { mute: false, gain: 100, monitor: true, headphone: 41, noiseCancellation: false, ncStrength: 21 },
      problem: null,
      boostDb: 0,
      // As a microphone reports it: flat EQ, cyan ring
      tone: { ...(structuredClone(DEFAULT_TONE) as unknown as Tone), source: "flat" },
      lighting: { enabled: true, brightness: 70, color: "cyan", linkage: false },
      settings: { tone: null, lighting: null, profiles: [], restoreOnConnect: false, boostDb: 10 },
    },
    problem: null,
    voiceDb: -21.5,
    latency: 30,
    calls: [],
  };
}

export function createMockBackend(state: MockState = initialState()) {
  const settings = () => state.snapshot.settings;
  const needDevice = () => {
    if (state.problem) fail(state.problem);
  };

  /** A voice that rises and falls, about −30 dB average with peaks at `voiceDb`. */
  const level = (): Level => {
    if (state.problem || state.snapshot.live?.mute) return { currentDb: -60, peakDb: -60, clipping: false };
    const t = Date.now() / 1000;
    const swing = (Math.sin(t * 5.1) + Math.sin(t * 2.3) * 0.6 + 1.6) / 3.2;
    const current = Math.max(-60, state.voiceDb - 14 + swing * 14);
    return { currentDb: round1(current), peakDb: state.voiceDb, clipping: state.voiceDb >= -1 };
  };

  const handlers: Record<string, (args: Args) => unknown> = {
    snapshot: () => {
      const s = structuredClone(state.snapshot);
      return state.problem ? { ...s, live: null, tone: null, lighting: null, problem: state.problem } : s;
    },
    level,
    apply_change: ({ change }) => {
      needDevice();
      state.snapshot = withChange(state.snapshot, change as Change);
      return null;
    },
    apply_eq: ({ id }) => {
      needDevice();
      const eq: EqBand[] | undefined =
        (PRESETS.find((p) => p.id === id)?.eq as unknown as EqBand[] | undefined) ??
        settings().profiles?.find((p) => p.id === id)?.eq;
      if (!eq) fail("invalidValue", `profile ${id}`);
      const tone = { ...toneOf(state.snapshot), source: id as string, eq: structuredClone(eq) };
      settings().tone = tone;
      state.snapshot.tone = structuredClone(tone);
      return null;
    },
    save_profile: ({ name }) => {
      const trimmed = String(name).trim();
      if (!trimmed) fail("invalidValue", "empty name");
      const profiles = settings().profiles ?? [];
      settings().profiles = profiles;
      const tone = toneOf(state.snapshot);
      const profile: Profile = { id: uniqueId(trimmed, profiles), name: trimmed, eq: tone.eq };
      profiles.push(profile);
      settings().tone = { ...tone, source: profile.id };
      if (state.snapshot.tone) state.snapshot.tone.source = profile.id;
      return profile;
    },
    rename_profile: ({ id, name }) => {
      const p = settings().profiles?.find((p) => p.id === id);
      if (!p || !String(name).trim()) fail("invalidValue", `profile ${id}`);
      p.name = String(name).trim();
      return null;
    },
    delete_profile: ({ id }) => {
      settings().profiles = (settings().profiles ?? []).filter((p) => p.id !== id);
      for (const tone of [settings().tone, state.snapshot.tone]) {
        if (tone && tone.source === id) tone.source = null;
      }
      return null;
    },
    set_boost: ({ db }) => {
      const v = Number(db);
      if (!(v >= 0 && v <= MAX_BOOST_DB)) fail("invalidValue", "boost");
      state.snapshot.boostDb = v;
      if (v > 0) settings().boostDb = v;
      return null;
    },
    calibrate: () => {
      needDevice();
      const live = state.snapshot.live;
      if (!live || state.voiceDb < -50) fail("noVoice");
      return propose(state.voiceDb, live.gain, state.snapshot.boostDb ?? 0);
    },
    apply_calibration: ({ gain, boostDb }) => {
      needDevice();
      state.snapshot = withChange(state.snapshot, { kind: "gain", value: Number(gain) });
      state.snapshot.boostDb = Number(boostDb);
      if (Number(boostDb) > 0) settings().boostDb = Number(boostDb);
      return null;
    },
    set_restore_on_connect: ({ on }) => {
      settings().restoreOnConnect = Boolean(on);
      return null;
    },
  };

  async function invoke(cmd: string, args: Args): Promise<unknown> {
    state.calls.push({ cmd, args });
    const handler = handlers[cmd];
    if (!handler) fail("other", `unknown command ${cmd}`);
    // The calibration listens for its seconds, like the real one.
    const delay = cmd === "calibrate" ? Number(args.seconds) * 1000 : cmd === "level" ? 0 : state.latency;
    if (state.latency > 0 && delay > 0) await new Promise((r) => setTimeout(r, delay));
    return handler(args);
  }

  return { state, invoke };
}

export type MockBackend = ReturnType<typeof createMockBackend>;
