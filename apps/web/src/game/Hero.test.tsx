// R987: Luck (Meditative #40 Feng Shui) rides on the hero's portrait while the side has any —
// the badge appears with Feng Shui on the field (the view carries Luck then) and is gone without
// it. The engine half — Luck 1 with Feng Shui on the field, 0 without — is proved in
// crates/cards/src/scripts/meditative/c040_feng_shui.rs.
//
// R847: a side whose Temporal Rift turn is granted wears "Rift turn used", on either seat. The engine
// half — the flag set when a Rift grants a turn, public on the view — is proved in
// crates/engine/tests/rules/extra_turns.rs.

import { cleanup, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { baseView, emptySide } from "../test/fixtures.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Hero, { RIFT_SPENT_LABEL, RIFT_SPENT_NOTE } from "./Hero.tsx";

afterEach(() => {
  cleanup();
});

const lookup = lookupFromDefs(CATALOG);

function withCatalog(node: ReactElement): ReactElement {
  return <CatalogContext.Provider value={lookup}>{node}</CatalogContext.Provider>;
}

describe("R987 the hero's Luck badge", () => {
  it("R987 the badge appears with Luck on the side and is gone without it", () => {
    const { unmount } = render(
      withCatalog(<Hero view={baseView({ you: emptySide("p1", { luck: 1 }) })} side="you" />),
    );
    const badge = screen.getByTestId("hero-luck");
    expect(badge.textContent).toBe("1");
    expect(badge.getAttribute("data-luck")).toBe("1");
    expect(badge.getAttribute("title")).toBe(
      "Luck 1: every roll your cards make that keeps a best rolls 1 more times",
    );
    unmount();
    render(withCatalog(<Hero view={baseView()} side="you" />));
    expect(screen.queryByTestId("hero-luck")).toBeNull();
  });
});

describe("R847 the hero's spent Temporal Rift badge", () => {
  it("R847 a side whose Rift turn is granted wears the badge on either seat, and a side without the flag wears none", () => {
    const view = baseView({
      you: emptySide("p1", { riftExtraTurn: true }),
      opponent: emptySide("p2", { hand: { count: 4 }, riftExtraTurn: true }),
    });
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={view} side={side} />));
      const badge = screen.getByTestId(`rift-spent-${side}`);
      expect(badge.textContent).toBe(RIFT_SPENT_LABEL);
      expect(badge.querySelector(".rift-badge")?.getAttribute("title")).toBe(RIFT_SPENT_NOTE);
      // Outside the view's modifiers (R169): their badge list is unchanged.
      expect(screen.getByTestId(`modifiers-${side}`).querySelectorAll(".modifier-badge")).toHaveLength(0);
      unmount();
    }
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={baseView()} side={side} />));
      expect(screen.queryByTestId(`rift-spent-${side}`)).toBeNull();
      unmount();
    }
  });
});
