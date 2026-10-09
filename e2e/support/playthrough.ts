// Spec 36's driver (#552, MN09): a hotseat game played by `window.__jackioh` alone, one step at a time,
// so every card of a set can be played once through the browser without a line of card knowledge.
//
// Each step reads the dev handle (the raw state only for whose move it is, as the AI's `seat_to_act`
// does; everything else from the seat's own `viewFor` and `legalActions`) and does exactly one thing:
//
//   - the device goes to the seat that owes the move, if another holds it;
//   - a mulligan still owed is answered keeping the whole hand;
//   - a prompt the seat holds is answered with its first legal answer (`legalActions` lists a
//     prompt's answers, R211, so every kind is answered with no picker logic here);
//   - a card still to be played that the seat holds and may play is played, its first legal play;
//   - otherwise the turn ends.
//
// Concede and the draw offer are listed among the legal actions at every moment and are never taken.
// `reduce` still refuses anything illegal (SPEC §9.3), so the driver can reach no state a player
// could not: dispatching is a way past the UI, never past the rules. The spec around it waits on the
// DOM between steps (`cy.settled()`) and asserts what the board, the log and the inspect view show.

import type { ActionBody, JackiOhDevHandle, PlayerId } from "./types.ts";

/** What one step did. A hand-over only moves the device, so the next step reads a board drawn for that seat. */
export type PlayStep =
  | { kind: "handOver"; seat: PlayerId }
  | { kind: "mulligan"; seat: PlayerId }
  | { kind: "answer"; seat: PlayerId; prompt: string; promptKind: string; shown: string | null; error: string | null }
  | { kind: "play"; seat: PlayerId; defId: string; instanceId: string; error: string | null }
  /** `held`: the cards still to play in the seat's hand; `deck`: the cards left in its deck. */
  | { kind: "end"; seat: PlayerId; turn: number; held: string[]; deck: number }
  | { kind: "over"; why: string };

/** The parts of `viewFor` the driver reads (a structural subset of `PlayerView`). */
type ViewLike = {
  pending?: { forYou?: boolean; kind?: string; prompt?: string } | null;
  you: { hand: { instanceId: string; defId: string }[] | { count: number }; libraryCount?: number };
};

type StateLike = {
  active: PlayerId;
  turn: number;
  result: unknown;
  pending: { playerId?: PlayerId } | null;
  mulligan?: Record<PlayerId, { keep: string[] | null } | undefined>;
};

type Handle = Required<Pick<JackiOhDevHandle, "view" | "legal" | "setSeat">> & JackiOhDevHandle;

/** The seat that owes the next move: a mulligan still owed, else the prompt's holder, else the active seat. */
export function seatToAct(state: StateLike): PlayerId {
  if (state.mulligan !== undefined) {
    const owing = (["p1", "p2"] as const).find((seat) => state.mulligan?.[seat]?.keep === null);
    if (owing !== undefined) return owing;
  }
  return state.pending?.playerId ?? state.active;
}

function dispatched(handle: Handle, body: ActionBody, seat: PlayerId): string | null {
  const result = handle.dispatch({ ...body, playerId: seat }) as unknown as { error?: string } | undefined;
  return result?.error ?? null;
}

/**
 * One step of the game, as the header says. `toPlay` is the set of definition ids still to be played;
 * the step plays one of them when it can and never anything else. `doc` is read for the open prompt's
 * `data-prompt-kind`, so the spec can check the board showed the prompt the view holds.
 */
export function playStep(handle: Handle, toPlay: ReadonlySet<string>, doc: Document): PlayStep {
  const state = handle.state as unknown as StateLike;
  if (state.result !== null && state.result !== undefined) return { kind: "over", why: "the game is over" };
  const seat = seatToAct(state);
  if (handle.seat !== seat) {
    handle.setSeat(seat);
    return { kind: "handOver", seat };
  }
  const view = handle.view() as unknown as ViewLike;
  const legal = handle.legal();

  if (state.mulligan !== undefined && state.mulligan[seat]?.keep === null) {
    const keepAll = legal
      .filter((body): body is Extract<ActionBody, { type: "mulligan" }> => body.type === "mulligan")
      .sort((a, b) => b.keep.length - a.keep.length)[0];
    if (keepAll === undefined) return { kind: "over", why: `${seat} owes a mulligan and has no answer` };
    dispatched(handle, keepAll, seat);
    return { kind: "mulligan", seat };
  }

  if (view.pending?.forYou === true) {
    const answer = legal.find((body) => body.type === "answer");
    if (answer === undefined) return { kind: "over", why: `${seat}'s ${view.pending.kind ?? "?"} prompt lists no answer` };
    const shown = doc.querySelector('[data-testid="prompt-modal"]')?.getAttribute("data-prompt-kind") ?? null;
    const error = dispatched(handle, answer, seat);
    return { kind: "answer", seat, prompt: view.pending.prompt ?? "", promptKind: view.pending.kind ?? "", shown, error };
  }

  const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
  for (const card of hand) {
    if (!toPlay.has(card.defId)) continue;
    const play = legal.find((body) => body.type === "play" && body.instanceId === card.instanceId);
    if (play === undefined) continue;
    const error = dispatched(handle, play, seat);
    return { kind: "play", seat, defId: card.defId, instanceId: card.instanceId, error };
  }

  const end = legal.find((body) => body.type === "endTurn");
  if (end === undefined) return { kind: "over", why: `${seat} can neither play nor end the turn` };
  const held = hand.filter((card) => toPlay.has(card.defId)).map((card) => card.defId);
  dispatched(handle, end, seat);
  return { kind: "end", seat, turn: state.turn, held, deck: view.you.libraryCount ?? 0 };
}

