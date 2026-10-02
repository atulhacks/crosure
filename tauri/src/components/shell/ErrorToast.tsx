import { X } from "lucide-react";
import { useWorkbench } from "../../store/workbench";

/** The last error, until dismissed. */
export function ErrorToast() {
  const error = useWorkbench((s) => s.error);
  const clear = useWorkbench((s) => s.clearError);
  if (!error) return null;
  return (
    <div className="fixed right-4 bottom-8 z-50 flex max-w-md items-start gap-2 rounded-lg border border-bad/40 bg-elevated px-3 py-2 text-sm shadow-2xl">
      <span className="text-bad">{error}</span>
      <button onClick={clear} className="text-muted hover:text-fg" aria-label="Dismiss">
        <X size={14} />
      </button>
    </div>
  );
}
