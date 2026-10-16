import { Link, Outlet } from "@tanstack/react-router";
import { Check, Menu, Mic } from "lucide-react";
import { DropdownMenu } from "radix-ui";
import { useState } from "react";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { LANGS, setLang, storedLang } from "@/lib/i18n";
import { useRestoreOnConnect, useSnapshot } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages";
import { ConnectionBanner } from "./connection-banner";

const TABS = [
  { to: "/", label: () => m.nav_basics() },
  { to: "/sound", label: () => m.nav_sound() },
  { to: "/light", label: () => m.nav_light() },
] as const;

const itemClass =
  "flex cursor-default items-center gap-2 rounded-md px-2.5 py-2 text-[13px] outline-none data-[highlighted]:bg-muted";

function AppMenu({ onAbout }: { onAbout: () => void }) {
  const { data } = useSnapshot();
  const restore = useRestoreOnConnect();
  const lang = storedLang();
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          type="button"
          aria-label={m.menu_label()}
          className="flex size-9 items-center justify-center rounded-lg border bg-card text-muted-foreground hover:text-foreground"
        >
          <Menu className="size-[18px]" aria-hidden />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={6}
          className="z-50 min-w-64 rounded-xl border bg-card p-1.5 shadow-xl"
        >
          <DropdownMenu.CheckboxItem
            className={itemClass}
            checked={data?.settings.restoreOnConnect ?? true}
            onCheckedChange={(on) => restore.mutate(on)}
          >
            <span className="flex size-4 items-center justify-center">
              <DropdownMenu.ItemIndicator>
                <Check className="size-4" aria-hidden />
              </DropdownMenu.ItemIndicator>
            </span>
            {m.menu_restore()}
          </DropdownMenu.CheckboxItem>
          <DropdownMenu.Separator className="my-1 h-px bg-border" />
          <DropdownMenu.Label className="px-2.5 py-1 text-xs text-muted-foreground">
            {m.menu_language()}
          </DropdownMenu.Label>
          <DropdownMenu.RadioGroup
            value={lang ?? "system"}
            onValueChange={(v) => setLang(v === "system" ? null : (v as "de" | "en"))}
          >
            {[{ id: "system", label: m.menu_language_system() }, ...LANGS].map((l) => (
              <DropdownMenu.RadioItem key={l.id} value={l.id} className={itemClass}>
                <span className="flex size-4 items-center justify-center">
                  <DropdownMenu.ItemIndicator>
                    <Check className="size-4" aria-hidden />
                  </DropdownMenu.ItemIndicator>
                </span>
                {l.label}
              </DropdownMenu.RadioItem>
            ))}
          </DropdownMenu.RadioGroup>
          <DropdownMenu.Separator className="my-1 h-px bg-border" />
          <DropdownMenu.Item className={itemClass} onSelect={onAbout}>
            <span className="size-4" />
            {m.menu_about()}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

function DeviceChip() {
  const { data } = useSnapshot();
  const connected = !!data?.live;
  return (
    <div className="flex items-center gap-2 rounded-full border bg-card px-3 py-1.5 text-[13px] text-muted-foreground">
      <span className={cn("size-2 rounded-full", connected ? "bg-success" : "bg-danger")} aria-hidden />
      <span className="font-medium text-foreground">PD200X</span>
      <span className="max-sm:hidden">{connected ? m.device_connected() : m.device_disconnected()}</span>
    </div>
  );
}

export function AppShell() {
  const { data } = useSnapshot();
  const [about, setAbout] = useState(false);
  return (
    <div className="flex h-full flex-col">
      <header className="flex flex-wrap items-center gap-x-4 gap-y-2 border-b px-5 py-2.5">
        <div className="flex items-center gap-2.5">
          <Mic className="size-[22px] text-accent" aria-hidden />
          <span className="text-[15px] font-semibold">pd200x-control</span>
        </div>
        <DeviceChip />
        <nav
          aria-label={m.nav_label()}
          className="ml-auto flex gap-1 rounded-[10px] border bg-card p-1 max-sm:order-last max-sm:ml-0 max-sm:w-full"
        >
          {TABS.map((t) => (
            <Link
              key={t.to}
              to={t.to}
              className="rounded-[7px] px-4 py-1.5 font-medium text-muted-foreground hover:text-foreground max-sm:flex-1 max-sm:text-center"
              activeProps={{ className: "bg-muted text-foreground", "aria-current": "page" }}
              activeOptions={{ exact: true }}
            >
              {t.label()}
            </Link>
          ))}
        </nav>
        <div className="max-sm:ml-auto">
          <AppMenu onAbout={() => setAbout(true)} />
        </div>
      </header>
      {data?.problem && (
        <div className="px-5 pt-4 sm:px-7">
          <ConnectionBanner problem={data.problem} />
        </div>
      )}
      <div className="min-h-0 flex-1">
        <Outlet />
      </div>
      <Dialog open={about} onOpenChange={setAbout}>
        <DialogContent>
          <DialogTitle>pd200x-control</DialogTitle>
          <DialogDescription>{m.about_text()}</DialogDescription>
          <p className="text-[13px] text-muted-foreground">
            {m.about_license()}
            <br />
            https://github.com/firsttris/pd200x-control
          </p>
          <p className="text-xs text-muted-foreground">{m.about_trademark()}</p>
        </DialogContent>
      </Dialog>
    </div>
  );
}
