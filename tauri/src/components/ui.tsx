import clsx from "clsx";
import type { ButtonHTMLAttributes, ReactNode } from "react";

/** Small toolbar button. */
export function Btn({ className, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      {...props}
      className={clsx(
        "inline-flex h-7 items-center gap-1.5 rounded-md border border-line bg-panel-2 px-2.5 text-xs",
        "text-fg hover:border-accent/60 hover:text-white disabled:opacity-40",
        className,
      )}
    />
  );
}

/** Panel header row. */
export function PanelHeader({ title, children }: { title: ReactNode; children?: ReactNode }) {
  return (
    <div className="flex h-9 shrink-0 items-center justify-between gap-2 border-b border-line bg-panel px-3">
      <div className="truncate text-xs font-semibold tracking-wide text-dim uppercase">{title}</div>
      <div className="flex items-center gap-1.5">{children}</div>
    </div>
  );
}

/** Pill used for tags and intents. */
export function Pill({ children, color }: { children: ReactNode; color?: string }) {
  return (
    <span
      className="inline-flex items-center rounded-full border px-1.5 py-px text-[10px] leading-4"
      style={{ borderColor: color ?? "#334155", color: color ?? "#94a3b8" }}
    >
      {children}
    </span>
  );
}
