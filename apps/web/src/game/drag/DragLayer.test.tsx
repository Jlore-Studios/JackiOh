// The pointer drag layer (`./DragLayer.tsx`, S9, R384, R510, R658): presses become drags
// past the threshold and releases travel the usual target path. The drags themselves are
// proved through `<Game/>` in `test/ux/drag-layer.test.tsx`, `test/ux/drag-continue.test.tsx`
// and `test/ux/drag-blocked.test.tsx`; here the layer's idle mount is pinned, so a layer
// that renders where none belongs fails colocated with the source.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { baseView } from "../../test/fixtures.ts";
import { IDLE } from "../actions.ts";
import DragLayer from "./DragLayer.tsx";

afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute("data-dragging");
});

describe("DragLayer idle", () => {
  it("mounts with no drag drawn", () => {
    render(
      <DragLayer
        view={baseView()}
        legal={[]}
        interaction={IDLE}
        onInteraction={() => undefined}
        onAction={vi.fn()}
      />,
    );
    expect(screen.queryByTestId("drag-layer")).toBeNull();
    expect(screen.queryByTestId("drag-blocked")).toBeNull();
  });
});
