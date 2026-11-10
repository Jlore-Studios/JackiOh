// A prompt's options dragged as well as clicked (`./OptionDrag.tsx`, R658): a press past
// the threshold lifts the option, a release outside the panel clicks it. The drags are proved
// through the prompt suites; here the layer's idle mount is pinned.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import OptionDrag from "./OptionDrag.tsx";

afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute("data-dragging");
});

describe("OptionDrag idle", () => {
  it("mounts with no drag drawn", () => {
    render(<OptionDrag />);
    expect(screen.queryByTestId("drag-layer")).toBeNull();
    expect(screen.queryByTestId("drag-option-ghost")).toBeNull();
  });
});
