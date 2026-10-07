// Fuzz-level invariants for summoning sickness, exertion, game over and hidden information (SPEC §4.1,
// §4.2, §10.3, §10.8, R53, R83, R171; docs/polish/4-edge-cases.md "Fuzz invariants"). `fuzz.test.ts` runs one monitor per game,
// and the edge-case hunters reuse it.
//
// THE MONITOR IS AN ORACLE, NOT THE FIX RESTATED. It never reads `summonedTurn` to decide who is
// sick. It keeps a shadow built only from the event stream:
//   - `turn`, from `turnStarted`;
//   - `entered`, the turn of each instance's latest entry, and `stint`, how many entries it has had.
//     An entry event is `cardPlayed`, `summoned`, `controlChanged`, `animated` (an Animated card
//     stepping from its backrow zone into a unit zone enters it on that turn, R383), or a
//     `transformed` whose new instance differs from the old. `fused` keeps the target's entry (R77), and a move along one
//     side or a Stack card resuming emits nothing and changes nothing;
//   - `lastAttack`, the turn and stint of each instance's latest declared (not forced) attack;
//   - `readied`, the Transform results R424 lets attack on the turn they entered: an attacker that
//     destroyed the Unit its declared attack targeted (`destroyed` naming it the killer, R42) and is
//     then `transformed` (Classic+ #73.1 Classic Golem) passes "may attack again this turn" to the new
//     instance, so that instance is not sick (I1) and carries no `summonedTurn` (I4a) for that stint.
//
// The six checks:
//   I1 no sick attack is ever offered (§4.1, §6.1, R83, R171). In the main phase with no prompt
//      open, a unit of the acting player that entered on this turn has no attack target unless it
//      has Rush or Charge, and no hero target unless it has Charge; the chosen `attack`, if any, is
//      checked the same way. It reads `attackTargets`, which is what `legalActions` enumerates with.
//   I2 one declared attack per stint per turn, two for a unit that had Windfury in that stint (R636).
//      A unit that re-entered may attack again within its keywords (R83, R171); forced attacks are not
//      declarations and are skipped (R53).
//   I3 every card on the field arrived by an event (§10.3: every visible state change emits one).
//   I4 the bookkeeping matches the shadow (white-box R171): (a) `summonedTurn` is the turn of the
//      latest entry, or absent on a `readied` stint (R424); (b) a spent attack exertion belongs to
//      the current stint, so an exertion left spent across an entry trips it.
//   I5 nothing happens after the game is over (§2.5, R216): `gameOver` is the last event an action
//      emits. Added in the hunt's fourth round, whose engine-invariants lens found the rest of an
//      effect list, and a trap's consumption, resolving after the check that ended the game.
//   I6 no seat is sent a card it may not read (§9.1, §10.8, R97, R177, issue #348). On every state the
//      fuzz reaches, setup's included, for both seats: `viewFor` and `legalActions` do not throw, and
//      neither names a hidden card. Hidden is read off the state's zones on their own terms, never
//      from `viewFor`'s rules: the other seat's hand while the game is live (R434), both libraries,
//      the other seat's face-down traps (a dormant one under a backrow top too, B5 E21), a card the other seat is setting face-down (R448) and a
//      mulligan return waiting for its shuffle (R224). The serialized view may not hold such a card's
//      instance id, nor a former id it had (R227's `formerId`, a transform, a fuse, read off the log's
//      raw events), nor its definition id unless a card the seat reads carries the same one: a card
//      the state holds outside the hidden set, or one the raw events name that is readable where it
//      is now, and the seat's own library only as `knownAs` records it (R311, R312). Never through the
//      view being checked, which could pair a hidden definition with any id it likes; the
//      seat's own prompt may offer its options (§10.8), and `legalActions` may name a face-down trap
//      by its bare id as a target (R177) and a hidden card by nothing else. The first run found one
//      leak, R763 (`trapFired` named a fired trap to its controller after it was shuffled into a
//      library), and five false positives, each fixed here and not in the engine:
//        - R466's `stolen`: a card its viewer read where it was taken is named to them openly;
//        - a `defs` body (R243): copied only for an id the rest of the view names, and it names its
//          ingredients and its `refs` (R279), the text of cards the seat reads;
//        - a definition a hidden card shares with a readable one (a token, a copy, a Book, a fused
//          card) is no secret, so a definition counts only while nothing readable carries it;
//        - Echo's `copies` (R399) names the definition of the card it copied (read off the copier);
//        - `swapsBook.from` (R671) names the Book a swap took the text from (read off the card).
//      A card in no pile that has no successor (no Transform or Fuse made it another, R177) reads only
//      when it is a token (R11); a Glitch's reset or boards (R676, R678) takes the rest unseen, and
//      the first oracle excused it, which was the engine's own R97 rule restated: R764 fixed it.
//      Judging former ids by the log found a sixth, fuzz seed 992: R419's Rollback recreated a
//      face-down trap #83 had transformed away, and the `formerId` it carries goes with the card
//      (R227) once that card is public, R177's mark on the old id notwithstanding.
//      `I6_GATE_STRIDE` is how often the 1,000-seed gate runs it.
//
// Every message leads with its id, names the instance, its def and the turn, and cites the SPEC
// reference, so the fuzz report's `signatureOf` groups one bug into one entry.

