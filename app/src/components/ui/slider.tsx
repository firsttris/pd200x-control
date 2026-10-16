import { Slider as SliderPrimitive } from "radix-ui";
import type * as React from "react";
import { cn } from "@/lib/utils";

/** One thumb; `value`/`onValueChange` take a plain number. */
export function Slider({
  className,
  value,
  onValueChange,
  onValueCommit,
  trackClassName,
  id,
  "aria-label": ariaLabel,
  ...props
}: Omit<
  React.ComponentProps<typeof SliderPrimitive.Root>,
  "value" | "onValueChange" | "onValueCommit" | "defaultValue"
> & {
  value: number;
  onValueChange?: (value: number) => void;
  onValueCommit?: (value: number) => void;
  trackClassName?: string;
}) {
  return (
    <SliderPrimitive.Root
      className={cn(
        "relative flex h-6 w-full touch-none select-none items-center data-[disabled]:opacity-50",
        className,
      )}
      value={[value]}
      onValueChange={(v) => onValueChange?.(v[0])}
      onValueCommit={(v) => onValueCommit?.(v[0])}
      {...props}
    >
      <SliderPrimitive.Track
        className={cn("relative h-1.5 grow overflow-hidden rounded-full bg-input", trackClassName)}
      >
        <SliderPrimitive.Range className="absolute h-full bg-accent" />
      </SliderPrimitive.Track>
      {/* The thumb is the focusable slider; label and id belong to it. */}
      <SliderPrimitive.Thumb
        id={id}
        aria-label={ariaLabel}
        className="block size-[18px] rounded-full border-[3px] border-accent bg-white shadow transition-[box-shadow] hover:ring-4 hover:ring-accent/20 focus-visible:ring-4 focus-visible:ring-accent/30 focus-visible:outline-none"
      />
    </SliderPrimitive.Root>
  );
}
