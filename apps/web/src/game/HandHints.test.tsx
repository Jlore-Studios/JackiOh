// #2267: the hand hints and their row (HandHints.tsx, handHints.ts). The shipped list holds
// the Tradable pip (#2269) and the Tribute pip (#2268, tested at the end); the `test-*` hints
// below are this file's own.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { faceModel } from "../cards/index.ts";
import { resetLocaleForTests, setLocale, t } from "../i18n/i18n.ts";
import { withCatalog } from "../test/board.tsx";
import { baseView, card } from "../test/fixtures.ts";
import Hand from "./Hand.tsx";
import HandHints, { handHintsTestid } from "./HandHints.tsx";
import { HAND_HINTS, TRIBUTE_HINT, hintsFor, type HandHint } from "./handHints.ts";
import { testid } from "./contract.ts";

const HERE = dirname(fileURLToPath(import.meta.url));

/** handHints.css without its comments, as the row's rules. */
function hintCss(): string {
  return readFileSync(join(HERE, "handHints.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
}

const COST: HandHint = {
  id: "test-cost",
  icon: "target",
  applies: (c) => ({ n: c.cost, label: t("Cost") }),
};

const DIMMED: HandHint = {
  id: "test-dimmed",
  icon: "cog",
  applies: () => ({ dimmed: true, label: t("Target") }),
};

const NEVER: HandHint = {
  id: "test-never",
  icon: "cog",
  applies: () => null,
};

const VIEW = baseView();

afterEach(() => {
  cleanup();
  resetLocaleForTests();
});

describe("#2267 the hand hints and their row", () => {
  it("#2267 #2268 the shipped list holds the Tradable and Tribute pips, so a plain card wears no hint", () => {
    expect(HAND_HINTS.map((hint) => hint.id)).toEqual(["tradable", "tribute"]);
    const { container } = render(<HandHints card={card()} view={VIEW} />);
    expect(container.firstChild).toBeNull();
  });

  it("#2267 a hint that returns null draws nothing, and a card with no hints has no row element", () => {
    const c = card();
    expect(hintsFor(c, VIEW, faceModel({ defId: c.defId, def: undefined, radiant: false }), [NEVER])).toEqual([]);
    const { container } = render(<HandHints card={c} view={VIEW} hints={[NEVER]} />);
    expect(container.firstChild).toBeNull();
    expect(screen.queryByTestId(handHintsTestid(c.instanceId))).toBeNull();
  });

  it("#2267 two hints draw two pips in the list's order", () => {
    const c = card();
    const { container } = render(<HandHints card={c} view={VIEW} hints={[COST, NEVER, DIMMED]} />);
    const pips = container.querySelectorAll(".hand-hint");
    expect(pips).toHaveLength(2);
    expect([...pips].map((pip) => pip.getAttribute("data-hint"))).toEqual(["test-cost", "test-dimmed"]);

    cleanup();
    const rerendered = render(<HandHints card={c} view={VIEW} hints={[DIMMED, COST]} />);
    const swapped = rerendered.container.querySelectorAll(".hand-hint");
    expect([...swapped].map((pip) => pip.getAttribute("data-hint"))).toEqual(["test-dimmed", "test-cost"]);
  });

  it("#2267 n is printed when given, and a hint without one prints no number", () => {
    const c = card({ cost: 3 });
    const { container } = render(<HandHints card={c} view={VIEW} hints={[COST, DIMMED]} />);
    expect(container.querySelector('[data-hint="test-cost"] .hand-hint-n')).toHaveTextContent("3");
    expect(container.querySelector('[data-hint="test-dimmed"] .hand-hint-n')).toBeNull();
  });

  it("#2267 dimmed: true sets data-dimmed, and a plain pip has none", () => {
    const { container } = render(<HandHints card={card()} view={VIEW} hints={[COST, DIMMED]} />);
    expect(container.querySelector('[data-hint="test-dimmed"]')).toHaveAttribute("data-dimmed", "true");
    expect(container.querySelector('[data-hint="test-cost"]')).not.toHaveAttribute("data-dimmed");
  });

  it("#2267 each pip's accessible label goes through t() in the reader's language", async () => {
    render(<HandHints card={card()} view={VIEW} hints={[COST, DIMMED]} />);
    expect(screen.getByRole("img", { name: "Cost" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Target" })).toBeInTheDocument();

    cleanup();
    await setLocale("zh-CN");
    render(<HandHints card={card()} view={VIEW} hints={[COST, DIMMED]} />);
    expect(screen.getByRole("img", { name: "费用" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "目标" })).toBeInTheDocument();
  });

  it("#2269 #2574 a hand card whose view has tradable: true gets the pip; one without does not", () => {
    const tradable = card({ tradable: true });
    const { container } = render(<HandHints card={tradable} view={VIEW} />);
    const pips = container.querySelectorAll(".hand-hint");
    expect(pips).toHaveLength(1);
    const pip = container.querySelector('[data-hint="tradable"]');
    expect(pip).not.toBeNull();
    expect(pip?.querySelector(".hand-hint-n")).toBeNull();
    expect(pip).not.toHaveAttribute("data-dimmed");
    expect(screen.getByRole("img", { name: "Tradable" })).toBe(pip);

    cleanup();
    const { container: bare } = render(<HandHints card={card()} view={VIEW} />);
    expect(bare.firstChild).toBeNull();
  });

  it("#2269 a Tradable card by its printed keywords gets the pip, dimmed once the turn's Trade is spent", async () => {
    const tradable = card({ defId: "meditative-053", cost: 1, tradable: true });
    const live = render(withCatalog(<HandHints card={tradable} view={baseView({ trades: { cost: 0, left: 1 } })} />));
    const pip = live.container.querySelector('[data-hint="tradable"]');
    expect(pip).not.toBeNull();
    expect(pip).not.toHaveAttribute("data-dimmed");
    expect(screen.getByRole("img", { name: "Tradable" })).toBe(pip);

    cleanup();
    render(withCatalog(<HandHints card={tradable} view={baseView({ trades: { cost: 0, left: 0 } })} />));
    const spent = screen.getByRole("img", { name: "No Trades left this turn" });
    expect(spent).toHaveAttribute("data-hint", "tradable");
    expect(spent).toHaveAttribute("data-dimmed", "true");

    cleanup();
    await setLocale("zh-CN");
    render(withCatalog(<HandHints card={tradable} view={baseView({ trades: { cost: 0, left: 0 } })} />));
    const zh = screen.getByRole("img", { name: "本回合没有剩余交易次数" });
    expect(zh).toHaveAttribute("data-hint", "tradable");
    expect(zh).toHaveAttribute("data-dimmed", "true");

    cleanup();
    await setLocale("zh-CN");
    render(withCatalog(<HandHints card={tradable} view={baseView({ trades: { cost: 0, left: 1 } })} />));
    expect(screen.getByRole("img", { name: "可交易" })).toHaveAttribute("data-hint", "tradable");
  });

  it("R2574000 the Tradable pip reads card.tradable instead of printed keywords", async () => {
    // Card with tradable: true and no Tradable keyword (core-005 is a plain card) shows the pip.
    const auraLent = card({ defId: "core-005", cost: 1, tradable: true });
    const live = render(withCatalog(<HandHints card={auraLent} view={baseView({ trades: { cost: 0, left: 1 } })} />));
    const pip = live.container.querySelector('[data-hint="tradable"]');
    expect(pip).not.toBeNull();
    expect(pip).not.toHaveAttribute("data-dimmed");
    expect(screen.getByRole("img", { name: "Tradable" })).toBe(pip);

    // Card with printed Tradable keyword but no card.tradable flag shows no pip.
    cleanup();
    const withoutFlag = card({ defId: "meditative-053", cost: 1 });
    const { container: bare } = render(withCatalog(<HandHints card={withoutFlag} view={baseView({ trades: { cost: 0, left: 1 } })} />));
    expect(bare.querySelector('[data-hint="tradable"]')).toBeNull();

    // Dims when trades.left === 0.
    cleanup();
    render(withCatalog(<HandHints card={auraLent} view={baseView({ trades: { cost: 0, left: 0 } })} />));
    const spent = screen.getByRole("img", { name: "No Trades left this turn" });
    expect(spent).toHaveAttribute("data-hint", "tradable");
    expect(spent).toHaveAttribute("data-dimmed", "true");

    // Localizes when dimmed in zh-CN.
    cleanup();
    await setLocale("zh-CN");
    render(withCatalog(<HandHints card={auraLent} view={baseView({ trades: { cost: 0, left: 0 } })} />));
    const zh = screen.getByRole("img", { name: "本回合没有剩余交易次数" });
    expect(zh).toHaveAttribute("data-hint", "tradable");
    expect(zh).toHaveAttribute("data-dimmed", "true");
  });

  it("#2267 handHints.css: the row takes no pointer event, sits top-left, and has no glow or motion", () => {
    const css = hintCss();
    expect(css).toMatch(/\.hand-hints \{[^}]*pointer-events: none;/);
    expect(css).toMatch(/\.hand-hints \{[^}]*top: 19%;/);
    expect(css).toMatch(/\.hand-hints \{[^}]*left: 4%;/);
    expect(css).toMatch(/\.hand-hint\[data-dimmed="true"\] \{[^}]*opacity: 0\.5;[^}]*filter: grayscale\(1\);/);
    expect(css).toMatch(/\.fx-hold-card \.hand-hints \{[^}]*display: none;/);
    expect(css).not.toMatch(/box-shadow|animation|transition/);
  });
});

describe("#2267 Hand.tsx draws the row on the viewer's own hand cards only", () => {
  it("#2267 each of the viewer's own hand cards wears the row inside its card", () => {
    const a = card({ instanceId: "hint-a" });
    const b = card({ instanceId: "hint-b" });
    render(<Hand side="you" view={VIEW} hand={[a, b]} hints={[COST]} />);
    for (const id of ["hint-a", "hint-b"] as const) {
      expect(within(screen.getByTestId(testid.handCard(id))).getByTestId(handHintsTestid(id))).toBeInTheDocument();
    }
  });

  it("#2267 the viewer's hand draws no row without a view, with the shipped list, or when no hint applies", () => {
    const first = render(<Hand side="you" hand={[card({ instanceId: "nv" })]} hints={[COST]} />);
    expect(first.container.querySelector(".hand-hints")).toBeNull();

    cleanup();
    const second = render(<Hand side="you" view={VIEW} hand={[card({ instanceId: "sl" })]} />);
    expect(second.container.querySelector(".hand-hints")).toBeNull();

    cleanup();
    const third = render(<Hand side="you" view={VIEW} hand={[card({ instanceId: "na" })]} hints={[NEVER]} />);
    expect(third.container.querySelector(".hand-hints")).toBeNull();
  });

  it("R434 #2267 neither the opponent's backs nor their hand revealed at the game's end wear a row", () => {
    const backs = render(<Hand side="opponent" view={VIEW} hand={{ count: 3 }} hints={[COST]} />);
    expect(backs.container.querySelector(".hand-hints")).toBeNull();

    cleanup();
    const revealed = render(
      <Hand side="opponent" view={VIEW} hand={[card({ instanceId: "r0" }), card({ instanceId: "r1" })]} hints={[COST]} />,
    );
    expect(screen.getByTestId("hand-opponent")).toHaveAttribute("data-revealed", "true");
    expect(revealed.container.querySelector(".hand-hints")).toBeNull();
  });

  it("#2267 a click on a hint row still lifts the card and reports its hand click", () => {
    const onClick = vi.fn();
    render(
      <Hand
        side="you"
        view={VIEW}
        hand={[card({ instanceId: "h1" })]}
        hints={[COST]}
        highlight={{ legal: new Set([testid.handCard("h1")]), selected: new Set() }}
        onClick={onClick}
      />,
    );
    fireEvent.click(screen.getByTestId(handHintsTestid("h1")).querySelector(".hand-hint") as Element);
    expect(screen.getByTestId(testid.handCard("h1")).closest(".hand-slot")).toHaveAttribute("data-lifted", "true");
    expect(onClick).toHaveBeenCalledWith({ on: "hand", instanceId: "h1" });
  });
});

describe("R2268000 the Tribute hint reads the engine's number", () => {
  it("R2268000 a hand card whose view has tribute: 2 wears the Tribute pip showing 2", () => {
    const c = card({ tribute: 2 });
    const face = faceModel({ defId: c.defId, def: undefined, radiant: false });
    expect(hintsFor(c, VIEW, face).map(({ hint, shown }) => [hint.id, shown.n])).toEqual([["tribute", 2]]);
    const { container } = render(<HandHints card={c} view={VIEW} />);
    expect(container.querySelector('[data-hint="tribute"] .hand-hint-n')).toHaveTextContent("2");
    expect(container.querySelector('[data-hint="tribute"] img.cf-icon--altar')).not.toBeNull();
    expect(screen.getByRole("img", { name: "Tribute" })).toBeInTheDocument();
  });

  it("R2268000 a hand card whose view has no tribute wears no Tribute pip", () => {
    const c = card();
    expect(TRIBUTE_HINT.applies(c, VIEW, faceModel({ defId: c.defId, def: undefined, radiant: false }))).toBeNull();
    const { container } = render(<HandHints card={c} view={VIEW} />);
    expect(container.querySelector('[data-hint="tribute"]')).toBeNull();
  });

  it("R2268000 the viewer's own hand card wears it with the shipped list", () => {
    render(<Hand side="you" view={VIEW} hand={[card({ instanceId: "tr", tribute: 1 })]} />);
    const pip = screen.getByTestId(testid.handCard("tr")).querySelector('[data-hint="tribute"] .hand-hint-n');
    expect(pip).toHaveTextContent("1");
  });
});
