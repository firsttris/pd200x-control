import { Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { api, errorMessage } from "@/lib/api";
import type { Proposal } from "@/lib/bindings";
import { dbToPercent, formatDb } from "@/lib/format";
import { useApplyCalibration, useLevel } from "@/lib/queries";
import { num } from "@/lib/utils";
import { m } from "@/paraglide/messages";
import { LevelMeter } from "./level-meter";

const SECONDS = 5;

type Step =
  | { kind: "intro" }
  | { kind: "listening"; left: number }
  | { kind: "result"; proposal: Proposal }
  | { kind: "error"; message: string };

function Listening({ left }: { left: number }) {
  const level = useLevel();
  return (
    <div className="flex flex-col gap-3">
      <p className="text-[15px] font-medium">{m.calibrate_listening({ seconds: left })}</p>
      <LevelMeter level={level} orientation="horizontal" count={30} className="h-4" />
    </div>
  );
}

/** "Pegel einmessen": listen for a few seconds, propose gain and boost, apply on request. */
export function CalibrateDialog({ disabled }: { disabled?: boolean }) {
  const [open, setOpen] = useState(false);
  const [step, setStep] = useState<Step>({ kind: "intro" });
  const apply = useApplyCalibration();

  useEffect(() => {
    if (step.kind !== "listening" || step.left <= 0) return;
    const id = setTimeout(() => setStep({ kind: "listening", left: step.left - 1 }), 1000);
    return () => clearTimeout(id);
  }, [step]);

  const start = async () => {
    setStep({ kind: "listening", left: SECONDS });
    try {
      const proposal = await api.calibrate(SECONDS);
      setStep({ kind: "result", proposal });
    } catch (e) {
      setStep({ kind: "error", message: errorMessage(e) });
    }
  };

  const change = (o: boolean) => {
    setOpen(o);
    if (!o) setStep({ kind: "intro" });
  };

  return (
    <>
      <Button variant="soft" className="ml-auto" disabled={disabled} onClick={() => setOpen(true)}>
        <Sparkles aria-hidden />
        {m.calibrate_button()}
      </Button>
      <Dialog open={open} onOpenChange={change}>
        <DialogContent>
          <DialogTitle>{m.calibrate_title()}</DialogTitle>
          {step.kind === "intro" && (
            <>
              <DialogDescription>{m.calibrate_intro({ seconds: SECONDS })}</DialogDescription>
              <div className="flex justify-end gap-2">
                <Button variant="outline" onClick={() => change(false)}>
                  {m.common_cancel()}
                </Button>
                <Button onClick={start}>{m.calibrate_start()}</Button>
              </div>
            </>
          )}
          {step.kind === "listening" && <Listening left={Math.max(step.left, 1)} />}
          {step.kind === "result" && (
            <>
              <DialogDescription>
                {m.calibrate_measured({ peak: formatDb(num(step.proposal.measuredPeakDb)) })}
              </DialogDescription>
              <dl className="grid grid-cols-3 gap-2 text-center">
                <div className="rounded-xl border bg-card-inset p-3">
                  <dt className="text-xs text-muted-foreground">{m.basics_gain()}</dt>
                  <dd className="font-mono text-lg">{step.proposal.gain}</dd>
                </div>
                <div className="rounded-xl border bg-card-inset p-3">
                  <dt className="text-xs text-muted-foreground">{m.basics_boost()}</dt>
                  <dd className="font-mono text-lg">{dbToPercent(num(step.proposal.boostDb))} %</dd>
                  <dd className="text-xs text-muted-foreground">
                    {formatDb(num(step.proposal.boostDb), { signed: true })}
                  </dd>
                </div>
                <div className="rounded-xl border bg-card-inset p-3">
                  <dt className="text-xs text-muted-foreground">{m.calibrate_expected()}</dt>
                  <dd className="font-mono text-lg text-success">{formatDb(num(step.proposal.expectedPeakDb))}</dd>
                </div>
              </dl>
              <div className="flex justify-end gap-2">
                <Button variant="outline" onClick={start}>
                  {m.calibrate_again()}
                </Button>
                <Button
                  onClick={() => {
                    apply.mutate({ gain: step.proposal.gain, boostDb: num(step.proposal.boostDb) });
                    change(false);
                  }}
                >
                  {m.calibrate_apply()}
                </Button>
              </div>
            </>
          )}
          {step.kind === "error" && (
            <>
              <DialogDescription role="alert">{step.message}</DialogDescription>
              <div className="flex justify-end gap-2">
                <Button variant="outline" onClick={() => change(false)}>
                  {m.common_cancel()}
                </Button>
                <Button onClick={start}>{m.calibrate_again()}</Button>
              </div>
            </>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
