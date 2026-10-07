// The v0.2.0 controls against the REAL engine. The fixture tests (ActivateControl, activate,
// PromptE18) pin each control's behaviour; here the view and the legal list are `viewFor` and
// `legalActions` of real states built through the web's own engine seam (the WebAssembly module,
// `../wasm`, docs/v0.3.0/SURFACE.md §10.3), so a field the engine renames or a body it shapes
// differently fails here, not in production. Every action a control sends must be one the engine
// accepts (CLAUDE.md rule 7).

import { CATALOG } from "@jackioh/cards";
import { DECK_SIZE, MAX_MANA } from "@jackioh/engine/config";
import type { Action, ActionBody, PlayerId, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createGame, legalActions, reduce, registeredCatalog, viewFor } from "../wasm/index.ts";
import { closeInspect } from "../cards/index.ts";
import { __resetSettingsForTests } from "../settings/index.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import Game from "./Game.tsx";

const lookup = lookupFromDefs(CATALOG);
const el = (id: string): HTMLElement => screen.getByTestId(id);

// ---------------------------------------------------------------------------------------------
// The boards. Each case sets the board the cards' test harness set for it (BUILD M4-T3), and sets
// it the same way: `createGame` deals two filler decks, both libraries are emptied and the instance
// ids start again at c1, the clock is turn 9 in p1's main phase (both sides at MAX_MANA), and each
// side's cards are laid out in the harness's order — hand, field and backrow, library, graveyard —
// so the ids follow the setup literal. The state is plain JSON (SURFACE §5.1), so the layout is
// written into it here, and every card laid out is a copy of one the engine dealt, so it carries
// every field the engine gives a new card. Nothing here decides a rule: the engine plays, lists and
// views every step.
// ---------------------------------------------------------------------------------------------

type GameState = Parameters<typeof reduce>[0];

/** A card as the state holds it: what a layout writes and a case reads, the rest untouched. */
type Instance = {
  id: string;
  defId: string;
  owner: PlayerId;
  controller: PlayerId;
  zone: { z: string; player: PlayerId; row?: Row; lane?: number };
  position?: "ATK" | "DEF";
  faceUp?: boolean;
  knownAs?: { defId: string; radiant: boolean };
  [field: string]: unknown;
};

type Side = {
  hero: { health: number; armor: number };
  mana: { current: number; max: number; nextTurnMod: number; permMod: number };
  hand: Instance[];
  library: Instance[];
  graveyard: Instance[];
  units: (Instance[] | null)[];
  backrow: (Instance | null)[];
  turnsStarted: number;
};

/** The fields of the state a layout writes. */
type Board = { active: PlayerId; turn: number; phase: string; nextId: number; players: Record<PlayerId, Side> };

type Row = "units" | "backrow";

/** A card on the field: its def, and optionally its lane (1-based) and, in the backrow, its face. */
type FieldSpec = { def: string; lane?: number; faceUp?: boolean };
type FieldEntry = string | FieldSpec;

type SideSetup = {
  hand?: readonly string[];
  field?: readonly FieldEntry[];
  backrow?: readonly FieldEntry[];
  library?: readonly string[];
  graveyard?: readonly string[];
  health?: number;
  mana?: number;
};

type ScenarioOptions = { active?: PlayerId; p1?: SideSetup; p2?: SideSetup };

type Scenario = {
  readonly state: GameState;
  view(player?: PlayerId): PlayerView;
  unit(player: PlayerId, lane: number): Instance | null;
  backrow(player: PlayerId, lane: number): Instance | null;
  hand(player?: PlayerId): Instance[];
  /** A play from the active side's hand, by instance or by def id; `zone` is the lane, its row the def's. */
  play(card: string | Instance, opts?: { zone?: number }): Scenario;
};

const SEED = "jackioh-harness";
/** A mid-game board: the active side has started ceil(9/2) = 5 turns and the other 4. */
const TURN = 9;
const PLAYERS = ["p1", "p2"] as const;

const board = (state: GameState): Board => state as unknown as Board;
const engineState = (layout: Board): GameState => layout as unknown as GameState;
const other = (player: PlayerId): PlayerId => (player === "p1" ? "p2" : "p1");

