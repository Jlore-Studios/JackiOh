// The locked-zone refused mark (`./BlockedMark.tsx`, #37): a red X where a card landed on
// a Locked zone, for BLOCKED_MARK_MS whatever the motion setting. The drop that raises it
// through `<Game/>` is proved in `test/ux/drag-blocked.test.tsx`; here the mark's own render
// and timing are pinned.

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import BlockedMark, { BLOCKED_MARK_MS } from "./BlockedMark.tsx";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("BLOCKED_MARK_MS", () => {
  it("holds the mark for 700 ms", () => {
    expect(BLOCKED_MARK_MS).toBe(700);
  });
});

describe("BlockedMark", () => {
  it("renders nothing with no blocked drop", () => {
    const { container } = render(<BlockedMark blocked={null} onDone={() => undefined} />);
    expect(container.textContent).toBe("");
    expect(screen.queryByTestId("drag-blocked")).toBeNull();
  });

  it("marks the locked zone it landed on", () => {
    render(
      <BlockedMark
        blocked={{ key: 1, testid: "zone-you-units-3", x: 10, y: 20, size: 44 }}
        onDone={() => undefined}
      />,
    );
    const mark = screen.getByTestId("drag-blocked");
    expect(mark).toHaveAttribute("data-zone", "zone-you-units-3");
  });

  it("takes the mark down after its hold", () => {
    vi.useFakeTimers();
    const onDone = vi.fn();
    render(
      <BlockedMark
        blocked={{ key: 2, testid: "zone-you-units-2", x: 0, y: 0, size: 30 }}
        onDone={onDone}
      />,
    );
    expect(screen.queryByTestId("drag-blocked")).not.toBeNull();
    act(() => {
      vi.advanceTimersByTime(BLOCKED_MARK_MS - 1);
    });
    expect(onDone).not.toHaveBeenCalled();
    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(onDone).toHaveBeenCalledTimes(1);
  });
});
