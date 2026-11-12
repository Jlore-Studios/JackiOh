// Drawing a card's face only once the module is in (`./WhenEngineReady.tsx`, `./useEngineReady.ts`,
// #778): behind `EngineGate` the module is already loaded by `test/setup.ts`, so these pin the ready
// branch — the children draw, the fallback does not. The unready branch belongs to the landing and
// shop screens' suites, which hold the module back.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { wasmLoaded } from "./index.ts";
import { useEngineReady } from "./useEngineReady.ts";
import { WhenEngineReady } from "./WhenEngineReady.tsx";

function Probe(): string {
  return useEngineReady() ? "ready" : "waiting";
}

afterEach(() => {
  cleanup();
});

describe("useEngineReady", () => {
  it("is ready once the module has loaded", () => {
    expect(wasmLoaded()).toBe(true);
    render(<Probe />);
    expect(screen.getByText("ready")).toBeInTheDocument();
  });
});

describe("WhenEngineReady", () => {
  it("draws the children once the module is in, not the fallback", () => {
    render(
      <WhenEngineReady fallback={<span>loading</span>}>
        {() => <span>face</span>}
      </WhenEngineReady>,
    );
    expect(screen.getByText("face")).toBeInTheDocument();
    expect(screen.queryByText("loading")).toBeNull();
  });

  it("stands in nothing when no fallback is given", () => {
    const { container } = render(<WhenEngineReady>{() => <span>face</span>}</WhenEngineReady>);
    expect(screen.getByText("face")).toBeInTheDocument();
    expect(container.textContent).toBe("face");
  });
});