function defOf(defId: string): { type: string; name: string } {
  const def = registeredCatalog()[defId];
  if (def === undefined) throw new Error(`no catalog card "${defId}"`);
  return def;
}

/**
 * Two legal decks for `createGame`: the first DECK_SIZE non-token Core cards in §5 index order, as
 * the harness's filler was. Both libraries are emptied straight after, so what they hold is moot.
 */
function fillerDeck(): string[] {
  return Object.values(registeredCatalog())
    .filter((def) => def.set === "Core" && !def.token && !def.tags.includes("Token"))
    .sort((a, b) => Number(a.index) - Number(b.index))
    .slice(0, DECK_SIZE)
    .map((def) => def.id);
}

/** §3.2: a Unit goes in the unit zones, a Field Spell, Trap or Field Trap in the backrow. */
function rowOf(defId: string): Row {
  const def = defOf(defId);
  if (def.type === "Spell") throw new Error(`"${def.name}" is a Spell; a Spell is never on the field (§3.2)`);
  return def.type === "Unit" ? "units" : "backrow";
}

/** Entries naming a lane keep it; the rest take the leftmost lane still free in their row, in list order. */
function assignLanes<T extends { lane?: number }>(entries: readonly T[]): Map<T, number> {
  const taken = new Set(entries.flatMap((entry) => (entry.lane === undefined ? [] : [entry.lane])));
  const lanes = new Map<T, number>();
  let next = 1;
  for (const entry of entries) {
    if (entry.lane !== undefined) {
      lanes.set(entry, entry.lane);
      continue;
    }
    while (taken.has(next)) next += 1;
    taken.add(next);
    lanes.set(entry, next);
  }
  return lanes;
}

function layOut(options: ScenarioOptions): GameState {
  const deck = fillerDeck();
  const layout = board(createGame({ seed: SEED, decks: [deck, deck] }));
  const template = layout.players.p1.library[0];
  if (template === undefined) throw new Error("createGame dealt no library");

  for (const player of PLAYERS) layout.players[player].library = [];
  layout.nextId = 1;
  const active = options.active ?? "p1";
  layout.active = active;
  layout.turn = TURN;
  layout.phase = "main";
  layout.players[active].turnsStarted = Math.ceil(TURN / 2);
  layout.players[other(active)].turnsStarted = Math.floor(TURN / 2);

  /**
   * A new card, numbered next: a copy of one the engine dealt (the state is JSON-only, so a JSON
   * round trip copies it faithfully), with only its identity and its zone changed.
   */
  const card = (defId: string, owner: PlayerId, zone: Instance["zone"]): Instance => {
    const copy = JSON.parse(JSON.stringify(template)) as Instance;
    const fresh: Instance = { ...copy, id: `c${String(layout.nextId)}`, defId, owner, controller: owner, zone };
    delete fresh.knownAs;
    layout.nextId += 1;
    return fresh;
  };

  for (const player of PLAYERS) {
    const setup = options[player] ?? {};
    const side = layout.players[player];
    for (const defId of setup.hand ?? []) side.hand.push(card(defId, player, { z: "hand", player }));

    // `field` and `backrow` are one board: each entry's row is its def's, and the lanes are handed
    // out per row, `field` entries first.
    const entries = [...(setup.field ?? []), ...(setup.backrow ?? [])].map((entry) => {
      const fields: FieldSpec = typeof entry === "string" ? { def: entry } : entry;
      return { ...fields, row: rowOf(fields.def) };
    });
    const lanes = new Map([
      ...assignLanes(entries.filter((entry) => entry.row === "units")),
      ...assignLanes(entries.filter((entry) => entry.row === "backrow")),
    ]);
    for (const entry of entries) {
      const lane = lanes.get(entry);
      if (lane === undefined) throw new Error(`${player}: no lane for "${entry.def}"`);
      const placed = card(entry.def, player, { z: "field", player, row: entry.row, lane });
      if (entry.row === "units") {
        placed.position = "ATK";
        side.units[lane - 1] = [placed];
      } else {
        // R33: a Trap is face-down until it fires, and `faceUp: false` is written as given.
        if (entry.faceUp !== undefined) placed.faceUp = entry.faceUp;
        side.backrow[lane - 1] = placed;
      }
    }

    // R311: a library stands for its owner's deck, which they know card by card, on its base face.
    for (const defId of setup.library ?? []) {
      const inLibrary = card(defId, player, { z: "library", player });
      inLibrary.knownAs = { defId, radiant: false };
      side.library.push(inLibrary);
    }
    for (const defId of setup.graveyard ?? []) side.graveyard.push(card(defId, player, { z: "graveyard", player }));

    // §2.3: max mana is the turns started, capped at MAX_MANA, and the refresh fills it; `mana` sets
    // current only.
    side.mana.max = Math.max(0, Math.min(side.turnsStarted, MAX_MANA) + side.mana.permMod);
    side.mana.current = Math.max(0, side.mana.max + side.mana.nextTurnMod);
    side.mana.nextTurnMod = 0;
    if (setup.mana !== undefined) side.mana.current = setup.mana;
    if (setup.health !== undefined) side.hero.health = setup.health;
  }
  return engineState(layout);
}

