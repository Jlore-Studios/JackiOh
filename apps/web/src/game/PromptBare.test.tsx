// #2738: a live card-layout picker is its cards alone over the dimmed board, as Hearthstone draws
// a Discover: no panel (`data-bare`), no title, count, board hint or quest banner, and no Confirm
// when one tap answers. A picker that needs more than a tap keeps its controls, and every non-card,
// folded, read-only and lesson picker keeps its panel and its words.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, ActivationView, MulliganView, PendingView, PlayerView } from "@jackioh/shared";

import type { Interaction } from "./actions.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import Prompt from "./Prompt.tsx";
import { baseView, card, emptySide, faceUpBackrow, pendingFor } from "../test/fixtures.ts";

afterEach(cleanup);

const lookup = lookupFromDefs(CATALOG);

function nameOf(defId: string): string {
  const def = CATALOG[defId];
  if (def === undefined) throw new Error(`the catalog has no ${defId}`);
  return def.name;
}

/** Route 1: an engine prompt in the view, over a hand of h1, h2 and h3. */
function renderPending(
  pending: PendingView,
  onAction = vi.fn(),
  opts: { readOnly?: boolean; lesson?: boolean } = {},
): ReturnType<typeof vi.fn> {
  const view = baseView({
    you: emptySide("p1", {
      hand: [card({ instanceId: "h1", defId: "core-002" }), card({ instanceId: "h2", defId: "core-019" }), card({ instanceId: "h3", defId: "core-004" })],
    }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
    pending,
  });
  render(
    <CatalogContext.Provider value={lookup}>
      <Prompt
        view={view}
        onAction={onAction}
        {...(opts.readOnly === true ? { readOnly: true } : {})}
        {...(opts.lesson === true ? { lesson: true } : {})}
      />
    </CatalogContext.Provider>,
  );
  return onAction;
}

/** Route 2: a play (or an activation) being built from `candidates`, the card `h1` in hand. */
function renderBuild(
  view: PlayerView,
  interaction: Interaction,
  onAction = vi.fn(),
  onCancel?: () => void,
): ReturnType<typeof vi.fn> {
  render(
    <CatalogContext.Provider value={lookup}>
      <Prompt
        view={view}
        interaction={interaction}
        legal={"candidates" in interaction ? interaction.candidates : []}
        onAction={onAction}
        {...(onCancel === undefined ? {} : { onCancel })}
      />
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

function playing(candidates: ActionBody[], instanceId = "h1"): Interaction {
  return { stage: "playing", instanceId, candidates, picked: {} };
}

const modal = (): HTMLElement => screen.getByTestId("prompt-modal");

type Route1Row = {
  label: string;
  pending: PendingView;
  key: string;
  selection: unknown[];
  /** The dialog's exact accessible name, or null when only a non-empty name is held. */
  aria: string | null;
  /** An answer keeps its problem as its title (R420); every other bare picker has none. */
  problem?: string;
};

const PICKLE = [
  { key: "mode:discard", label: "Discard 1" },
  { key: "mode:exile", label: "Exile 1" },
  { key: "mode:draw", label: "Draw 1" },
];

const ROUTE1: readonly Route1Row[] = [
  {
    label: "discover",
    pending: pendingFor("discover", [
      { key: "mode:core-043", label: "Flood", defId: "core-043" },
      { key: "mode:core-055", label: "Archivist", defId: "core-055" },
      { key: "mode:core-066", label: "Lava Golem", defId: "core-066" },
    ]),
    key: "mode:core-055",
    selection: [{ pick: "mode", option: "core-055" }],
    aria: "Choose (discover)",
  },
  {
    label: "KY's Trial numbers",
    pending: pendingFor(
      "discover",
      ["7", "42", "100"].map((n) => ({ key: `mode:${n}`, label: n })),
      { prompt: "KY's Trial: Discover a number" },
    ),
    key: "mode:42",
    selection: [{ pick: "mode", option: "42" }],
    aria: "KY's Trial: Discover a number",
  },
  {
    label: "KY's Trial sets",
    pending: pendingFor(
      "mode",
      ["Core", "Classic+", "Meditative"].map((set) => ({ key: `mode:${set}`, label: set })),
      { prompt: "KY's Trial: Discover a set", sourceDefId: "core-082" },
    ),
    key: "mode:Classic+",
    selection: [{ pick: "mode", option: "Classic+" }],
    aria: null,
  },
  {
    label: "Pickle",
    pending: pendingFor("mode", PICKLE, { prompt: "Pickle: choose one", sourceDefId: "classic-008", sourceRadiant: false }),
    key: "mode:exile",
    selection: [{ pick: "mode", option: "exile" }],
    aria: `${nameOf("classic-008")}: choose one`,
  },
  {
    label: "hand",
    pending: pendingFor("hand", [
      { key: "instance:h1", label: "One", instanceId: "h1" },
      { key: "instance:h2", label: "Two", instanceId: "h2" },
    ]),
    key: "instance:h2",
    selection: [{ pick: "instance", instanceId: "h2" }],
    aria: "Choose (hand)",
  },
  {
    label: "pick",
    pending: pendingFor("pick", [
      { key: "instance:h1", label: "One", instanceId: "h1" },
      { key: "instance:h2", label: "Two", instanceId: "h2" },
    ]),
    key: "instance:h2",
    selection: [{ pick: "instance", instanceId: "h2" }],
    aria: "Choose (pick)",
  },
  {
    label: "answer",
    pending: pendingFor(
      "answer",
      ["A", "B", "C", "D"].map((letter) => ({ key: `mode:${letter}`, label: letter })),
      { prompt: "482 − 197 = ?" },
    ),
    key: "mode:B",
    selection: [{ pick: "mode", option: "B" }],
    aria: "482 − 197 = ?",
    problem: "482 − 197 = ?",
  },
  {
    label: "reward",
    pending: pendingFor(
      "reward",
      [
        { key: "mode:A", label: "Heal your hero 6" },
        { key: "mode:B", label: "Deal 3 damage" },
      ],
      { prompt: "Quest complete! Draw 2 cards", sourceDefId: "classic-090" },
    ),
    key: "mode:B",
    selection: [{ pick: "mode", option: "B" }],
    aria: "Quest complete! Draw 2 cards",
  },
  {
    label: "embiggen",
    pending: pendingFor("embiggen", [
      { key: "false", label: "Normal" },
      { key: "true", label: "Embiggened" },
    ]),
    key: "true",
    selection: [{ pick: "mode", option: "true" }],
    aria: "Choose (embiggen)",
  },
  {
    label: "x",
    pending: pendingFor("x", ["0", "1", "2"].map((n) => ({ key: n, label: n }))),
    key: "2",
    selection: [{ pick: "mode", option: "2" }],
    aria: "Choose (x)",
  },
];

describe("#2738 a card picker's open picker is its cards alone", () => {
  for (const row of ROUTE1) {
    it(`#2738 ${row.label}: the open picker is its cards alone, and one click answers`, () => {
      const onAction = renderPending(row.pending);
      expect(modal()).toHaveAttribute("data-prompt-layout", "cards");
      expect(modal()).toHaveAttribute("data-bare", "true");
      expect(modal()).toHaveAttribute("role", "dialog");
      if (row.aria === null) expect(modal().getAttribute("aria-label") ?? "").not.toBe("");
      else expect(modal().getAttribute("aria-label")).toBe(row.aria);
      if (row.problem === undefined) expect(modal().querySelector(".prompt-title")).toBeNull();
      else expect(modal().querySelector(".prompt-title")?.textContent).toBe(row.problem);
      expect(modal().querySelector(".prompt-count")).toBeNull();
      expect(modal().querySelector(".prompt-board-note")).toBeNull();
      expect(modal().querySelector(".prompt-board-hint")).toBeNull();
      expect(modal().querySelector(".prompt-quest-banner")).toBeNull();
      expect(screen.queryByTestId("prompt-submit")).toBeNull();
      expect(screen.getByTestId("prompt-peek")).toBeInTheDocument();

      fireEvent.click(screen.getByTestId(`prompt-option-${row.key}`));
      expect(onAction).toHaveBeenCalledTimes(1);
      expect(onAction).toHaveBeenCalledWith({ type: "answer", choiceId: "ch1", selection: row.selection });
    });
  }
});

type Route2Row = {
  label: string;
  view: PlayerView;
  interaction: Interaction;
  key: string;
  body: ActionBody;
  /** The dialog's exact accessible name, or null when only a non-empty name is held. */
  aria: string | null;
};

function abilitiesFixture(): { view: PlayerView; interaction: Interaction } {
  const abilities = (count: number): ActivationView[] =>
    Array.from({ length: count }, (_, at) => ({ ability: `a${String(at)}`, label: `Ability ${String(at)}`, usesLeft: 1, usable: true }));
  const view = baseView({
    you: emptySide("p1", {
      backrow: [faceUpBackrow("p1", { instanceId: "h1", defId: "classic-020", activations: abilities(2) }), null, null, null, null],
    }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
  });
  const interaction: Interaction = {
    stage: "activating",
    instanceId: "h1",
    candidates: abilities(2).map((a): ActionBody => ({ type: "activate", instanceId: "h1", ability: a.ability })),
    picked: {},
    cardAbilities: abilities(2).map((a) => a.ability),
  };
  return { view, interaction };
}

const EQUIP_ZONE = { row: "units" as const, lane: 1 };
const EQUIP_CANDIDATES: ActionBody[] = [
  { type: "play", instanceId: "h1", zone: EQUIP_ZONE },
  { type: "play", instanceId: "h1", zone: EQUIP_ZONE, equip: true },
];

const FACE_ZONE = { row: "backrow" as const, lane: 1 };
const FACE_CANDIDATES: ActionBody[] = [
  { type: "play", instanceId: "h1", zone: FACE_ZONE },
  { type: "play", instanceId: "h1", zone: FACE_ZONE, faceDown: "cloaked" },
];

const PLAGUE_ZONE = { row: "units" as const, lane: 5 };
const PLAGUE_MANA: ActionBody = { type: "play", instanceId: "h1", zone: PLAGUE_ZONE };
const PLAGUE_PAID: ActionBody = { type: "play", instanceId: "h1", zone: PLAGUE_ZONE, plague: { from: "fs2", tokens: 2 } };

const PRICE_ZONE = { row: "units" as const, lane: 1 };
const PRICE_CANDIDATES: ActionBody[] = [
  { type: "play", instanceId: "h1", zone: PRICE_ZONE, embiggen: false },
  { type: "play", instanceId: "h1", zone: PRICE_ZONE, embiggen: true },
];

const POCKET_MODES = ["health", "board", "library"];

function handPickView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [card({ instanceId: "h1", defId: "core-002" }), card({ instanceId: "h2", defId: "core-019" }), card({ instanceId: "h3", defId: "core-004" })],
    }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
  });
}

