import clsx from "clsx";
import { Bot } from "lucide-react";
import { useEffect, useRef } from "react";
import { clock, KIND_COLOR } from "../../lib/format";
import { useWorkbench } from "../../store/workbench";
import { Pill } from "../ui/Pill";

/** The investigation as a chronological flight log: time, step, command, outcome. */
export function FlightLog() {
  const graph = useWorkbench((s) => s.graph);
  const selected = useWorkbench((s) => s.selected);
  const selectNode = useWorkbench((s) => s.selectNode);
  const end = useRef<HTMLDivElement>(null);
  const nodes = graph?.nodes ?? [];
  const t0 = nodes[0]?.ts_ms ?? 0;
  useEffect(() => end.current?.scrollIntoView({ block: "end" }), [nodes.length]);
  return (
    <div className="scroll-host h-full overflow-auto py-1">
      {nodes.map((n) => (
        <button
          key={n.id}
          onClick={() => selectNode(n.id)}
          className={clsx(
            "ease relative flex w-full gap-3 px-3 py-1.5 text-left hover:bg-hover",
            selected === n.id && "bg-active",
            n.tags.includes("dead_end") && "opacity-50",
          )}
        >
          {n.on_key_path && (
            <span className="absolute inset-y-1 left-0 w-[2px] rounded-full bg-brand" />
          )}
          <span className="w-9 shrink-0 pt-px font-mono text-2xs text-faint nums">
            {clock(n.ts_ms - t0)}
          </span>
          <span
            className="mt-[5px] h-1.5 w-1.5 shrink-0 rounded-full"
            style={{ background: KIND_COLOR[n.kind] }}
          />
          <span className="min-w-0 flex-1">
            <span className="flex items-center gap-1.5">
              <span className="truncate font-mono text-xs text-fg">{n.command ?? n.label}</span>
              {n.actor === "agent" && <Bot size={11} />}
              <span className="ml-auto font-mono text-2xs text-faint nums">#{n.seq}</span>
            </span>
            <span className="block truncate text-2xs text-muted">{n.summary}</span>
            {(n.intent?.chip || n.tags.length > 0) && (
              <span className="mt-1 flex gap-1">
                {n.intent?.chip && <Pill tone="brand">{n.intent.chip.replace(/_/g, " ")}</Pill>}
                {n.tags.map((t) => (
                  <Pill key={t} tone={t === "key_step" ? "brand" : "muted"}>
                    {t.replace("_", " ")}
                  </Pill>
                ))}
              </span>
            )}
          </span>
        </button>
      ))}
      <div ref={end} />
    </div>
  );
}
