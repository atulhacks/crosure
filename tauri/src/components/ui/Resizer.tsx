import clsx from "clsx";
import { useRef } from "react";

/**
 * A drag handle between panes. Reports the pointer delta from where the drag
 * started; the caller turns it into a size. `invert` is for panes anchored on
 * the far side (right column, bottom console).
 */
export function Resizer({
  axis,
  start,
  onResize,
  onEnd,
  invert = false,
}: {
  axis: "x" | "y";
  start: number;
  onResize: (px: number) => void;
  onEnd: (px: number) => void;
  invert?: boolean;
}) {
  const origin = useRef<{ pos: number; size: number } | null>(null);
  const last = useRef(start);
  const size = (e: React.PointerEvent) => {
    const o = origin.current;
    if (!o) return start;
    const delta = (axis === "x" ? e.clientX : e.clientY) - o.pos;
    return o.size + (invert ? -delta : delta);
  };
  return (
    <div
      role="separator"
      aria-orientation={axis === "x" ? "vertical" : "horizontal"}
      className={clsx(
        "ease group relative z-10 shrink-0 bg-transparent",
        axis === "x"
          ? "-mx-[2px] w-[5px] cursor-col-resize"
          : "-my-[2px] h-[5px] cursor-row-resize",
      )}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        origin.current = { pos: axis === "x" ? e.clientX : e.clientY, size: start };
      }}
      onPointerMove={(e) => {
        if (!origin.current) return;
        last.current = size(e);
        onResize(last.current);
      }}
      onPointerUp={(e) => {
        if (!origin.current) return;
        e.currentTarget.releasePointerCapture(e.pointerId);
        origin.current = null;
        onEnd(last.current);
      }}
    >
      <div
        className={clsx(
          "ease absolute bg-transparent group-hover:bg-brand/50",
          axis === "x" ? "inset-y-0 left-[2px] w-px" : "inset-x-0 top-[2px] h-px",
        )}
      />
    </div>
  );
}
