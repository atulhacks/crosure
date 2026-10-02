import type { ReactNode } from "react";

/** Column header row for a data list. Widths are Tailwind classes matching the rows. */
export function ColHead({ cols }: { cols: [string, string][] }) {
  return (
    <div className="row sticky top-0 z-10 h-7 border-b bg-bg/95 backdrop-blur">
      {cols.map(([label, cls]) => (
        <span key={label} className={`label ${cls}`}>
          {label}
        </span>
      ))}
    </div>
  );
}

/** One-line summary above a result list. */
export function ResultTitle({ children }: { children: ReactNode }) {
  return (
    <div className="flex h-10 items-center gap-2 border-b px-4 text-sm text-muted">{children}</div>
  );
}
