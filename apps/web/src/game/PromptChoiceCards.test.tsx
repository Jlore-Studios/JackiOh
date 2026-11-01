// #2646: every choice of DISCOVER_OPTION_LIMIT options or fewer whose options name no card is a Discover
// of choice cards (ChoiceCard.tsx): each option drawn as a card with art, its label for a name and what
// it does as its text, and no cost, stats or card id. One test per picker the issue moves, each also
// rendered with one option past the limit, where the picker keeps its list. R2646000: an engine
// prompt's asking card, when the view names it, titles the picker and gives each option its words.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, ActivationView, PendingOption, PendingView, PlayerView } from "@jackioh/shared";

import { CARD_SETTINGS_DEFAULTS, INSPECT_HOVER, INSPECT_SHEET, closeInspect, writeCardSettings } from "../cards/index.ts";
import { HOVER_DELAY_MS, LONG_PRESS_MS } from "../cards/inspect/constants.ts";
import { setMarkOf } from "../cards/setMark.ts";
import type { Interaction } from "./actions.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { DISCOVER_OPTION_LIMIT } from "./contract.ts";
import { MODE_TEXT } from "./modeText.ts";
import Prompt from "./Prompt.tsx";
import { baseView, card, emptySide, faceUpBackrow, pendingFor } from "../test/fixtures.ts";

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

type Pending = Extract<PendingView, { forYou: true }>;

