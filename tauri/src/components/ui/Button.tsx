import clsx from "clsx";
import type { ButtonHTMLAttributes } from "react";

type Variant = "default" | "ghost" | "primary";

/**
 * The one button. `default` is a bordered control, `ghost` is borderless for
 * toolbars, `primary` is the single brand-coloured call to action on a screen.
 */
export function Button({
  variant = "default",
  className,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant }) {
  return (
    <button
      {...props}
      className={clsx(
        "ease inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-md px-2.5 text-sm whitespace-nowrap",
        "disabled:pointer-events-none disabled:opacity-40",
        variant === "default" &&
          "border border-line bg-elevated text-fg hover:border-line-strong hover:bg-hover",
        variant === "ghost" && "text-muted hover:bg-hover hover:text-fg",
        variant === "primary" && "bg-brand font-medium text-[#1a1208] hover:brightness-110",
        className,
      )}
    />
  );
}

/** Square icon-only ghost button with a tooltip. */
export function IconButton({ className, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      {...props}
      className={clsx(
        "ease inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-muted hover:bg-hover hover:text-fg",
        className,
      )}
    />
  );
}
