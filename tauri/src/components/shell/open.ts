import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { inTauri } from "../../api";

/** Picks a binary with the native dialog (or a prompt in the browser preview) and opens it. */
export async function pickAndOpen(open: (p: string) => Promise<void>) {
  const path = inTauri
    ? await openDialog({ multiple: false, directory: false, title: "Open binary" })
    : window.prompt("Path to a binary (dev preview)");
  if (typeof path === "string" && path) await open(path);
}
