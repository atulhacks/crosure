import { X } from "lucide-react";
import { useWorkbench } from "../store/workbench";

/** Shows the last error until dismissed. */
export function ErrorToast() {
  const error = useWorkbench((s) => s.error);
  const clear = useWorkbench((s) => s.clearError);
  if (!error) return null;
  return (
    <div className="fixed right-4 bottom-12 z-50 flex max-w-md items-start gap-2 rounded-md border border-bad/60 bg-panel-2 px-3 py-2 text-xs shadow-xl">
      <span className="text-bad">{error}</span>
      <button onClick={clear} className="text-dim hover:text-fg" aria-label="dismiss">
        <X size={14} />
      </button>
    </div>
  );
}
