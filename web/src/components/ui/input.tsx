import type { InputHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export function Input({
  className,
  ...props
}: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={cn(
        "w-full rounded-sm border border-steel/30 bg-white/80 px-3 py-2 text-sm text-ink outline-none transition placeholder:text-steel/60 focus:border-ember",
        className,
      )}
      {...props}
    />
  );
}
