// #3119, R3119000: the 404 panel's play. The path picks the lost card and seeds the glitch; the path's
// words stay the element's text; a face-down way out turns over on its first tap and follows on the
// next; the lost card speaks through the audio engine; under Reduce motion all of it lies still.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { flavourFor } from "../cards/flavour.ts";
import { GLITCH_GLYPHS } from "../cards/glitch.ts";

const reduced = vi.hoisted(() => ({ value: false }));
const engine = vi.hoisted(() => ({ unlock: vi.fn(), playVoice: vi.fn(() => true) }));
const navigateTo = vi.hoisted(() => vi.fn());

vi.mock("../settings/motion.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  reducedMotionNow: () => reduced.value,
}));
vi.mock("../audio/engine.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  getAudioEngine: () => engine,
}));
vi.mock("../net/navigate.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  navigate: navigateTo,
}));

const { GlitchPath, NotFoundPlay, notFoundTestid } = await import("./NotFoundPlay.tsx");
const { GLITCH_SETTLE_STEP_MS, LOST_CARDS, WAYS_OUT, glitchedPath, lostCardFor, lostCardLine } = await import("./lost.ts");

beforeEach(() => {
  reduced.value = false;
  engine.unlock.mockClear();
  engine.playVoice.mockClear();
  navigateTo.mockClear();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("R3119000 the 404 panel's play", () => {
  it("R3119000 every lost card is in the catalog, with a voice line and a flavour line", () => {
    for (const id of LOST_CARDS) {
      expect(CATALOG[id], id).toBeDefined();
      expect(lostCardLine(id), id).not.toBeNull();
      expect(flavourFor(id)?.flavour, id).toBeTruthy();
    }
  });

  it("R3119000 the path picks the lost card: the same path always the same card, and different paths reach different cards", () => {
    expect(lostCardFor("/nope")).toBe(lostCardFor("/nope"));
    const picked = new Set(["/nope", "/decks/extra", "/login/again", "/a", "/b", "/c", "/d", "/e", "/f", "/g"].map(lostCardFor));
    expect(picked.size).toBeGreaterThan(1);
    for (const id of picked) expect(LOST_CARDS).toContain(id);
  });

  it("R3119000 the glitched path keeps its settled characters and turns every other into a Glitch glyph, the same each time", () => {
    const path = "/no such/menu";
    const glyphs = new Set([...GLITCH_GLYPHS]);
    const none = [...glitchedPath(path, 0)];
    expect(none).toHaveLength([...path].length);
    for (const char of none) expect(glyphs.has(char), char).toBe(true);
    expect(glitchedPath(path, 0)).toBe(glitchedPath(path, 0));
    expect(glitchedPath(path, 3).slice(0, 3)).toBe("/no");
    expect(glitchedPath(path, [...path].length)).toBe(path);
  });

  it("R3119000 the path element's text is the path while it glitches, and the glyphs settle to nothing", () => {
    vi.useFakeTimers();
    render(<GlitchPath path="/nope" />);
    const code = screen.getByTestId(notFoundTestid.path);
    expect(code.textContent).toBe("/nope");
    expect(code.dataset.settling).toBe("true");
    expect(code.dataset.glitch).toHaveLength("/nope".length);

    act(() => {
      vi.advanceTimersByTime(GLITCH_SETTLE_STEP_MS * "/nope".length);
    });
    expect(code.textContent).toBe("/nope");
    expect(code.dataset.settling).toBe("false");
    expect(code.dataset.glitch).toBeUndefined();

    // A pointer over the settled path glitches it again.
    fireEvent.pointerEnter(code);
    expect(code.dataset.settling).toBe("true");
  });

  it("R3119000 a face-down way out turns over on a tap and follows on the next", () => {
    render(<NotFoundPlay path="/nope" />);
    for (const way of WAYS_OUT) {
      const card = screen.getByTestId(notFoundTestid.way(way.key));
      expect(card.getAttribute("href")).toBe(way.to);
      expect(card.dataset.face).toBe("down");

      fireEvent.click(card);
      expect(card.dataset.face).toBe("up");
      expect(navigateTo).not.toHaveBeenCalled();

      fireEvent.click(card);
      expect(navigateTo).toHaveBeenCalledWith(way.to);
      navigateTo.mockClear();
    }
  });

  it("R3119000 a mouse over a way out turns it over, so one click follows it", () => {
    render(<NotFoundPlay path="/nope" />);
    const card = screen.getByTestId(notFoundTestid.way("almanac"));
    fireEvent.pointerEnter(card, { pointerType: "mouse" });
    expect(card.dataset.face).toBe("up");
    fireEvent.click(card);
    expect(navigateTo).toHaveBeenCalledWith(WAYS_OUT[2].to);
  });

  it("R3119000 a tap on the lost card says its line through the audio engine", () => {
    render(<NotFoundPlay path="/nope" />);
    const defId = lostCardFor("/nope");
    const lost = screen.getByTestId(notFoundTestid.lost);
    expect(lost.dataset.defId).toBe(defId);
    expect(lost.textContent).toContain(flavourFor(defId)?.flavour ?? "");

    fireEvent.click(screen.getByRole("button", { name: /^Hear / }));
    expect(engine.unlock).toHaveBeenCalledTimes(1);
    expect(engine.playVoice).toHaveBeenCalledWith(defId, lostCardLine(defId), 0, expect.any(Number));
  });

  it("R3119000 under Reduce motion the ways out lie face up and the path is settled", () => {
    reduced.value = true;
    render(
      <>
        <GlitchPath path="/nope" />
        <NotFoundPlay path="/nope" />
      </>,
    );
    expect(screen.getByTestId(notFoundTestid.path).dataset.settling).toBe("false");
    for (const way of WAYS_OUT) expect(screen.getByTestId(notFoundTestid.way(way.key)).dataset.face).toBe("up");
    fireEvent.click(screen.getByTestId(notFoundTestid.way("play")));
    expect(navigateTo).toHaveBeenCalledWith(WAYS_OUT[0].to);
  });
});
