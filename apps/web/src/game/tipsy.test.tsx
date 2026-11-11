// Stinkyscape+ E5 (issue #3548, R3548000–R3548004): the Tipsy stein, the Tipsy blur and the
// infinite mana tray, drawn from the view and nothing else (CLAUDE.md rule 7).

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { renderBoard, withCatalog } from "../test/board.tsx";
import { baseView, emptySide, unit } from "../test/fixtures.ts";
import Hero from "./Hero.tsx";

afterEach(() => {
  cleanup();
});

describe("R3548000 the Tipsy stein", () => {
  it("R3548000 the stein shows the count on either seat above 0, and nothing at none", () => {
    const view = baseView({
      you: emptySide("p1", { tipsy: 4 }),
      opponent: emptySide("p2", { tipsy: 2 }),
    });
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={view} side={side} />));
      const badge = document.querySelector(".hero-tipsy");
      expect(badge?.getAttribute("data-tipsy")).toBe(side === "you" ? "4" : "2");
      expect(badge?.getAttribute("title")).toBe("Tipsy counter");
      expect(badge?.querySelector("img.cf-icon")).not.toBeNull();
      unmount();
    }
    render(withCatalog(<Hero view={baseView()} side="you" />));
    expect(document.querySelector(".hero-tipsy")).toBeNull();
  });
});

describe("R3548001 the Tipsy blur", () => {
  it("R3548001 a cost blur marks the root", () => {
    const board = renderBoard(
      baseView({
        you: emptySide("p1", { units: [unit("p1", { tipsyBlur: { kind: "cost" } }), null, null, null, null] }),
      }),
    );
    expect(board.querySelector('[data-tipsy-blur="cost"]')).not.toBeNull();
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
    const blurred = board.querySelector(".card-name .tipsy-blur");
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
    expect(chip?.getAttribute("data-tipsy-blur")).toBe("true");
  });

  it("R3548001 no blur changes nothing", () => {
    const board = renderBoard(
      baseView({ you: emptySide("p1", { units: [unit("p1"), null, null, null, null] }) }),
    );
    expect(board.querySelector("[data-tipsy-blur]")).toBeNull();
    expect(board.querySelector(".tipsy-blur")).toBeNull();
    expect(screen.queryByTitle("Tipsy counter")).toBeNull();
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
