/**
 * How a change shows in the snapshot, before the device confirms it: used
 * for optimistic updates and by the mock backend. Mirrors `remember` in
 * src-tauri/src/hub.rs plus the live registers.
 */
import { type Change, DEFAULT_LIGHTING, DEFAULT_TONE, type Lighting, type Snapshot, type Tone } from "./bindings";

const clone = <T>(v: T): T => structuredClone(v);

/** The tone on the microphone, else as saved, else the default. */
export function toneOf(snapshot: Snapshot | undefined): Tone {
  return clone(snapshot?.tone ?? snapshot?.settings.tone ?? (DEFAULT_TONE as unknown as Tone));
}

/** The ring light on the microphone, else as saved, else the default. */
export function lightingOf(snapshot: Snapshot | undefined): Lighting {
  return clone(snapshot?.lighting ?? snapshot?.settings.lighting ?? (DEFAULT_LIGHTING as unknown as Lighting));
}

export function withChange(snapshot: Snapshot, change: Change): Snapshot {
  const next = clone(snapshot);
  const live = next.live;
  // Device state and settings both follow the change.
  const tones = () => {
    next.settings.tone = next.settings.tone ?? toneOf(snapshot);
    return next.tone ? [next.tone, next.settings.tone] : [next.settings.tone];
  };
  const lights = () => {
    next.settings.lighting = next.settings.lighting ?? lightingOf(snapshot);
    return next.lighting ? [next.lighting, next.settings.lighting] : [next.settings.lighting];
  };
  switch (change.kind) {
    case "mute":
      if (live) live.mute = change.value;
      break;
    case "gain":
      if (live) live.gain = change.value;
      break;
    case "monitor":
      if (live) live.monitor = change.value;
      break;
    case "headphone":
      if (live) live.headphone = change.value;
      break;
    case "noiseCancellation":
      if (live) live.noiseCancellation = change.value;
      break;
    case "ncStrength":
      if (live) live.ncStrength = change.value;
      break;
    case "eqBand":
      for (const t of tones()) {
        t.eq[change.value.index] = change.value.band;
        t.source = null;
      }
      break;
    case "compressor":
      for (const t of tones()) t.compressor = change.value;
      break;
    case "limiter":
      for (const t of tones()) t.limiter = change.value;
      break;
    case "rgbEnabled":
      for (const l of lights()) l.enabled = change.value;
      break;
    case "rgbBrightness":
      for (const l of lights()) l.brightness = change.value;
      break;
    case "rgbColor":
      for (const l of lights()) l.color = change.value;
      break;
    case "linkage":
      for (const l of lights()) l.linkage = change.value;
      break;
  }
  return next;
}