/* ------------------------------------------------------------------------------- the decks */

/** The fields of a catalog entry (`crates/cards/catalog.json`) the decks are built from. */
export type CatalogRow = {
  id: string;
  name: string;
  set: string;
  type: string;
  token?: boolean;
  tags: string[];
  cost: number | "X" | { base: number; embiggen: number };
  base: { text: string };
};

/** R1420: the cards of `set` a deck may hold once the set is previewed: every entry but its tokens, in catalog order. */
export function deckableOf(catalog: Readonly<Record<string, CatalogRow>>, set: string): CatalogRow[] {
  return Object.values(catalog).filter((def) => def.set === set && def.token !== true && !def.tags.includes("Token"));
}

/**
 * A card no player ever plays from a hand: it replaces itself the moment it enters one (Meditative #37
 * CN in a bottle, MD-B19, R925, R926). Its turn in the play-through is that replacement.
 */
export function replacedOnArrival(def: CatalogRow): boolean {
  return /^When this enters your hand:/.test(def.base.text);
}

/** A seat's share of one game: few enough Units and backrow cards that its zones never fill. */
export const SEAT_SHARE = { units: 3, backrow: 2, cards: 7 } as const;

function share(def: CatalogRow): "units" | "backrow" | "spells" {
  if (def.type === "Unit") return "units";
  return def.type === "Spell" ? "spells" : "backrow";
}

/**
 * The next game's two decks out of the cards still to play, in catalog order, each seat within
 * SEAT_SHARE. A seat left with nothing holds `filler`, a card of a set that ships, so every deck has a card.
 */
export function nextGame(cards: readonly CatalogRow[], filler: CatalogRow): [CatalogRow[], CatalogRow[]] {
  const seats: [CatalogRow[], CatalogRow[]] = [[], []];
  for (const def of cards) {
    const seat = seats.find((held) => {
      const kind = share(def);
      const cap = kind === "spells" ? SEAT_SHARE.cards : SEAT_SHARE[kind];
      return held.length < SEAT_SHARE.cards && held.filter((other) => share(other) === kind).length < cap;
    });
    seat?.push(def);
  }
  if (seats[1].length === 0) seats[1].push(filler);
  return seats;
}

/** A card's printed price out of play (R65): an X card's is 0, an Embiggen card's its base price. */
export function printedCost(def: CatalogRow): number {
  if (typeof def.cost === "number") return def.cost;
  return def.cost === "X" ? 0 : def.cost.base;
}

/* ------------------------------------------------------------------------------- the words */

/** A catalog id (`core-001`, `classicplus-t-ai-01`, `meditative-039-5`): never in a line a player reads. */
const CATALOG_ID = /\b(?:core|classic|classicplus|meditative)-(?:t-)?[0-9a-z]+(?:-[0-9a-z]+)?\b/;
/** What a slip upstream prints: a value with no words. */
const LEAKS = /\bundefined\b|\bNaN\b|\bnull\b|\[object /;
/** A placeholder a text never filled ("{chance}"); a computed value a face prints in play, "{7}", is R280's. */
const UNFILLED = /\{[A-Za-z_][^}]*\}/;
/** An engine identifier: camelCase (an event or key, `jadeChanged`) or snake_case (`set_hand_cap`). */
const IDENTIFIER = /\b[a-z]+[A-Z][A-Za-z]*\b|\b[a-z]+_[a-z_]+\b/;

/**
 * Every way one line of the log fails to read as English: it opens with a capital (or a card's name
 * in the Chinese a card shown in Chinese is named by, R1301), and prints no id, no engine identifier
 * and no unfilled value. Empty means it reads.
 */
export function englishProblems(line: string, where: string): string[] {
  const problems: string[] = [];
  const text = line.trim();
  if (text === "") return [`${where}: an empty line`];
  if (!/^[A-Z0-9㐀-鿿]/.test(text)) problems.push(`${where}: "${text}" does not open with a capital`);
  if (CATALOG_ID.test(text)) problems.push(`${where}: "${text}" prints a card id`);
  if (LEAKS.test(text) || /[{}]/.test(text)) problems.push(`${where}: "${text}" prints an unfilled value`);
  if (IDENTIFIER.test(text)) problems.push(`${where}: "${text}" prints an engine identifier`);
  return problems;
}

/** The same for an inspect view's text, which is a card's face and notes rather than a sentence. */
export function noteProblems(text: string, where: string): string[] {
  const problems: string[] = [];
  if (text.trim() === "") problems.push(`${where}: the inspect view is empty`);
  if (CATALOG_ID.test(text)) problems.push(`${where}: the inspect view prints a card id`);
  if (LEAKS.test(text) || UNFILLED.test(text)) problems.push(`${where}: the inspect view prints an unfilled value`);
  return problems;
}
