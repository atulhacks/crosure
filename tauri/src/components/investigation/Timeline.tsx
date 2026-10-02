import clsx from "clsx";
import { Pause, Play, SkipBack, SkipForward } from "lucide-react";
import { useEffect, useState } from "react";
import { KIND_COLOR } from "../../lib/format";
import { useWorkbench } from "../../store/workbench";
import type { GraphNode } from "../../types";
import { IconButton } from "../ui/Button";

/** One step on the strip: a thin bar in its kind colour, capped amber on the key path, dotted when an AI took it. */
function Tick({
  s,
  past,
  current,
  onPick,
}: {
  s: GraphNode;
  past: boolean;
  current: boolean;
  onPick: () => void;
}) {
  return (
    <button
      role="option"
      aria-selected={current}
      onClick={onPick}
      title={`#${s.seq} ${s.actor === "agent" ? "AI · " : ""}${s.command ?? s.label}`}
      className="group relative flex h-full w-[5px] shrink-0 flex-col items-center justify-end gap-[2px]"
    >
      {s.actor === "agent" && (
        <span className="h-[3px] w-[3px] rounded-full bg-[var(--kind-agent)]" />
      )}
      <span
        className={clsx(
          "ease w-[3px] rounded-[1px] group-hover:opacity-100",
          past ? "opacity-85" : "opacity-20",
          current ? "h-3.5" : "h-2.5",
        )}
        style={{
          background: KIND_COLOR[s.kind],
          boxShadow: s.on_key_path ? "inset 0 2px 0 var(--brand)" : undefined,
        }}
      />
      {current && <span className="absolute -bottom-[3px] h-[2px] w-[5px] rounded-full bg-brand" />}
    </button>
  );
}

/** The investigation as a strip of steps. Pick, step or play to see the graph as it was. */
export function Timeline() {
  const upto = useWorkbench((s) => s.upto);
  const headSeq = useWorkbench((s) => s.headSeq);
  const steps = useWorkbench((s) => s.timeline);
  const setUpto = useWorkbench((s) => s.setUpto);
  const [playing, setPlaying] = useState(false);
  const pos = upto ?? headSeq;
  const go = (n: number) => {
    setPlaying(false);
    setUpto(n >= headSeq ? null : Math.max(0, n));
  };
  const prevSeq = [...steps].reverse().find((s) => s.seq < pos)?.seq ?? 0;
  const nextSeq = steps.find((s) => s.seq > pos)?.seq ?? headSeq;

  useEffect(() => {
    if (!playing) return;
    const seqs = steps.map((s) => s.seq);
    let i = 0;
    setUpto(seqs[0] ?? 0);
    const timer = setInterval(() => {
      i += 1;
      if (i >= seqs.length) {
        setPlaying(false);
        setUpto(null);
      } else setUpto(seqs[i]);
    }, 550);
    return () => clearInterval(timer);
  }, [playing, steps, setUpto]);

  return (
    <div className="flex h-9 shrink-0 items-center gap-1 border-t px-1.5">
      <IconButton onClick={() => go(prevSeq)} title="Previous step">
        <SkipBack size={12} />
      </IconButton>
      <IconButton
        onClick={() => setPlaying(!playing)}
        title={playing ? "Pause" : "Replay from the first step"}
      >
        {playing ? <Pause size={12} /> : <Play size={12} />}
      </IconButton>
      <IconButton onClick={() => go(nextSeq)} title="Next step">
        <SkipForward size={12} />
      </IconButton>
      <div
        className="scroll-host mx-1.5 flex h-6 min-w-0 flex-1 items-end gap-[3px] overflow-x-auto overflow-y-hidden pb-[3px]"
        role="listbox"
        aria-label="Steps"
      >
        {steps.map((s) => (
          <Tick
            key={s.id}
            s={s}
            past={s.seq <= pos}
            current={s.seq === pos}
            onPick={() => go(s.seq)}
          />
        ))}
      </div>
      <span className="w-10 text-right font-mono text-2xs text-muted nums">#{pos}</span>
      <button
        onClick={() => go(headSeq)}
        className={clsx(
          "ease h-6 rounded-md px-2 font-mono text-2xs tracking-wider",
          upto === null ? "text-brand" : "text-muted hover:bg-hover hover:text-fg",
        )}
      >
        LIVE
      </button>
    </div>
  );
}
