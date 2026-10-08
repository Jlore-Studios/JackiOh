// Polish 6: the prompt pickers that offer cards (a mulligan, a Discover, a card from hand) draw each
// option as the card's face, with the same hover preview the hand has, instead of a text square.
// The picking itself is Prompt.test.tsx's and is unchanged: the option is still the
// `prompt-option-<key>` button, and a click still picks it.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, PlayerView } from "@jackioh/shared";

import { CARD_SETTINGS_DEFAULTS, INSPECT_HOVER, closeInspect, writeCardSettings } from "../cards/index.ts";
import { HOVER_DELAY_MS } from "../cards/inspect/constants.ts";
import type { Interaction } from "./actions.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Prompt from "./Prompt.tsx";
import { baseView, card, emptySide, pendingFor } from "../test/fixtures.ts";

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
  vi.useRealTimers();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
});

const lookup = lookupFromDefs(CATALOG);

function nameOf(defId: string): string {
  const def = CATALOG[defId];
  if (def === undefined) throw new Error(`the catalog has no ${defId}`);
  return def.name;
}

/** The printed price a face's gem shows for a card with no live cost (a Discover option). */
function costOf(defId: string): string {
  const cost = CATALOG[defId]?.cost;
  return typeof cost === "number" || typeof cost === "string" ? String(cost) : String(cost?.base);
}

function renderPrompt(view: PlayerView, onAction = vi.fn()): void {
  render(
    <CatalogContext.Provider value={lookup}>
      <Prompt view={view} onAction={onAction} />
    </CatalogContext.Provider>,
  );
}

const DISCOVER = ["core-043", "core-055", "core-066"] as const;

function discoverView(): PlayerView {
  return baseView({
    you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-002" })] }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
    pending: pendingFor(
      "discover",
      DISCOVER.map((defId) => ({ key: `mode:${defId}`, label: defId, defId })),
    ),
  });
}

