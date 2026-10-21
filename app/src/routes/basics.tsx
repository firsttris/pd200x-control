import { CircleAlert, Mic, MicOff } from "lucide-react";
import { useState } from "react";
import { CalibrateDialog } from "@/components/calibrate-dialog";
import { LevelMeter, METER_SCALE } from "@/components/level-meter";
import { RecordingCard } from "@/components/recording-card";
import { Button } from "@/components/ui/button";
import { Card, CardTitle, Value } from "@/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { MAX_BOOST_DB } from "@/lib/bindings";
import { dbToPercent, formatDb, formatNumber, percentToDb } from "@/lib/format";
import { useApply, useDraft, useLevel, useSetBoost, useSetProtected, useSnapshot } from "@/lib/queries";
import { cn, num } from "@/lib/utils";
import { m } from "@/paraglide/messages";

function LevelPanel({ muted, connected, onMute }: { muted: boolean; connected: boolean; onMute: () => void }) {
  const level = useLevel();
  const peak = num(level.peakDb, -60);
  const inTarget = peak >= -12 && peak <= -6;
  return (
    <aside className="flex w-full shrink-0 flex-col gap-3 border-b p-4 lg:w-64 lg:border-r lg:border-b-0">
      <h2 className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">{m.basics_level()}</h2>
      <div className="flex min-h-48 flex-1 gap-3 max-lg:h-56">
        <div className="flex w-7 flex-col justify-between py-0.5 text-right font-mono text-[11px] text-muted-foreground">
          {METER_SCALE.map((db) => (
            <span key={db}>{formatNumber(db)}</span>
          ))}
        </div>
        <LevelMeter level={level} className="w-11 rounded-lg border bg-card-inset p-1" />
        <div className="flex flex-col gap-1 pt-6">
          <span className="text-[11px] text-muted-foreground">{m.basics_target()}</span>
          <span className="text-xs font-medium text-success">−12 … −6 dB</span>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-2">
        <div className="rounded-xl border bg-card p-2.5">
          <div className="text-[11px] text-muted-foreground">{m.basics_peak()}</div>
          <div
            className={cn(
              "font-mono text-lg tabular-nums",
              inTarget && "text-success",
              level.clipping && "text-danger",
            )}
          >
            {formatNumber(peak, 1)}
          </div>
        </div>
        <div className="rounded-xl border bg-card p-2.5">
          <div className="text-[11px] text-muted-foreground">{m.basics_current()}</div>
          <div className="font-mono text-lg tabular-nums">{formatNumber(num(level.currentDb, -60), 1)}</div>
        </div>
      </div>
      <Button
        size="lg"
        variant={muted ? "destructive" : "outline"}
        aria-pressed={muted}
        disabled={!connected}
        onClick={onMute}
        className={cn("w-full whitespace-normal", muted && "border border-danger/50")}
      >
        {muted ? <MicOff aria-hidden /> : <Mic aria-hidden />}
        {muted ? m.basics_muted() : m.basics_mute()}
      </Button>
    </aside>
  );
}

const MAX_PERCENT = dbToPercent(MAX_BOOST_DB);

/** The Flatpak cannot restart pipewire-pulse itself (`pipewire::RESTART_COMMAND`). */
const RESTART_PULSE = "systemctl --user restart pipewire-pulse";

function BoostControl({
  boostDb,
  remembered,
  protectedFromApps,
  sandboxed,
}: {
  boostDb: number | null;
  remembered: number;
  protectedFromApps: boolean;
  sandboxed: boolean;
}) {
  const setBoost = useSetBoost();
  const setProtected = useSetProtected();
  // Switching restarts pipewire-pulse (the user does that in the Flatpak): ask first (null = no question open).
  const [confirm, setConfirm] = useState<boolean | null>(null);
  const available = boostDb !== null;
  const db = num(boostDb);
  const draft = useDraft(dbToPercent(db), (p) => setBoost.mutate(percentToDb(p)), 120);
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center gap-2.5">
        <Switch
          checked={db > 0}
          disabled={!available}
          aria-label={m.basics_boost_on()}
          onCheckedChange={(on) => setBoost.mutate(on ? remembered || 10 : 0)}
        />
        <label htmlFor="boost" className="font-medium">
          {m.basics_boost()}
        </label>
        <span className="text-xs text-muted-foreground max-sm:hidden">{m.basics_boost_hint()}</span>
        <span className="ml-auto text-xs text-muted-foreground">
          {formatDb(percentToDb(draft.value), { signed: true })}
        </span>
        <Value className="ml-0">{draft.value} %</Value>
      </div>
      <Slider
        id="boost"
        aria-label={m.basics_boost()}
        min={100}
        max={MAX_PERCENT}
        step={5}
        disabled={!available}
        value={draft.value}
        onValueChange={draft.change}
        onValueCommit={draft.commit}
      />
      <div className="flex items-center justify-between gap-3 text-[11px] text-muted-foreground">
        <span>100 %</span>
        {available ? (
          <div className="flex items-center gap-2" title={m.basics_protect_hint()}>
            <Switch
              id="protect"
              className="scale-90"
              checked={protectedFromApps}
              disabled={setProtected.isPending}
              onCheckedChange={(on) => setConfirm(on)}
            />
            <label htmlFor="protect" className="text-xs font-medium text-foreground">
              {m.basics_protect()}
            </label>
            <span className="text-xs max-md:hidden">{m.basics_protect_short()}</span>
          </div>
        ) : (
          <span>{m.error_noAudioSource()}</span>
        )}
        <span>{MAX_PERCENT} %</span>
      </div>
      <Dialog open={confirm !== null} onOpenChange={(open) => !open && setConfirm(null)}>
        <DialogContent>
          <DialogTitle>{m.basics_protect()}</DialogTitle>
          {confirm && <DialogDescription>{m.protect_confirm_on()}</DialogDescription>}
          {sandboxed ? (
            <>
              <p className="leading-relaxed text-muted-foreground">{m.protect_confirm_restart_flatpak()}</p>
              <pre className="overflow-x-auto rounded-lg bg-card-inset p-3 font-mono text-xs">{RESTART_PULSE}</pre>
            </>
          ) : (
            <p className="leading-relaxed text-muted-foreground">{m.protect_confirm_restart()}</p>
          )}
          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={() => setConfirm(null)}>
              {m.common_cancel()}
            </Button>
            <Button
              onClick={() => {
                if (confirm !== null) setProtected.mutate(confirm);
                setConfirm(null);
              }}
            >
              {confirm ? m.protect_switch_on() : m.protect_switch_off()}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function SliderRow({
  id,
  label,
  hint,
  value,
  max,
  disabled,
  suffix,
  onSend,
}: {
  id: string;
  label: string;
  hint?: string;
  value: number;
  max: number;
  disabled: boolean;
  suffix?: string;
  onSend: (v: number) => void;
}) {
  const draft = useDraft(value, onSend);
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-baseline gap-2">
        <label htmlFor={id} className="font-medium">
          {label}
        </label>
        {hint && <span className="text-xs text-muted-foreground">{hint}</span>}
        <Value>
          {draft.value}
          {suffix}
        </Value>
      </div>
      <Slider
        id={id}
        aria-label={label}
        min={0}
        max={max}
        step={1}
        disabled={disabled}
        value={draft.value}
        onValueChange={draft.change}
        onValueCommit={draft.commit}
      />
    </div>
  );
}

