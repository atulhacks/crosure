import { Cpu, Settings2 } from "lucide-react";
import { useAgent } from "../../store/agent";
import { Button } from "../ui/Button";

/** Shown until a provider is ready. */
export function AgentSetup() {
  const setSettingsOpen = useAgent((s) => s.setSettingsOpen);
  const status = useAgent((s) => s.status);
  return (
    <div className="flex h-full flex-col justify-center gap-3 px-4">
      <div className="flex items-center gap-2 text-sm font-medium">
        <Cpu size={14} className="text-brand" /> Choose an AI model
      </div>
      <p className="text-xs text-muted">
        The agent reverses the binary with the same tools you use, and every call it makes is
        recorded on the graph with its reason. Use Claude, OpenAI, Gemini, OpenRouter, Groq,
        DeepSeek or Mistral, or a local model through Ollama or LM Studio, so samples never leave
        your machine.
      </p>
      <Button variant="primary" className="self-start" onClick={() => setSettingsOpen(true)}>
        <Settings2 size={13} /> Set up providers
      </Button>
      {status && status.provider && !status.configured && (
        <p className="text-2xs text-faint">
          {status.label || status.provider} needs {status.model ? "an API key" : "a model"}.
        </p>
      )}
    </div>
  );
}
