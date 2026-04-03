import * as React from "react";
import { cn } from "@/lib/utils";

const Textarea = React.forwardRef<HTMLTextAreaElement, React.ComponentProps<"textarea">>(
  ({ className, ...props }, ref) => {
    return (
      <textarea
        className={cn(
          "flex min-h-[80px] w-full rounded-md border border-oklch(0.923 0.003 48.717) bg-oklch(1 0 0) px-3 py-2 text-base placeholder:text-oklch(0.553 0.013 58.071) focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-oklch(0.709 0.01 56.259) focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
          className,
        )}
        ref={ref}
        {...props}
      />
    );
  },
);
Textarea.displayName = "Textarea";

export { Textarea };
