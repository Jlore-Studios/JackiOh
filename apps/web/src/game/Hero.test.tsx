// R987: Luck (Meditative #40 Feng Shui) rides on the hero's portrait while the side has any —
// the badge appears with Feng Shui on the field (the view carries Luck then) and is gone without
// it. The engine half — Luck 1 with Feng Shui on the field, 0 without — is proved in
// crates/cards/src/scripts/meditative/c040_feng_shui.rs.

import { cleanup, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { baseView, emptySide } from "../test/fixtures.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Hero from "./Hero.tsx";

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
