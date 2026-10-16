import type * as React from "react";
import { cn } from "@/lib/utils";

export function Input({ className, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      className={cn(
        "h-9 w-full rounded-lg border border-input bg-card-inset px-3 text-sm outline-none focus-visible:border-ring",
        className,
      )}
      {...props}
    />
  );
}
