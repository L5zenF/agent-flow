import type { PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

export function Card({
  className,
  children,
}: PropsWithChildren<{ className?: string }>) {
  return (
    <section
      className={cn(
        "rounded-lg border border-zinc-200 bg-white p-4 shadow-sm",
        className,
      )}
    >
      {children}
    </section>
  );
}
