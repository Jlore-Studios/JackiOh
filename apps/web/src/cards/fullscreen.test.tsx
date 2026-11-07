// R3345000: Mythic full-screen art view, trigger, tilt and depth planes tests.
// Presentation only (CLAUDE.md rule 7).

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ART_PLANES } from "./art/index.ts";
import { FULLSCREEN_HOLD_MS, FULLSCREEN_SLOP_PX } from "./constants.ts";
import {
  canOpenFullScreen,
  closeFullScreenArt,
  useFullScreenTrigger,
} from "./fullscreen.ts";
import { FullScreenArt, FullScreenArtHost } from "./FullScreenArt.tsx";
import { isHeld, resetTiltForTests, useTiltHold } from "./tilt.ts";

beforeEach(() => {
  vi.useFakeTimers();
  resetTiltForTests();
  closeFullScreenArt();
});

afterEach(() => {
  cleanup();
  closeFullScreenArt();
  resetTiltForTests();
  vi.useRealTimers();
});

describe("R3345000 canOpenFullScreen", () => {
  it("R3345000 allows any Mythic, printed Mythic tokens, or cards wearing full-art level/version", () => {
    expect(canOpenFullScreen({ rarity: "Mythic" })).toBe(true);
    expect(canOpenFullScreen({ printedRarity: "Mythic", rarity: "Token" })).toBe(true);
    expect(canOpenFullScreen({ rarity: "Common", level: "version-full-art" })).toBe(true);
    expect(canOpenFullScreen({ rarity: "Rare", version: "version-full-art" })).toBe(true);

    expect(canOpenFullScreen({ rarity: "Common" })).toBe(false);
    expect(canOpenFullScreen({ rarity: "Rare" })).toBe(false);
    expect(canOpenFullScreen({ rarity: "Legendary" })).toBe(false);
  });
});

