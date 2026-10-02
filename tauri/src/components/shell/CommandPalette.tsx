import { CornerDownLeft, FunctionSquare, Play, TerminalSquare } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { fuzzyFilter } from "../../lib/fuzzy";
import { hex } from "../../lib/format";
import { useUi } from "../../store/ui";
import { useWorkbench } from "../../store/workbench";

interface Item {
  id: string;
  label: string;
  hint?: string;
  icon: ReactNode;
  run: () => void;
}

/** Ctrl+K: go to any function, run any action, or send a console command. */
export function CommandPalette() {
  const { paletteOpen, setPaletteOpen, toggleConsole, setInvMode, setDisasmMode, setDockTab } =
    useUi();
  const { functions, act, openTab, openFunction, verify, exportSession, runConsole, opened } =
    useWorkbench();
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        if (useWorkbench.getState().opened) setPaletteOpen(!useUi.getState().paletteOpen);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setPaletteOpen]);

  const actions: Item[] = useMemo(() => {
    const a = (id: string, label: string, run: () => void, hint?: string): Item => ({
      id,
      label,
      run,
      hint,
      icon: <Play size={12} />,
    });
    return [
      a("agent", "Ask the AI agent", () => setDockTab("agent"), "Ctrl+L"),
      a("strings", "Show strings", () => openTab("strings")),
      a("imports", "Show imports", () => openTab("imports")),
      a("info", "Binary info", () => act({ op: "info" })),
      a("entry", "Go to entry point", () =>
        act({ op: "disasm", target: hex(opened?.info.entry ?? 0) }),
      ),
      a("verify", "Verify hash chain", () => verify()),
      a("export", "Export session", () => exportSession()),
      a("console", "Toggle console", toggleConsole, "Ctrl+`"),
      a("log", "Investigation: flight log", () => setInvMode("log")),
      a("graph", "Investigation: graph", () => setInvMode("graph")),
      a("cfg", "Disassembly: block graph", () => setDisasmMode("graph")),
      a("linear", "Disassembly: linear", () => setDisasmMode("linear")),
    ];
  }, [
    act,
    openTab,
    verify,
    exportSession,
    toggleConsole,
    setInvMode,
    setDisasmMode,
    setDockTab,
    opened,
  ]);

  const items: Item[] = useMemo(() => {
    const fns = fuzzyFilter(q, functions, (f) => f.name, 40).map((f) => ({
      id: `fn-${f.addr}`,
      label: f.name,
      hint: hex(f.addr),
      icon: <FunctionSquare size={12} />,
      run: () => openFunction(f.addr),
    }));
    const acts = fuzzyFilter(q, actions, (x) => x.label, 20);
    const cmd: Item[] = q.trim()
      ? [
          {
            id: "run-console",
            label: `Run "${q.trim()}" in console`,
            icon: <TerminalSquare size={12} />,
            run: () => runConsole(q.trim()),
          },
        ]
      : [];
    return q ? [...fns.slice(0, 12), ...acts, ...cmd] : [...acts, ...fns.slice(0, 8)];
  }, [q, functions, actions, openFunction, runConsole]);

  useEffect(() => {
    if (paletteOpen) {
      setQ("");
      setSel(0);
      setTimeout(() => input.current?.focus(), 0);
    }
  }, [paletteOpen]);

  if (!paletteOpen) return null;
  const choose = (item?: Item) => {
    if (!item) return;
    setPaletteOpen(false);
    item.run();
  };
  return (
    <div
      className="fixed inset-0 z-50 flex justify-center bg-black/40 pt-[12vh]"
      onMouseDown={() => setPaletteOpen(false)}
    >
      <div
        className="h-fit w-[560px] overflow-hidden rounded-lg border border-line-strong bg-elevated shadow-2xl"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <input
          ref={input}
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setSel(0);
          }}
          onKeyDown={(e) => {
            if (e.key === "Escape") setPaletteOpen(false);
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setSel((s) => Math.min(items.length - 1, s + 1));
            }
            if (e.key === "ArrowUp") {
              e.preventDefault();
              setSel((s) => Math.max(0, s - 1));
            }
            if (e.key === "Enter") choose(items[sel]);
          }}
          placeholder="Function name, action, or console command…"
          className="h-10 w-full border-b bg-transparent px-3 text-base outline-none placeholder:text-faint"
        />
        <div className="max-h-[50vh] overflow-auto py-1">
          {items.map((it, i) => (
            <button
              key={it.id}
              onMouseEnter={() => setSel(i)}
              onClick={() => choose(it)}
              className={`flex h-7 w-full items-center gap-2 px-3 text-left text-sm ${i === sel ? "bg-active text-fg" : "text-muted"}`}
            >
              <span className="text-faint">{it.icon}</span>
              <span className="truncate">{it.label}</span>
              {it.hint && (
                <span className="ml-auto font-mono text-2xs text-faint nums">{it.hint}</span>
              )}
              {i === sel && !it.hint && <CornerDownLeft size={11} className="ml-auto text-faint" />}
            </button>
          ))}
          {items.length === 0 && <div className="px-3 py-2 text-sm text-faint">No matches</div>}
        </div>
      </div>
    </div>
  );
}
