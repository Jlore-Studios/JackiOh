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
  FULLSCREEN_BLEED_PCT,
  FULLSCREEN_COVER_DIGITS,
  FULLSCREEN_COVER_SCALE_MAX,
  FULLSCREEN_DIORAMA_PERSPECTIVE_VMAX,
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
import {
  FullScreenArt,
  FullScreenArtHost,
  fullscreenCoverScale,
  fullscreenCovers,
  fullscreenDioramaPerspectivePx,
  sliceShiftPx,
} from "./FullScreenArt.tsx";
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

  it("R3350000 a tap that pauses the view holds a diorama's loops still, and the next sets them going again", () => {
    const diorama = dioramaOf("core-100");
    if (diorama === null) throw new Error("Ceaseless Void has no diorama");
    render(<FullScreenArt defId="core-100" onClose={vi.fn()} />);
    const dialog = screen.getByTestId("fullscreen-art");
    // Whether every slice draws its moving picture (`moving`) or its still one.
    const draws = (moving: boolean): boolean =>
      [...dialog.querySelectorAll<HTMLElement>(".fullscreen-art-plane[data-slice]")].every(
        (shown, index) => shown.style.backgroundImage === dioramaSliceBackground("core-100", false, index, moving),
      );
    // Its loops run inside its slices' own pictures, so a slice with one draws a picture of its own while the view moves.
    expect(diorama.slices.some((_, index) => dioramaSliceBackground("core-100", false, index, true) !== dioramaSliceBackground("core-100", false, index, false))).toBe(true);
    expect(draws(true)).toBe(true);
    expect(draws(false)).toBe(false);
    fireEvent.click(dialog, { detail: 1 });
    expect(dialog).toHaveAttribute("data-paused", "true");
    expect(draws(false)).toBe(true);
    act(() => {
      vi.advanceTimersByTime(500);
    });
    fireEvent.click(dialog, { detail: 1 });
    expect(draws(true)).toBe(true);
  });

  it("R3350000 the full-screen view writes a diorama's perspective and cover scale as it turns, and a card with none writes neither", () => {
    const { innerWidth: width, innerHeight: height } = window;
    render(<FullScreenArt defId="core-100" onClose={vi.fn()} />);
    const dialog = screen.getByTestId("fullscreen-art");
    expect(dialog).toHaveAttribute("data-diorama", "true");
    // Level as it opens, at its pictures' own size, under the screen's own perspective.
    expect(dialog.style.getPropertyValue("--fs-perspective")).toBe(`${String(fullscreenDioramaPerspectivePx(width, height))}px`);
    expect(dialog.style.getPropertyValue("--fs-cover")).toBe("1");
    // The pointer at the top right turns it the whole way about both axes, and it grows as far as the screen needs.
    fireEvent.pointerMove(window, { clientX: width, clientY: 0 });
    const cover = fullscreenCoverScale(width, height, DIORAMA_TURN_MAX_DEG, DIORAMA_TURN_MAX_DEG);
    expect(cover).toBeGreaterThan(1);
    expect(dialog.style.getPropertyValue("--fs-cover")).toBe(String(cover));
    expect(dialog.style.getPropertyValue("--fs-perspective")).toBe(`${String(fullscreenDioramaPerspectivePx(width, height))}px`);
    // Back in the middle, level and its own size again.
    fireEvent.pointerMove(window, { clientX: width / 2, clientY: height / 2 });
    expect(dialog.style.getPropertyValue("--fs-cover")).toBe("1");
    cleanup();
    // A card with no diorama keeps the view's own perspective and size.
    render(<FullScreenArt defId="core-052" onClose={vi.fn()} />);
    const plain = screen.getByTestId("fullscreen-art");
    fireEvent.pointerMove(window, { clientX: width, clientY: 0 });
    expect(plain).not.toHaveAttribute("data-diorama");
    expect(plain.style.getPropertyValue("--fs-perspective")).toBe("");
    expect(plain.style.getPropertyValue("--fs-cover")).toBe("");
  });
});

