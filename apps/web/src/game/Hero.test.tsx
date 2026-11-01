// R987: Luck (Meditative #40 Feng Shui) rides on the hero's portrait while the side has any. The
// engine half is proved in crates/cards/src/scripts/meditative/c040_feng_shui.rs.
//
// R847: a side whose Temporal Rift turn is granted wears "Rift turn used", on either seat. The engine
// half — the flag set when a Rift grants a turn, public on the view — is proved in
// crates/engine/tests/rules/extra_turns.rs.
//
// R1639000: a Heroic Power used this turn wears "Used", on either seat. R1840005: a seat's Credits are
// a badge by the hero from their first gain, a count of 0 included.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { withCatalog } from "../test/board.tsx";
import { baseView, emptySide, heroPower } from "../test/fixtures.ts";
import Hero, { RIFT_SPENT_LABEL, RIFT_SPENT_NOTE } from "./Hero.tsx";

afterEach(() => {
  cleanup();
});

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

describe("R1639000 a Heroic Power used this turn", () => {
  const used = { ...heroPower, usedThisTurn: true };
  const ping = { ...heroPower, instanceId: "power-2", title: "Ping", x: 1 };

  it("R1639000 the opponent's power tags wear Used and say so in their title only when used, extra powers included", () => {
    const extra = { ...ping, usedThisTurn: true };
    const opponent = (power: typeof used, powers: (typeof used)[]) =>
      baseView({ opponent: emptySide("p2", { hand: { count: 4 }, hero: { health: 30, armor: 0, power, powers } }) });
    const { container, unmount } = render(withCatalog(<Hero view={opponent(used, [used, extra])} side="opponent" />));
    const tags = container.querySelectorAll(".power-tag");
    expect(tags).toHaveLength(2);
    for (const tag of tags) expect(tag.querySelector(".power-used")).toHaveTextContent(/^Used$/);
    expect(tags[0]).toHaveAttribute("title", "Ranching: used this turn");
    expect(tags[1]).toHaveAttribute("title", "Ping: used this turn");
    unmount();

    const fresh = render(withCatalog(<Hero view={opponent(heroPower, [heroPower])} side="opponent" />));
    const tag = fresh.container.querySelector(".power-tag");
    expect(tag?.querySelector(".power-used")).toBeNull();
    expect(tag).not.toHaveAttribute("title");
    expect(tag).toHaveTextContent(/^Ranching2$/);
  });

  it("R1639000 the viewer's own power button wears Used, and its title and accessible name say it was used", () => {
    const yours = (power: typeof used, powers: (typeof used)[]) =>
      baseView({ you: emptySide("p1", { hero: { health: 30, armor: 0, power, powers } }) });
    const { unmount } = render(withCatalog(<Hero view={yours(used, [used, ping])} side="you" />));
    const button = screen.getByTestId("power");
    expect(button.querySelector(".power-used")).not.toBeNull();
    expect(button).toHaveAttribute("title", "Ranching: used this turn");
    expect(button).toHaveAccessibleName(/^Used Ranching/);
    const other = screen.getByTestId("power-power-2");
    expect(other.querySelector(".power-used")).toBeNull();
    expect(other).toHaveAttribute("title", "Ping: Activate, spend (1)");
    unmount();

    render(withCatalog(<Hero view={yours(heroPower, [heroPower])} side="you" />));
    const fresh = screen.getByTestId("power");
    expect(fresh.querySelector(".power-used")).toBeNull();
    expect(fresh).toHaveAttribute("title", "Ranching: Activate, spend (2)");
  });
});

describe("R1840005 the hero's Credits badge", () => {
  it("R1840005 a count of 0 still shows on either seat, unlike the Jade Counter's, and a seat without Credits draws none", () => {
    const view = baseView({
      you: emptySide("p1", { marketCredits: 0, jade: 0 }),
      opponent: emptySide("p2", { hand: { count: 4 }, marketCredits: 0 }),
    });
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={view} side={side} />));
      const badge = screen.getByTestId(`market-credits-${side}`);
      expect(badge.textContent).toBe("0");
      expect(badge.getAttribute("data-credits")).toBe("0");
      expect(badge.classList.contains("hero-credits")).toBe(true);
      expect(badge.getAttribute("title")).toBe("Credits");
      // R961: a Jade Counter of 0 draws nothing, which is the difference.
      expect(document.querySelector(".hero-jade")).toBeNull();
      unmount();
    }
    for (const side of ["you", "opponent"] as const) {
      const { unmount } = render(withCatalog(<Hero view={baseView()} side={side} />));
      expect(screen.queryByTestId(`market-credits-${side}`)).toBeNull();
      unmount();
    }
  });
});