describe("card options are card faces", () => {
  it("B42 each Discover option draws the full face of its card, named, with art, and no text square", () => {
    renderPrompt(discoverView());
    for (const defId of DISCOVER) {
      const option = screen.getByTestId(`prompt-option-mode:${defId}`);
      const face = option.querySelector(".cf-option > .cf");
      expect(face, defId).not.toBeNull();
      expect(face?.getAttribute("data-layout"), defId).toBe("full");
      expect(face?.querySelector(".card-name")?.textContent, defId).toBe(nameOf(defId));
      expect(face?.querySelector(".cf-art"), defId).not.toBeNull();
      expect(option.querySelector(".prompt-card-name"), defId).toBeNull();
      // The name, then the cost its gem shows (integration: the cost is read out, not only drawn).
      expect(option.getAttribute("aria-label"), defId).toBe(`${nameOf(defId)}, costs (${costOf(defId)})`);
    }
  });

  it("B42 a mulligan's options are the faces of the hand cards they name", () => {
    const view = baseView({
      you: emptySide("p1", {
        hand: [card({ instanceId: "h1", defId: "core-002" }), card({ instanceId: "h2", defId: "core-019" })],
      }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
      pending: pendingFor(
        "mulligan",
        [
          { key: "h1", label: "One", instanceId: "h1" },
          { key: "h2", label: "Two", instanceId: "h2" },
        ],
        { min: 0, max: 2, prompt: "Keep which cards?" },
      ),
    });
    renderPrompt(view);
    const names = ["h1", "h2"].map(
      (key) => screen.getByTestId(`prompt-option-${key}`).querySelector(".cf .card-name")?.textContent,
    );
    expect(names).toEqual([nameOf("core-002"), nameOf("core-019")]);
  });

  // Integration (task 3's review of the mulligan): every card shows its cost, the one the hand
  // shows, opens kept (R9 names the cards kept; Hearthstone keeps the hand until a card is marked),
  // and says what Confirm will do to it.
  it("a mulligan's options show each card's live cost, open kept, and say Keep or Redraw", () => {
    const onAction = vi.fn();
    const view = baseView({
      you: emptySide("p1", {
        hand: [card({ instanceId: "h1", defId: "core-002", cost: 0 }), card({ instanceId: "h2", defId: "core-019", cost: 5 })],
      }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
      pending: pendingFor(
        "mulligan",
        [
          { key: "h1", label: "One", instanceId: "h1" },
          { key: "h2", label: "Two", instanceId: "h2" },
        ],
        { min: 0, max: 2, prompt: "Keep which cards?" },
      ),
    });
    renderPrompt(view, onAction);
    const option = (key: string): HTMLElement => screen.getByTestId(`prompt-option-${key}`);

    expect(option("h1").querySelector(".cost-gem")?.getAttribute("data-cost")).toBe("0");
    expect(option("h2").querySelector(".cost-gem")?.getAttribute("data-cost")).toBe("5");
    expect(option("h2").getAttribute("aria-label")).toBe(`${nameOf("core-019")}, costs (5)`);

    for (const key of ["h1", "h2"]) {
      expect(option(key)).toHaveAttribute("aria-pressed", "true");
      expect(option(key)).toHaveAttribute("data-verdict", "keep");
      expect(option(key).querySelector(".prompt-card-verdict")?.textContent).toBe("Keep");
    }

    fireEvent.click(option("h2"));
    expect(option("h2")).toHaveAttribute("aria-pressed", "false");
    expect(option("h2")).toHaveAttribute("data-verdict", "redraw");
    expect(option("h2").querySelector(".prompt-card-verdict")?.textContent).toBe("Redraw");

    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(onAction).toHaveBeenCalledWith({ type: "mulligan", keep: ["h1"] });
  });

  it("Confirm on an untouched mulligan keeps the whole hand", () => {
    const onAction = vi.fn();
    const view = baseView({
      you: emptySide("p1", {
        hand: [card({ instanceId: "h1", defId: "core-002" }), card({ instanceId: "h2", defId: "core-019" })],
      }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
      pending: pendingFor(
        "mulligan",
        [
          { key: "h1", label: "One", instanceId: "h1" },
          { key: "h2", label: "Two", instanceId: "h2" },
        ],
        { min: 0, max: 2 },
      ),
    });
    renderPrompt(view, onAction);
    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(onAction).toHaveBeenCalledWith({ type: "mulligan", keep: ["h1", "h2"] });
  });

  it("a Discover has no verdicts: nothing is picked before the player picks", () => {
    renderPrompt(discoverView());
    for (const defId of DISCOVER) {
      const option = screen.getByTestId(`prompt-option-mode:${defId}`);
      expect(option).toHaveAttribute("aria-pressed", "false");
      expect(option).not.toHaveAttribute("data-verdict");
    }
  });

  it("resting a mouse on an option previews the card, and a click still picks it", () => {
    vi.useFakeTimers();
    const onAction = vi.fn();
    renderPrompt(discoverView(), onAction);
    const option = screen.getByTestId("prompt-option-mode:core-055");
    fireEvent.pointerEnter(option, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS);
    });
    expect(screen.getByTestId(INSPECT_HOVER).querySelector(".card-name")?.textContent).toBe(nameOf("core-055"));

    fireEvent.pointerLeave(option, { pointerType: "mouse" });
    fireEvent.click(option);
    expect(onAction).toHaveBeenCalledTimes(1);
  });
});

describe("R247: #82 KY's Trial's Discover offers numbers", () => {
  /**
   * The chooser's view exactly as `viewFor` builds it for #82 (the "R247 …" test beside its script,
   * crates/cards/src/scripts/core/c082_kys_trial.rs): each option is the number, keyed `mode:<n>`
   * and labelled `<n>`, with no definition.
   */
  const NUMBERS = ["17", "42", "88"] as const;

  function trialView(): PlayerView {
    return baseView({
      you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: "core-002" })] }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
      pending: pendingFor(
        "discover",
        NUMBERS.map((n) => ({ key: `mode:${n}`, label: n })),
        { prompt: "KY's Trial: Discover a number from 1 to 100" },
      ),
    });
  }

  /** The cards those numbers name, which the picker must not show. */
  const NAMED = NUMBERS.map((n) => Object.values(CATALOG).find((def) => def.index === n)?.name ?? n);

  it("R247 draws each option as its number on a card back, with no face, name or text of the card it names", () => {
    renderPrompt(trialView());
    const modal = screen.getByTestId("prompt-modal");
    expect(modal).toHaveAttribute("data-prompt-kind", "discover");
    for (const n of NUMBERS) {
      const option = screen.getByTestId(`prompt-option-mode:${n}`);
      expect(option).toHaveAttribute("data-number", n);
      expect(option.querySelector(".prompt-number-value")?.textContent).toBe(n);
      expect(option.querySelector(".cf-back")).not.toBeNull();
      expect(option.querySelector(".cf"), n).toBeNull();
      expect(option.querySelector(".card-name"), n).toBeNull();
      expect(option.getAttribute("aria-label")).toBe(`Number ${n}`);
    }
    for (const name of NAMED) expect(modal.textContent).not.toContain(name);
  });

  it("R247 a number opens no preview, and picking one answers with that number", () => {
    vi.useFakeTimers();
    const onAction = vi.fn();
    renderPrompt(trialView(), onAction);
    const option = screen.getByTestId("prompt-option-mode:42");
    fireEvent.pointerEnter(option, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS * 2);
    });
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();

    fireEvent.click(option);
    expect(onAction).toHaveBeenCalledWith({ type: "answer", choiceId: "ch1", selection: [{ pick: "mode", option: "42" }] });
  });

  it("R247 every other Discover still offers its cards' faces", () => {
    renderPrompt(discoverView());
    for (const defId of DISCOVER) {
      const option = screen.getByTestId(`prompt-option-mode:${defId}`);
      expect(option).not.toHaveAttribute("data-number");
      expect(option.querySelector(".cf .card-name")?.textContent).toBe(nameOf(defId));
    }
  });
});

