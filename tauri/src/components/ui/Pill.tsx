import clsx from "clsx";
import type { ReactNode } from "react";

/** Tag / intent pill. `tone` picks a theme colour. */
export function Pill({
  children,
  tone = "muted",
  className,
}: {
  children: ReactNode;
  tone?: "muted" | "brand" | "bad" | "good" | "warn";
  className?: string;
}) {
  const color = {
    muted: "text-muted border-line-strong",
    brand: "text-brand border-brand/40 bg-brand-soft",
    bad: "text-bad border-bad/40",
    good: "text-good border-good/40",
    warn: "text-warn border-warn/40",
  }[tone];
  return (
    <span
      className={clsx(
        "inline-flex h-4 items-center rounded-full border px-1.5 text-2xs leading-none",
        color,
        className,
      )}
    >
      {children}
    </span>
  );
}

/** A keyboard shortcut hint. */
export function Kbd({ children }: { children: ReactNode }) {
  return (
    <kbd className="inline-flex h-4 min-w-4 items-center justify-center rounded-sm border px-1 font-mono text-2xs text-faint">
      {children}
    </kbd>
  );
}
