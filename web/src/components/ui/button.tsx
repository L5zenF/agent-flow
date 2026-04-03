import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-oklch(0.709 0.01 56.259) focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default:
          "bg-oklch(0.216 0.006 56.043) text-oklch(0.985 0.001 106.423) hover:bg-oklch(0.216 0.006 56.043)/90",
        destructive:
          "bg-oklch(0.577 0.245 27.325) text-white hover:bg-oklch(0.577 0.245 27.325)/90",
        outline:
          "border border-oklch(0.923 0.003 48.717) bg-oklch(1 0 0) hover:bg-oklch(0.97 0.001 106.424)",
        secondary:
          "bg-oklch(0.97 0.001 106.424) text-oklch(0.216 0.006 56.043) hover:bg-oklch(0.97 0.001 106.424)/80",
        ghost: "hover:bg-oklch(0.97 0.001 106.424)",
        link: "underline-offset-4 hover:underline",
      },
      size: {
        default: "h-10 px-4 py-2",
        sm: "h-9 rounded-md px-3",
        lg: "h-11 rounded-md px-8",
        icon: "h-10 w-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  },
);
Button.displayName = "Button";

export { Button, buttonVariants };
