import type { TextareaHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

export function Textarea({
  className,
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      className={cn(
        "min-h-24 w-full rounded-sm border border-steel/30 bg-white/80 px-3 py-2 text-sm text-ink outline-none transition placeholder:text-steel/60 focus:border-ember",
        className,
      )}
      {...props}
    />
  );
}
