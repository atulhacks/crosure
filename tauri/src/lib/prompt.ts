const ATTACHED = "\n\nAttached context:";

/**
 * Splits a sent prompt into what the analyst typed and whether `@` context was attached.
 *
 * @example splitPrompt("hi\n\nAttached context:\n…") // { text: "hi", attached: true }
 */
export function splitPrompt(prompt: string): { text: string; attached: boolean } {
  const i = prompt.indexOf(ATTACHED);
  return i < 0 ? { text: prompt, attached: false } : { text: prompt.slice(0, i), attached: true };
}
