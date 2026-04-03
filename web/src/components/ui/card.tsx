import type { PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

export function Card({
  className,
  children,
}: PropsWithChildren<{ className?: string }>) {
  return (
    <section
      className={cn(
        "rounded-sm border border-steel/20 bg-paper/90 p-4 shadow-panel",
        className,
      )}
    >
      {children}
    </section>
  );
}
