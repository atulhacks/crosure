import { KeyRound } from "lucide-react";
import { useState } from "react";
import { useAgent } from "../../store/agent";
import { Button } from "../ui/Button";

/** Connect Claude: the key is saved locally and only sent to the Anthropic API. */
export function AgentSetup() {
  const configure = useAgent((s) => s.configure);
  const error = useAgent((s) => s.error);
  const [key, setKey] = useState("");
  return (
    <div className="flex h-full flex-col justify-center gap-3 px-4">
      <div className="flex items-center gap-2 text-sm font-medium">
        <KeyRound size={14} className="text-brand" /> Connect Claude
      </div>
      <p className="text-xs text-muted">
        The agent reverses the binary with the same tools you use. Every call it makes is recorded
        on the graph, with its reason, so you can audit and replay it.
      </p>
      <form
        className="flex gap-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          if (key.trim()) configure(key.trim(), null);
        }}
      >
        <input
          type="password"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          placeholder="Anthropic API key (sk-ant-…)"
          className="h-7 min-w-0 flex-1 rounded-md border bg-bg px-2 font-mono text-xs outline-none placeholder:text-faint focus:border-brand/60"
        />
        <Button type="submit" variant="primary">
          Save
        </Button>
      </form>
      <p className="text-2xs text-faint">
        Stored in ~/.crosure/agent.json (readable only by you), or set ANTHROPIC_API_KEY. Samples
        stay local; only tool results (disassembly, strings) are sent to the model.
      </p>
      {error && <p className="text-xs text-bad">{error}</p>}
    </div>
  );
}
