import type { Live } from "../../types";

/** The last `n` characters of `text`, cut at a line start when possible. */
export function tail(text: string, n: number): string {
  if (text.length <= n) return text;
  const cut = text.slice(-n);
  const nl = cut.indexOf("\n");
  return `…${nl >= 0 && nl < n / 2 ? cut.slice(nl + 1) : cut}`;
}

/** The reply as it streams in: reasoning (dimmed), text, and the tool call being written. */
export function LiveReply({ live }: { live: Live }) {
  return (
    <div className="flex flex-col gap-1 text-xs">
      {live.thinking && (
        <p className="whitespace-pre-wrap italic text-faint">{tail(live.thinking, 600)}</p>
      )}
      {live.text && <p className="whitespace-pre-wrap text-fg">{tail(live.text, 2000)}</p>}
      {live.tool && <span className="font-mono text-2xs text-muted">preparing {live.tool}…</span>}
    </div>
  );
}