describe("R3345000 FullScreenArt rendering and depth planes", () => {
  it("R3345000 renders 4 depth planes, close button, and aria modal dialog", () => {
    const onClose = vi.fn();
    render(<FullScreenArt defId="core-100" name="The Ceaseless Void" onClose={onClose} />);

    const dialog = screen.getByTestId("fullscreen-art");
    expect(dialog).toHaveAttribute("role", "dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveAttribute("aria-label", "The Ceaseless Void");

    const planes = dialog.querySelectorAll(".fullscreen-art-plane");
    expect(planes.length).toBe(4);
    const planeNames = Array.from(planes).map((p) => p.getAttribute("data-plane"));
    expect(planeNames).toEqual(ART_PLANES);

    for (const plane of planes) {
      expect((plane as HTMLElement).style.backgroundImage).toMatch(/^url\(/);
    }

    const closeBtn = screen.getByTestId("fullscreen-art-close");
    fireEvent.click(closeBtn);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R3345000 tap pauses and resumes it, and second tap closes it", () => {
    const onClose = vi.fn();
    render(<FullScreenArt defId="core-100" onClose={onClose} />);
    const dialog = screen.getByTestId("fullscreen-art");

    expect(dialog).not.toHaveAttribute("data-paused");

    // First tap: pauses
    fireEvent.click(dialog, { detail: 1 });
    expect(dialog).toHaveAttribute("data-paused", "true");

    // Advance time past double tap threshold (e.g. 500ms)
    act(() => {
      vi.advanceTimersByTime(500);
    });

    // Second tap after delay: resumes
    fireEvent.click(dialog, { detail: 1 });
    expect(dialog).not.toHaveAttribute("data-paused");
    expect(onClose).not.toHaveBeenCalled();

    // Double tap / quick second tap closes
    fireEvent.click(dialog, { detail: 2 });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R3345000 releasing held pointer closes the view", () => {
    const onClose = vi.fn();
    render(<FullScreenArt defId="core-100" heldPointerId={42} onClose={onClose} />);

    // Unrelated pointer does not close
    fireEvent.pointerUp(window, { pointerId: 99 });
    expect(onClose).not.toHaveBeenCalled();

    // Releasing held pointer closes
    fireEvent.pointerUp(window, { pointerId: 42 });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R3345000 Escape key closes the view", () => {
    const onClose = vi.fn();
    render(<FullScreenArt defId="core-100" onClose={onClose} />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("R3345000 tilt listener updates CSS variables on pointer move", () => {
    render(<FullScreenArt defId="core-100" onClose={vi.fn()} />);
    const dialog = screen.getByTestId("fullscreen-art");

    fireEvent.pointerMove(window, { clientX: window.innerWidth * 0.75, clientY: window.innerHeight * 0.25 });
    expect(dialog.style.getPropertyValue("--cf-tilt-x")).not.toBe("");
    expect(dialog.style.getPropertyValue("--cf-tilt-y")).not.toBe("");
    expect(dialog.style.getPropertyValue("--cf-tilt-rx")).toContain("deg");
    expect(dialog.style.getPropertyValue("--cf-tilt-ry")).toContain("deg");
    expect(dialog.style.getPropertyValue("--cf-glare-x")).toContain("%");
    expect(dialog.style.getPropertyValue("--cf-glare-y")).toContain("%");
  });
});

describe("R3345000 useFullScreenTrigger hook", () => {
  function TriggerCard({
    card,
    enabled = true,
  }: {
    card: { defId: string; rarity?: string; name?: string };
    enabled?: boolean;
  }) {
    const { handlers } = useFullScreenTrigger(card, { enabled });
    return (
      <button type="button" data-testid="card-trigger" {...handlers}>
        Card
      </button>
    );
  }

  it("R3345000 holding past FULLSCREEN_HOLD_MS opens full-screen view", () => {
    render(
      <>
        <TriggerCard card={{ defId: "core-100", rarity: "Mythic", name: "The Ceaseless Void" }} />
        <FullScreenArtHost />
      </>,
    );

    const trigger = screen.getByTestId("card-trigger");
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();

    fireEvent.pointerDown(trigger, { pointerId: 1, clientX: 100, clientY: 100 });
    act(() => {
      vi.advanceTimersByTime(FULLSCREEN_HOLD_MS - 10);
    });
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();

    act(() => {
      vi.advanceTimersByTime(20);
    });
    expect(screen.getByTestId("fullscreen-art")).toBeInTheDocument();
  });

  it("R3345000 moving past FULLSCREEN_SLOP_PX cancels the hold", () => {
    render(
      <>
        <TriggerCard card={{ defId: "core-100", rarity: "Mythic" }} />
        <FullScreenArtHost />
      </>,
    );

    const trigger = screen.getByTestId("card-trigger");
    fireEvent.pointerDown(trigger, { pointerId: 1, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(trigger, { pointerId: 1, clientX: 100 + FULLSCREEN_SLOP_PX + 5, clientY: 100 });
    act(() => {
      vi.advanceTimersByTime(FULLSCREEN_HOLD_MS + 50);
    });
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();
  });

  it("R3345000 non-mythic card has no handlers and does not open", () => {
    render(
      <>
        <TriggerCard card={{ defId: "core-001", rarity: "Common" }} />
        <FullScreenArtHost />
      </>,
    );

    const trigger = screen.getByTestId("card-trigger");
    fireEvent.pointerDown(trigger, { pointerId: 1, clientX: 100, clientY: 100 });
    act(() => {
      vi.advanceTimersByTime(FULLSCREEN_HOLD_MS + 50);
    });
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();
  });
});

describe("R3345000 animation queue hold guard", () => {
  function QueueHolder({ held }: { held: boolean }) {
    useTiltHold(held);
    return null;
  }

  it("R3345000 isHeld reflects queue hold status", () => {
    expect(isHeld()).toBe(false);
    const { rerender } = render(<QueueHolder held={true} />);
    expect(isHeld()).toBe(true);
    rerender(<QueueHolder held={false} />);
    expect(isHeld()).toBe(false);
  });
});
