import { MicIllustration, RGB_HEX } from "@/components/mic-illustration";
import { Card, CardTitle, Value } from "@/components/ui/card";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import type { RgbColor } from "@/lib/bindings";
import { useApply, useDraft, useSnapshot } from "@/lib/queries";
import { lightingOf } from "@/lib/snapshot";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";

const COLORS: { id: RgbColor; label: () => string }[] = [
  { id: "rainbow", label: m.color_rainbow },
  { id: "white", label: m.color_white },
  { id: "red", label: m.color_red },
  { id: "orange", label: m.color_orange },
  { id: "yellow", label: m.color_yellow },
  { id: "green", label: m.color_green },
  { id: "cyan", label: m.color_cyan },
  { id: "blue", label: m.color_blue },
  { id: "purple", label: m.color_purple },
];

const RAINBOW = "conic-gradient(#ef4444,#f59e0b,#eab308,#22c55e,#06b6d4,#3b82f6,#a855f7,#ef4444)";

export function LightPage() {
  const { data } = useSnapshot();
  const apply = useApply();
  const connected = !!data?.live;
  const light = lightingOf(data);
  const brightness = useDraft(light.brightness, (v) => apply.mutate({ kind: "rgbBrightness", value: v }));
  const lit = light.enabled && !(light.linkage && data?.live?.mute);
  const colorName = COLORS.find((c) => c.id === light.color)?.label() ?? "";
  return (
    <main className="flex h-full flex-col gap-4 overflow-auto p-4 sm:px-6 lg:flex-row">
      <div className="flex min-h-80 flex-1 flex-col items-center justify-center gap-2 overflow-hidden rounded-2xl border bg-[radial-gradient(ellipse_at_50%_45%,var(--card)_0%,var(--card-inset)_70%)] p-4">
        <MicIllustration color={light.color} brightness={brightness.value} lit={lit} />
        <p className="text-[13px] text-muted-foreground">
          {lit ? m.light_preview({ color: colorName, brightness: brightness.value }) : m.light_preview_off()}
        </p>
      </div>
      <div className="flex w-full shrink-0 flex-col gap-4 lg:w-[440px]">
        <Card>
          <div className="flex items-center gap-2.5">
            <CardTitle>
              <label htmlFor="rgb-on">{m.light_ring()}</label>
            </CardTitle>
            <Switch
              id="rgb-on"
              className="ml-auto"
              checked={light.enabled}
              disabled={!connected}
              onCheckedChange={(on) => apply.mutate({ kind: "rgbEnabled", value: on })}
            />
          </div>
          <fieldset className="flex flex-col gap-2.5" disabled={!connected}>
            <legend className="mb-2.5 font-medium">{m.light_color()}</legend>
            <div className="grid grid-cols-9 gap-2">
              {COLORS.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  aria-label={c.label()}
                  aria-pressed={light.color === c.id}
                  onClick={() => apply.mutate({ kind: "rgbColor", value: c.id })}
                  className={cn(
                    "aspect-square rounded-full border-[3px] disabled:opacity-50",
                    light.color === c.id ? "border-foreground" : "border-card",
                  )}
                  style={{ background: c.id === "rainbow" ? RAINBOW : RGB_HEX[c.id] }}
                />
              ))}
            </div>
          </fieldset>
          <div className="flex flex-col gap-1.5">
            <div className="flex items-baseline gap-2">
              <label htmlFor="brightness" className="font-medium">
                {m.light_brightness()}
              </label>
              <Value>{brightness.value} %</Value>
            </div>
            <Slider
              id="brightness"
              aria-label={m.light_brightness()}
              min={0}
              max={100}
              step={1}
              disabled={!connected || !light.enabled}
              value={brightness.value}
              onValueChange={brightness.change}
              onValueCommit={brightness.commit}
            />
          </div>
        </Card>
        <Card className="gap-2.5">
          <div className="flex items-center gap-2.5">
            <CardTitle>
              <label htmlFor="linkage">{m.light_linkage()}</label>
            </CardTitle>
            <Switch
              id="linkage"
              className="ml-auto"
              checked={light.linkage}
              disabled={!connected}
              onCheckedChange={(on) => apply.mutate({ kind: "linkage", value: on })}
            />
          </div>
          <p className="leading-relaxed text-muted-foreground">{m.light_linkage_hint()}</p>
        </Card>
        <p className="rounded-2xl border border-dashed p-4 text-[13px] leading-relaxed text-muted-foreground">
          {m.light_note()}
        </p>
      </div>
    </main>
  );
}
