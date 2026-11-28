// Stinkyscape+ E5 (issue #3548, R3548000–R3548004): the Woozy stein, the Woozy blur and the
// infinite mana tray, drawn from the view and nothing else (CLAUDE.md rule 7).

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { renderBoard, withCatalog } from "../test/board.tsx";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import Hero from "./Hero.tsx";

afterEach(() => {
  cleanup();
});

describe("R3548000 the Woozy stein", () => {
  it("R3548000 the stein shows the count on either seat above 0, and nothing at none", () => {
    const view = baseView({
      you: emptySide("p1", { tipsy: 4 }),
      opponent: emptySide("p2", { tipsy: 2 }),
    });
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={view} side={side} />));
      // #4251: the stein rides the tray as a labelled chip, outside the overflow list, not on the
      // portrait under the health badge.
      const tray = document.querySelector(`[data-testid="hero-counters-${side}"]`);
      expect(tray).not.toBeNull();
      const badge = tray?.querySelector(".hero-woozy");
      expect(badge?.getAttribute("data-woozy")).toBe(side === "you" ? "4" : "2");
      expect(badge?.getAttribute("title")).toBe("Woozy counter");
      expect(badge?.textContent).toBe(side === "you" ? "Woozy 4" : "Woozy 2");
      expect(badge?.getAttribute("aria-label")).toBe(side === "you" ? "Woozy 4" : "Woozy 2");
      expect(badge?.querySelector("img.cf-icon")).not.toBeNull();
      expect(badge?.closest('[data-testid^="tray-list-"]')).toBeNull();
      unmount();
    }
    render(withCatalog(<Hero view={baseView()} side="you" />));
    expect(document.querySelector(".hero-woozy")).toBeNull();
  });
});

describe("R3548001 the Woozy blur", () => {
  it("R3548001 a cost blur marks the root", () => {
    const board = renderBoard(
      baseView({
        you: emptySide("p1", { units: [unit("p1", { tipsyBlur: { kind: "cost" } }), null, null, null, null] }),
      }),
    );
    expect(board.querySelector('[data-woozy-blur="cost"]')).not.toBeNull();
  });

  it("R3548001 a name blur wraps one word", () => {
    const board = renderBoard(
      baseView({
        you: emptySide("p1", { units: [unit("p1", { tipsyBlur: { kind: "name", word: 1 } }), null, null, null, null] }),
      }),
    );
    const name = board.querySelector(".card-name")?.textContent ?? "";
    const words = name.split(" ").filter((word) => word.length > 0);
    expect(words.length).toBeGreaterThan(0);
    const blurred = board.querySelector(".card-name .woozy-blur");
    expect(blurred?.textContent).toBe(words[1 % words.length]);
  });

  it("R3548001 a keyword blur marks its chip", () => {
    const board = renderBoard(
      baseView({
        you: emptySide("p1", {
          units: [
            unit("p1", { keywords: [{ kind: "Taunt" }], tipsyBlur: { kind: "keyword", keyword: "Taunt" } }),
            null,
            null,
            null,
            null,
          ],
        }),
      }),
    );
    const chip = board.querySelector('[data-keyword="Taunt"]');
    expect(chip?.getAttribute("data-woozy-blur")).toBe("true");
  });

  it("R3548001 no blur changes nothing", () => {
    const board = renderBoard(
      baseView({ you: emptySide("p1", { units: [unit("p1"), null, null, null, null] }) }),
    );
    expect(board.querySelector("[data-woozy-blur]")).toBeNull();
    expect(board.querySelector(".woozy-blur")).toBeNull();
    expect(screen.queryByTitle("Woozy counter")).toBeNull();
  });
});

describe("R3548004 infinite mana", () => {
  it("R3548004 the mana tray reads ∞", () => {
    const board = renderBoard(baseView({ you: emptySide("p1", { infiniteMana: true }) }));
    const tray = board.querySelector('[data-testid="mana-you"]');
    expect(tray?.getAttribute("data-infinite")).toBe("true");
    expect(tray?.querySelector(".mana-text")?.textContent).toBe("∞");
    expect(tray?.getAttribute("title")).toBe("Mana: infinite");
  });
});
