import { Pencil, Plus, RotateCcw, Trash2 } from "lucide-react";
import { useState } from "react";
import { bandLabel, EqGraph, MiniCurve } from "@/components/eq-graph";
import { ProfileDialog } from "@/components/profile-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardTitle, Value } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { type Dynamics, type EqBand, type FilterType, PRESETS, type Profile } from "@/lib/bindings";
import { HPF_INDEX, LPF_INDEX } from "@/lib/eq";
import { formatDb, formatHz, formatNumber } from "@/lib/format";
import { useApply, useApplyEq, useDeleteProfile, useDraft, useSnapshot } from "@/lib/queries";
import { toneOf } from "@/lib/snapshot";
import { clamp, cn, num } from "@/lib/utils";
import { m } from "@/paraglide/messages";

export function presetName(id: string): string {
  const names: Record<string, () => string> = {
    flat: m.preset_flat,
    low_cut: m.preset_low_cut,
    mid_boost: m.preset_mid_boost,
    low_cut_mid_boost: m.preset_low_cut_mid_boost,
    bright: m.preset_bright,
    natural: m.preset_natural,
    classic: m.preset_classic,
    deep: m.preset_deep,
  };
  return names[id]?.() ?? id;
}

const PRESET_LIST = PRESETS as unknown as { id: string; eq: EqBand[] }[];

function Tile({
  name,
  bands,
  active,
  own,
  disabled,
  onPick,
}: {
  name: string;
  bands: EqBand[];
  active: boolean;
  own?: boolean;
  disabled: boolean;
  onPick: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      disabled={disabled}
      onClick={onPick}
      className={cn(
        "flex w-full flex-col gap-1.5 rounded-xl border p-2 text-left disabled:opacity-60",
        active ? "border-accent bg-accent-soft text-accent" : "bg-card text-muted-foreground hover:text-foreground",
        own && !active && "text-warning",
      )}
    >
      <MiniCurve bands={bands} />
      <span className="truncate text-xs font-medium text-foreground">{name}</span>
    </button>
  );
}

function ProfileTile({ profile, active, disabled }: { profile: Profile; active: boolean; disabled: boolean }) {
  const applyEq = useApplyEq();
  const del = useDeleteProfile();
  return (
    <div className="group relative">
      <Tile
        name={profile.name}
        bands={profile.eq}
        active={active}
        own
        disabled={disabled}
        onPick={() => applyEq.mutate(profile.id)}
      />
      <div className="absolute top-1 right-1 flex gap-0.5 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
        <ProfileDialog
          rename={profile}
          trigger={
            <button
              type="button"
              aria-label={m.sound_profile_rename({ name: profile.name })}
              className="rounded-md bg-card/90 p-1 hover:bg-muted"
            >
              <Pencil className="size-3.5" aria-hidden />
            </button>
          }
        />
        <button
          type="button"
          aria-label={m.sound_profile_delete({ name: profile.name })}
          className="rounded-md bg-card/90 p-1 text-danger hover:bg-muted"
          onClick={() => del.mutate(profile.id)}
        >
          <Trash2 className="size-3.5" aria-hidden />
        </button>
      </div>
    </div>
  );
}

const TYPES: { id: FilterType; label: () => string }[] = [
  { id: "peaking", label: m.eq_type_peaking },
  { id: "lowShelf", label: m.eq_type_lowShelf },
  { id: "highShelf", label: m.eq_type_highShelf },
];

function NumberField({
  id,
  label,
  value,
  step,
  min,
  max,
  disabled,
  onCommit,
}: {
  id: string;
  label: string;
  value: number;
  step: number;
  min: number;
  max: number;
  disabled?: boolean;
  onCommit: (v: number) => void;
}) {
  const [text, setText] = useState<string | null>(null);
  const commit = () => {
    if (text === null) return;
    const v = Number(text.replace(",", "."));
    if (Number.isFinite(v)) onCommit(clamp(v, min, max));
    setText(null);
  };
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="text-xs text-muted-foreground">
        {label}
      </label>
      <Input
        id={id}
        inputMode="decimal"
        disabled={disabled}
        value={text ?? formatNumber(value, step < 1 ? (step < 0.1 ? 2 : 1) : 0)}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => e.key === "Enter" && commit()}
        className="font-mono"
      />
    </div>
  );
}

