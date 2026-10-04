import { Cpu, KeyRound, Settings2 } from "lucide-react";
import { useAgent } from "../../store/agent";
import { Button } from "../ui/Button";

/** Shown until the analyst has chosen a provider and it is ready. */
export function AgentSetup() {
  const setSettingsOpen = useAgent((s) => s.setSettingsOpen);
  const setActive = useAgent((s) => s.setActive);
  const status = useAgent((s) => s.status);
  const detected = status?.first_run && status.configured && status.key_source === "env";
  return (
    <div className="flex h-full flex-col justify-center gap-3 px-4">
      <div className="flex items-center gap-2 text-sm font-medium">
        <Cpu size={14} className="text-brand" /> Choose an AI model
      </div>
      <p className="text-xs text-muted">
        The agent reverses the binary with the same tools you use, and every call it makes is
        recorded on the graph with its reason. Use Claude, OpenAI, Gemini, DeepSeek, Z.ai, Moonshot
        (Kimi), xAI, Qwen, Mistral, OpenRouter or any OpenAI- or Anthropic-compatible API, or a
        local model through Ollama, LM Studio, llama.cpp or vLLM, so samples never leave your
        machine.
      </p>
      {detected && status && (
        <div className="flex flex-col gap-2 rounded-md border px-3 py-2.5">
          <p className="flex items-center gap-1.5 text-xs text-muted">
            <KeyRound size={12} className="text-brand" />
            Found a key in <span className="font-mono text-fg">{status.key_env}</span>
          </p>
          <Button
            variant="primary"
            className="self-start"
            onClick={() => setActive(status.provider)}
          >
            Use {status.label} · <span className="font-mono">{status.model}</span>
          </Button>
        </div>
      )}
      <Button
        variant={detected ? "default" : "primary"}
        className="self-start"
        onClick={() => setSettingsOpen(true)}
      >
        <Settings2 size={13} /> {detected ? "Choose another provider" : "Set up providers"}
      </Button>
      {status && status.provider && !status.configured && (
        <p className="text-2xs text-faint">
          {status.label || status.provider} needs {status.model ? "an API key" : "a model"}.
        </p>
      )}
    </div>
  );
}
