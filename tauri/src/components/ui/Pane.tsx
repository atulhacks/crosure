import clsx from "clsx";
import type { ReactNode } from "react";

/**
 * A docked pane: title bar (label · count · actions) over a body. Every
 * surface uses it, so headers, counts and spacing match everywhere.
 */
export function Pane({
  title,
  count,
  actions,
  children,
  className,
  bodyClassName,
  scroll = true,
}: {
  title?: ReactNode;
  count?: number | string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
  scroll?: boolean;
}) {
  return (
    <section className={clsx("flex min-h-0 min-w-0 flex-col", className)}>
      {title !== undefined && (
        <header className="flex h-[var(--bar-h)] shrink-0 items-center gap-2 border-b px-3">
          <span className="label truncate">{title}</span>
          {count !== undefined && <span className="nums text-2xs text-faint">{count}</span>}
          {actions && <div className="ml-auto flex items-center gap-1">{actions}</div>}
        </header>
      )}
      <div className={clsx("min-h-0 flex-1", scroll && "scroll-host overflow-auto", bodyClassName)}>
        {children}
      </div>
    </section>
  );
}

/** A quiet placeholder for an empty pane: a glyph, a line, a hint. */
export function Empty({
  icon,
  title,
  hint,
}: {
  icon?: ReactNode;
  title: string;
  hint?: ReactNode;
}) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center text-muted">
      {icon && <div className="mb-1 opacity-50">{icon}</div>}
      <div className="text-sm">{title}</div>
      {hint && <div className="max-w-[34ch] text-xs text-faint">{hint}</div>}
    </div>
  );
}