describe("R3350000 a turned diorama keeps the full-screen view covered", () => {
  /** The component specs' desktop and phone (1280x720, 390x844), a phone on its side, and two wide screens. */
  const SCREENS = [
    [1280, 720],
    [390, 844],
    [844, 390],
    [1920, 1080],
    [2560, 1080],
  ] as const;
  const MAX = DIORAMA_TURN_MAX_DEG;
  /** One step of the cover scale as it is written. */
  const STEP = 10 ** -FULLSCREEN_COVER_DIGITS;

  it("R3350000 a level view keeps its pictures' own size, and a turned one grows as far as it must to cover the screen, never past FULLSCREEN_COVER_SCALE_MAX", () => {
    for (const [width, height] of SCREENS) {
      const at = `${String(width)}x${String(height)}`;
      const perspective = fullscreenDioramaPerspectivePx(width, height);
      expect(perspective, at).toBe(FULLSCREEN_DIORAMA_PERSPECTIVE_VMAX * Math.max(width, height));
      expect(fullscreenCovers(width, height, 0, 0, perspective, 1), at).toBe(true);
      expect(fullscreenCoverScale(width, height, 0, 0), at).toBe(1);
      for (const [rx, ry] of [
        [MAX, MAX],
        [-MAX, MAX],
        [MAX, 0],
        [0, -MAX],
      ] as const) {
        const pose = `${at} turned ${String(rx)}° and ${String(ry)}°`;
        // At its own size the turn would show the black past its pictures.
        expect(fullscreenCovers(width, height, rx, ry, perspective, 1), pose).toBe(false);
        const scale = fullscreenCoverScale(width, height, rx, ry);
        expect(scale, pose).toBeGreaterThan(1);
        expect(scale, pose).toBeLessThanOrEqual(FULLSCREEN_COVER_SCALE_MAX);
        // It covers at the scale it gives, and a step below it does not.
        expect(fullscreenCovers(width, height, rx, ry, perspective, scale), pose).toBe(true);
        expect(fullscreenCovers(width, height, rx, ry, perspective, scale - STEP), pose).toBe(false);
        // Either sign of a turn is the same.
        expect(fullscreenCoverScale(width, height, -rx, -ry), pose).toBe(scale);
      }
      // The corner pose needs more than either axis alone.
      expect(fullscreenCoverScale(width, height, MAX, MAX), at).toBeGreaterThan(Math.max(fullscreenCoverScale(width, height, MAX, 0), fullscreenCoverScale(width, height, 0, MAX)));
    }
  });

  it("R3350000 the farthest a slice shifts counts against the bleed, and a corner behind the eye covers nothing", () => {
    // A screen whose bleed (FULLSCREEN_BLEED_PCT a side) is less than the farthest a slice shifts grows even level.
    const shift = Math.max(Math.abs(FULLSCREEN_SLICE_SHIFT_FAR_PX), Math.abs(FULLSCREEN_SLICE_SHIFT_NEAR_PX));
    const small = Math.floor((shift * 100) / FULLSCREEN_BLEED_PCT) - 10;
    expect(fullscreenCovers(small, small, 0, 0, fullscreenDioramaPerspectivePx(small, small), 1)).toBe(false);
    expect(fullscreenCoverScale(small, small, 0, 0)).toBeGreaterThan(1);
    // Under a perspective shorter than the planes reach toward the eye, a turned corner passes behind it.
    expect(fullscreenCovers(1280, 720, MAX, MAX, 10, 1)).toBe(false);
    expect(fullscreenCovers(1280, 720, MAX, MAX, fullscreenDioramaPerspectivePx(1280, 720), FULLSCREEN_COVER_SCALE_MAX)).toBe(true);
  });

  it("R3350000 the stylesheet turns a diorama as fullscreenCovers models it: its bleed, its perspective and its transform", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const css = readFileSync(join(here, "fullscreen.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    // The planes run FULLSCREEN_BLEED_PCT past the viewport on each side.
    const planes = /\.fullscreen-art-planes\s*\{([^}]*)\}/.exec(css)?.[1] ?? "";
    expect(planes).toMatch(new RegExp(`(^|;|\\s)inset:\\s*-${String(FULLSCREEN_BLEED_PCT)}%;`));
    // A diorama's view takes FullScreenArt's perspective, its fallback the same FULLSCREEN_DIORAMA_PERSPECTIVE_VMAX.
    expect(css).toMatch(
      new RegExp(`\\.fullscreen-art\\[data-diorama\\]\\s*\\{\\s*perspective: var\\(--fs-perspective, ${String(FULLSCREEN_DIORAMA_PERSPECTIVE_VMAX * 100)}vmax\\);\\s*\\}`),
    );
    // Scaled first, then turned about y, then about x: the order fullscreenCovers turns its corners in.
    expect(css).toMatch(
      /\.fullscreen-art-planes\[data-diorama\]\s*\{\s*transform: rotateX\(var\(--cf-tilt-rx, 0deg\)\) rotateY\(var\(--cf-tilt-ry, 0deg\)\) scale\(var\(--fs-cover, 1\)\);\s*\}/,
    );
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