const ABILITIES = abilitiesFixture();

const ROUTE2: readonly Route2Row[] = [
  {
    label: "Pocket Chaos",
    view: handView("core-087"),
    interaction: playing(POCKET_MODES.map((mode): ActionBody => ({ type: "play", instanceId: "h1", modes: [mode] }))),
    key: "board",
    body: { type: "play", instanceId: "h1", modes: ["board"] },
    aria: `${nameOf("core-087")}: Choose one`,
  },
  {
    label: "abilities",
    view: ABILITIES.view,
    interaction: ABILITIES.interaction,
    key: "a1",
    body: { type: "activate", instanceId: "h1", ability: "a1" },
    aria: `${nameOf("classic-020")}: Choose an ability`,
  },
  {
    label: "Equip or Stack",
    view: handView("meditative-065"),
    interaction: playing(EQUIP_CANDIDATES),
    key: "true",
    body: EQUIP_CANDIDATES[1] as ActionBody,
    aria: "Equip or Stack?",
  },
  {
    label: "face-down",
    view: handView("core-002"),
    interaction: playing(FACE_CANDIDATES),
    key: "cloaked",
    body: FACE_CANDIDATES[1] as ActionBody,
    aria: "Play it how?",
  },
  {
    label: "Plague Counters",
    view: handView("core-004"),
    interaction: playing([PLAGUE_MANA, PLAGUE_PAID]),
    key: JSON.stringify(["fs2", 2]),
    body: PLAGUE_PAID,
    aria: "Pay with Plague Counters?",
  },
  {
    label: "embiggen price",
    view: handView("core-002"),
    interaction: playing(PRICE_CANDIDATES),
    key: "true",
    body: PRICE_CANDIDATES[1] as ActionBody,
    aria: `${nameOf("core-002")}: Play it as it is, or Radiant?`,
  },
  {
    label: "X 1–3",
    view: baseView({
      you: emptySide("p1", { hand: [card({ instanceId: "h2", defId: "core-019" })] }),
      opponent: emptySide("p2", { hand: { count: 3 } }),
    }),
    interaction: playing(
      [1, 2, 3].map((x): ActionBody => ({ type: "play", instanceId: "h2", x })),
      "h2",
    ),
    key: "2",
    body: { type: "play", instanceId: "h2", x: 2 },
    aria: `${nameOf("core-019")}: Choose X`,
  },
  {
    label: "hand pick",
    view: handPickView(),
    interaction: playing([
      { type: "play", instanceId: "h1", targets: [{ pick: "instance", instanceId: "h2" }] },
      { type: "play", instanceId: "h1", targets: [{ pick: "instance", instanceId: "h3" }] },
    ]),
    key: "instance:h3",
    body: { type: "play", instanceId: "h1", targets: [{ pick: "instance", instanceId: "h3" }] },
    aria: "Choose a card in your hand",
  },
];