function BandEditor({
  index,
  band,
  disabled,
  onChange,
}: {
  index: number;
  band: EqBand;
  disabled: boolean;
  onChange: (band: EqBand) => void;
}) {
  const pass = index === HPF_INDEX || index === LPF_INDEX;
  const name = index === HPF_INDEX ? m.eq_hpf() : index === LPF_INDEX ? m.eq_lpf() : m.eq_band({ band: index + 1 });
  return (
    <div className="grid items-end gap-3 rounded-xl border bg-card-inset p-3 sm:grid-cols-[auto_1fr_1fr_1fr_1fr]">
      <div className="flex items-center gap-2.5 pb-2">
        <Switch
          id="band-on"
          checked={band.enabled}
          disabled={disabled}
          onCheckedChange={(on) => onChange({ ...band, enabled: on })}
        />
        <label htmlFor="band-on" className="font-semibold whitespace-nowrap">
          {name}
        </label>
      </div>
      <div className="flex flex-col gap-1">
        <label htmlFor="band-type" className="text-xs text-muted-foreground">
          {m.eq_type()}
        </label>
        {pass ? (
          <span id="band-type" className="flex h-9 items-center text-muted-foreground">
            {index === HPF_INDEX ? m.eq_type_highPass() : m.eq_type_lowPass()}
          </span>
        ) : (
          <select
            id="band-type"
            disabled={disabled}
            value={band.filter}
            onChange={(e) => onChange({ ...band, filter: e.target.value as FilterType })}
            className="h-9 rounded-lg border border-input bg-card px-2"
          >
            {TYPES.map((t) => (
              <option key={t.id} value={t.id}>
                {t.label()}
              </option>
            ))}
          </select>
        )}
      </div>
      <NumberField
        id="band-freq"
        label={m.eq_frequency()}
        value={band.frequency}
        step={1}
        min={20}
        max={22000}
        disabled={disabled}
        onCommit={(v) => onChange({ ...band, frequency: Math.round(v) })}
      />
      <NumberField
        id="band-gain"
        label={m.eq_gain()}
        value={num(band.gainDb)}
        step={0.5}
        min={-12}
        max={12}
        disabled={disabled || pass}
        onCommit={(v) => onChange({ ...band, gainDb: v })}
      />
      <NumberField
        id="band-q"
        label={m.eq_q()}
        value={num(band.q, 1)}
        step={0.01}
        min={0.1}
        max={10}
        disabled={disabled || pass}
        onCommit={(v) => onChange({ ...band, q: v })}
      />
    </div>
  );
}

function DynamicsCard({
  id,
  title,
  hint,
  scale,
  value,
  min,
  disabled,
  onSend,
}: {
  id: string;
  title: string;
  hint: string;
  scale: string;
  value: Dynamics;
  min: number;
  disabled: boolean;
  onSend: (d: Dynamics) => void;
}) {
  const draft = useDraft(num(value.thresholdDb), (v) => onSend({ ...value, thresholdDb: v }), 120);
  return (
    <Card className="gap-2">
      <div className="flex items-center gap-2.5">
        <Switch
          id={`${id}-on`}
          checked={value.enabled}
          disabled={disabled}
          aria-label={title}
          onCheckedChange={(on) => onSend({ ...value, enabled: on })}
        />
        <CardTitle>
          <label htmlFor={id}>{title}</label>
        </CardTitle>
        <span className="text-xs text-muted-foreground max-sm:hidden">{hint}</span>
        <Value>{formatDb(draft.value, { digits: 0 })}</Value>
      </div>
      <Slider
        id={id}
        aria-label={title}
        min={min}
        max={0}
        step={1}
        disabled={disabled || !value.enabled}
        value={draft.value}
        onValueChange={draft.change}
        onValueCommit={draft.commit}
      />
      <div className="flex justify-between text-[11px] text-muted-foreground">
        <span>{formatDb(min, { digits: 0 })}</span>
        <span>{scale}</span>
        <span>0 dB</span>
      </div>
    </Card>
  );
}

