import { Pause, Play, Radio } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useWorkbench } from "../store/workbench";
import { Btn } from "./ui";

/** Replay scrubber: shows the graph as it was after any step. */
export function Timeline() {
  const { upto, headSeq, setUpto } = useWorkbench();
  const [playing, setPlaying] = useState(false);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);
  const value = upto ?? headSeq;

  useEffect(() => {
    if (!playing) return;
    let n = 0;
    setUpto(0);
    timer.current = setInterval(() => {
      n += 1;
      if (n > headSeq) {
        setPlaying(false);
        setUpto(null);
      } else {
        setUpto(n);
      }
    }, 650);
    return () => {
      if (timer.current) clearInterval(timer.current);
    };
  }, [playing, headSeq, setUpto]);

  return (
    <div className="flex h-10 shrink-0 items-center gap-3 border-t border-line bg-panel px-3 text-xs">
      <Btn onClick={() => setPlaying(!playing)} title="Replay the investigation">
        {playing ? <Pause size={13} /> : <Play size={13} />} Replay
      </Btn>
      <input
        type="range"
        min={0}
        max={headSeq}
        value={value}
        className="flex-1 accent-cyan-400"
        onChange={(e) => {
          setPlaying(false);
          const n = Number(e.target.value);
          setUpto(n >= headSeq ? null : n);
        }}
      />
      <span className="w-28 text-right font-mono text-dim">
        step {value} / {headSeq}
      </span>
      <Btn
        onClick={() => {
          setPlaying(false);
          setUpto(null);
        }}
        className={upto === null ? "text-good" : ""}
      >
        <Radio size={13} /> Live
      </Btn>
    </div>
  );
}
