import { type KeyboardEvent, type PointerEvent, useRef } from "react";
import type { EqBand } from "@/lib/bindings";
import { curvePath, DB_RANGE, freqToX, HPF_INDEX, LPF_INDEX, roundFreq, xToFreq } from "@/lib/eq";
import { formatDb, formatHz } from "@/lib/format";
import { clamp, cn, num } from "@/lib/utils";
import { m } from "@/paraglide/messages";

const W = 1000;
const H = 220;
const GRID_HZ = [50, 100, 200, 500, 1000, 2000, 5000, 10000];
const GRID_DB = [12, 6, 0, -6, -12];

export function bandLabel(index: number): string {
  if (index === HPF_INDEX) return m.eq_hpf_short();
  if (index === LPF_INDEX) return m.eq_lpf_short();
  return String(index + 1);
}

const isPass = (i: number) => i === HPF_INDEX || i === LPF_INDEX;
const yOf = (db: number) => H / 2 - (clamp(db, -DB_RANGE, DB_RANGE) / DB_RANGE) * (H / 2);

/**
 * The EQ curve with a handle per band. Drag: frequency left/right, level
 * up/down (not for HPF/LPF); mouse wheel: Q. Arrow keys do the same.
 */
export function EqGraph({
  bands,
  selected,
  disabled,
  onSelect,
  onChange,
}: {
  bands: EqBand[];
  selected: number;
  disabled?: boolean;
  onSelect: (index: number) => void;
  onChange: (index: number, band: EqBand) => void;
}) {
  const svg = useRef<SVGSVGElement>(null);
  const dragging = useRef<number | null>(null);

  const fromPointer = (e: PointerEvent, index: number) => {
    const rect = svg.current?.getBoundingClientRect();
    if (!rect) return;
    const x = (e.clientX - rect.left) / rect.width;
    const y = (e.clientY - rect.top) / rect.height;
    const band = bands[index];
    const frequency = roundFreq(xToFreq(x));
    const gainDb = isPass(index) ? num(band.gainDb) : Math.round(clamp((0.5 - y) * 2 * DB_RANGE, -12, 12) * 2) / 2;
    onChange(index, { ...band, enabled: true, frequency, gainDb });
  };

  const key = (e: KeyboardEvent, index: number) => {
    const band = bands[index];
    const f = band.frequency;
    const g = num(band.gainDb);
    const next: Partial<EqBand> | null =
      e.key === "ArrowRight"
        ? { frequency: roundFreq(f * 1.06) }
        : e.key === "ArrowLeft"
          ? { frequency: roundFreq(f / 1.06) }
          : e.key === "ArrowUp" && !isPass(index)
            ? { gainDb: clamp(g + 0.5, -12, 12) }
            : e.key === "ArrowDown" && !isPass(index)
              ? { gainDb: clamp(g - 0.5, -12, 12) }
              : null;
    if (!next) return;
    e.preventDefault();
    onChange(index, { ...band, enabled: true, ...next });
  };

  return (
    <svg
      ref={svg}
      viewBox={`0 0 ${W} ${H}`}
      className={cn("block h-auto w-full touch-none select-none rounded-xl bg-card-inset", disabled && "opacity-60")}
      role="img"
      aria-label={m.eq_graph_label()}
      onPointerMove={(e) => dragging.current !== null && fromPointer(e, dragging.current)}
      onPointerUp={() => (dragging.current = null)}
      onPointerLeave={() => (dragging.current = null)}
    >
      {GRID_HZ.map((f) => (
        <g key={f}>
          <line x1={freqToX(f) * W} x2={freqToX(f) * W} y1={0} y2={H} className="stroke-border" strokeWidth={1} />
          <text x={freqToX(f) * W + 4} y={H - 6} className="fill-muted-foreground font-mono" fontSize={11}>
            {formatHz(f)}
          </text>
        </g>
      ))}
      {GRID_DB.map((db) => (
        <g key={db}>
          <line x1={0} x2={W} y1={yOf(db)} y2={yOf(db)} className="stroke-border" strokeWidth={db === 0 ? 1.5 : 1} />
          <text x={6} y={yOf(db) - 4} className="fill-muted-foreground font-mono" fontSize={11}>
            {db > 0 ? `+${db}` : db === 0 ? "0 dB" : `−${-db}`}
          </text>
        </g>
      ))}
      <path d={`${curvePath(bands, W, H)} L${W} ${H / 2} L0 ${H / 2} Z`} className="fill-accent/10" />
      <path d={curvePath(bands, W, H)} className="fill-none stroke-accent" strokeWidth={2.5} />
      {bands.map((band, i) => {
        // Handles at 20 Hz or 22 kHz stay fully inside the graph.
        const cx = clamp(freqToX(band.frequency) * W, 16, W - 16);
        const cy = isPass(i) ? H / 2 : yOf(num(band.gainDb));
        const active = i === selected;
        const label = bandLabel(i);
        return (
          <g
            key={label}
            role="slider"
            tabIndex={disabled ? -1 : 0}
            aria-label={m.eq_band_label({ band: label })}
            aria-valuemin={20}
            aria-valuemax={22000}
            aria-valuenow={band.frequency}
            aria-valuetext={`${formatHz(band.frequency)}${isPass(i) ? "" : `, ${formatDb(num(band.gainDb), { signed: true })}`}`}
            className={cn("cursor-grab outline-none", disabled && "pointer-events-none")}
            onPointerDown={(e) => {
              (e.target as Element).setPointerCapture?.(e.pointerId);
              dragging.current = i;
              onSelect(i);
            }}
            onWheel={(e) => {
              if (isPass(i)) return;
              const q = clamp(Math.round((num(band.q, 1) + (e.deltaY < 0 ? 0.1 : -0.1)) * 100) / 100, 0.1, 10);
              onChange(i, { ...band, q });
            }}
            onFocus={() => onSelect(i)}
            onKeyDown={(e) => key(e, i)}
          >
            <circle
              cx={cx}
              cy={cy}
              r={active ? 15 : 12}
              className={cn(
                active ? "fill-accent stroke-foreground" : "fill-card stroke-foreground",
                !band.enabled && "stroke-muted-foreground opacity-50",
              )}
              strokeWidth={2}
              strokeDasharray={band.enabled ? undefined : "3 3"}
            />
            <text
              x={cx}
              y={cy + 4}
              textAnchor="middle"
              fontSize={11}
              fontWeight={600}
              className={active ? "fill-accent-foreground" : "fill-foreground"}
            >
              {label}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

/** A small curve for the preset tiles. */
export function MiniCurve({ bands, className }: { bands: EqBand[]; className?: string }) {
  return (
    <svg viewBox="0 0 100 44" className={cn("block h-11 w-full rounded-md bg-card-inset", className)} aria-hidden>
      <line x1={0} x2={100} y1={22} y2={22} className="stroke-border" strokeWidth={1} />
      <path d={curvePath(bands, 100, 44, 15, 60)} className="fill-none stroke-current" strokeWidth={2} />
    </svg>
  );
}
