import { PlugZap, ShieldAlert } from "lucide-react";
import { m } from "@/paraglide/messages";

const UDEV = `curl -LO https://raw.githubusercontent.com/firsttris/pd200x-control/main/packaging/99-pd200x.rules
sudo cp 99-pd200x.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger`;

/** Why the controls are off: no microphone, or no permission to open it. */
export function ConnectionBanner({ problem }: { problem: string }) {
  if (problem === "permissionDenied") {
    return (
      <div role="alert" className="flex gap-3 rounded-2xl border border-warning/40 bg-warning/10 p-4">
        <ShieldAlert className="mt-0.5 size-5 shrink-0 text-warning" aria-hidden />
        <div className="flex min-w-0 flex-col gap-2">
          <p className="font-semibold">{m.banner_permission_title()}</p>
          <p className="text-muted-foreground">{m.banner_permission_text()}</p>
          <pre className="overflow-x-auto rounded-lg bg-card-inset p-3 font-mono text-xs">{UDEV}</pre>
        </div>
      </div>
    );
  }
  return (
    <div role="alert" className="flex items-center gap-3 rounded-2xl border bg-card p-4">
      <PlugZap className="size-5 shrink-0 text-muted-foreground" aria-hidden />
      <div>
        <p className="font-semibold">{m.banner_notFound_title()}</p>
        <p className="text-muted-foreground">{m.banner_notFound_text()}</p>
      </div>
    </div>
  );
}
