// R3345000: Mythic full-screen art view, trigger, tilt and depth planes tests.
// Presentation only (CLAUDE.md rule 7).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import Card from "../game/Card.tsx";
import { WearerProvider } from "../game/wearer.ts";
import { card as cardView, unit as unitView } from "../test/fixtures.ts";
import { LONG_PRESS_MS } from "./inspect/constants.ts";
import { ART_PLANES } from "./art/index.ts";
import { dioramaOf, dioramaSliceBackground } from "./art/dioramas/index.ts";
import {
  CARD_TILT_MAX_DEG,
  DIORAMA_TURN_MAX_DEG,
  FULLSCREEN_HOLD_MS,
  FULLSCREEN_SLICE_SHIFT_FAR_PX,
  FULLSCREEN_SLICE_SHIFT_NEAR_PX,
  FULLSCREEN_SLOP_PX,
} from "./constants.ts";
import {
  canOpenFullScreen,
  closeFullScreenArt,
  useFullScreenTrigger,
} from "./fullscreen.ts";
import { FullScreenArt, FullScreenArtHost, sliceShiftPx } from "./FullScreenArt.tsx";
import { InspectSheet } from "./inspect/InspectSheet.tsx";
import { faceModel } from "./model.ts";
import { FullArtShown } from "./fullArt.ts";
import { VERSION_ALT_FULL_ART_ID } from "./versions.ts";
import { isHeld, resetTiltForTests, useTiltHold } from "./tilt.ts";
import { setReducedMotion } from "../test/setup.ts";

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
    // R3350000: a Mythic with a diorama draws its slices instead (below); a card with none draws the four planes.
    render(<FullScreenArt defId="core-052" name="Silly Silas" onClose={onClose} />);

    const dialog = screen.getByTestId("fullscreen-art");
    expect(dialog).toHaveAttribute("role", "dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
    expect(dialog).toHaveAttribute("aria-label", "Silly Silas");

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

  it("R3345000 opened by a tap from the inspect sheet, a tap pauses and resumes it and no release closes it", () => {
    const sheet = (defId: string) => (
      <InspectSheet face={faceModel({ defId, def: CATALOG[defId], radiant: false })} onClose={() => undefined} />
    );
    // A Common that prints no full art has no way in; a Mythic has the button.
    const common = render(sheet("core-003"));
    expect(screen.queryByTestId("fullscreen-art-open")).toBeNull();
    common.unmount();
    render(
      <>
        {sheet("core-100")}
        <FullScreenArtHost />
      </>,
    );
    fireEvent.click(screen.getByTestId("fullscreen-art-open"));
    const dialog = screen.getByTestId("fullscreen-art");
    // No pointer is held, so a release of any pointer leaves it open.
    fireEvent.pointerUp(window, { pointerId: 1 });
    expect(dialog).toBeInTheDocument();
    fireEvent.click(dialog, { detail: 1 });
    expect(dialog).toHaveAttribute("data-paused", "true");
    act(() => {
      vi.advanceTimersByTime(500);
    });
    fireEvent.click(dialog, { detail: 1 });
    expect(dialog).not.toHaveAttribute("data-paused");
    // A second tap closes it.
    fireEvent.click(dialog, { detail: 2 });
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();
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

  it("R3350000 the full-screen view draws a diorama's slices by depth and turns to DIORAMA_TURN_MAX_DEG", () => {
    const diorama = dioramaOf("core-100");
    if (diorama === null) throw new Error("Ceaseless Void has no diorama");
    render(<FullScreenArt defId="core-100" radiant onClose={vi.fn()} />);
    const dialog = screen.getByTestId("fullscreen-art");
    expect(dialog.querySelector(".fullscreen-art-planes")?.getAttribute("data-diorama")).toBe("true");
    const slices = [...dialog.querySelectorAll<HTMLElement>(".fullscreen-art-plane")];
    expect(slices.map((slice) => slice.getAttribute("data-slice"))).toEqual(diorama.slices.map((_, index) => String(index)));
    expect(slices.map((slice) => slice.getAttribute("data-plane"))).toEqual(diorama.slices.map((slice) => slice.plane));
    diorama.slices.forEach((slice, index) => {
      const shown = slices[index];
      // Its own slice's picture, moving by its loop (the viewer's motion settings allow it here), on the face asked for.
      expect(shown?.style.backgroundImage).toBe(dioramaSliceBackground("core-100", true, index, true));
      // The farthest shifts against the tilt, the nearest with it, by depth.
      expect(shown?.style.getPropertyValue("--fs-slice-shift")).toBe(`${sliceShiftPx(slice.depth)}px`);
      expect(shown?.getAttribute("data-bend") ?? undefined).toBe(slice.bend?.axis);
      expect(shown?.style.getPropertyValue("--cf-slice-bend")).toBe(slice.bend === undefined ? "" : `${slice.bend.deg}deg`);
    });
    expect(sliceShiftPx(0)).toBe(FULLSCREEN_SLICE_SHIFT_FAR_PX);
    expect(sliceShiftPx(1)).toBe(FULLSCREEN_SLICE_SHIFT_NEAR_PX);
    // The pointer at the right edge turns it the diorama's whole way, at the top the whole way back.
    fireEvent.pointerMove(window, { clientX: window.innerWidth, clientY: 0 });
    expect(dialog.style.getPropertyValue("--cf-tilt-ry")).toBe(`${DIORAMA_TURN_MAX_DEG}deg`);
    expect(dialog.style.getPropertyValue("--cf-tilt-rx")).toBe(`${DIORAMA_TURN_MAX_DEG}deg`);
    cleanup();
    // A card with no diorama turns as far as a face does.
    render(<FullScreenArt defId="core-052" onClose={vi.fn()} />);
    fireEvent.pointerMove(window, { clientX: window.innerWidth, clientY: window.innerHeight / 2 });
    expect(screen.getByTestId("fullscreen-art").style.getPropertyValue("--cf-tilt-ry")).toBe(`${CARD_TILT_MAX_DEG}deg`);
  });

  it("R3350000 under reduced motion the full-screen view draws a diorama's slices still", () => {
    setReducedMotion(true);
    try {
      render(<FullScreenArt defId="core-100" onClose={vi.fn()} />);
      const slices = [...screen.getByTestId("fullscreen-art").querySelectorAll<HTMLElement>(".fullscreen-art-plane[data-slice]")];
      expect(slices.length).toBe(dioramaOf("core-100")?.slices.length);
      slices.forEach((slice, index) => expect(slice.style.backgroundImage).toBe(dioramaSliceBackground("core-100", false, index, false)));
    } finally {
      setReducedMotion(false);
    }
  });
});

describe("R3345000 the full-screen stylesheet's planes", () => {
  it("R3350000 a diorama's slice shifts by its depth after the planes' steps, and a bent one turns about its axis", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const css = readFileSync(join(here, "fullscreen.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    const slice = /\.fullscreen-art-plane\[data-slice\]\s*\{([^}]*)\}/.exec(css);
    expect(slice?.[1]).toMatch(/translate:\s*calc\(var\(--cf-tilt-x, 0\) \* var\(--fs-slice-shift, 0px\)\)\s+calc\(var\(--cf-tilt-y, 0\) \* var\(--fs-slice-shift, 0px\)\)/);
    // It follows every plane's rule, so its shift is the one a slice takes.
    expect(css.indexOf(".fullscreen-art-plane[data-slice]")).toBeGreaterThan(css.lastIndexOf('.fullscreen-art-plane[data-plane="nearest"]'));
    expect(css).toMatch(/\.fullscreen-art-planes\[data-diorama\] \.fullscreen-art-plane\[data-bend="y"\]\s*\{\s*rotate: y var\(--cf-slice-bend, 0deg\);\s*\}/);
    expect(css).toMatch(/\.fullscreen-art-planes\[data-diorama\] \.fullscreen-art-plane\[data-bend="x"\]\s*\{\s*rotate: x var\(--cf-slice-bend, 0deg\);\s*\}/);
  });

  it("R3345000 each plane's tilt parallax is a translate, so its dolly keyframes' transform does not replace it", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const css = readFileSync(join(here, "fullscreen.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    for (const plane of ART_PLANES) {
      const rule = new RegExp(`\\.fullscreen-art-plane\\[data-plane="${plane}"\\]\\s*\\{([^}]*)\\}`).exec(css)?.[1] ?? "";
      // The pointer's or device's pose shifts the plane by its depth, in the standalone property.
      expect(rule).toMatch(/translate:\s*calc\(var\(--cf-tilt-x, 0\) \* -?\d+px\)\s+calc\(var\(--cf-tilt-y, 0\) \* -?\d+px\)/);
      // The dolly animates `transform`, so the rule itself must set none for the two to compose.
      expect(rule).toContain(`animation: fullscreen-dolly-${plane} `);
      expect(rule).not.toMatch(/(^|[^-])transform:/);
      const frames = new RegExp(`@keyframes fullscreen-dolly-${plane}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(css)?.[1] ?? "";
      expect(frames).toContain("transform:");
      expect(frames).not.toContain("translate:");
    }
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

  it("R3345000 a Common wearing a full-art level opens by a hold, its own copy only", () => {
    // core-003 is no Mythic and prints no full art of its own: only the level its copy wears (here Alt full art, a pick shown as one's own) makes it open.
    function Wearing({ worn }: { worn: boolean }) {
      const { handlers } = useFullScreenTrigger({ defId: "core-003", rarity: "Common" }, { worn });
      return <button type="button" data-testid="card-trigger" {...handlers} />;
    }
    const hold = (): void => {
      fireEvent.pointerDown(screen.getByTestId("card-trigger"), { pointerId: 1, clientX: 100, clientY: 100 });
      act(() => {
        vi.advanceTimersByTime(FULLSCREEN_HOLD_MS + 10);
      });
    };
    const { unmount } = render(
      <>
        <FullArtShown defId="core-003" pick={VERSION_ALT_FULL_ART_ID}>
          <Wearing worn={true} />
        </FullArtShown>
        <FullScreenArtHost />
      </>,
    );
    hold();
    expect(screen.getByTestId("fullscreen-art")).toBeInTheDocument();
    unmount();
    closeFullScreenArt();

    render(
      <>
        <Wearing worn={false} />
        <FullScreenArtHost />
      </>,
    );
    hold();
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();
  });

  it("R3345000 a Common the Full-art cards setting prints full art on, with no pick, does not open", () => {
    // core-008 is a Common that prints full art by the setting alone: the default look is not a level worn, so a hold leaves it to the long press.
    function Plain() {
      const { handlers } = useFullScreenTrigger({ defId: "core-008", rarity: "Common" }, { worn: true });
      return <button type="button" data-testid="card-trigger" {...handlers} />;
    }
    render(
      <WearerProvider seat="p1" viewer="p1">
        <Plain />
        <FullScreenArtHost />
      </WearerProvider>,
    );
    fireEvent.pointerDown(screen.getByTestId("card-trigger"), { pointerId: 1, clientX: 100, clientY: 100 });
    act(() => {
      vi.advanceTimersByTime(FULLSCREEN_HOLD_MS + 10);
    });
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();
  });

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
        <TriggerCard card={{ defId: "core-002", rarity: "Common" }} />
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

describe("R3345000 a board card's full-screen hold follows the animation queue", () => {
  const MYTHIC = "core-100";

  function QueueHolder({ held }: { held: boolean }) {
    useTiltHold(held);
    return null;
  }

  /** A finger down for longer than the long press, and still down: lifting it closes the view (R3345000). */
  function pressAndHold(element: HTMLElement): void {
    fireEvent.pointerDown(element, { pointerId: 1, pointerType: "touch", button: 0, clientX: 10, clientY: 10 });
    act(() => {
      vi.advanceTimersByTime(LONG_PRESS_MS + 50);
    });
  }

  function lift(element: HTMLElement): void {
    fireEvent.pointerUp(element, { pointerId: 1, pointerType: "touch", button: 0, clientX: 10, clientY: 10 });
  }

  /** A Zone's mount of a board unit: `Card` with the unit as its card, the way Zone.tsx renders it. */
  function Board({ owner, held }: { owner: "p1" | "p2"; held: boolean }) {
    const unit = unitView(owner, { instanceId: "mythic-unit", defId: MYTHIC });
    return (
      <WearerProvider seat="p1" viewer="p1">
        <Card testId="card-mythic-unit" card={unit} unit={unit} onField />
        <QueueHolder held={held} />
        <FullScreenArtHost />
      </WearerProvider>
    );
  }

  it("R3345000 your own board Mythic opens it on a hold while the queue is idle", () => {
    render(<Board owner="p1" held={false} />);
    pressAndHold(screen.getByTestId("card-mythic-unit"));
    expect(screen.getByTestId("fullscreen-art")).toBeInTheDocument();
  });

  it("R3345000 your own board Mythic does not open it while the queue drains,", () => {
    render(<Board owner="p1" held={true} />);
    pressAndHold(screen.getByTestId("card-mythic-unit"));
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();
  });

  it("R3345000 the guard follows the queue: a drain that starts or ends after the card rendered is honoured", () => {
    const { rerender } = render(<Board owner="p1" held={false} />);
    // The drain starts after the card rendered idle: the hold must not open.
    rerender(<Board owner="p1" held={true} />);
    pressAndHold(screen.getByTestId("card-mythic-unit"));
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();
    expect(isHeld()).toBe(true);

    // The drain ends: the very next hold opens, with no unrelated re-render in between.
    lift(screen.getByTestId("card-mythic-unit"));
    rerender(<Board owner="p1" held={false} />);
    expect(isHeld()).toBe(false);
    pressAndHold(screen.getByTestId("card-mythic-unit"));
    expect(screen.getByTestId("fullscreen-art")).toBeInTheDocument();
  });

  it("R3345000 a drain that starts while the finger is already down stops the hold from opening", () => {
    const { rerender } = render(<Board owner="p1" held={false} />);
    const unit = screen.getByTestId("card-mythic-unit");
    fireEvent.pointerDown(unit, { pointerId: 1, pointerType: "touch", button: 0, clientX: 10, clientY: 10 });
    rerender(<Board owner="p1" held={true} />);
    act(() => {
      vi.advanceTimersByTime(LONG_PRESS_MS + 50);
    });
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();
  });

  it("R3345000 the opponent's board Mythic never opens it", () => {
    render(<Board owner="p2" held={false} />);
    pressAndHold(screen.getByTestId("card-mythic-unit"));
    expect(screen.queryByTestId("fullscreen-art")).toBeNull();
  });

  it("R3345000 a Mythic in your own hand opens it, queue draining or not", () => {
    const mythic = cardView({ instanceId: "mythic-hand", defId: MYTHIC });
    render(
      <WearerProvider seat="p1" viewer="p1">
        <Card testId="hand-card-mythic-hand" card={mythic} inOwnHand />
        <QueueHolder held={true} />
        <FullScreenArtHost />
      </WearerProvider>,
    );
    pressAndHold(screen.getByTestId("hand-card-mythic-hand"));
    expect(screen.getByTestId("fullscreen-art")).toBeInTheDocument();
  });
});
