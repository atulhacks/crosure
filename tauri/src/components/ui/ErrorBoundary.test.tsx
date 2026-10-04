import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "./ErrorBoundary";

function Boom(): never {
  throw new Error("kaboom");
}

describe("ErrorBoundary", () => {
  it("shows the error instead of unmounting the app", () => {
    (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
    const quiet = vi.spyOn(console, "error").mockImplementation(() => {});
    const host = document.createElement("div");
    act(() =>
      createRoot(host).render(
        <div>
          <span>sibling</span>
          <ErrorBoundary name="Agent">
            <Boom />
          </ErrorBoundary>
        </div>,
      ),
    );
    expect(host.textContent).toContain("sibling");
    expect(host.textContent).toContain("Agent hit an error");
    expect(host.textContent).toContain("kaboom");
    quiet.mockRestore();
  });
});
