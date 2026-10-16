import type { Level } from "@/lib/bindings";
import { cn, num } from "@/lib/utils";

const FLOOR = -60;
const SEGMENTS = 40;

/** dB of each segment, loudest first. */
function segments(count: number): number[] {
  return Array.from({ length: count }, (_, i) => -(i * -FLOOR) / count);
}

function color(db: number): string {
  if (db > -6) return "bg-danger";
  if (db > -12) return "bg-warning";
  return "bg-success";
}

/**
 * A segment meter in dBFS with the peak as a lit segment. Vertical (loud at the
 * top) or horizontal (loud at the right).
 */
export function LevelMeter({
  level,
  orientation = "vertical",
  count = SEGMENTS,
  className,
}: {
  level: Level;
  orientation?: "vertical" | "horizontal";
  count?: number;
  className?: string;
}) {
  const current = num(level.currentDb, FLOOR);
  const peak = num(level.peakDb, FLOOR);
  const step = -FLOOR / count;
  const list = segments(count);
  const ordered = orientation === "vertical" ? list : [...list].reverse();
  return (
    // biome-ignore lint/a11y/useSemanticElements: <meter> cannot draw segments with a separate peak
    <div
      role="meter"
      aria-valuemin={FLOOR}
      aria-valuemax={0}
      aria-valuenow={Math.round(current)}
      aria-valuetext={`${Math.round(current)} dB`}
      className={cn("flex gap-[2px]", orientation === "vertical" ? "flex-col" : "flex-row", className)}
    >
      {ordered.map((db) => {
        const lit = db <= current || (peak > FLOOR && Math.abs(db - peak) < step / 2 + 0.01);
        return (
          <div
            key={db}
            className={cn("flex-1 rounded-[2px] transition-opacity duration-75", color(db), !lit && "opacity-15")}
          />
        );
      })}
    </div>
  );
}

export const METER_SCALE = [0, -6, -12, -18, -24, -36, -48, -60];