export function SoundPage() {
  const { data } = useSnapshot();
  const apply = useApply();
  const applyEq = useApplyEq();
  const [selected, setSelected] = useState(2);
  const connected = !!data?.live;
  const tone = toneOf(data);
  const profiles = data?.settings.profiles ?? [];
  const source = tone.source;
  const sourceName = source ? (profiles.find((p) => p.id === source)?.name ?? presetName(source)) : m.sound_custom();
  const changeBand = (index: number, band: EqBand) => apply.mutate({ kind: "eqBand", value: { index, band } });

  return (
    <main className="flex h-full flex-col gap-3 overflow-auto p-4 sm:px-6">
      <section className="flex flex-col gap-2.5" aria-labelledby="profiles-title">
        <div className="flex flex-wrap items-center gap-3">
          <h2 id="profiles-title" className="text-base font-semibold">
            {m.sound_profiles()}
          </h2>
          <span className="text-xs text-muted-foreground">{m.sound_profiles_hint()}</span>
          <ProfileDialog
            trigger={
              <Button variant="outline" size="sm" className="ml-auto" disabled={!connected}>
                <Plus aria-hidden />
                {m.sound_profile_new()}
              </Button>
            }
          />
        </div>
        <div className="grid grid-cols-[repeat(auto-fill,minmax(110px,1fr))] gap-2.5">
          {PRESET_LIST.map((p) => (
            <Tile
              key={p.id}
              name={presetName(p.id)}
              bands={p.eq}
              active={source === p.id}
              disabled={!connected}
              onPick={() => applyEq.mutate(p.id)}
            />
          ))}
          {profiles.map((p) => (
            <ProfileTile key={p.id} profile={p} active={source === p.id} disabled={!connected} />
          ))}
        </div>
      </section>

      <Card>
        <div className="flex flex-wrap items-center gap-3">
          <CardTitle>{m.sound_eq()}</CardTitle>
          <span className="text-xs text-muted-foreground max-md:hidden">{m.sound_eq_hint()}</span>
          <span className={cn("ml-auto text-xs", source ? "text-muted-foreground" : "text-warning")}>{sourceName}</span>
          <Button
            variant="ghost"
            size="sm"
            disabled={!connected}
            onClick={() => applyEq.mutate(source ?? "flat")}
            title={m.sound_eq_reset()}
          >
            <RotateCcw aria-hidden />
            {m.sound_eq_reset()}
          </Button>
        </div>
        <EqGraph
          bands={tone.eq}
          selected={selected}
          disabled={!connected}
          onSelect={setSelected}
          onChange={changeBand}
        />
        <div className="flex flex-wrap gap-1.5" role="tablist" aria-label={m.sound_bands()}>
          {tone.eq.map((band, i) => (
            <button
              key={bandLabel(i)}
              type="button"
              role="tab"
              aria-selected={i === selected}
              onClick={() => setSelected(i)}
              className={cn(
                "rounded-lg border px-2.5 py-1 text-xs",
                i === selected ? "border-accent bg-accent-soft" : "bg-card",
                !band.enabled && "text-muted-foreground",
              )}
            >
              <span className="font-semibold">{bandLabel(i)}</span>
              <span className="text-muted-foreground"> · </span>
              <span className="font-mono">{formatHz(band.frequency)}</span>
            </button>
          ))}
        </div>
        <BandEditor
          index={selected}
          band={tone.eq[selected]}
          disabled={!connected}
          onChange={(band) => changeBand(selected, band)}
        />
      </Card>

      <div className="grid gap-3 md:grid-cols-2">
        <DynamicsCard
          id="compressor"
          title={m.sound_compressor()}
          hint={m.sound_compressor_hint()}
          scale={m.sound_threshold()}
          value={tone.compressor}
          min={-40}
          disabled={!connected}
          onSend={(d) => apply.mutate({ kind: "compressor", value: d })}
        />
        <DynamicsCard
          id="limiter"
          title={m.sound_limiter()}
          hint={m.sound_limiter_hint()}
          scale={m.sound_ceiling()}
          value={tone.limiter}
          min={-80}
          disabled={!connected}
          onSend={(d) => apply.mutate({ kind: "limiter", value: d })}
        />
      </div>
    </main>
  );
}
