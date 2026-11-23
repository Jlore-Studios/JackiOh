// R3778000: Meditative #65 Keymaster Keenus's "Aura: Has {keywords} random combat keywords" follows its
// number, against the REAL engine. Keenus is played, then a Book of Buff or a Book of Nerf (its `times`
// tuned down to one) is cast on it; the seed is the first whose one Upgrade or Degrade draws the Number
// row. The board renders the view's keywords (CLAUDE.md rule 7): a Buff adds a 6th mark and keeps the 5,
// a Nerf takes one off.

import { CATALOG } from "@jackioh/cards";
import { DECK_SIZE } from "@jackioh/engine/config";
import type { Action, ActionBody, PlayerId, PlayerView } from "@jackioh/shared";
import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createGame, legalActions, reduce, registeredCatalog, viewFor } from "../wasm/index.ts";
import { closeInspect } from "../cards/index.ts";
import { __resetSettingsForTests } from "../settings/index.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import Game from "./Game.tsx";

type GameState = Parameters<typeof reduce>[0];

/** The fields of the state this file writes; everything else is the engine's, untouched. */
type Instance = { id: string; defId: string; radiant: boolean; zone: { z: string; player: PlayerId }; [field: string]: unknown };
type Side = { mana: { current: number; max: number }; hand: Instance[]; library: Instance[] };
type Board = { active: PlayerId; turn: number; phase: string; players: Record<PlayerId, Side> };

const lookup = lookupFromDefs(CATALOG);
const KEENUS = "meditative-065";
const BOOK_OF_BUFF = "classicplus-071";
const BOOK_OF_NERF = "classicplus-072";
/** A mid-game turn with mana for both. */
const TURN = 9;
const MANA = 10;
/** How many seeds the search tries for one whose Buff or Nerf draws the Number row. */
const SEEDS = 64;

function fillerDeck(): string[] {
  return Object.values(registeredCatalog())
    .filter((def) => def.set === "Core" && !def.token && !def.tags.includes("Token"))
    .sort((a, b) => Number(a.index) - Number(b.index))
    .slice(0, DECK_SIZE)
    .map((def) => def.id);
}

/** p1's hand is Keenus (`c1`) and the Book (`c2`), its `times` 4 tuned to 1 (R386); each a copy of a dealt card. */
function board(seed: string, book: string): GameState {
  const deck = fillerDeck();
  const state = createGame({ seed, decks: [deck, deck] });
  const layout = state as unknown as Board;
  const template = layout.players.p1.library[0];
  if (template === undefined) throw new Error("createGame dealt no library");
  const copy = (id: string, defId: string, extra: Record<string, unknown> = {}): Instance => {
    const card: Instance = {
      ...(JSON.parse(JSON.stringify(template)) as Instance),
      id,
      defId,
      radiant: false,
      zone: { z: "hand", player: "p1" },
      ...extra,
    };
    delete card.knownAs;
    return card;
  };
  for (const player of ["p1", "p2"] as const) {
    layout.players[player].library = [];
    layout.players[player].hand = [];
  }
  layout.players.p1.hand.push(copy("c1", KEENUS), copy("c2", book, { tuning: { numbers: { times: -3 } } }));
  layout.active = "p1";
  layout.turn = TURN;
  layout.phase = "main";
  layout.players.p1.mana.max = MANA;
  layout.players.p1.mana.current = MANA;
  return state;
}

function applied(state: GameState, body: ActionBody | undefined): GameState {
  if (body === undefined) throw new Error("no legal play to send");
  const result = reduce(state, { ...body, playerId: "p1", nonce: `3778-${JSON.stringify(body)}` } as Action);
  expect(result.error).toBeUndefined();
  return result.state;
}

function keenusOf(view: PlayerView) {
  const unit = view.you.units.find((candidate) => candidate?.instanceId === "c1");
  if (unit === undefined || unit === null) throw new Error("Keenus is not on p1's field");
  return unit;
}

/** The Aura's keywords in the view: every keyword Keenus has but its printed Equip. */
function auraKinds(view: PlayerView): string[] {
  return keenusOf(view)
    .keywords.map((keyword) => keyword.kind)
    .filter((kind) => kind !== "Equip");
}

/** Keenus played, then the Book cast on it, on the first seed whose one change moves `keywords` to `want`. */
function tuned(book: string, want: number): { before: string[]; state: GameState } {
  for (let n = 0; n < SEEDS; n += 1) {
    let state = board(`jackioh-3778-${n}`, book);
    state = applied(
      state,
      legalActions(state, "p1").find((body) => body.type === "play" && body.instanceId === "c1"),
    );
    const before = auraKinds(viewFor(state, "p1"));
    state = applied(
      state,
      legalActions(state, "p1").find(
        (body) => body.type === "play" && body.instanceId === "c2" && JSON.stringify(body).includes('"c1"'),
      ),
    );
    if (keenusOf(viewFor(state, "p1")).params?.keywords === want) return { before, state };
  }
  throw new Error(`no seed among ${SEEDS} moves Keenus's keywords to ${want}`);
}

/** The keyword marks the board draws for Keenus, on its face or in its zone's corner (#2932). */
function renderedKinds(state: GameState): string[] {
  render(
    <CatalogContext.Provider value={lookup}>
      <Game view={viewFor(state, "p1")} legal={[]} onAction={vi.fn()} />
    </CatalogContext.Provider>,
  );
  const card = screen.getByTestId(testid.card("c1"));
  const zone = card.parentElement ?? card;
  return [...zone.querySelectorAll(".keyword-icon")].map((mark) => mark.getAttribute("data-keyword") ?? "");
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

describe("R3778000 the Aura's random combat keywords follow its number on the board", () => {
  it("R3778000 a Buff of Keymaster Keenus's number shows a 6th keyword and keeps the 5", () => {
    const { before, state } = tuned(BOOK_OF_BUFF, 6);
    expect(before).toHaveLength(5);
    const after = auraKinds(viewFor(state, "p1"));
    expect(after).toHaveLength(6);
    for (const kind of before) expect(after, kind).toContain(kind);
    const marks = renderedKinds(state);
    for (const kind of after) expect(marks, kind).toContain(kind);
  });

  it("R3778000 a Nerf of Keymaster Keenus's number takes one keyword off the card", () => {
    const { before, state } = tuned(BOOK_OF_NERF, 4);
    expect(before).toHaveLength(5);
    const after = auraKinds(viewFor(state, "p1"));
    expect(after).toHaveLength(4);
    for (const kind of after) expect(before, kind).toContain(kind);
    const gone = before.filter((kind) => !after.includes(kind));
    expect(gone).toHaveLength(1);
    expect(gone[0]).toBe(before[4]);
    const marks = renderedKinds(state);
    for (const kind of after) expect(marks, kind).toContain(kind);
    expect(marks).not.toContain(gone[0]);
  });
});