describe("#2738 a play's card picker is its cards alone", () => {
  for (const row of ROUTE2) {
    it(`#2738 ${row.label}: the open picker is its cards alone, and one click answers`, () => {
      const onAction = renderBuild(row.view, row.interaction);
      expect(modal()).toHaveAttribute("data-prompt-layout", "cards");
      expect(modal()).toHaveAttribute("data-bare", "true");
      expect(modal()).toHaveAttribute("role", "dialog");
      if (row.aria === null) expect(modal().getAttribute("aria-label") ?? "").not.toBe("");
      else expect(modal().getAttribute("aria-label")).toBe(row.aria);
      expect(modal().querySelector(".prompt-title")).toBeNull();
      expect(modal().querySelector(".prompt-count")).toBeNull();
      expect(modal().querySelector(".prompt-board-note")).toBeNull();
      expect(modal().querySelector(".prompt-board-hint")).toBeNull();
      expect(modal().querySelector(".prompt-quest-banner")).toBeNull();
      expect(screen.queryByTestId("prompt-submit")).toBeNull();
      expect(screen.getByTestId("prompt-peek")).toBeInTheDocument();

      fireEvent.click(screen.getByTestId(`prompt-option-${row.key}`));
      expect(onAction).toHaveBeenCalledTimes(1);
      expect(onAction).toHaveBeenCalledWith(row.body);
    });
  }
});

