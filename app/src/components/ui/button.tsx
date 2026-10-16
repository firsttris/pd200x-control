import { cva, type VariantProps } from "class-variance-authority";
import { Slot } from "radix-ui";
import type * as React from "react";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-lg text-[13px] font-medium transition-[filter,background-color,color] disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "bg-accent text-accent-foreground font-semibold hover:brightness-110",
        soft: "border border-accent/50 bg-accent-soft text-foreground hover:brightness-110",
        outline: "border border-border bg-card text-foreground hover:bg-muted",
        ghost: "bg-transparent text-muted-foreground hover:bg-muted hover:text-foreground",
        destructive: "bg-danger/15 text-danger hover:bg-danger/25",
      },
      size: {
        default: "h-9 px-3.5",
        sm: "h-8 rounded-md px-2.5",
        lg: "h-12 px-4 text-[15px] rounded-xl",
        icon: "size-9 rounded-lg",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  },
);

export interface ButtonProps extends React.ComponentProps<"button">, VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export function Button({ className, variant, size, asChild = false, type, ...props }: ButtonProps) {
  const Comp = asChild ? Slot.Root : "button";
  return (
    <Comp
      data-slot="button"
      type={asChild ? undefined : (type ?? "button")}
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}
