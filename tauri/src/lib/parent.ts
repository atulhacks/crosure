import type { ParentRef } from "../types";

/** What the UI knows when it is about to run an op. */
export interface ParentContext {
  /** Step the analyst jumped back to on the canvas, if any. */
  branchFrom: string | null;
  /** Latest recorded step id. */
  headId: string | null;
  /** Step that produced the view the analyst clicked in. */
  viewStepId: string | null;
  /** True when the op was started by clicking inside that view. */
  fromView: boolean;
}

/**
 * Chooses the explicit parent edge for the next op.
 * - Jumped back to an older step → `branch` from it.
 * - Clicked something in a result view → `derived_from` that view's step.
 * - Otherwise none (the recorder adds a `next` edge itself).
 *
 * @example
 * nextParent({ branchFrom: "a", headId: "c", viewStepId: "b", fromView: true })
 * // { id: "a", rel: "branch" }
 */
export function nextParent(ctx: ParentContext): ParentRef | null {
  if (ctx.branchFrom && ctx.branchFrom !== ctx.headId) {
    return { id: ctx.branchFrom, rel: "branch" };
  }
  if (ctx.fromView && ctx.viewStepId) {
    return { id: ctx.viewStepId, rel: "derived_from" };
  }
  return null;
}