function mulliganView(mulligan: MulliganView): PlayerView {
  return baseView({
    you: emptySide("p1", {
      hand: [card({ instanceId: "h1", defId: "core-002" }), card({ instanceId: "h2", defId: "core-019" }), card({ instanceId: "h3", defId: "core-004" })],
    }),
    opponent: emptySide("p2", { hand: { count: 3 } }),
    pending: pendingFor(
      "mulligan",
      ["h1", "h2", "h3"].map((id) => ({ key: id, label: id, instanceId: id })),
      { min: 0, max: 3, prompt: "Keep which cards?" },
    ),
    mulligan,
  });
}

/** C #44 Back from the GY: Units from the graveyard with a total cost of (5) or less. */
function graveyardView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      graveyard: [
        card({ instanceId: "g1", defId: "core-002", cost: 2 }),
        card({ instanceId: "g2", defId: "core-019", cost: 3 }),
        card({ instanceId: "g3", defId: "core-043", cost: 4, radiant: true }),
      ],
    }),
    pending: pendingFor(
      "pick",
      [
        { key: "instance:g1", label: "Speciesist", instanceId: "g1", defId: "core-002", cost: 2 },
        { key: "instance:g2", label: "Midrange Menace", instanceId: "g2", defId: "core-019", cost: 3 },
        { key: "instance:g3", label: "Big Felinor", instanceId: "g3", defId: "core-043", cost: 4, radiant: true },
      ],
      { prompt: "Summon Units with a total cost of (5) or less", min: 0, max: 3, budget: 5 },
    ),
  });
}

