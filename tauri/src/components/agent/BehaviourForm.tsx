import type { Permission, Profile } from "../../types";

/** Tools that change what others read, plus the verdict: the ones worth confirming. */
export const CONFIRMABLE = ["rename_function", "add_comment", "record_verdict"] as const;

/** True when every confirmable tool is set to confirm. */
export function confirmsChanges(p: Record<string, Permission>): boolean {
  return CONFIRMABLE.every((t) => p[t] === "confirm");
}

/** Sets every confirmable tool to confirm (or back to allow). */
export function withConfirm(
  p: Record<string, Permission>,
  on: boolean,
): Record<string, Permission> {
  const next = { ...p };
  for (const t of CONFIRMABLE) next[t] = on ? "confirm" : "allow";
  return next;
}

/** Instructions, approvals and the default profile. */
export function BehaviourForm({
  instructions,
  setInstructions,
  permissions,
  setPermissions,
  defaultProfile,
  setDefaultProfile,
}: {
  instructions: string;
  setInstructions: (v: string) => void;
  permissions: Record<string, Permission>;
  setPermissions: (v: Record<string, Permission>) => void;
  defaultProfile: Profile;
  setDefaultProfile: (v: Profile) => void;
}) {
  return (
    <div className="flex flex-col gap-5">
      <label className="flex flex-col gap-1">
        <span className="label">Analyst instructions</span>
        <textarea
          value={instructions}
          onChange={(e) => setInstructions(e.target.value)}
          rows={6}
          placeholder={
            "Added to every conversation, e.g.\n- Map behaviour to MITRE ATT&CK techniques.\n- List IOCs in a table.\n- Write for first-year students."
          }
          className="resize-y rounded-md border bg-bg px-2 py-1.5 text-sm outline-none placeholder:text-faint focus:border-brand/60"
        />
      </label>
      <label className="flex items-start gap-2 text-sm">
        <input
          type="checkbox"
          className="mt-1"
          checked={confirmsChanges(permissions)}
          onChange={(e) => setPermissions(withConfirm(permissions, e.target.checked))}
        />
        <span>
          Ask me before the agent renames a function, adds a comment, or records a verdict
          <span className="block text-xs text-muted">
            You get Allow / Deny in the conversation. Denied calls are not recorded.
          </span>
        </span>
      </label>
      <label className="flex flex-col gap-1">
        <span className="label">New conversations start as</span>
        <select
          value={defaultProfile}
          onChange={(e) => setDefaultProfile(e.target.value as Profile)}
          className="h-7 w-56 rounded-md border bg-bg px-1.5 text-sm"
        >
          <option value="investigate">Investigate (all tools)</option>
          <option value="read_only">Read-only (no renames or comments)</option>
          <option value="ask">Ask (no tools)</option>
        </select>
      </label>
    </div>
  );
}
