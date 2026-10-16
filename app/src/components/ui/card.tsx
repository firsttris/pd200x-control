import type * as React from "react";
import { cn } from "@/lib/utils";

export function Card({ className, ...props }: React.ComponentProps<"section">) {
  return <section className={cn("flex flex-col gap-4 rounded-2xl border bg-card p-5", className)} {...props} />;
}

export function CardTitle({ className, ...props }: React.ComponentProps<"h2">) {
  return <h2 className={cn("text-base font-semibold", className)} {...props} />;
}

/** A value next to a control, e.g. `100` or `+14,6 dB`. */
export function Value({ className, ...props }: React.ComponentProps<"output">) {
  return (
    <output
      className={cn(
        "ml-auto shrink-0 rounded-md border bg-card-inset px-2 py-0.5 font-mono text-[13px] tabular-nums",
        className,
      )}
      {...props}
    />
  );
}