describe("#492 one card picker, front and centre, for every short choice", () => {
  const AURA = "core-046"; // Suppressive Aura, "2 embiggen 4"
  const APPROPRIATIONS = "classicplus-040"; // an X Spell with a "Choose one" of four

  function handView(defId: string, over: { cost?: number; embiggenCost?: number; radiant?: boolean } = {}): PlayerView {
    return baseView({
      you: emptySide("p1", { hand: [card({ instanceId: "h1", defId, ...over })] }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
    });
  }

  function renderPlay(view: PlayerView, candidates: ActionBody[], onAction = vi.fn()): void {
    const interaction: Interaction = { stage: "playing", instanceId: "h1", candidates, picked: {} };
    render(
      <CatalogContext.Provider value={lookup}>
        <Prompt view={view} interaction={interaction} legal={candidates} onAction={onAction} />
      </CatalogContext.Provider>,
    );
  }

  const PRICES: ActionBody[] = [
    { type: "play", instanceId: "h1", zone: { row: "backrow", lane: 1 }, embiggen: false },
    { type: "play", instanceId: "h1", zone: { row: "backrow", lane: 1 }, embiggen: true },
  ];

  /** What a face's gem says, and the smaller price beside it, if any. */
  function gem(option: HTMLElement): { text: string; alt: string | null } {
    const copy = option.querySelector(".cost-gem")?.cloneNode(true) as Element | undefined;
    const alt = copy?.querySelector(".cf-cost-alt")?.textContent ?? null;
    copy?.querySelector(".cf-cost-alt")?.remove();
    return { text: (copy?.textContent ?? "").trim(), alt };
  }

  it("#492 the embiggen price is the card itself twice: at the price it shows in hand, and at its embiggen price", () => {
    const onAction = vi.fn();
    renderPlay(handView(AURA, { cost: 2, embiggenCost: 4 }), PRICES, onAction);

    const modal = screen.getByTestId("prompt-modal");
    expect(modal).toHaveAttribute("data-prompt-kind", "embiggen");
    expect(modal).toHaveAttribute("data-prompt-layout", "cards");
    expect(modal.querySelector(".prompt-title-source")?.textContent).toBe(nameOf(AURA));

    const normal = screen.getByTestId("prompt-option-false");
    const bigger = screen.getByTestId("prompt-option-true");
    for (const option of [normal, bigger]) {
      expect(option.querySelector(".cf-option > .cf .card-name")?.textContent).toBe(nameOf(AURA));
      expect(option.querySelector(".card-text")?.textContent).toBe(CATALOG[AURA]?.base.text);
    }
    expect(gem(normal)).toEqual({ text: "2", alt: null });
    expect(normal.querySelector(".prompt-card-caption")?.textContent).toBe("NormalPay (2)");
    expect(normal.getAttribute("aria-label")).toBe(`Normal: ${nameOf(AURA)}, costs (2)`);
    expect(gem(bigger)).toEqual({ text: "4", alt: null });
    expect(bigger.querySelector(".prompt-card-caption")?.textContent).toBe("EmbiggenedPay (4)");
    expect(bigger.getAttribute("aria-label")).toBe(`Embiggened: ${nameOf(AURA)}, costs (4)`);

    fireEvent.click(bigger);
    expect(onAction).toHaveBeenCalledWith(PRICES[1]);
  });

  it("#492 a Radiant card offers its Radiant face at both prices", () => {
    renderPlay(handView(AURA, { cost: 2, embiggenCost: 4, radiant: true }), PRICES);
    for (const key of ["false", "true"]) {
      const option = screen.getByTestId(`prompt-option-${key}`);
      expect(option.querySelector(".card-text")?.textContent).toBe(CATALOG[AURA]?.radiant.text);
    }
  });

  it("#492 under a discount both cards show the view's prices: cost on the Normal card, embiggenCost on the Embiggened one", () => {
    renderPlay(handView(AURA, { cost: 1, embiggenCost: 3 }), PRICES);
    const normal = screen.getByTestId("prompt-option-false");
    const bigger = screen.getByTestId("prompt-option-true");
    expect(gem(normal)).toEqual({ text: "1", alt: null });
    expect(normal.querySelector(".prompt-card-caption-detail")?.textContent).toBe("Pay (1)");
    expect(gem(bigger)).toEqual({ text: "3", alt: null });
    expect(bigger.querySelector(".cost-gem")).toHaveAttribute("data-tone", "down");
    expect(bigger.querySelector(".prompt-card-caption-detail")?.textContent).toBe("Pay (3)");
    expect(bigger.getAttribute("aria-label")).toBe(`Embiggened: ${nameOf(AURA)}, costs (3)`);
  });

  it("#492 R363: a discount the embiggen price alone reaches lowers only the Embiggened card", () => {
    // Professor Curvature's "(4)+ Cost" discount: 2 stays 2, 4 becomes 3. Only the view can say so.
    renderPlay(handView(AURA, { cost: 2, embiggenCost: 3 }), PRICES);
    expect(gem(screen.getByTestId("prompt-option-false"))).toEqual({ text: "2", alt: null });
    expect(gem(screen.getByTestId("prompt-option-true"))).toEqual({ text: "3", alt: null });
  });

  it("#492 under a surcharge the Embiggened card's gem shows the raised price", () => {
    renderPlay(handView(AURA, { cost: 3, embiggenCost: 5 }), PRICES);
    const bigger = screen.getByTestId("prompt-option-true");
    expect(gem(bigger)).toEqual({ text: "5", alt: null });
    expect(bigger.querySelector(".cost-gem")).toHaveAttribute("data-tone", "up");
    expect(bigger.querySelector(".prompt-card-caption-detail")?.textContent).toBe("Pay (5)");
  });

  it("#492 a card view without embiggenCost shows the printed embiggen price while the card stands at its printed price, and else claims none", () => {
    renderPlay(handView(AURA, { cost: 2 }), PRICES);
    expect(gem(screen.getByTestId("prompt-option-true"))).toEqual({ text: "4", alt: null });
    cleanup();

    renderPlay(handView(AURA, { cost: 1 }), PRICES);
    const bigger = screen.getByTestId("prompt-option-true");
    // CLAUDE.md rule 7: the client never works out a discounted embiggen price.
    expect(gem(bigger)).toEqual({ text: "?", alt: null });
    expect(bigger.querySelector(".prompt-card-caption-detail")?.textContent).toBe("Pay the embiggen price");
  });

  it("#492 a few X values are the card once per X, each captioned with its X", () => {
    const onAction = vi.fn();
    const candidates: ActionBody[] = [1, 2, 3].map((x) => ({ type: "play", instanceId: "h1", x }));
    renderPlay(handView(APPROPRIATIONS, { cost: 0 }), candidates, onAction);

    expect(screen.getByTestId("prompt-modal")).toHaveAttribute("data-prompt-kind", "x");
    for (const x of [1, 2, 3]) {
      const option = screen.getByTestId(`prompt-option-${String(x)}`);
      expect(option.querySelector(".cf .card-name")?.textContent).toBe(nameOf(APPROPRIATIONS));
      expect(option.querySelector(".prompt-card-caption")?.textContent).toBe(`X = ${String(x)}`);
      expect(option).not.toHaveAttribute("data-price");
    }
    fireEvent.click(screen.getByTestId("prompt-option-3"));
    expect(onAction).toHaveBeenCalledWith({ type: "play", instanceId: "h1", x: 3 });
  });

  it("#492 a short Choose one, an embiggen price and a short X are the one card layout: cards in a row in the middle", () => {
    const modes = ["Military", "Education", "Culture", "Healthcare"];
    const pickers: [PlayerView, ActionBody[]][] = [
      [handView(APPROPRIATIONS), modes.map((mode): ActionBody => ({ type: "play", instanceId: "h1", x: 1, modes: [mode] }))],
      [handView(AURA, { cost: 2, embiggenCost: 4 }), PRICES],
      [handView(APPROPRIATIONS), [1, 2].map((x): ActionBody => ({ type: "play", instanceId: "h1", x }))],
    ];
    for (const [view, candidates] of pickers) {
      renderPlay(view, candidates);
      const modal = screen.getByTestId("prompt-modal");
      expect(modal).toHaveAttribute("data-prompt-layout", "cards");
      const row = screen.getByTestId("prompt-cards");
      expect(row.parentElement).toBe(modal);
      const options = screen.getAllByTestId(/^prompt-option-/);
      expect(options).toHaveLength(candidates.length);
      for (const option of options) {
        expect(option.parentElement).toBe(row);
        expect(option).toHaveClass("prompt-card");
      }
      cleanup();
    }
  });
});
