import type { PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

export function Badge({
  className,
  children,
}: PropsWithChildren<{ className?: string }>) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full border border-steel/20 bg-white/70 px-2 py-1 font-mono text-[11px] uppercase tracking-[0.24em] text-steel",
        className,
      )}
    >
      {children}
    </span>
  );
}