/** Build a board, then step it with the engine's own `reduce`; every refusal throws its message. */
function scenario(options: ScenarioOptions = {}): Scenario {
  let state = layOut(options);
  let step = 0;
  const live = (): Board => board(state);

  const self: Scenario = {
    get state() {
      return state;
    },
    view: (player) => viewFor(state, player ?? live().active),
    unit: (player, lane) => live().players[player].units[lane - 1]?.[0] ?? null,
    backrow: (player, lane) => live().players[player].backrow[lane - 1] ?? null,
    hand: (player) => [...live().players[player ?? live().active].hand],
    play(ref, opts = {}) {
      const hand = live().players[live().active].hand;
      const inst = hand.find((held) => (typeof ref === "string" ? held.defId === ref || held.id === ref : held.id === ref.id));
      if (inst === undefined) throw new Error(`play: ${typeof ref === "string" ? ref : ref.id} is not in ${live().active}'s hand`);
      step += 1;
      const body = {
        type: "play",
        playerId: inst.controller,
        instanceId: inst.id,
        ...(opts.zone === undefined ? {} : { zone: { row: rowOf(inst.defId), lane: opts.zone } }),
        nonce: `h${String(step)}`,
      } as Action;
      const result = reduce(state, body);
      if (result.error !== undefined) throw new Error(`${result.error} — play ${inst.id} ${inst.defId}`);
      state = result.state;
      return self;
    },
  };
  return self;
}

beforeEach(() => {
  __resetSettingsForTests();
});

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
});

/** `<Game/>` on `player`'s real view and legal list; returns the spy every control reports to. */
function renderReal(s: Scenario, player: PlayerId = "p1") {
  const legal = legalActions(s.state, player);
  const onAction = vi.fn<(body: ActionBody) => void>();
  render(
    <CatalogContext.Provider value={lookup}>
      <Game view={s.view(player)} legal={legal} onAction={onAction} />
    </CatalogContext.Provider>,
  );
  return { legal, onAction };
}

/** The one body the controls sent, which the engine must list and accept. */
function sent(s: Scenario, legal: readonly ActionBody[], onAction: ReturnType<typeof vi.fn>, player: PlayerId = "p1"): ActionBody {
  expect(onAction).toHaveBeenCalledTimes(1);
  const body = onAction.mock.calls[0]?.[0] as ActionBody;
  expect(legal).toContainEqual(body);
  expect(reduce(s.state, { ...body, playerId: player, nonce: "v020-real" } as Action).error).toBeUndefined();
  return body;
}

