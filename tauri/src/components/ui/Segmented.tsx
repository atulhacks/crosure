import clsx from "clsx";
import type { ReactNode } from "react";

/** A small two-or-three-way toggle (e.g. Linear | Graph). */
export function Segmented<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: { value: T; label: ReactNode }[];
  onChange: (v: T) => void;
}) {
  return (
    <div className="inline-flex h-6 items-center rounded-md border p-0.5">
      {options.map((o) => (
        <button
          key={o.value}
          onClick={() => onChange(o.value)}
          className={clsx(
            "ease h-full rounded-[3px] px-2 text-xs",
            o.value === value ? "bg-active text-fg" : "text-muted hover:text-fg",
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
