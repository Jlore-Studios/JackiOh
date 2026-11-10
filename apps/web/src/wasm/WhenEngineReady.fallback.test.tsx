// The fallback branch of `WhenEngineReady` (`./WhenEngineReady.tsx`, #778): while the
// WebAssembly module is out, the fallback stands in and the children are never worked out.
// The ready branch lives in `WhenEngineReady.test.tsx` (the module is already loaded by
// `test/setup.ts`); the full arrive-then-draw flow lives in `routes/engine-gate.test.tsx`.
// This file holds the module back with a mocked `useEngineReady`, so Stryker's `related`
// runs the unready branch colocated with the source.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { WhenEngineReady } from "./WhenEngineReady.tsx";

vi.mock("./useEngineReady.ts", () => ({ useEngineReady: () => false }));

afterEach(() => {
  cleanup();
});

describe("WhenEngineReady while the module is out", () => {
  it("stands in the fallback and never works out the children", () => {
    const children = vi.fn(() => <span>face</span>);
    render(
      <WhenEngineReady fallback={<span>loading</span>}>
        {children}
      </WhenEngineReady>,
    );
    expect(screen.getByText("loading")).toBeInTheDocument();
    expect(screen.queryByText("face")).toBeNull();
    expect(children).not.toHaveBeenCalled();
  });

  it("stands in nothing when no fallback is given", () => {
    const children = vi.fn(() => <span>face</span>);
    const { container } = render(<WhenEngineReady>{children}</WhenEngineReady>);
    expect(container.textContent).toBe("");
    expect(children).not.toHaveBeenCalled();
  });
});
