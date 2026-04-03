import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-full border border-oklch(0.923 0.003 48.717) px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-oklch(0.709 0.01 56.259) focus:ring-offset-2",
  {
    variants: {
      variant: {
        default:
          "border-transparent bg-oklch(0.216 0.006 56.043) text-oklch(0.985 0.001 106.423) hover:bg-oklch(0.216 0.006 56.043)/80",
        secondary:
          "border-transparent bg-oklch(0.97 0.001 106.424) text-oklch(0.216 0.006 56.043) hover:bg-oklch(0.97 0.001 106.424)/80",
        destructive:
          "border-transparent bg-oklch(0.577 0.245 27.325) text-white hover:bg-oklch(0.577 0.245 27.325)/80",
        outline: "text-oklch(0.147 0.004 49.25)",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };
