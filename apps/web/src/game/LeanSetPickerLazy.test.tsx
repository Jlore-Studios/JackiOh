// R2031000: the shell's way to the "More cards from" picker (#778). If the engine's module never
// arrives it shows nothing, without throwing: the shell that mounts it must keep painting.

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

afterEach(() => {
  cleanup();
  vi.doUnmock("../wasm/index.ts");
  vi.resetModules();
});

const noop = (): void => undefined;

describe("R2031000 the lazy More cards from picker", () => {
  it("R2031000 shows the picker once the engine is in", async () => {
    const { LeanSetPickerLazy } = await import("./LeanSetPickerLazy.tsx");
    render(<LeanSetPickerLazy value="Classic" onChange={noop} testid="lazy-lean" />);
    expect(((await screen.findByTestId("lazy-lean")) as HTMLSelectElement).value).toBe("Classic");
  });

  it("R2031000 shows nothing, and throws nothing, when the engine never arrives", async () => {
    const loadWasm = vi.fn((): Promise<void> => Promise.reject(new Error("Failed to fetch /assets/jackioh_wasm_bg.wasm")));
    vi.resetModules();
    vi.doMock("../wasm/index.ts", async (importOriginal) => ({
      ...(await importOriginal<Record<string, unknown>>()),
      loadWasm,
    }));
    const { LeanSetPickerLazy } = await import("./LeanSetPickerLazy.tsx");
    render(
      <div data-testid="shell">
        <LeanSetPickerLazy value="Classic" onChange={noop} testid="lazy-lean" />
      </div>,
    );
    await vi.waitFor(() => {
      expect(loadWasm).toHaveBeenCalledTimes(1);
    });
    // Let the refused load and the picker's code settle: a lazy load that threw would take the shell with it.
    for (let turn = 0; turn < 5; turn += 1) {
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
    }
    expect(screen.getByTestId("shell")).toBeEmptyDOMElement();
  });
});