describe("the Activate control on real views", () => {
  it("C #23 Devil's Pact: the Field Spell's one ability is a control whose press sends the listed activate", () => {
    const s = scenario({ p1: { backrow: ["classic-023"], field: ["core-008"], mana: 10 }, p2: { field: ["core-008"] } });
    const pact = s.backrow("p1", 1);
    if (pact === null) throw new Error("Devil's Pact is not in the backrow");
    const { legal, onAction } = renderReal(s);

    expect(el(testid.activate(pact.id))).toHaveAttribute("data-legal", "true");
    fireEvent.click(el(testid.activate(pact.id)));
    expect(sent(s, legal, onAction)).toEqual({ type: "activate", instanceId: pact.id, ability: "pact" });
  });

  it("C #21 Turtinator: the Tribute is forced (R683 excludes itself), so the press asks for the target and sends a body the engine lists", () => {
    const s = scenario({ p1: { field: ["classic-021", "core-011"] }, p2: { field: ["core-008"] } });
    const turtle = s.unit("p1", 1);
    const timmy = s.unit("p1", 2);
    if (turtle === null || timmy === null) throw new Error("the board is not set");
    const { legal, onAction } = renderReal(s);

    // Only Timmy can pay the Tribute, so no tribute picker opens: the target comes first and the
    // forced Tribute rides along in the sent body.
    fireEvent.click(el(testid.activate(turtle.id)));
    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "target");
    fireEvent.click(el(testid.hero("opponent")));
    expect(sent(s, legal, onAction)).toEqual(
      expect.objectContaining({ type: "activate", instanceId: turtle.id, ability: "eat", tributes: [timmy.id], targets: [{ pick: "hero", player: "p2" }] }),
    );
  });
});

describe("the E18 pickers on real views", () => {
  it("C #18 Glitch in the System: the play's number is a number picker from 0 to 10, and the pick sends that play", () => {
    const s = scenario({ p1: { hand: ["classic-018"], field: ["core-012"] }, p2: { field: ["core-008"] } });
    const glitch = s.hand("p1")[0];
    if (glitch === undefined) throw new Error("Glitch is not in hand");
    const { legal, onAction } = renderReal(s);

    fireEvent.click(el(testid.handCard(glitch.id)));
    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "number");
    expect(screen.getAllByTestId(/^prompt-option-/).map((option) => option.textContent)).toEqual(
      ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"],
    );
    fireEvent.click(el("prompt-option-2"));
    expect(sent(s, legal, onAction)).toEqual({ type: "play", instanceId: glitch.id, modes: ["2"] });
  });

  it("C #44 Back from the GY: the budgeted pick shows the engine's budget and costs, and its answer is accepted", () => {
    const s = scenario({
      p1: { hand: ["classic-044", "core-008"], graveyard: ["core-002", "core-019", "core-025"] },
      p2: { hand: ["core-008"] },
    });
    s.play("classic-044");
    const pending = s.view("p1").pending;
    if (pending === null || !pending.forYou) throw new Error("no pick is open for p1");
    expect(pending.kind).toBe("pick");
    const { legal, onAction } = renderReal(s);

    expect(el("prompt-modal")).toHaveAttribute("data-prompt-kind", "pick");
    expect(el("prompt-budget").textContent).toBe(`(0) of (${String(pending.budget)}) spent`);
    const cheapest = [...pending.options].sort((a, b) => (a.cost ?? 0) - (b.cost ?? 0))[0];
    if (cheapest === undefined) throw new Error("the pick offers nothing");
    fireEvent.click(el(`prompt-option-${cheapest.key}`));
    expect(el("prompt-budget").textContent).toBe(`(${String(cheapest.cost)}) of (${String(pending.budget)}) spent`);
    fireEvent.click(el("prompt-submit"));
    expect(sent(s, legal, onAction)).toEqual(expect.objectContaining({ type: "answer", selection: [{ pick: "instance", instanceId: cheapest.instanceId }] }));
  });
});

describe("Animated on real views", () => {
  it("C #5 Tesla: once it animates into a unit zone, both seats see it there, wearing the Animated treatment", () => {
    const s = scenario({
      active: "p2",
      p1: { hand: ["core-010"], backrow: [{ def: "classic-005", faceUp: false, lane: 3 }], health: 20, library: ["core-008", "core-008"] },
      p2: { hand: ["core-008", "core-010"], library: ["core-008", "core-008"] },
    });
    const vanilla = s.hand("p2")[0];
    if (vanilla === undefined) throw new Error("p2 holds nothing to play");
    s.play(vanilla, { zone: 2 });
    const tesla = s.unit("p1", 3);
    if (tesla === null || tesla.defId !== "classic-005") throw new Error("Tesla did not animate");

    for (const seat of ["p1", "p2"] as const) {
      renderReal(s, seat);
      const card = el(testid.card(tesla.id));
      expect(card.querySelector('[data-keyword-fx="Animated"]'), `${seat} sees the Animated cog`).not.toBeNull();
      cleanup();
    }
  });
});