export function BasicsPage() {
  const { data } = useSnapshot();
  const apply = useApply();
  const live = data?.live;
  const connected = !!live;
  const gain = live?.gain ?? 0;
  return (
    // The page scrolls as a whole, so the level column ends where the cards end.
    <div className="h-full overflow-auto">
      <div className="flex flex-col lg:flex-row">
        <LevelPanel
          muted={!!live?.mute}
          connected={connected}
          onMute={() => apply.mutate({ kind: "mute", value: !live?.mute })}
        />
        <main className="flex min-w-0 flex-1 flex-col gap-3 p-4 sm:px-6">
          <Card>
            <div className="flex items-center gap-3">
              <CardTitle>{m.basics_volume()}</CardTitle>
              <CalibrateDialog disabled={!connected} />
            </div>
            <div className="flex flex-col gap-1.5">
              <SliderRow
                id="gain"
                label={m.basics_gain()}
                hint={m.basics_gain_hint()}
                value={gain}
                max={100}
                disabled={!connected}
                onSend={(v) => apply.mutate({ kind: "gain", value: v })}
              />
              <div className="grid grid-cols-3 text-center text-[11px] text-muted-foreground">
                <span>0–10 cm</span>
                <span>11–30 cm</span>
                <span>31–50 cm</span>
              </div>
              {connected && gain >= 100 && (
                <p className="flex items-center gap-1.5 text-xs text-warning">
                  <CircleAlert className="size-3.5" aria-hidden />
                  {m.basics_gain_max()}
                </p>
              )}
            </div>
            <BoostControl
              boostDb={data?.boostDb ?? null}
              remembered={num(data?.settings.boostDb, 10)}
              protectedFromApps={!!data?.protected}
              sandboxed={!!data?.sandboxed}
            />
          </Card>

          <div className="grid gap-3 md:grid-cols-2">
            <Card>
              <CardTitle>{m.basics_headphones()}</CardTitle>
              <div className="flex items-center gap-2.5">
                <Switch
                  id="monitor"
                  checked={!!live?.monitor}
                  disabled={!connected}
                  onCheckedChange={(on) => apply.mutate({ kind: "monitor", value: on })}
                />
                <label htmlFor="monitor" className="font-medium">
                  {m.basics_monitor()}
                </label>
                <span className="text-xs text-muted-foreground">{m.basics_monitor_hint()}</span>
              </div>
              <SliderRow
                id="headphone"
                label={m.basics_headphone_volume()}
                value={live?.headphone ?? 0}
                max={100}
                disabled={!connected}
                onSend={(v) => apply.mutate({ kind: "headphone", value: v })}
              />
            </Card>

            <Card>
              <CardTitle>{m.basics_noise()}</CardTitle>
              <div className="flex items-center gap-2.5">
                <Switch
                  id="nc"
                  checked={!!live?.noiseCancellation}
                  disabled={!connected}
                  onCheckedChange={(on) => apply.mutate({ kind: "noiseCancellation", value: on })}
                />
                <label htmlFor="nc" className="font-medium">
                  {m.basics_nc()}
                </label>
              </div>
              <SliderRow
                id="nc-strength"
                label={m.basics_nc_strength()}
                value={live?.ncStrength ?? 0}
                max={40}
                suffix=" / 40"
                disabled={!connected || !live?.noiseCancellation}
                onSend={(v) => apply.mutate({ kind: "ncStrength", value: v })}
              />
            </Card>
          </div>

          <RecordingCard connected={connected} />
        </main>
      </div>
    </div>
  );
}
