// R3047004: a Resource in its owner's hand while the Marketplace is open prints, in braces, the Credits it
// sells for this turn. The engine works the number out (`preview::preview_of`: the Marketplace's quote,
// R1853002, as an R280 preview labeled "Credits") and the view carries it on the hand card, where the
// client's existing preview rendering (RulesText.tsx, R280) prints it: this file proves the web's half, a
// Resource's face with the value and without it. Which Resources carry it, for whom and when is
// crates/engine/tests/rules/marketplace.rs's.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { PlayerView } from "@jackioh/shared";

import Board from "../game/Board.tsx";
import { testid } from "../game/contract.ts";
import { withCatalog } from "../test/board.tsx";
import { baseView, card, emptySide } from "../test/fixtures.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";

/** Stinkyscape's Korrelite, a Resource whose face holds no word "Credits" of its own. */
const KORRELITE = "stinkyscape-t-korrelite";
/** The label the engine gives the quote (`preview::SELL_QUOTE_LABEL`). */
const LABEL = "Credits";

beforeEach(() => {
  // R1017001: a full-art face prints none of its words; these read them.
  writeCardSettings({ fullArt: false });
});

afterEach(() => {
  cleanup();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
});

/** p1's hand: a base Resource quoted 4, a Radiant one quoted 9 and a Resource with no quote (the Marketplace shut). */
function view(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "q4", defId: KORRELITE, cost: 0, preview: [{ label: LABEL, value: 4 }] }),
        card({ instanceId: "q9", defId: KORRELITE, radiant: true, cost: 0, preview: [{ label: LABEL, value: 9 }] }),
        card({ instanceId: "q0", defId: KORRELITE, cost: 0 }),
      ],
    }),
  });
}

function values(root: Element): string[] {
  return [...root.querySelectorAll(".cf-value")].map((value) => value.textContent ?? "");
}

describe("R3047004 a Resource in hand shows what it sells for", () => {
  it("R3047004 the quote is printed in braces after the card's words, with its label as the tooltip", () => {
    render(withCatalog(<Board view={view()} />));
    const base = screen.getByTestId(testid.handCard("q4"));
    expect(values(base)).toEqual(["{4}"]);
    const value = base.querySelector(".cf-value");
    expect(value).toHaveAttribute("data-label", LABEL);
    expect(value).toHaveAttribute("title", "Credits: currently 4");
    expect(base.querySelector(".card-text")?.textContent).toContain("{4}");
  });

  it("R3047004 a Radiant Resource prints its own quote", () => {
    render(withCatalog(<Board view={view()} />));
    expect(values(screen.getByTestId(testid.handCard("q9")))).toEqual(["{9}"]);
  });

  it("R3047004 a Resource the view gives no quote prints none, as before the Marketplace opens", () => {
    render(withCatalog(<Board view={view()} />));
    const none = screen.getByTestId(testid.handCard("q0"));
    expect(values(none)).toEqual([]);
    expect(none.querySelector(".card-text")?.textContent ?? "").not.toContain("{");
  });
});