import type { ActionBody, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { PLAYER_IDS, hasKeyword, opponentOf } from "@jackioh/shared";
import {
  WINDFURY_ATTACKS,
  activeUnitsOf,
  announceOf,
  attackTargets,
  cardTypeOf,
  defOf,
  enchantmentsOfKind,
  findInstance,
  legalActions,
  mulliganPromptFor,
  returnedAwaitingShuffle,
  subsystems,
  unitView,
  viewFor,
  type CardInstance,
  type GameState,
} from "@jackioh/engine";

/**
 * I6 under the 1,000-seed gate: every this-many-th state, setup's and the last always (the monitor
 * budget of docs/polish/4-edge-cases.md: within 25% of the time without it). `pnpm test`'s 100-seed
 * waves check every state.
 *
 * Measured on 2026-10-06 (four cores, idle). `pnpm fuzz`, seeds 1–1000, both files, before I6 and
 * with no monitor in fuzz-handicap: 342 s. With I1–I6 in both files and every state checked: 502 s
 * (×1.47, over the budget, and 0 violations). With this stride: 386 s (×1.13). Once definitions
 * were judged through the state and the log rather than the view: 411 s (×1.20), and with a card in no pile
 * judged hidden unless it is a token: 419 s (×1.23).
 * `pnpm test --project cards fuzz`, seeds 1–100, every state: 38 s before, 55 s after (×1.45).
 */
export const I6_GATE_STRIDE = 5;

/** What one seat may not read: instance ids, and the definitions that name a card only it hides. */
type HiddenSet = {
  /** Instance id -> where the card sits, for the message. */
  ids: Map<string, string>;
  /** Definition id -> where a hidden card of it sits. A definition any readable card carries is not here. */
  defs: Map<string, string>;
  /** Face-down traps' ids: R177 lets `legalActions` offer one as a bare target. */
  bare: Set<string>;
  /** The bare ids the viewer's own prompt offers: R177 lets the view carry each as a bare option's `instanceId`, nowhere else. */
  offeredBare: Set<string>;
};

/** R33, R686: another seat's armed Trap or Field Trap, not flipped (`faceUp`) and not revealed (R638). */
function faceDownTo(state: GameState, card: CardInstance, viewer: PlayerId): boolean {
  if (card.controller === viewer || card.faceUp === true || card.revealed === true) return false;
  const type = cardTypeOf(state, card);
  return type === "Trap" || type === "Field Trap";
}

/** R448: a card the other seat is setting face-down waits in `resolving` for its announce window. */
function setFaceDownBy(state: GameState, card: CardInstance, viewer: PlayerId): boolean {
  const record = announceOf(state, card.id);
  return record?.faceDown === true && record.player !== viewer;
}

/** R224: the cards a mulligan returned that wait in a work item for their shuffle-back: in no pile. */
function awaitingShuffle(state: GameState): CardInstance[] {
  const waiting = new Set(returnedAwaitingShuffle(state));
  return state.work.flatMap((item) => {
    const owed = item.resume.data["owed"];
    const returned: unknown = owed !== null && typeof owed === "object" ? (owed as { returned?: unknown }).returned : undefined;
    return Array.isArray(returned) ? (returned as CardInstance[]).filter((card) => waiting.has(card.id)) : [];
  });
}

/** Every card the state holds, in every pile (`findInstance`'s piles). */
function cardsOf(state: GameState): CardInstance[] {
  return PLAYER_IDS.flatMap((player) => {
    const side = state.players[player];
    const piles: readonly (CardInstance | null | undefined)[] = [
      ...side.hand,
      ...side.library,
      ...side.graveyard,
      ...side.exile,
      ...side.units.flatMap((pile) => pile ?? []),
      ...side.backrow,
      ...(side.backrowPiles ?? []).flat(),
      ...(side.carried ?? []),
      ...side.resolving,
    ];
    return piles.filter((card): card is CardInstance => card !== null && card !== undefined);
  });
}

/**
 * What the log says of the cards it names, for one viewer: the state keeps no zone for a card that
 * took a new id or ceased to exist, so the log's raw (unredacted) events are the only witness of it.
 */
type Lineage = {
  /**
   * Each vanished id's successor: R227's `formerId` (a card set face-down took a fresh id), §6.3's
   * `transformed`, R77's `fused`, and R316's `copyOf` (a copy refused before it existed is that card's).
   */
  next: Map<string, string>;
  /** R177: an id that ceased to exist where this viewer could not read it (`transformed.hiddenFrom`). */
  unread: Set<string>;
  /** Each id, and the definitions the engine's own events pair with it. */
  ties: Map<string, Set<string>>;
};

function lineageOf(state: GameState, viewer: PlayerId): Lineage {
  const next = new Map<string, string>();
  const unread = new Set<string>();
  const ties = new Map<string, Set<string>>();
  const tie = (id: unknown, ...values: unknown[]): void => {
    if (typeof id !== "string") return;
    const tied = ties.get(id) ?? new Set<string>();
    for (const def of stringsOf(...values)) tied.add(def);
    ties.set(id, tied);
  };
  for (const event of state.applied.flatMap((entry) => entry.events)) {
    walk(
      event,
      "event",
      () => undefined,
      (object) => {
        const id = object["instanceId"];
        const hiddenTo = object["hiddenFrom"];
        const unreadHere = Array.isArray(hiddenTo) && hiddenTo.includes(viewer);
        // A card transformed where the viewer could not read it was never this viewer's to read (R177).
        tie(id, object["defId"], unreadHere ? undefined : object["fromDefId"]);
        tie(object["newInstanceId"], object["toDefId"]);
        tie(object["resultInstanceId"], object["defId"]);
        if (typeof id !== "string") return;
        const former = object["formerId"];
        // R419: Classic+ #35 Rollback recreates a card transformed away under a fresh id whose
        // `formerId` is the old one, so the card exists again and R177's mark is spent: its old id
        // goes with it, a trap's that fired into exile included (fuzz seed 992).
        if (typeof former === "string") {
          next.set(former, id);
          unread.delete(former);
        }
        const copyOf = object["copyOf"];
        if (typeof copyOf === "string") next.set(id, copyOf);
        const fresh = object["newInstanceId"];
        if (object["type"] === "transformed" && typeof fresh === "string" && fresh !== id) {
          next.set(id, fresh);
          if (unreadHere) unread.add(id);
        }
      },
    );
    if (event.type === "fused") {
      for (const id of event.instanceIds) if (id !== event.resultInstanceId) next.set(id, event.resultInstanceId);
    }
  }
  return { next, unread, ties };
}

/**
 * Whether this viewer reads the card an id names, judged by where the card is now: a card the state
 * holds reads unless the hidden set has it, and a vanished one by its successor. A vanished card with
 * none reads only when every definition the log pairs with it is a token's: R11's tokens go public
 * (leaving the field, discarded, burned), where any other card in no pile went unseen, out of a
 * Glitch's reset or boards (R676, R678), and stays as hidden as it was. An id neither the state nor
 * the log knows vouches for nothing.
 */
function standingOf(
  state: GameState,
  held: ReadonlySet<string>,
  viewer: PlayerId,
  ids: ReadonlyMap<string, string>,
  lineage: Lineage,
  start: string,
): { reads: boolean; where: string } | null {
  let id = start;
  for (let hops = 0; hops <= lineage.next.size; hops += 1) {
    const where = ids.get(id);
    if (where !== undefined) return { reads: false, where };
    if (held.has(id)) return { reads: true, where: "a pile it reads" };
    if (lineage.unread.has(id)) return { reads: false, where: `a pile ${viewer} could not read when it was transformed` };
    const successor = lineage.next.get(id);
    if (successor === undefined) {
      const defs = lineage.ties.get(id);
      if (defs === undefined) return null;
      // R11: a token that ceased to exist (it left the field, was discarded or burned) was public when
      // it went. Any other card in no pile went unseen (a Glitch's reset or boards, R676, R678) and
      // stays as hidden as it was.
      const token = [...defs].every((def) => defOf(state, def).token);
      return { reads: token, where: "no pile (it ceased to exist)" };
    }
    id = successor;
  }
  return null;
}

/**
 * The hidden set, worked out from the state's zones on their own terms and never from `viewFor`'s
 * rules (the oracle must not be the fix restated): the other seat's hand while the game is live
 * (R434), both libraries (§9.1), the other seat's face-down traps (dormant ones under a backrow top
 * included, B5 E21), a card the other seat is setting face-down (R448), mulligan returns waiting for
 * their shuffle (R224), and every former id whose card is now one of these (R227, R177). Then it
 * takes out every definition the viewer reads through a card: one the state holds and the viewer
 * reads, one the log names that the viewer reads where it is now, its own library's cards as it was
 * shown them going in (R311, R312), and the options of its own prompt (§10.8, R177). Never through
 * the view being checked: a view may name a definition beside any id it likes.
 */
function hiddenFrom(state: GameState, viewer: PlayerId): HiddenSet {
  const ids = new Map<string, string>();
  const defs = new Map<string, string>();
  const bare = new Set<string>();
  const hide = (card: CardInstance, where: string, withDef: boolean): void => {
    ids.set(card.id, where);
    if (withDef) defs.set(card.defId, where);
  };
  const rival = opponentOf(viewer);
  if (state.result === null) {
    for (const card of state.players[rival].hand) hide(card, `${rival}'s hand`, true);
  }
  for (const player of PLAYER_IDS) {
    const side = state.players[player];
    // R312: the owner reads a library card's definition only as `knownAs` records it, below.
    for (const card of side.library) hide(card, `${player}'s library`, true);
    for (const card of side.backrow) {
      if (card === null || !faceDownTo(state, card, viewer)) continue;
      hide(card, `${player}'s face-down trap`, true);
      bare.add(card.id);
    }
    // B5 E21: the dormant cards under a backrow top keep their backrow zone, so a Trap among them is
    // face-down as a top one is. They are not on the field for effects, so none is a bare target.
    for (const card of (side.backrowPiles ?? []).flat()) {
      if (faceDownTo(state, card, viewer)) hide(card, `${player}'s dormant face-down trap`, true);
    }
    for (const card of side.resolving) {
      if (setFaceDownBy(state, card, viewer)) hide(card, `${player}'s card being set face-down`, true);
    }
  }
  // The viewer's own mulligan returns are its opening hand, which it read (R224).
  for (const card of awaitingShuffle(state)) hide(card, "a mulligan return awaiting its shuffle", card.owner !== viewer);

  // §10.8: the viewer's own prompt offers what it may choose among, a revealed library card included.
  const offeredBare = new Set<string>();
  const prompt = state.pending?.playerId === viewer ? state.pending : mulliganPromptFor(state, viewer);
  for (const option of prompt?.options ?? []) {
    const selection = option.selection;
    if (selection.pick === "mode") defs.delete(selection.option);
    if (selection.pick !== "instance") continue;
    // R177: a face-down trap is offered by its id alone: its id stays hidden everywhere but that
    // option's `instanceId`, and its definition stays hidden everywhere.
    if (bare.has(selection.instanceId)) {
      offeredBare.add(selection.instanceId);
      continue;
    }
    ids.delete(selection.instanceId);
    const card = findInstance(state, selection.instanceId);
    if (card !== undefined) defs.delete(card.defId);
  }

  // R227, R177: an id a card had before is that card, so it is hidden wherever the card is.
  const lineage = lineageOf(state, viewer);
  // The end of a chain counts too (a card that ceased to exist where the viewer could not read it).
  const held = new Set(cardsOf(state).map((card) => card.id));
  for (const id of new Set([...lineage.next.keys(), ...lineage.unread, ...lineage.ties.keys()])) {
    if (held.has(id)) continue;
    const standing = ids.has(id) ? null : standingOf(state, held, viewer, ids, lineage, id);
    if (standing !== null && !standing.reads) ids.set(id, `${standing.where} (a former id)`);
  }

  // A definition a card the viewer reads carries is no secret, whatever else carries it (a token, a
  // copy, a Book, a fused card): every card the state holds outside the hidden set, with the Spell
  // an Echo copies (R399) and the Book a swap took its text from (R671).
  for (const card of cardsOf(state)) {
    if (ids.has(card.id)) continue;
    defs.delete(card.defId);
    const copy = subsystems.copiedTextOf(state, card);
    if (copy !== null) defs.delete(copy.defId);
    for (const swap of enchantmentsOfKind(card, "swapsBook")) defs.delete(swap.from);
  }
  // R311, R312: the viewer's own library reads as it was shown going in, and a card never shown not at all.
  for (const card of state.players[viewer].library) if (card.knownAs !== undefined) defs.delete(card.knownAs.defId);
  // Every card the log names that the viewer reads where it is now, as the engine's events named it.
  for (const [id, tied] of lineage.ties) {
    if (standingOf(state, held, viewer, ids, lineage, id)?.reads === true) for (const def of tied) defs.delete(def);
  }
  return { ids, defs, bare, offeredBare };
}

/** R466: a `stolen` event reads openly to a viewer who could read the card in the zone it was taken from. */
function readableWhereTaken(event: Extract<GameEvent, { type: "stolen" }>, viewer: PlayerId): boolean {
  if (event.readableFrom !== undefined) return event.readableFrom.includes(viewer);
  if (event.zone === "graveyard" || event.zone === "exile" || event.zone === "resolving") return true;
  return event.zone === "hand" && event.from === viewer;
}

/** Every string and object inside `value`, with the path it was found at. Values only, never keys. */
function walk(
  value: unknown,
  path: string,
  onString: (text: string, at: string) => void,
  onObject: (object: Record<string, unknown>) => void,
): void {
  if (typeof value === "string") {
    onString(value, path);
  } else if (Array.isArray(value)) {
    value.forEach((item: unknown, at) => walk(item, `${path}[${at}]`, onString, onObject));
  } else if (value !== null && typeof value === "object") {
    const object = value as Record<string, unknown>;
    onObject(object);
    for (const [key, item] of Object.entries(object)) walk(item, `${path}.${key}`, onString, onObject);
  }
}

function stringsOf(...values: unknown[]): string[] {
  return values.filter((value): value is string => typeof value === "string");
}

/**
 * I6 for one seat on one state: what `viewFor` and `legalActions` sent it names no card it may not
 * read. A hidden card's instance id may not appear anywhere in the view, nor its definition id unless
 * a card the viewer reads carries the same one (§10.8); `legal` may offer a face-down trap's bare id
 * as a target (R177) and nothing else of a hidden card.
 */
export function hiddenInformationViolations(
  state: GameState,
  viewer: PlayerId,
  view: PlayerView,
  legal: readonly ActionBody[],
): string[] {
  const { ids, defs, bare, offeredBare } = hiddenFrom(state, viewer);

  // The view's events are the tail of the log's, in order: line them up to find a `stolen` among them.
  const raw = state.applied.flatMap((entry) => entry.events);
  const offset = raw.length - view.events.length;
  const events = view.events.filter((_, at) => {
    const source = raw[offset + at];
    return !(source !== undefined && source.type === "stolen" && readableWhereTaken(source, viewer));
  });
  // `defs` (R243) is left out: a definition is copied only for an id the rest of the view names, and
  // its body names its ingredients and its `refs` (R279), the text of cards the viewer reads.
  const { defs: _definitions, ...rest } = view;
  const scanned = { ...rest, events };

  const named = new Map<string, string>();
  walk(
    scanned,
    "view",
    (text, at) => {
      // R177: the viewer's own prompt names a face-down trap by its id, as a bare option.
      if (offeredBare.has(text) && /^view\.pending\.options\[\d+\]\.instanceId$/.test(at)) return;
      if (!named.has(text)) named.set(text, at);
    },
    () => undefined,
  );
  const offered = new Map<string, string>();
  walk(
    legal,
    "legal",
    (text, at) => {
      if (!offered.has(text)) offered.set(text, at);
    },
    () => undefined,
  );

  const found: string[] = [];
  const report = (kind: "card" | "definition", into: string, needle: string, path: string, where: string): void => {
    found.push(
      `I6 hidden ${kind} in ${into}: ${viewer} is sent "${needle}" at ${path}, a card in ${where}, on turn ${state.turn} ` +
        `(§9.1, §10.8, R97${into === "legalActions" ? ", R177" : ""})`,
    );
  };
  for (const [id, where] of ids) {
    const at = named.get(id);
    if (at !== undefined) report("card", "the view", id, at, where);
    const offer = offered.get(id);
    if (offer !== undefined && !bare.has(id)) report("card", "legalActions", id, offer, where);
  }
  for (const [def, where] of defs) {
    const at = named.get(def);
    if (at !== undefined) report("definition", "the view", def, at, where);
    const offer = offered.get(def);
    if (offer !== undefined) report("definition", "legalActions", def, offer, where);
  }
  return found;
}

export type InvariantMonitor = {
  /** I1 and I3 on the state the next action is chosen in. [] when clean. */
  before(state: GameState, player: PlayerId, action: ActionBody): string[];
  /** Feeds one action's events into the shadow, then I2, I4 and I5 against the resulting state. */
  after(events: readonly GameEvent[], state: GameState): string[];
  /** I6 for both seats on a state the fuzz reached, setup's included. [] when clean. */
  hidden(state: GameState): string[];
};

type AttackMark = { turn: number; stint: number; count: number };

/** Every card on the field, a card dormant under a Stack pile and the backrow included (§3.2). */
function fieldCards(state: GameState): CardInstance[] {
  return PLAYER_IDS.flatMap((player) => {
    const side = state.players[player];
    return [
      ...side.units.flatMap((pile) => pile ?? []),
      ...side.backrow.flatMap((card) => (card === null ? [] : [card])),
    ];
  });
}

function nameOf(card: CardInstance): string {
  return `${card.id} (${card.defId}, ${card.controller}'s)`;
}

export function createInvariantMonitor(start: GameState): InvariantMonitor {
  let turn = start.turn;
  const entered = new Map<string, number>();
  const stint = new Map<string, number>();
  const lastAttack = new Map<string, AttackMark>();
  // R636: the stint in which each instance was last seen with Windfury. Read from the states between
  // actions and after each one, so a unit that dies on its second attack was seen with it before —
  // and from `keywordGranted` events inside the action, so a unit granted Windfury mid-action is seen
  // before its declarations: R44's AI turn can summon a Conjure token and fight with it twice inside
  // one outer action, where no between-action state ever holds it.
  const windfury = new Map<string, number>();
  const readied = new Set<string>();
  // R424: the latest declared attack, and whether its attacker destroyed its target (R42).
  let attack: { attackerId: string; targetId: string; killed: boolean } | null = null;

  function noteWindfury(state: GameState): void {
    for (const card of fieldCards(state)) {
      if (hasKeyword(unitView(state, card).keywords, "Windfury")) windfury.set(card.id, stint.get(card.id) ?? 0);
    }
  }

  /** R424: a readied stint whose body carries no `summonedTurn`, as the readying transform leaves it. */
  function isReadied(card: CardInstance): boolean {
    return readied.has(card.id) && card.summonedTurn === undefined;
  }

  function enter(id: string): void {
    entered.set(id, turn);
    stint.set(id, (stint.get(id) ?? 0) + 1);
    readied.delete(id);
  }

  /** I1 for one unit and one would-be target set. */
  function sickAttack(state: GameState, unit: CardInstance, targetIds: readonly string[], what: string): string | null {
    if (targetIds.length === 0) return null;
    const keywords = unitView(state, unit).keywords;
    if (hasKeyword(keywords, "Charge")) return null;
    if (!hasKeyword(keywords, "Rush")) {
      return (
        `I1 sick attack ${what}: ${nameOf(unit)} entered on turn ${state.turn} and has neither Rush ` +
        `nor Charge, yet may attack ${targetIds.join(", ")} (§4.1, R171)`
      );
    }
    const heroes = targetIds.filter((id) => id.startsWith("hero-"));
    if (heroes.length === 0) return null;
    return (
      `I1 sick hero attack ${what}: ${nameOf(unit)} entered on turn ${state.turn} with Rush and no ` +
      `Charge, yet may attack ${heroes.join(", ")} (§6.1, R171)`
    );
  }

  return {
    before(state, player, action): string[] {
      const found: string[] = [];
      noteWindfury(state);

      // I3: nothing is on the field that no event put there.
      for (const card of fieldCards(state)) {
        if (!entered.has(card.id)) {
          found.push(`I3 silent arrival: ${nameOf(card)} is on the field on turn ${state.turn} with no entry event (§10.3)`);
        }
      }

      // I1: only where attacks can be offered at all.
      if (state.result !== null || state.pending !== null || state.phase !== "main" || player !== state.active) {
        return found;
      }
      for (const unit of activeUnitsOf(state, player)) {
        if (entered.get(unit.id) !== state.turn || isReadied(unit)) continue;
        const targets = attackTargets(state, unit).map((target) =>
          target.kind === "hero" ? `hero-${target.player}` : target.instance.id,
        );
        const violation = sickAttack(state, unit, targets, "offered");
        if (violation !== null) found.push(violation);
      }
      if (action.type === "attack") {
        const attacker = findInstance(state, action.attackerId);
        if (attacker !== undefined && entered.get(attacker.id) === state.turn && !isReadied(attacker)) {
          const violation = sickAttack(state, attacker, [action.targetId], "chosen");
          if (violation !== null) found.push(violation);
        }
      }
      return found;
    },

    after(events, state): string[] {
      const found: string[] = [];
      // R424: the Golem's transform follows its combat inside the one attack action, so an attack the
      // record holds from an earlier action readies nothing (a later Transmogulate of that unit is not it).
      attack = null;

      // I5: the check that ends the game is the last thing that happens (§2.5, R216).
      const over = events.findIndex((event) => event.type === "gameOver");
      if (over >= 0 && over < events.length - 1) {
        const later = events.slice(over + 1).map((event) => event.type);
        found.push(`I5 event after game over: ${later.join(", ")} followed gameOver on turn ${turn} (§2.5, R216)`);
      }

      for (const event of events) {
        switch (event.type) {
          case "turnStarted":
            turn = event.turn;
            attack = null;
            break;
          case "cardPlayed":
          case "summoned":
          case "controlChanged":
            enter(event.instanceId);
            break;
          case "animated":
            // R383: moving into the unit row is entering it on that turn; a carried Unit stepping
            // down off its carrier (C+ #33, carriers.ts) was a Unit on the field all along and enters nothing.
            if (event.carried !== true) enter(event.instanceId);
            break;
          case "transformed":
            if (event.newInstanceId !== event.instanceId) enter(event.newInstanceId);
            if (attack?.killed === true && attack.attackerId === event.instanceId) readied.add(event.newInstanceId);
            break;
          case "destroyed":
            if (attack !== null && attack.targetId === event.instanceId && attack.attackerId === event.killerId) {
              attack.killed = true;
            }
            break;
          case "keywordGranted": {
            // A grant the stream saw is a grant the unit holds from this stint on: without this, a
            // Windfury granted mid-action reads as a second attack without Windfury (see above).
            // `lost` (R46) takes a keyword away instead of giving it, so it is never recorded.
            if (event.keyword.kind === "Windfury" && event.lost !== true) {
              windfury.set(event.instanceId, stint.get(event.instanceId) ?? 0);
            }
            break;
          }
          case "attackDeclared": {
            // R53: a forced attack is not a declaration and spends nothing (nor readies, R424).
            attack = null;
            if (event.forced) break;
            const current = stint.get(event.attackerId) ?? 0;
            const previous = lastAttack.get(event.attackerId);
            const repeat = previous !== undefined && previous.turn === turn && previous.stint === current;
            const mark: AttackMark = { turn, stint: current, count: repeat ? previous.count + 1 : 1 };
            const allowed = windfury.get(event.attackerId) === current ? WINDFURY_ATTACKS : 1;
            if (mark.count > allowed) {
              const unit = findInstance(state, event.attackerId);
              const who = unit === undefined ? event.attackerId : nameOf(unit);
              found.push(
                `I2 extra attack: ${who} declared attack number ${mark.count} on turn ${turn} without re-entering ` +
                  `the field (§4.1 one exertion per turn, R171; Windfury two, R636)`,
              );
            }
            lastAttack.set(event.attackerId, mark);
            attack = { attackerId: event.attackerId, targetId: event.targetId, killed: false };
            break;
          }
          default:
            break;
        }
      }

      noteWindfury(state);

      // I4: the engine's bookkeeping agrees with the shadow.
      for (const card of fieldCards(state)) {
        const at = entered.get(card.id);
        if (at === undefined) continue; // I3 reports it before the next action.
        if (card.summonedTurn !== at && !isReadied(card)) {
          found.push(
            `I4 entry mismatch: ${nameOf(card)} has summonedTurn ${String(card.summonedTurn)} on turn ` +
              `${state.turn}, but its latest entry event was on turn ${at} (§4.1, R83, R171)`,
          );
        }
        if (card.exertion.attacked) {
          const mark = lastAttack.get(card.id);
          if (mark === undefined || mark.stint !== (stint.get(card.id) ?? 0)) {
            found.push(
              `I4 stale exertion: ${nameOf(card)} has a spent attack on turn ${state.turn} that it ` +
                `did not declare since its latest entry (§4.1, R171)`,
            );
          }
        }
      }

      return found;
    },

    hidden(state): string[] {
      const found: string[] = [];
      for (const viewer of PLAYER_IDS) {
        try {
          found.push(...hiddenInformationViolations(state, viewer, viewFor(state, viewer), legalActions(state, viewer)));
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          found.push(`I6 view threw: viewFor or legalActions for ${viewer} on turn ${state.turn}: ${message} (§10.8)`);
        }
      }
      return found;
    },
  };
}