/** Route 1: an engine prompt in the view. */
function renderPending(pending: PendingView, onAction = vi.fn()): ReturnType<typeof vi.fn> {
  const view = baseView({
    you: emptySide("p1", { hand: [card({ instanceId: "h0", defId: "core-002" })] }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
    pending,
  });
  render(
    <CatalogContext.Provider value={lookup}>
      <Prompt view={view} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return onAction;
}

/** Route 2: a play (or an activation) being built from `candidates`, the card `h1` in hand. */
function renderBuild(view: PlayerView, interaction: Interaction, onAction = vi.fn()): ReturnType<typeof vi.fn> {
  render(
    <CatalogContext.Provider value={lookup}>
      <Prompt view={view} interaction={interaction} legal={"candidates" in interaction ? interaction.candidates : []} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return onAction;
}

function handView(defId: string): PlayerView {
  return baseView({
    you: emptySide("p1", { hand: [card({ instanceId: "h1", defId })] }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
  });
}

function playing(candidates: ActionBody[]): Interaction {
  return { stage: "playing", instanceId: "h1", candidates, picked: {} };
}

const modal = (): HTMLElement => screen.getByTestId("prompt-modal");

/**
 * The option is a choice card: art, a name and words, and nothing a card to play has (no gem, no
 * stats), so it reads as a Discover's card that is no card.
 */
function expectChoiceCard(key: string, art: "art" | "set" = "art"): HTMLElement {
  const option = screen.getByTestId(`prompt-option-${key}`);
  expect(option, key).toHaveClass("prompt-card--choice");
  expect(option.parentElement, key).toBe(screen.getByTestId("prompt-cards"));
  if (art === "art") expect(option.querySelector(".choice-card-art .cf-art"), key).not.toBeNull();
  else expect(option.querySelector(".choice-card-art img.prompt-set-mark"), key).not.toBeNull();
  const name = option.querySelector(".choice-card-name")?.textContent ?? "";
  const text = option.querySelector(".choice-card-text")?.textContent ?? "";
  expect(name, key).not.toBe("");
  expect(text, key).not.toBe("");
  expect(option.getAttribute("aria-label"), key).toBe(`${name}: ${text}`);
  for (const part of [".cf", ".cf-gem", ".cost-gem", ".card-cost", ".cf-attack", ".cf-health", ".cf-set"]) {
    expect(option.querySelector(part), `${key} ${part}`).toBeNull();
  }
  return option;
}

/** A picker of DISCOVER_OPTION_LIMIT + 1 options keeps its list: no card layout, no choice card. */
function expectList(kind: string): void {
  expect(modal()).toHaveAttribute("data-prompt-kind", kind);
  expect(modal()).not.toHaveAttribute("data-prompt-layout");
  expect(document.querySelector(".choice-card")).toBeNull();
}

function expectCards(kind: string): void {
  expect(modal()).toHaveAttribute("data-prompt-kind", kind);
  expect(modal()).toHaveAttribute("data-prompt-layout", "cards");
}

const OVER = DISCOVER_OPTION_LIMIT + 1;

describe("#2646 route 2: a play's short choices are choice cards", () => {
  it("#2646 a play's Choose one (Pocket Chaos) is choice cards in MODE_TEXT's words, and more options a list", () => {
    const modes = ["health", "board", "library"];
    const onAction = renderBuild(handView("core-087"), playing(modes.map((mode): ActionBody => ({ type: "play", instanceId: "h1", modes: [mode] }))));
    expectCards("discover");
    for (const mode of modes) {
      const option = expectChoiceCard(mode);
      expect(option.querySelector(".choice-card-name")?.textContent).toBe(MODE_TEXT["core-087"]?.[mode]?.label);
      expect(option.querySelector(".choice-card-text")?.textContent).toBe(MODE_TEXT["core-087"]?.[mode]?.detail);
    }
    fireEvent.click(screen.getByTestId("prompt-option-board"));
    expect(onAction).toHaveBeenCalledWith({ type: "play", instanceId: "h1", modes: ["board"] });
    cleanup();

    const lanes = Array.from({ length: OVER }, (_, at) => `lane-${String(at + 1)}`);
    renderBuild(handView("meditative-098"), playing(lanes.map((mode): ActionBody => ({ type: "play", instanceId: "h1", modes: [mode] }))));
    expectList("mode");
  });

  it("#2646 a card's abilities (R1751000) are choice cards, and more abilities a list", () => {
    const abilities = (count: number): ActivationView[] =>
      Array.from({ length: count }, (_, at) => ({ ability: `a${String(at)}`, label: `Ability ${String(at)}`, usesLeft: 1, usable: true }));
    const viewWith = (count: number): PlayerView =>
      baseView({
        you: emptySide("p1", {
          backrow: [faceUpBackrow("p1", { instanceId: "h1", defId: "classic-020", activations: abilities(count) }), null, null, null, null],
        }),
        opponent: emptySide("p2", { hand: { count: 3 } }),
      });
    const activating = (count: number): Interaction => ({
      stage: "activating",
      instanceId: "h1",
      candidates: abilities(count).map((a): ActionBody => ({ type: "activate", instanceId: "h1", ability: a.ability })),
      picked: {},
      cardAbilities: abilities(count).map((a) => a.ability),
    });
    const onAction = renderBuild(viewWith(2), activating(2));
    expectCards("discover");
    expect(expectChoiceCard("a0").querySelector(".choice-card-text")?.textContent).toBe("1 use left this turn");
    expectChoiceCard("a1");
    fireEvent.click(screen.getByTestId("prompt-option-a1"));
    expect(onAction).toHaveBeenCalledWith({ type: "activate", instanceId: "h1", ability: "a1" });
    cleanup();

    renderBuild(viewWith(OVER), activating(OVER));
    expectList("mode");
  });

  it("#2646 Equip or Stack (R1086) is two choice cards, Equip first", () => {
    const zone = { row: "units" as const, lane: 1 };
    const candidates: ActionBody[] = [
      { type: "play", instanceId: "h1", zone },
      { type: "play", instanceId: "h1", zone, equip: true },
    ];
    const onAction = renderBuild(handView("meditative-065"), playing(candidates));
    expectCards("mode");
    expect(screen.getAllByTestId(/^prompt-option-/).map((option) => option.getAttribute("data-testid"))).toEqual(["prompt-option-true", "prompt-option-false"]);
    expect(expectChoiceCard("true").querySelector(".choice-card-name")?.textContent).toBe("Equip");
    expect(expectChoiceCard("false").querySelector(".choice-card-name")?.textContent).toBe("Stack");
    fireEvent.click(screen.getByTestId("prompt-option-true"));
    expect(onAction).toHaveBeenCalledWith(candidates[1]);
    // Equip or Stack is a yes or no: it never has more than two options, so it never reaches the list.
  });

  it("#2646 how a card is played (R783004) is choice cards, and every way a list", () => {
    const zone = { row: "backrow" as const, lane: 1 };
    const ways = ["endOfThisTurn", "startOfNextTurn", "endOfNextTurn", "cloaked"] as const;
    const short: ActionBody[] = [{ type: "play", instanceId: "h1", zone }, { type: "play", instanceId: "h1", zone, faceDown: "cloaked" }];
    const onAction = renderBuild(handView("core-002"), playing(short));
    expectCards("mode");
    expect(expectChoiceCard("up").querySelector(".choice-card-name")?.textContent).toBe("Face-up");
    expect(expectChoiceCard("cloaked").querySelector(".choice-card-text")?.textContent).toBe("Set face-down: reveal it with its Activate");
    fireEvent.click(screen.getByTestId("prompt-option-cloaked"));
    expect(onAction).toHaveBeenCalledWith(short[1]);
    cleanup();

    renderBuild(handView("core-002"), playing([{ type: "play", instanceId: "h1", zone }, ...ways.map((faceDown): ActionBody => ({ type: "play", instanceId: "h1", zone, faceDown }))]));
    expectList("mode");
    expect(screen.getByTestId("prompt-option-up").textContent).toBe("Play it face-up");
  });

  it("#2646 a play's Plague Counter payment (B5 E11) is choice cards, and many ways a row of chips", () => {
    const zone = { row: "units" as const, lane: 5 };
    const pay = (tokens: number): ActionBody => ({ type: "play", instanceId: "h1", zone, plague: { from: "fs2", tokens } });
    const mana: ActionBody = { type: "play", instanceId: "h1", zone };
    const onAction = renderBuild(handView("core-004"), playing([mana, pay(2)]));
    expectCards("number");
    expect(expectChoiceCard("none").querySelector(".choice-card-name")?.textContent).toBe("Pay in mana only");
    expectChoiceCard(JSON.stringify(["fs2", 2]));
    fireEvent.click(screen.getByTestId(`prompt-option-${JSON.stringify(["fs2", 2])}`));
    expect(onAction).toHaveBeenCalledWith(pay(2));
    cleanup();

    renderBuild(handView("core-004"), playing([mana, ...Array.from({ length: OVER - 1 }, (_, at) => pay(at + 1))]));
    expectList("number");
    expect(document.querySelector(".prompt-plague")).not.toBeNull();
  });
});

describe("#2646 route 1: an engine prompt's short choices are choice cards", () => {
  const PICKLE: PendingOption[] = [
    { key: "mode:discard", label: "Discard 1" },
    { key: "mode:exile", label: "Exile 1" },
    { key: "mode:draw", label: "Draw 1" },
  ];

  it("#2646 an engine Choose one (Classic #8 Pickle, by the opponent) is choice cards, and more options a list", () => {
    const onAction = renderPending(pendingFor("mode", PICKLE, { prompt: "Pickle: choose one", sourceDefId: "classic-008", sourceRadiant: false }));
    expectCards("discover");
    for (const option of PICKLE) {
      const drawn = expectChoiceCard(option.key);
      // The label is the engine's, which carries the count; the words are the card's.
      expect(drawn.querySelector(".choice-card-name")?.textContent).toBe(option.label);
      expect(drawn.querySelector(".choice-card-text")?.textContent).toBe(MODE_TEXT["classic-008"]?.[option.key.slice("mode:".length)]?.detail);
    }
    // The title names the asking card once, though the engine's prompt names it too.
    expect(modal().querySelector(".prompt-title-source")?.textContent).toBe(nameOf("classic-008"));
    expect(modal().querySelector(".prompt-title-ask")?.textContent).toBe("choose one");
    fireEvent.click(screen.getByTestId("prompt-option-mode:exile"));
    expect(onAction).toHaveBeenCalledWith({ type: "answer", choiceId: "ch1", selection: [{ pick: "mode", option: "exile" }] });
    cleanup();

    renderPending(pendingFor("mode", Array.from({ length: OVER }, (_, at) => ({ key: `mode:o${String(at)}`, label: `Option ${String(at)}` }))));
    expectList("mode");
  });

  it("#2646 a multiple-choice problem's answers (Classic+ #42) are choice cards named by their letters, Stop a button under them, and more answers a list", () => {
    const answers = ["285", "295", "185", "286"];
    const options: PendingOption[] = [...answers.map((text, at) => ({ key: `mode:${"ABCD"[at] ?? ""}`, label: text })), { key: "none", label: "Stop" }];
    const onAction = renderPending(pendingFor("answer", options, { prompt: "482 − 197 = ?", streak: 1 }));
    expectCards("answer");
    answers.forEach((text, at) => {
      const letter = "ABCD"[at] ?? "";
      const option = expectChoiceCard(`mode:${letter}`);
      expect(option).toHaveAttribute("data-letter", letter);
      expect(option.getAttribute("aria-label")).toBe(`${letter}: ${text}`);
    });
    const stop = screen.getByTestId("prompt-option-none");
    expect(stop).not.toHaveClass("prompt-card");
    expect(stop.parentElement).toBe(modal());
    expect(screen.getByTestId("prompt-cards")).toHaveAttribute("role", "group");
    fireEvent.click(screen.getByTestId("prompt-option-mode:B"));
    expect(onAction).toHaveBeenCalledWith({ type: "answer", choiceId: "ch1", selection: [{ pick: "mode", option: "B" }] });
    cleanup();

    renderPending(pendingFor("answer", Array.from({ length: OVER }, (_, at) => ({ key: `mode:${"ABCDE"[at] ?? ""}`, label: String(at) })), { prompt: "?" }));
    expectList("answer");
    expect(document.querySelector(".prompt-answers")).not.toBeNull();
  });

  it("#2646 a quest's rewards (Classic #90) are choice cards named by their letters, and more rewards a list", () => {
    const rewards = ["Heal your hero 6", "Deal 3 damage to a target"];
    const onAction = renderPending(
      pendingFor(
        "reward",
        rewards.map((label, at) => ({ key: `mode:${"AB"[at] ?? ""}`, label })),
        { prompt: "Quest complete! Draw 2 cards", sourceDefId: "classic-090" },
      ),
    );
    expectCards("reward");
    expect(modal().querySelector(".prompt-title")?.textContent).toBe("Draw 2 cards");
    rewards.forEach((label, at) => {
      const option = expectChoiceCard(`mode:${"AB"[at] ?? ""}`);
      expect(option.querySelector(".choice-card-name")?.textContent).toBe(`Reward ${"AB"[at] ?? ""}`);
      expect(option.querySelector(".choice-card-text")?.textContent).toBe(label);
    });
    fireEvent.click(screen.getByTestId("prompt-option-mode:B"));
    expect(onAction).toHaveBeenCalledWith({ type: "answer", choiceId: "ch1", selection: [{ pick: "mode", option: "B" }] });
    cleanup();

    renderPending(pendingFor("reward", Array.from({ length: OVER }, (_, at) => ({ key: `mode:r${String(at)}`, label: `Reward ${String(at)}` })), { prompt: "Quest complete!" }));
    expectList("reward");
  });

  it("#2646 R831004 a Discover of sets (#82 KY's Trial) is choice cards whose art is the set's mark, and more sets a list", () => {
    const sets = ["Core", "Classic+", "Meditative"];
    const onAction = renderPending(pendingFor("mode", sets.map((set) => ({ key: `mode:${set}`, label: set })), { prompt: "KY's Trial: Discover a set", sourceDefId: "core-082" }));
    expectCards("discover");
    for (const set of sets) {
      const option = expectChoiceCard(`mode:${set}`, "set");
      expect(option).toHaveAttribute("data-set", set);
      expect(option.querySelector("img.prompt-set-mark")?.getAttribute("src")).toBe(setMarkOf(set).src);
      expect(option.querySelector(".choice-card-text")?.textContent).toContain(set);
    }
    fireEvent.click(screen.getByTestId("prompt-option-mode:Classic+"));
    expect(onAction).toHaveBeenCalledWith({ type: "answer", choiceId: "ch1", selection: [{ pick: "mode", option: "Classic+" }] });
    cleanup();

    const five = ["Core", "Classic", "Classic+", "Meditative", "Stinkyscape"];
    renderPending(pendingFor("mode", five.map((set) => ({ key: `mode:${set}`, label: set })), { sourceDefId: "core-082" }));
    expectList("mode");
  });

  it("R2646000 an engine mode prompt names its asking card in its title and reads each option's words off it", () => {
    const pending: Pending = {
      ...(pendingFor("mode", [
        { key: "mode:health", label: "health" },
        { key: "mode:board", label: "board" },
      ]) as Pending),
      prompt: "Choose one",
      sourceDefId: "core-087",
      sourceRadiant: false,
    };
    renderPending(pending);
    expect(modal().querySelector(".prompt-title-source")?.textContent).toBe(nameOf("core-087"));
    expect(screen.getByTestId("prompt-option-mode:health").querySelector(".choice-card-text")?.textContent).toBe(MODE_TEXT["core-087"]?.health?.detail);
    cleanup();

    // No asking card named (hidden from the chooser, or none): no title source and no words to read.
    const { sourceDefId: _s, sourceRadiant: _r, ...unnamed } = pending;
    renderPending(unnamed);
    expect(modal().querySelector(".prompt-title-source")).toBeNull();
    expect(screen.getByTestId("prompt-option-mode:health").querySelector(".choice-card-text")).toBeNull();
  });
});

describe("#2646 a choice card is no card", () => {
  function pickle(): PendingView {
    return pendingFor(
      "mode",
      [
        { key: "mode:discard", label: "Discard 1" },
        { key: "mode:exile", label: "Exile 1" },
      ],
      { sourceDefId: "classic-008" },
    );
  }

  it("#2646 a choice card opens no inspect overlay or long-press sheet", () => {
    vi.useFakeTimers();
    renderPending(pickle());
    const option = screen.getByTestId("prompt-option-mode:discard");
    fireEvent.pointerEnter(option, { pointerType: "mouse" });
    act(() => {
      vi.advanceTimersByTime(HOVER_DELAY_MS * 2);
    });
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();
    fireEvent.pointerDown(option, { pointerType: "touch", pointerId: 3, isPrimary: true, button: 0 });
    act(() => {
      vi.advanceTimersByTime(LONG_PRESS_MS * 2);
    });
    expect(screen.queryByTestId(INSPECT_HOVER)).toBeNull();
    expect(screen.queryByTestId(INSPECT_SHEET)).toBeNull();
    // It names no card, so nothing in it does either.
    expect(within(option).queryByTestId(/^card-/)).toBeNull();
    expect(option).not.toHaveAttribute("data-def-id");
  });

  it("#2646 an option keeps its art across renders, and two options of one prompt differ", () => {
    const art = (key: string): string => screen.getByTestId(`prompt-option-${key}`).querySelector(".cf-art")?.getAttribute("style") ?? "";
    renderPending(pickle());
    const discard = art("mode:discard");
    const exile = art("mode:exile");
    expect(discard).not.toBe("");
    expect(discard).not.toBe(exile);
    cleanup();
    renderPending(pickle());
    expect(art("mode:discard")).toBe(discard);
    expect(art("mode:exile")).toBe(exile);
  });
});