describe("#2738 a picker that needs more than a tap keeps its controls", () => {
  it("#2738 the mulligan keeps Ready and the opponent's status in one row under the cards", () => {
    const onAction = vi.fn();
    render(
      <CatalogContext.Provider value={lookup}>
        <Prompt view={mulliganView({ youReady: false, opponentReady: false })} onAction={onAction} />
      </CatalogContext.Provider>,
    );
    expect(modal()).toHaveAttribute("data-bare", "true");
    expect(modal().querySelector(".prompt-title")).toBeNull();
    expect(modal().querySelector(".prompt-count")).toBeNull();
    const ready = screen.getByTestId("prompt-submit");
    expect(ready).toHaveTextContent("Ready");
    expect(ready.parentElement).toHaveClass("prompt-actions");
    expect(screen.getByTestId("mulligan-opponent-status")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("prompt-option-h2"));
    expect(onAction).not.toHaveBeenCalled();
    fireEvent.click(ready);
    expect(onAction).toHaveBeenCalledWith({ type: "mulligan", keep: ["h1", "h3"] });
  });

  it("#2738 a multi-pick keeps a Confirm that waits for its minimum", () => {
    const onAction = renderPending(
      pendingFor("pick", [
        { key: "instance:h1", label: "One", instanceId: "h1" },
        { key: "instance:h2", label: "Two", instanceId: "h2" },
        { key: "instance:h3", label: "Three", instanceId: "h3" },
      ], { min: 1, max: 2 }),
    );
    expect(modal()).toHaveAttribute("data-bare", "true");
    const confirm = screen.getByTestId("prompt-submit");
    expect(confirm.parentElement).toHaveClass("prompt-actions");
    expect(confirm).toHaveAttribute("aria-disabled", "true");
    fireEvent.click(confirm);
    expect(onAction).not.toHaveBeenCalled();

    fireEvent.click(screen.getByTestId("prompt-option-instance:h1"));
    fireEvent.click(screen.getByTestId("prompt-option-instance:h2"));
    expect(confirm).toHaveAttribute("aria-disabled", "false");
    fireEvent.click(confirm);
    expect(onAction).toHaveBeenCalledWith({
      type: "answer",
      choiceId: "ch1",
      selection: [
        { pick: "instance", instanceId: "h1" },
        { pick: "instance", instanceId: "h2" },
      ],
    });
  });

  it("#2738 R515 a budget pick keeps its budget line and its Confirm", () => {
    render(
      <CatalogContext.Provider value={lookup}>
        <Prompt view={graveyardView()} onAction={vi.fn()} />
      </CatalogContext.Provider>,
    );
    expect(modal()).toHaveAttribute("data-bare", "true");
    expect(screen.getByTestId("prompt-budget")).toHaveTextContent("(0) of (5) spent");
    expect(screen.getByTestId("prompt-submit").parentElement).toHaveClass("prompt-actions");
  });

  it("#2738 a play's embiggen keeps Cancel and no Confirm", () => {
    const onCancel = vi.fn();
    renderBuild(handView("core-002"), playing(PRICE_CANDIDATES), vi.fn(), onCancel);
    expect(modal()).toHaveAttribute("data-bare", "true");
    expect(screen.queryByTestId("prompt-submit")).toBeNull();
    const cancel = screen.getByTestId("prompt-cancel");
    expect(cancel.parentElement).toHaveClass("prompt-actions");
    fireEvent.click(cancel);
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("#2738 a play's hand pick that may answer with nothing keeps Confirm", () => {
    renderBuild(
      handPickView(),
      playing([
        { type: "play", instanceId: "h1", targets: [] },
        { type: "play", instanceId: "h1", targets: [{ pick: "instance", instanceId: "h2" }] },
      ]),
    );
    expect(modal()).toHaveAttribute("data-prompt-kind", "hand");
    expect(modal()).toHaveAttribute("data-bare", "true");
    expect(screen.getByTestId("prompt-submit")).toBeInTheDocument();
  });
});

describe("#2738 non-card, folded, read-only and lesson pickers keep the panel", () => {
  it("#2738 a target, a five-option mode list and a number pad keep the panel and their words", () => {
    const cases: PendingView[] = [
      pendingFor("target", [
        { key: "instance:e1", label: "Enemy unit", instanceId: "e1" },
        { key: "hero:p2", label: "Enemy hero", player: "p2" },
      ]),
      pendingFor(
        "mode",
        Array.from({ length: 5 }, (_, at) => ({ key: `mode:m${String(at)}`, label: `Mode ${String(at)}` })),
      ),
      pendingFor("number", Array.from({ length: 11 }, (_, n) => ({ key: `mode:${String(n)}`, label: String(n) }))),
    ];
    for (const pending of cases) {
      renderPending(pending);
      expect(modal()).not.toHaveAttribute("data-bare");
      expect(modal().querySelector(".prompt-title")).not.toBeNull();
      expect(screen.getByTestId("prompt-submit")).toBeInTheDocument();
      cleanup();
    }
  });

  it("#2738 a folded discover keeps its title and loses its bare panel", () => {
    renderPending(
      pendingFor("discover", [
        { key: "mode:core-043", label: "Flood", defId: "core-043" },
        { key: "mode:core-055", label: "Archivist", defId: "core-055" },
      ]),
    );
    expect(modal()).toHaveAttribute("data-bare", "true");
    fireEvent.click(screen.getByTestId("prompt-peek"));
    expect(modal()).not.toHaveAttribute("data-bare");
    expect(modal().querySelector(".prompt-title")).not.toBeNull();
  });

  it("#2738 a read-only discover keeps its title and loses its bare panel once unfolded", () => {
    renderPending(
      pendingFor("discover", [
        { key: "mode:core-043", label: "Flood", defId: "core-043" },
        { key: "mode:core-055", label: "Archivist", defId: "core-055" },
      ]),
      vi.fn(),
      { readOnly: true },
    );
    expect(modal()).not.toHaveAttribute("data-bare");
    fireEvent.click(screen.getByTestId("prompt-peek"));
    expect(modal()).not.toHaveAttribute("data-bare");
    expect(modal().querySelector(".prompt-title")).not.toBeNull();
  });

  it("#2738 a lesson's mulligan keeps its panel, instruction and count", () => {
    renderPending(
      pendingFor(
        "mulligan",
        ["h1", "h2", "h3"].map((id) => ({ key: id, label: id, instanceId: id })),
        { min: 0, max: 3, prompt: "Keep which cards?" },
      ),
      vi.fn(),
      { lesson: true },
    );
    expect(modal()).not.toHaveAttribute("data-bare");
    expect(modal().querySelector(".prompt-title")).toHaveTextContent("Keep which cards?");
    expect(modal().querySelector(".prompt-count")).toHaveTextContent("Choose 0–3: 3 chosen");
  });
});

describe("#2738 prompt.css draws a bare picker with no panel", () => {
  const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "prompt.css"), "utf8");

  it("#2738 the bare rule clears the background, border and shadow", () => {
    const start = css.indexOf('.prompt[data-prompt-layout="cards"][data-bare] {');
    expect(start).toBeGreaterThan(-1);
    const block = css.slice(start, css.indexOf("}", start));
    expect(block).toContain("background: transparent;");
    expect(block).toContain("border: 0;");
    expect(block).toContain("box-shadow: none;");
  });

  it("#2738 the bare picker hugs its cards off a phone", () => {
    expect(css).toMatch(
      /@media \(min-width: 601px\) and \(min-height: 501px\) \{\s*\.prompt\[data-prompt-layout="cards"\]\[data-bare\] \{[^}]*width: fit-content;/,
    );
  });
});
