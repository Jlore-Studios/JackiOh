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
    // Centred on the zone and as large as it.
    expect(mark.style.left).toBe("10px");
    expect(mark.style.top).toBe("20px");
    expect(mark.style.width).toBe("44px");
    expect(mark.style.height).toBe("44px");
  });

  it("never takes a hit, so the drop under it reads through", () => {
    render(
      <BlockedMark
        blocked={{ key: 1, testid: "zone-you-units-3", x: 10, y: 20, size: 44 }}
        onDone={() => undefined}
      />,
    );
    expect(screen.getByTestId("drag-blocked").parentElement?.style.pointerEvents).toBe("none");
  });

  it("holds afresh for a new mark, and never calls back once it is gone", () => {
    vi.useFakeTimers();
    const onDone = vi.fn();
    const { rerender, unmount } = render(
      <BlockedMark
        blocked={{ key: 3, testid: "zone-you-units-1", x: 0, y: 0, size: 30 }}
        onDone={onDone}
      />,
    );
    act(() => {
      vi.advanceTimersByTime(BLOCKED_MARK_MS - 1);
    });
    rerender(
      <BlockedMark
        blocked={{ key: 4, testid: "zone-you-units-2", x: 0, y: 0, size: 30 }}
        onDone={onDone}
      />,
    );
    act(() => {
      vi.advanceTimersByTime(1);
    });
    // The first mark's hold ended with it: only the new one's runs.
    expect(onDone).not.toHaveBeenCalled();
    act(() => {
      vi.advanceTimersByTime(BLOCKED_MARK_MS - 2);
    });
    unmount();
    act(() => {
      vi.advanceTimersByTime(BLOCKED_MARK_MS);
    });
    expect(onDone).not.toHaveBeenCalled();
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
