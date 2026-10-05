// The Heroic Powers on the hero panel (Hero.tsx; SPEC §8 #98, R43, R510, patch v0.2.1): each power
// a round crest with its own art, keyed by the stored name the view gives it (R103), its X in a mana
// gem, its printed title (Tank Up on Armor Up's Radiant face), a gold rim when Radiant, greyed when
// spent, and its words — the power as the catalog prints it, `{shot}` filled — as its tooltip and
// accessible name. Each card prints only the power it rolled, and each power a player controls is its
// own control (a further one, stolen or copied, its crest alone); the opponent's are tags with the
// same crest. Every view is a fixture shaped as `viewFor` builds it.

import { CATALOG } from "@jackioh/cards";
import type { HeroPowerView, PlayerView } from "@jackioh/shared";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { POWER_ART, POWER_WORDS, powerArtOf } from "../cards/index.ts";
import { baseView, emptySide, heroPower } from "../test/fixtures.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid, type Highlight } from "./contract.ts";
import Hero, { POWER_USED_NOTE } from "./Hero.tsx";

const lookup = lookupFromDefs(CATALOG);

afterEach(() => {
  cleanup();
});

function power(over: Partial<HeroPowerView> = {}): HeroPowerView {
  return { ...heroPower, ...over };
}

function viewWith(yours: readonly HeroPowerView[], theirs: readonly HeroPowerView[] = []): PlayerView {
  return baseView({
    you: emptySide("p1", { hero: { health: 30, armor: 0, powers: [...yours], power: yours[0] ?? null } }),
    opponent: emptySide("p2", { hero: { health: 30, armor: 0, powers: [...theirs], power: theirs[0] ?? null } }),
  });
}

function highlight(legal: readonly string[]): Highlight {
  return { legal: new Set(legal), selected: new Set(), glow: new Set(legal) };
}

function renderHero(view: PlayerView, side: "you" | "opponent" = "you", extra: Partial<Parameters<typeof Hero>[0]> = {}) {
  const onClick = vi.fn();
  const utils = render(
    <CatalogContext.Provider value={lookup}>
      <Hero view={view} side={side} onClick={onClick} {...extra} />
    </CatalogContext.Provider>,
  );
  return { ...utils, onClick };
}

const crestOf = (element: HTMLElement): HTMLElement => {
  const crest = element.querySelector<HTMLElement>(".power-crest");
  if (crest === null) throw new Error("no crest");
  return crest;
};

describe("patch v0.2.1: each Heroic Power has its own art on the hero", () => {
  it("every one of the thirteen powers has a crest of its own: its own glyph and its own colours", () => {
    expect(Object.keys(POWER_ART)).toEqual(Object.keys(POWER_WORDS));
    const glyphs = Object.values(POWER_ART).map((art) => art.glyph);
    const colours = Object.values(POWER_ART).map((art) => `${art.light}|${art.dark}`);
    expect(new Set(glyphs).size).toBe(glyphs.length);
    expect(new Set(colours).size).toBe(colours.length);
    // A name the table does not know still draws a crest.
    expect(powerArtOf("not-a-power").path.d).not.toBe("");
  });

  it("the crest is keyed by the stored name and draws that power's glyph; the title is the printed one", () => {
    for (const name of Object.keys(POWER_WORDS)) {
      cleanup();
      renderHero(viewWith([power({ name, ability: name })]));
      const button = screen.getByTestId(testid.power);
      const crest = crestOf(button);
      expect(button).toHaveAttribute("data-power", name);
      expect(crest).toHaveAttribute("data-power-art", name);
      expect(crest).toHaveAttribute("data-glyph", POWER_ART[name]?.glyph);
      expect(crest.querySelector("svg path")?.getAttribute("d")).toBe(powerArtOf(name).path.d);
      expect(crest.style.getPropertyValue("--power-light")).toBe(POWER_ART[name]?.light);
      expect(button.querySelector(".power-title")).toHaveTextContent(POWER_WORDS[name]?.title ?? "");
    }
  });

  it("the X sits in a mana gem on the crest, and the button carries it", () => {
    renderHero(viewWith([power({ name: "recruit", ability: "recruit", x: 3 })]));
    const button = screen.getByTestId(testid.power);
    expect(button).toHaveAttribute("data-x", "3");
    expect(crestOf(button).querySelector(".power-x")).toHaveTextContent("3");
  });

  it("a Radiant power is marked and prints its Radiant title: Armor Up's is Tank Up, wearing the same shield", () => {
    renderHero(viewWith([power({ name: "armor", ability: "armor", radiant: true, x: 1 })]));
    const button = screen.getByTestId(testid.power);
    expect(button).toHaveAttribute("data-radiant", "true");
    expect(button.querySelector(".power-title")).toHaveTextContent("Tank Up");
    expect(crestOf(button)).toHaveAttribute("data-glyph", POWER_ART.armor?.glyph);
    expect(button.getAttribute("title")).toBe("Activate: Spend (1): Tank Up: Your hero gains 4 Armor. Refresh this power.");

    cleanup();
    renderHero(viewWith([power({ name: "armor", ability: "armor", x: 1 })]));
    expect(screen.getByTestId(testid.power)).not.toHaveAttribute("data-radiant");
    expect(screen.getByTestId(testid.power).querySelector(".power-title")).toHaveTextContent("Armor Up");
  });

  it("R666 the tooltip and accessible name are the power as the catalog prints it, {shot} filled from the view", () => {
    renderHero(viewWith([power({ name: "burn", ability: "burn", radiant: true, x: 1, params: { shot: 8 } })]));
    const button = screen.getByTestId(testid.power);
    const words = "Activate: Spend (1): Steady Shot: Deal 8 damage to the enemy hero. Upgrade this permanently by +2 damage.";
    expect(button.getAttribute("title")).toBe(words);
    expect(button).toHaveAccessibleName(`Heroic Power: ${words}`);
    expect(button.getAttribute("title")).not.toMatch(/[{}]/);
  });

  it("R666 with no catalog loaded, the view's number still fills {shot}", () => {
    render(<Hero view={viewWith([power({ name: "burn", ability: "burn", x: 1, params: { shot: 2 } })])} side="you" />);
    expect(screen.getByTestId(testid.power).getAttribute("title")).toBe(
      "Activate: Spend (1): Steady Shot: Deal 2 damage to the enemy hero.",
    );
  });

  it("a spent power is drawn spent and says so; whether it may be pressed is still `legal` alone", () => {
    renderHero(viewWith([power({ name: "ping", ability: "ping", x: 1, usedThisTurn: true })]));
    const button = screen.getByTestId(testid.power);
    expect(button).toHaveAttribute("data-used", "true");
    expect(button.getAttribute("title")).toBe(`Activate: Spend (1): Ping: Pierce. Deal 1 damage. ${POWER_USED_NOTE}`);
    expect(button).toBeDisabled();
  });
});

