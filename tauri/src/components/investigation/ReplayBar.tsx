import { Pause, Play } from "lucide-react";
import { useEffect, useState } from "react";
import { useWorkbench } from "../../store/workbench";
import { Button, IconButton } from "../ui/Button";

/** Scrub or play back the investigation as it happened. */
export function ReplayBar() {
  const upto = useWorkbench((s) => s.upto);
  const headSeq = useWorkbench((s) => s.headSeq);
  const setUpto = useWorkbench((s) => s.setUpto);
  const [playing, setPlaying] = useState(false);
  const value = upto ?? headSeq;

  useEffect(() => {
    if (!playing) return;
    let n = 0;
    setUpto(0);
    const timer = setInterval(() => {
      n += 1;
      if (n > headSeq) {
        setPlaying(false);
        setUpto(null);
      } else setUpto(n);
    }, 600);
    return () => clearInterval(timer);
  }, [playing, headSeq, setUpto]);

  return (
    <div className="flex h-9 shrink-0 items-center gap-2 border-t px-2">
      <IconButton
        onClick={() => setPlaying(!playing)}
        title={playing ? "Pause" : "Replay from the start"}
      >
        {playing ? <Pause size={13} /> : <Play size={13} />}
      </IconButton>
      <input
        type="range"
        min={0}
        max={Math.max(headSeq, 1)}
        value={value}
        aria-label="Replay position"
        onChange={(e) => {
          setPlaying(false);
          const n = Number(e.target.value);
          setUpto(n >= headSeq ? null : n);
        }}
        className="h-1 flex-1 cursor-pointer accent-[var(--brand)]"
      />
      <span className="w-14 text-right font-mono text-2xs text-muted nums">
        {value}/{headSeq}
      </span>
      <Button
        variant="ghost"
        className={upto === null ? "text-brand" : ""}
        onClick={() => {
          setPlaying(false);
          setUpto(null);
        }}
      >
        Live
      </Button>
    </div>
  );
}
