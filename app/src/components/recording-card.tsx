import { Circle, Play, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardTitle } from "@/components/ui/card";
import { MAX_RECORDING_SECONDS, type RecordingInfo } from "@/lib/bindings";
import { formatDb } from "@/lib/format";
import { useLevel, usePlayRecording, useRecording, useStartRecording, useStopRecording } from "@/lib/queries";
import { cn, num } from "@/lib/utils";
import { m } from "@/paraglide/messages";
import { LevelMeter } from "./level-meter";

function clock(ms: number): string {
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/**
 * Bars of the recording, scaled to its loudest part so quiet recordings show
 * their shape too (the dB values say how loud it was); during playback the
 * played part is coloured.
 */
function Waveform({ info, progress }: { info: RecordingInfo; progress: number | null }) {
  const n = info.waveform.length;
  const loudest = Math.max(1e-6, ...info.waveform.map((v) => num(v)));
  return (
    <svg viewBox={`0 0 ${n * 4} 60`} className="block h-16 w-full" preserveAspectRatio="none" aria-hidden>
      {info.waveform.map((v, i) => {
        const h = Math.max(1.5, (num(v) / loudest) * 56);
        const played = progress !== null && i / n <= progress;
        return (
          <rect
            // biome-ignore lint/suspicious/noArrayIndexKey: bars are positions, not items
            key={i}
            x={i * 4}
            y={30 - h / 2}
            width={2.6}
            height={h}
            rx={1}
            className={played ? "fill-accent" : progress === null ? "fill-foreground/60" : "fill-muted-foreground/40"}
          />
        );
      })}
    </svg>
  );
}

function Verdict({ peak }: { peak: number }) {
  if (peak < -12) return <p className="text-xs text-warning">{m.recording_quiet()}</p>;
  if (peak > -3) return <p className="text-xs text-danger">{m.recording_loud()}</p>;
  return <p className="text-xs text-success">{m.recording_good()}</p>;
}

function LiveMeter() {
  const level = useLevel();
  return <LevelMeter level={level} orientation="horizontal" count={40} className="h-16" />;
}

/** "Probeaufnahme": record up to 30 s through PipeWire and play it back. */
export function RecordingCard({ connected }: { connected: boolean }) {
  const { data } = useRecording();
  const start = useStartRecording();
  const stop = useStopRecording();
  const play = usePlayRecording();
  const state = data?.state ?? "idle";
  const info = data?.info ?? null;
  const elapsed = data?.elapsedMs ?? 0;
  const recording = state === "recording";
  const playing = state === "playing";
  const progress = playing && info ? Math.min(1, elapsed / Math.max(1, info.durationMs)) : null;

  return (
    <Card>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        <CardTitle>{m.recording_title()}</CardTitle>
        <span className="text-xs text-muted-foreground">{m.recording_hint()}</span>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        {recording ? (
          <Button variant="destructive" className="border border-danger/50" onClick={() => stop.mutate()}>
            <Square aria-hidden />
            {m.recording_stop()}
          </Button>
        ) : (
          <Button variant="outline" disabled={!connected || start.isPending} onClick={() => start.mutate()}>
            <Circle className="fill-danger text-danger" aria-hidden />
            {info ? m.recording_again() : m.recording_start()}
          </Button>
        )}
        {playing ? (
          <Button variant="soft" onClick={() => stop.mutate()}>
            <Square aria-hidden />
            {m.recording_stop_playback()}
          </Button>
        ) : (
          <Button variant="soft" disabled={!info || recording} onClick={() => play.mutate()}>
            <Play aria-hidden />
            {m.recording_play()}
          </Button>
        )}
        <span className={cn("ml-auto font-mono text-[13px] tabular-nums", recording && "text-danger")} role="timer">
          {recording
            ? `${clock(elapsed)} / ${clock(MAX_RECORDING_SECONDS * 1000)}`
            : info
              ? playing
                ? `${clock(elapsed)} / ${clock(info.durationMs)}`
                : clock(info.durationMs)
              : ""}
        </span>
      </div>
      <div className="rounded-xl border bg-card-inset p-3">
        {recording ? (
          <LiveMeter />
        ) : info ? (
          <Waveform info={info} progress={progress} />
        ) : (
          <p className="flex h-16 items-center justify-center text-[13px] text-muted-foreground">
            {m.recording_empty()}
          </p>
        )}
      </div>
      {info && !recording && (
        <div className="flex flex-wrap items-center gap-x-5 gap-y-1">
          <span className="text-[13px]">
            <span className="text-muted-foreground">{m.basics_peak()} </span>
            <span className="font-mono">{formatDb(num(info.peakDb))}</span>
          </span>
          <span className="text-[13px]">
            <span className="text-muted-foreground">{m.recording_mean()} </span>
            <span className="font-mono">{formatDb(num(info.meanDb))}</span>
          </span>
          <Verdict peak={num(info.peakDb, -96)} />
        </div>
      )}
    </Card>
  );
}
