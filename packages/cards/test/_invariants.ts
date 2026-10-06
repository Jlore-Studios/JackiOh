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
//      instance id, nor its definition id unless a card the seat reads carries the same one; the
//      seat's own prompt may offer its options (§10.8), and `legalActions` may name a face-down trap
//      by its bare id as a target (R177) and a hidden card by nothing else. The first run found one
//      leak, R752 (`trapFired` named a fired trap to its controller after it was shuffled into a
//      library), and five false positives, each fixed here and not in the engine:
//        - R466's `stolen`: a card its viewer read where it was taken is named to them openly;
//        - a `defs` body (R243): copied only for an id the rest of the view names, and it names its
//          ingredients and its `refs` (R279), the text of cards the seat reads;
//        - a definition a hidden card shares with a readable one (a token, a copy, a Book, a fused
//          card) is no secret, so a definition counts only while nothing readable carries it;
//        - Echo's `copies` (R399) names the definition of the card it copied;
//        - `swapsBook.from` (R671) names the Book a swap took the text from.
//      `I6_GATE_STRIDE` is how often the 1,000-seed gate runs it.
//
// Every message leads with its id, names the instance, its def and the turn, and cites the SPEC
// reference, so the fuzz report's `signatureOf` groups one bug into one entry.

import type { ActionBody, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { PLAYER_IDS, hasKeyword, opponentOf } from "@jackioh/shared";
import {
  HIDDEN_ID,
  WINDFURY_ATTACKS,
  activeUnitsOf,
  announceOf,
  attackTargets,
  cardTypeOf,
  findInstance,
  legalActions,
  mulliganPromptFor,
  returnedAwaitingShuffle,
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
 * (×1.47, over the budget, and 0 violations). With this stride: 386 s (×1.13).
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

/**
 * The hidden set, worked out from the state's zones on their own terms and never from `viewFor`'s
 * rules (the oracle must not be the fix restated): the other seat's hand while the game is live
 * (R434), both libraries (§9.1; the viewer's own definitions travel in `ownLibrary`, R310), the other
 * seat's face-down traps (dormant ones under a backrow top included, B5 E21), a card the other seat is setting face-down (R448) and mulligan returns
 * waiting for their shuffle (R224). Then it takes out every definition the viewer reads elsewhere
 * (its own cards, every public pile, the top of every unit pile) and the options of its own prompt
 * (§10.8, R177).
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
    for (const card of side.library) hide(card, `${player}'s library`, player !== viewer);
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
  for (const card of awaitingShuffle(state)) hide(card, "a mulligan return awaiting its shuffle", card.owner !== viewer);

  // What the viewer may read: a definition one of these carries is no secret, whatever else carries it.
  const readable = (cards: readonly (CardInstance | null | undefined)[]): void => {
    for (const card of cards) if (card !== null && card !== undefined) defs.delete(card.defId);
  };
  readable([...state.players[viewer].hand, ...state.players[viewer].library]);
  if (state.result !== null) readable(state.players[rival].hand);
  for (const player of PLAYER_IDS) {
    const side = state.players[player];
    readable(side.graveyard);
    readable(side.exile);
    readable(side.units.map((pile) => pile?.[0]));
    readable(side.carried ?? []);
    readable(side.backrow.filter((card) => card !== null && !faceDownTo(state, card, viewer)));
    readable(side.resolving.filter((card) => !setFaceDownBy(state, card, viewer)));
  }

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

  // A definition a readable card carries is no secret: a token, a copy, a Book, a fused card.
  const readableId = (value: unknown): boolean => typeof value === "string" && value !== HIDDEN_ID && !ids.has(value);
  const named = new Map<string, string>();
  walk(
    scanned,
    "view",
    (text, at) => {
      // R177: the viewer's own prompt names a face-down trap by its id, as a bare option.
      if (offeredBare.has(text) && /^view\.pending\.options\[\d+\]\.instanceId$/.test(at)) return;
      if (!named.has(text)) named.set(text, at);
    },
    (object) => {
      if (readableId(object["instanceId"])) {
        const copies = object["copies"];
        const enchantments = object["enchantments"];
        const swapped = Array.isArray(enchantments)
          ? enchantments.map((enchantment: unknown) => (enchantment as { from?: unknown } | null)?.from)
          : [];
        // R399: Echo's `copies`; R671: a swapped Book's `from`.
        for (const def of stringsOf(
          object["defId"],
          object["fromDefId"],
          (copies as { defId?: unknown } | null | undefined)?.defId,
          ...swapped,
        )) {
          defs.delete(def);
        }
      }
      if (readableId(object["newInstanceId"])) for (const def of stringsOf(object["toDefId"])) defs.delete(def);
      if (readableId(object["resultInstanceId"])) for (const def of stringsOf(object["defId"])) defs.delete(def);
    },
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