describe("R510 each Heroic Power a player controls is its own control on the hero", () => {
  const stolen = power({ instanceId: "power-2", name: "pluck", ability: "pluck", x: 2 });

  it("a further power (a stolen one) is its own control, its crest alone, live by `legal` alone", () => {
    const { onClick } = renderHero(viewWith([heroPower, stolen]), "you", {
      highlight: highlight([testid.power, testid.powerOf("power-2")]),
    });

    expect(screen.getByTestId(testid.power)).toHaveAttribute("data-instance-id", "power-1");
    const extra = screen.getByTestId(testid.powerOf("power-2"));
    expect(extra).toHaveClass("power-extra");
    expect(extra).toHaveAttribute("data-power", "pluck");
    expect(extra).toHaveAccessibleName("Heroic Power: Activate: Spend (2): Pluck: Add a random Fruit to your hand. It costs (0).");
    expect(crestOf(extra)).toHaveAttribute("data-power-art", "pluck");
    expect(extra).toBeEnabled();
    fireEvent.click(extra);
    expect(onClick.mock.calls.map((call) => call[0])).toEqual([{ on: "activate", instanceId: "power-2" }]);
  });

  it("a further power `legal` does not list is drawn and greyed, and sends nothing", () => {
    renderHero(viewWith([heroPower, stolen]), "you", { highlight: highlight([testid.power]) });
    expect(screen.getByTestId(testid.powerOf("power-2"))).toBeDisabled();
  });

  it("the opponent's powers are tags with the same crest and words, which nothing presses", () => {
    const theirs = power({ instanceId: "power-9", name: "insect", ability: "insect", radiant: true, x: 2 });
    renderHero(viewWith([], [theirs, power({ instanceId: "power-8", name: "felinor", ability: "felinor", x: 1 })]), "opponent");

    expect(screen.queryByTestId(testid.power)).toBeNull();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
    const tags = screen.getAllByRole("img");
    expect(tags.map((tag) => tag.getAttribute("data-power"))).toEqual(["insect", "felinor"]);
    expect(tags[1]).toHaveClass("power-extra");
    expect(tags[0]).toHaveAttribute("data-radiant", "true");
    expect(tags[0]).toHaveAccessibleName("Opponent's Heroic Power: Activate: Spend (2): Die Insect: Lucky 1. Deal 8 damage to a random enemy.");
  });

  it("Enter on a live power presses the power, not the hero it sits on", async () => {
    const user = userEvent.setup();
    const { onClick } = renderHero(viewWith([heroPower]), "you", {
      highlight: highlight([testid.power, testid.hero("you")]),
    });

    screen.getByTestId(testid.power).focus();
    await user.keyboard("{Enter}");

    expect(onClick.mock.calls.map((call) => call[0])).toEqual([{ on: "activate", instanceId: "power-1" }]);
  });
});
