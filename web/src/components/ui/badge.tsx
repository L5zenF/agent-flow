import type { PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

export function Badge({
  className,
  children,
}: PropsWithChildren<{ className?: string }>) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-md border border-zinc-200 bg-zinc-50 px-2 py-1 font-mono text-[11px] uppercase tracking-[0.18em] text-zinc-600",
        className,
      )}
    >
      {children}
    </span>
  );
}
