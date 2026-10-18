// The event feed (SPEC §10.10, BUILD M5-T1): `view.events` as readable lines.
//
// A plain list, deliberately: the animation runner owns motion (M5-T4) and nothing here
// animates. Every line is built from the event payload plus public board data, and card
// identities that are only in the view by accident are not printed — `drawn`, `addedToHand` and
// `shuffledIn` carry a `defId` for the animation layer, so this log says "drew a card" rather
// than naming it, and a redaction slip upstream cannot turn into a leak on screen.
//
// A line never prints an id. A card is named from the board as it stands, else from what the
// window's own public events said it was (`summoned`, `cardPlayed`, a move to the graveyard …),
// else as "a unit" or "a card". The R154 sentinel reads as "a face-down trap", never as "hidden".
// `cardResolved` gets no line: it is bookkeeping, and "… finished resolving" read like a debug
// trace between the lines that matter.
//
// A line about a card can be looked at: hovering it with a mouse opens that card's face beside the
// log, and a click, a tap, a long-press or Enter opens it in a sheet (the cards module's inspect,
// one overlay at a time). The card is the one the line names, found exactly as the name was, so a
// line never opens a card it does not print: a sentinel, a hero, a draw or a line that says "a
// unit" opens nothing. What opens is the card in play (faces.ts, SPEC §10.10): as it stands where
// the view still lists it, else its definition as the game shows it; and a match-made card (a
// Fuse's, R243) is named and drawn from the definition the view carries for it. A card the view
// says is Chinese (ME-CN, R1301) is named by its Chinese name, and a translation itself gets no line:
// the card's words changed language, and the board already shows it.
//
// The window is not the whole game: the lines whose events have left it come from
// `useLogHistory.ts` (R745), kept as they read when they arrived, with a definition's face and no
// instance id. A divider stands before each turn's first line.

import { Fragment, memo, useContext, useLayoutEffect, useRef, type MouseEvent, type ReactElement } from "react";

import type { GameEvent, LibraryOverflowOutcome, PlayerId, PlayerView, PromptKind } from "@jackioh/shared";

import { useInspectTrigger } from "../cards/index.ts";
import { chineseName, drawnInChinese, listSeparator, readerKeyword } from "../cards/chinese.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { GLITCH_WORDS } from "../cards/glitch.ts";
import { markWords } from "../cards/marks.ts";
import { chaosNames, chaosRollOf } from "../fx/chaos.ts";
import { CatalogContext, withMatchDefs, type CardLookup } from "./catalog.ts";
import { LOG_HISTORY_LIMIT } from "./config.ts";
import { sideOf, testid, touchHoldMode } from "./contract.ts";
import { cardInView, namedFace } from "./faces.ts";
import { outcomeFor, resultReason } from "./Result.tsx";
import { PREDICTED_WORDS, SECRET_WORDS } from "./secrets.ts";
import { CHAOS_TEXT } from "./showcase/constants.ts";
import { useLogHistory, type KeptLine, type LoggedLine } from "./useLogHistory.ts";


/** What a player is asked for, by prompt kind, in the log's words. */
const PROMPT_WORDS: Readonly<Record<PromptKind, string>> = {
  discover: "a card to Discover",
  target: "a target",
  mode: "a mode",
  mulligan: "cards to keep",
  hand: "a card from hand",
  zone: "a zone",
  tribute: "a Tribute",
  direction: "a direction",
  x: "X",
  embiggen: "the normal or the Embiggened version",
  number: "a number",
  answer: "an answer",
  cell: "a cell",
  reward: "a reward",
  pick: "cards to take",
  craft: "a card to craft",
  market: "a deal at the night market",
};

/** A number's key in words: a declared number's camelCase split ("drawLimit" is "draw limit"). */
function keyWords(key: string): string {
  return key.replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase();
}

export type LogProps = {
  view: PlayerView;
  /** True once a hidden log (the phone's popover) is opened, so it can scroll to its newest line. */
  revealed?: boolean;
};

/** The button inside a line that names a card; `data-def-id` says which. */
export const LOG_CARD_TESTID = "log-card";

/** R97: the id an event carries in place of a card this seat may not read (`view.ts`). */
const HIDDEN_CARD = "hidden";

type Naming = {
  /** A card's printed name, or its def id when no catalog is loaded (see catalog.ts). */
  def: (defId: string) => string;
  /**
   * A card an event names by both its instance and its definition: as `def` names it, but in Chinese
   * while the view shows that instance in Chinese (R1301), so the lines about one card name it as the
   * board does, whichever field the event carries (#552).
   */
  card: (instanceId: string, defId: string) => string;
  /** A card by name where it is public, else "a unit" (or `unknown`). Heroes read as a seat's. */
  instance: (instanceId: string, unknown?: string) => string;
  seat: (player: PlayerId) => string;
  /** "your" or "the opponent's". */
  whose: (player: PlayerId) => string;
};

function seatLabel(view: PlayerView, player: PlayerId): string {
  return sideOf(view, player) === "you" ? t("You") : t("Opponent");
}

function whoseLabel(view: PlayerView, player: PlayerId): string {
  return sideOf(view, player) === "you" ? t("your") : t("the opponent's");
}

/** The log's own words ("your hero", "a unit") open a line with a capital; a card's name is as printed. */
const OWN_WORDS = /^(?:a|an|your|the) /;

function capitalised(text: string): string {
  return OWN_WORDS.test(text) ? `${text.charAt(0).toUpperCase()}${text.slice(1)}` : text;
}

function zoneLabel(row: string, lane: number): string {
  return row === "units" ? t("lane {lane}", { lane }) : t("backrow lane {lane}", { lane });
}

/**
 * Instance → definition, from the window's events about public zones (a card played, summoned,
 * destroyed or sent to a pile), so a unit that has since left the board is still named. The
 * hand's own events (`drawn`, `addedToHand`, `shuffledIn`) are not read, as above, and neither is
 * the sentinel.
 */
function publicNames(events: readonly GameEvent[]): Map<string, string> {
  const names = new Map<string, string>();
  const note = (instanceId: string, defId: string): void => {
    if (instanceId !== HIDDEN_CARD && defId !== HIDDEN_CARD) names.set(instanceId, defId);
  };
  for (const event of events) {
    switch (event.type) {
      case "cardPlayed":
      case "summoned":
      case "destroyed":
      case "enteredGraveyard":
      case "exiled":
      case "bounced":
      case "burned":
      case "libraryOverflow":
      case "discarded":
      case "radiantSet":
      case "deradianted":
      case "trapFired":
        note(event.instanceId, event.defId);
        break;
      case "fused":
        note(event.resultInstanceId, event.defId);
        break;
      case "transformed":
        note(event.instanceId, event.toDefId);
        break;
      default:
        break;
    }
  }
  return names;
}

function defIdOfInstance(view: PlayerView, instanceId: string): string | undefined {
  for (const seat of [view.you, view.opponent]) {
    for (const unit of seat.units) {
      if (unit !== null && unit.instanceId === instanceId) return unit.defId;
    }
    for (const entry of seat.backrow) {
      if (entry !== null && !entry.faceDown && entry.instanceId === instanceId) return entry.defId;
    }
    for (const card of [...seat.graveyard, ...seat.exile]) {
      if (card.instanceId === instanceId) return card.defId;
    }
    if (Array.isArray(seat.hand)) {
      for (const card of seat.hand) {
        if (card.instanceId === instanceId) return card.defId;
      }
    }
  }
  return undefined;
}

/** R370: the cost the view gives the face-down card in a backrow lane, while one stands there. */
function faceDownCostAt(view: PlayerView, player: PlayerId, lane: number): number | undefined {
  const seat = sideOf(view, player) === "you" ? view.you : view.opponent;
  const entry = seat.backrow[lane - 1];
  if (entry === null || entry === undefined || !entry.faceDown) return undefined;
  return "cost" in entry && typeof entry.cost === "number" ? entry.cost : undefined;
}

/** R404: an open quest's words, off the quest line its card's view carries, or null once it is not open. */
function questText(view: PlayerView, instanceId: string, quest: string): string | null {
  return cardInView(view, instanceId)?.quest?.open.find((open) => open.id === quest)?.text ?? null;
}

/**
 * What became of a card a full deck turned away (R316), as its line reads. Each line is one
 * sentence for the reader's language (R711000), as are the other tables of whole lines below.
 */
const OVERFLOW_LINE: Readonly<Record<LibraryOverflowOutcome, string>> = {
  notCreated: "{whose} deck is full: {card} was not created",
  graveyard: "{whose} deck is full: {card} went to the graveyard",
  ceased: "{whose} deck is full: {card} ceased to exist",
};

/** A counter's line, by the counter. */
const COUNTER_LINE: Readonly<Record<Extract<GameEvent, { type: "counterChanged" }>["counter"], string>> = {
  plague: "{target} plague counters: {value}",
  grade: "{target} grade counters: {value}",
  brittle: "{target} brittle counters: {value}",
};

const ROTATED_LINE: Readonly<Record<Extract<GameEvent, { type: "rotated" }>["direction"], string>> = {
  left: "The board rotated left",
  right: "The board rotated right",
};

function describe(event: GameEvent, view: PlayerView, name: Naming): string | null {
  /** A card an overflow names: by name where the viewer reads it, else "a card" (R97). */
  const named = (defId: string): string => (defId === HIDDEN_CARD ? t("a card") : name.def(defId));
  /** The same, naming the card as the board shows that instance (R1301's Chinese name included). */
  const namedCard = (instanceId: string, defId: string): string => (defId === HIDDEN_CARD ? t("a card") : name.card(instanceId, defId));
  switch (event.type) {
    case "cardPlayed":
      return t("{seat} played {card} for {cost}", { seat: name.seat(event.player), card: name.def(event.defId), cost: event.costPaid });
    case "summoned": {
      // R370: a card set face down is a trap the viewer may not read; the line says so, with the
      // cost its back shows while it still stands there.
      const where = { whose: name.whose(event.player), zone: zoneLabel(event.row, event.lane) };
      if (event.defId === HIDDEN_CARD && event.row === "backrow") {
        const cost = faceDownCostAt(view, event.player, event.lane);
        return capitalised(
          cost === undefined
            ? t("{whose} {zone}: a face-down trap was set", where)
            : t("{whose} {zone}: a face-down trap was set, ({cost}) Cost", { ...where, cost }),
        );
      }
      return capitalised(t("{card} entered {whose} {zone}", { card: name.card(event.instanceId, event.defId), ...where }));
    }
    case "damage": {
      // R1360: the Armor's part of the hit, where it took one.
      const absorbed = event.absorbed ?? 0;
      const hit = { target: capitalised(name.instance(event.targetId)), amount: event.amount, absorbed };
      if (absorbed > 0) {
        return event.combat
          ? t("{target} took {amount} damage in combat, {absorbed} absorbed by Armor", hit)
          : t("{target} took {amount} damage, {absorbed} absorbed by Armor", hit);
      }
      return event.combat ? t("{target} took {amount} damage in combat", hit) : t("{target} took {amount} damage", hit);
    }
    case "damageAbsorbed": {
      // R1361: the Armor took the whole hit, so nothing was dealt.
      const hit = { target: capitalised(name.instance(event.targetId)), absorbed: event.absorbed };
      return event.combat ? t("{target}'s Armor absorbed {absorbed} damage in combat", hit) : t("{target}'s Armor absorbed {absorbed} damage", hit);
    }
    case "secretChosen":
      // R860: the line names only that a secret is held — never the choice.
      return t("{seat} kept a secret", { seat: name.seat(event.player) });
    case "secretRevealed":
      // R864: revealed, the choice is public history.
      return capitalised(t("{whose} secret was {choice}", { whose: name.whose(event.player), choice: t(SECRET_WORDS[event.choice]) }));
    case "predicted":
      // R864: the guess and what it did to the reward.
      return t("{seat} guessed {guess}: {outcome}", {
        seat: name.seat(event.player),
        guess: t(SECRET_WORDS[event.guess]),
        outcome: t(PREDICTED_WORDS[event.outcome]),
      });
    case "scriptFailed":
      // R915004: a custom card's hook did nothing; the engine's words say why.
      return t("{card}'s script failed: {message}", {
        card: event.instanceId === null ? named(event.defId) : namedCard(event.instanceId, event.defId),
        message: event.message,
      });
    case "healthLost":
      return t("{seat} lost {amount} health", { seat: name.seat(event.player), amount: event.amount });
    case "healed":
      return t("{target} healed {amount}", { target: capitalised(name.instance(event.targetId)), amount: event.amount });
    case "divineShieldLost":
      return t("{target} lost its Divine Shield", { target: capitalised(name.instance(event.instanceId)) });
    case "destroyed":
      return capitalised(t("{card} was destroyed", { card: name.card(event.instanceId, event.defId) }));
    case "cardResolved":
      return null;
    case "enteredGraveyard":
      return capitalised(t("{card} went to {whose} graveyard", { card: name.card(event.instanceId, event.defId), whose: name.whose(event.owner) }));
    case "exiled":
      return capitalised(t("{card} was exiled", { card: name.card(event.instanceId, event.defId) }));
    case "bounced":
      return capitalised(t("{card} returned to {whose} hand", { card: name.card(event.instanceId, event.defId), whose: name.whose(event.owner) }));
    case "burned":
      // R317: a full hand's card, named where the viewer reads it (the sentinel is "a card").
      return capitalised(t("{whose} hand is full: {card} burned", { whose: name.whose(event.owner), card: named(event.defId) }));
    case "fatigue":
      // R315: public, and it names no card. The hit is the `damage` line after it.
      // R373: the rules' library reads as the Deck.
      return capitalised(t("{whose} deck is empty: fatigue {count}", { whose: name.whose(event.player), count: event.count }));
    case "libraryOverflow":
      // R316: the card a full library turned away, named only where the viewer reads it.
      return capitalised(t(OVERFLOW_LINE[event.outcome], { whose: name.whose(event.player), card: named(event.defId) }));
    case "discarded":
      return t("{seat} discarded {card}", { seat: name.seat(event.owner), card: name.def(event.defId) });
    case "drawn":
      return t("{seat} drew a card", { seat: name.seat(event.player) });
    case "addedToHand":
      return t("{seat} added a card to hand", { seat: name.seat(event.player) });
    case "shuffledIn":
      return t("{seat} shuffled a card into the deck", { seat: name.seat(event.player) });
    case "buffed":
      // R177: a buff on a card this seat may not read arrives as the sentinel with 0/0, which says
      // nothing of its size, so the line does not claim one.
      return event.instanceId === HIDDEN_CARD
        ? t("A hidden card was buffed")
        : t("{target} gained +{attack}/+{health}", { target: capitalised(name.instance(event.instanceId)), attack: event.attack, health: event.health });
    case "keywordGranted": {
      // R46: the same event reports the Taunt an Indestructible unit's knock-down takes.
      const change = { target: capitalised(name.instance(event.instanceId)), keyword: readerKeyword(event.keyword.kind) };
      return event.lost === true ? t("{target} lost {keyword}", change) : t("{target} gained {keyword}", change);
    }
    case "counterChanged":
      return t(COUNTER_LINE[event.counter], { target: capitalised(name.instance(event.instanceId)), value: event.value });
    case "jadeChanged":
      return capitalised(t("{whose} Jade Counter is {value}", { whose: name.whose(event.player), value: event.value }));
    case "costChanged": {
      // R177: a card this seat may not read arrives with its cost redacted to a negative sentinel.
      const target = capitalised(name.instance(event.instanceId, t("a card")));
      return event.cost < 0 ? t("{target} changed cost", { target }) : t("{target} now costs ({cost})", { target, cost: event.cost });
    }
    case "modifierChanged": {
      // The id is the engine's handle ("m123"); the label, while the view still lists it, is the words.
      const label = [...view.you.modifiers, ...view.opponent.modifiers].find((mod) => mod.id === event.modifierId)?.label;
      const change = { seat: name.seat(event.player), what: label === undefined ? t("an effect") : `"${t(label)}"` };
      return event.added ? t("{seat} gained {what}", change) : t("{seat} lost {what}", change);
    }
    case "radiantSet":
      return capitalised(t("{card} became Radiant", { card: name.card(event.instanceId, event.defId) }));
    case "deradianted":
      // MD-D6: the mirror of `radiantSet`'s line.
      return capitalised(t("{card} is no longer Radiant", { card: name.card(event.instanceId, event.defId) }));
    case "manaSpent":
      // MD-D26: the spend is noise beside the play or activation it paid for; `manaChanged` says the total.
      return null;
    case "transformed":
      // R97: a card the viewer may not read is a hidden card on both sides; with nothing to name the
      // line says only that it changed (a hidden card in a hand becoming another, #552).
      if (event.fromDefId === HIDDEN_CARD && event.toDefId === HIDDEN_CARD) return t("A hidden card was transformed");
      return capitalised(t("{from} became {to}", { from: name.def(event.fromDefId), to: name.card(event.newInstanceId, event.toDefId) }));
    case "fused":
      return t("{count} cards fused into {card}", { count: event.instanceIds.length, card: name.def(event.defId) });
    case "positionSwitched":
      return t("{target} switched to {position}", { target: capitalised(name.instance(event.instanceId)), position: t(event.position) });
    case "controlChanged":
      return t("{target} moved to {whose} {zone}", {
        target: capitalised(name.instance(event.instanceId)),
        whose: name.whose(event.controller),
        zone: zoneLabel(event.row, event.lane),
      });
    case "rotated":
      return t(ROTATED_LINE[event.direction]);
    case "swapped":
      // Built from the event so the line reads as the engine names it; zh-CN.json holds each whole sentence.
      return t(`Both players swapped ${event.what}`);
    case "locked":
      return t("{whose} {zone} was locked", { whose: capitalised(name.whose(event.player)), zone: zoneLabel(event.row, event.lane) });
    case "trapFired":
      // R154: the other seat reads the sentinel, and a sentinel is not a name.
      return event.defId === HIDDEN_CARD
        ? t("{whose} face-down trap fired", { whose: capitalised(name.whose(event.controller)) })
        : t("{whose} trap {card} fired", { whose: capitalised(name.whose(event.controller)), card: name.def(event.defId) });
    case "attackDeclared": {
      const attack = { attacker: capitalised(name.instance(event.attackerId)), target: name.instance(event.targetId) };
      return event.forced ? t("{attacker} attacked {target} (forced)", attack) : t("{attacker} attacked {target}", attack);
    }
    case "attackCancelled":
      return t("{attacker}'s attack was cancelled", { attacker: capitalised(name.instance(event.attackerId)) });
    case "manaChanged": {
      const mana = { seat: name.seat(event.player), current: event.current, max: event.max };
      return sideOf(view, event.player) === "you" ? t("{seat} have {current}/{max} mana", mana) : t("{seat} has {current}/{max} mana", mana);
    }
    case "turnStarted": {
      // R845: an extra turn's line says so.
      const turn = { turn: event.turn, seat: name.seat(event.player) };
      return event.extra === true ? t("Turn {turn}: {seat} (extra turn)", turn) : t("Turn {turn}: {seat}", turn);
    }
    case "turnEnded":
      return t("{seat} ended turn {turn} with {mana} mana unspent", { seat: name.seat(event.player), turn: event.turn, mana: event.unspentMana });
    case "turnAutoEnded":
      return t("{seat} had no moves left, so turn {turn} ended", { seat: name.seat(event.player), turn: event.turn });
    case "promptOpened":
      return t("{seat} must choose {what}", { seat: name.seat(event.player), what: t(PROMPT_WORDS[event.kind]) });
    case "promptAnswered":
      return t("{seat} chose", { seat: name.seat(event.player) });
    case "drawOffered":
      return t("{seat} offered a draw", { seat: name.seat(event.player) });
    case "drawAnswered":
      return event.accept
        ? t("{seat} accepted the draw", { seat: name.seat(event.player) })
        : t("{seat} declined the draw", { seat: name.seat(event.player) });
    case "emoted":
      return t("{seat} emoted", { seat: name.seat(event.player) });
    case "gameOver": {
      const why = t(resultReason(outcomeFor({ winner: event.winner, reason: event.reason }, view.viewer), event.reason));
      return event.winner === "draw" ? t("Draw. {why}", { why }) : t("{seat} won. {why}", { seat: seatLabel(view, event.winner), why });
    }
    // ---- Patch v0.2.0 (docs/classic-sets.md B3, B5) ----
    case "cardAnnounced":
      // The play's own line is its `cardPlayed`; a countered one has its `countered` line.
      return null;
    case "countered":
      return event.to === "hand"
        ? t("{card} was countered and stolen", { card: named(event.defId) })
        : t("{card} was countered", { card: named(event.defId) });
    case "stolen":
      return capitalised(t(STOLEN_LINE[event.zone], { seat: name.seat(event.to), card: named(event.defId), whose: name.whose(event.from) }));
    case "unlocked":
      return t("{whose} {zone} was unlocked", { whose: capitalised(name.whose(event.player)), zone: zoneLabel(event.row, event.lane) });
    case "activated":
      return t("{seat} activated {card}", { seat: name.seat(event.player), card: name.def(event.defId) });
    case "animated":
      return t("{card} came to life in {whose} {zone}", {
        card: name.def(event.defId),
        whose: name.whose(event.player),
        zone: zoneLabel("units", event.unitLane),
      });
    case "deanimated":
      return t("{card} returned to {whose} {zone}", {
        card: name.def(event.defId),
        whose: name.whose(event.player),
        zone: zoneLabel("backrow", event.backrowLane),
      });
    case "crumbled":
      return capitalised(t("{card} crumbled", { card: namedCard(event.instanceId, event.defId) }));
    // R1320: players read the engine's Degrade as a Nerf and its Upgrade as a Buff.
    case "degraded":
      return capitalised(t("{card} was nerfed", { card: namedCard(event.instanceId, event.defId) }));
    case "upgraded":
      return capitalised(t("{card} was buffed", { card: namedCard(event.instanceId, event.defId) }));
    case "numberChanged": {
      const card = namedCard(event.instanceId, event.defId);
      if (event.key === HIDDEN_CARD) return capitalised(t("{card} changed", { card }));
      // A cost is a price (R432); any other key is a word, a declared number's camelCase split ("draw limit").
      return capitalised(
        event.key === "cost"
          ? t("{card} now costs ({value})", { card, value: event.value })
          : t("{card}'s {key} became {value}", { card, key: t(keyWords(event.key)), value: event.value }),
      );
    }
    case "redirected":
      return t("{what} was redirected to {target}", { what: capitalised(t(REDIRECTED[event.what])), target: name.instance(event.toId) });
    case "healthSet":
      return t("{whose} hero's health was set to {health}", { whose: capitalised(name.whose(event.player)), health: event.health });
    case "questProgressed": {
      // R404: the event names the quest by its id; its words are the card's view's while it is open.
      const text = questText(view, event.instanceId, event.quest);
      const quest = {
        whose: name.whose(event.player),
        card: name.instance(event.instanceId, t("card")),
        progress: event.progress,
        goal: event.goal,
      };
      return capitalised(
        text === null
          ? t("{whose} {card} quest: {progress}/{goal}", quest)
          : t("{whose} {card} quest: {text}, {progress}/{goal}", { ...quest, text: t(text) }),
      );
    }
    case "questCompleted":
      // A completed quest has left the view's open quests, so its words are gone with it.
      return capitalised(t("{whose} {card} completed a quest", { whose: name.whose(event.player), card: name.instance(event.instanceId, t("card")) }));
    case "curveDrawn":
      // R831003: the curve itself is the board's to show; the line says whose it was.
      return t("{seat} drew a curve across the board", { seat: name.seat(event.player) });
    case "rolledBack":
      return event.turnsAgo === 1
        ? t("The board went back {n} turn", { n: event.turnsAgo })
        : t("The board went back {n} turns", { n: event.turnsAgo });
    case "chaosRolled": {
      // R436: the effects in the words the reveal shows (fx/chaos.ts), never the engine's keys. The
      // roll is public; a card the viewer may not read is still a Call to Chaos (R97).
      const roll = chaosRollOf(event);
      const who = event.defId === HIDDEN_CARD ? t(CHAOS_TEXT.title) : name.def(event.defId);
      return roll === null ? null : t("{who} rolled: {names}", { who, names: chaosNames(roll).map((effect) => t(effect)).join(listSeparator()) });
    }
    case "flickered":
      return capitalised(t("{card} flickered", { card: name.card(event.instanceId, event.defId) }));
    case "drawLimited":
      return t("{seat} could not draw more this turn", { seat: name.seat(event.player) });
    case "discardPrevented":
      return t("{seat} could not be made to discard", { seat: name.seat(event.player) });
    case "turnCutShort":
      return capitalised(t("{whose} turn was cut short", { whose: name.whose(event.player) }));
    case "glitched":
      // R676: what a Glitch did, under its corrupted name (cards/glitch.ts).
      return t("{glitch}: {outcome}", { glitch: GLITCH_WORDS.name, outcome: t(GLITCH_OUTCOME_LINE[event.outcome]) });
    case "fengShui":
      // R983: whose play was judged, and how.
      return capitalised(t("{whose} play was {outcome}", { whose: name.whose(event.player), outcome: t(FENG_SHUI_LINE[event.outcome]) }));
    case "marked":
      // R437: the mark by the name its badge says (cards/marks.ts), never the engine's key; a marked
      // card the viewer may not read (a face-down trap) is "a card".
      return event.added
        ? t("{target} was marked ({mark})", { target: capitalised(name.instance(event.instanceId, t("a card"))), mark: t(markWords(event.mark).name) })
        : null;
    case "translated":
      // R1301: only the language the card is shown in changed; its face says so.
      return null;
  }
}

/** The words a view's lines use for cards and seats, naming a card from the board, else from `remembered`. */
function namingFor(view: PlayerView, lookup: CardLookup | null, remembered: ReadonlyMap<string, string>): Naming {
  const def = (defId: string): string => {
    if (defId === HIDDEN_CARD) return t("a hidden card");
    const info = lookup?.(defId, false);
    const english = info?.name ?? defId;
    return drawnInChinese(false) ? chineseName(defId, english, info?.def) : english;
  };
  return {
    def,
    card: (instanceId, defId) => {
      if (defId === HIDDEN_CARD || instanceId === HIDDEN_CARD) return def(defId);
      const isZh = drawnInChinese(cardInView(view, instanceId)?.chinese === true);
      if (!isZh) return def(defId);
      const info = lookup?.(defId, false);
      return chineseName(defId, info?.name ?? defId, info?.def);
    },
    instance: (instanceId, unknown) => {
      if (instanceId === `hero-${view.you.player}`) return t("your hero");
      if (instanceId === `hero-${view.opponent.player}`) return t("the opponent's hero");
      const defId = defIdOfInstance(view, instanceId) ?? remembered.get(instanceId);
      if (defId === undefined) return unknown ?? t("a unit");
      const info = lookup?.(defId, false);
      const english = info?.name ?? defId;
      // R1301, R711000: a card the view shows in Chinese is named in Chinese.
      return drawnInChinese(cardInView(view, instanceId)?.chinese === true) ? chineseName(defId, english, info?.def) : english;
    },
    seat: (player) => seatLabel(view, player),
    whose: (player) => whoseLabel(view, player),
  };
}

/** R676–R679: each Glitch outcome, as its log line ends. */
const GLITCH_OUTCOME_LINE: Readonly<Record<Extract<GameEvent, { type: "glitched" }>["outcome"], string>> = {
  reset: "the match resets",
  swap: "the players swap seats",
  boards: "other games' boards appear",
  void: "the match never happened",
};

/** R983: each Feng Shui outcome, as its log line ends. */
const FENG_SHUI_LINE: Readonly<Record<Extract<GameEvent, { type: "fengShui" }>["outcome"], string>> = {
  positive: "auspicious (吉)",
  negative: "inauspicious (凶)",
};

/** Where a stolen card was taken from, as its line reads (R373: the rules' library is the Deck). */
const STOLEN_LINE: Readonly<Record<Extract<GameEvent, { type: "stolen" }>["zone"], string>> = {
  hand: "{seat} stole {card} from {whose} hand",
  library: "{seat} stole {card} from {whose} deck",
  resolving: "{seat} stole {card} from {whose} play",
  graveyard: "{seat} stole {card} from {whose} graveyard",
  exile: "{seat} stole {card} from {whose} exile",
  field: "{seat} stole {card} from {whose} side of the field",
};

/** What a redirect moved. */
const REDIRECTED: Readonly<Record<Extract<GameEvent, { type: "redirected" }>["what"], string>> = {
  damage: "a hit",
  attack: "an attack",
  target: "a target",
};

/** The card a line is about: the one it names, by definition, and the face it wears where the view shows it. */
type LineCard = { defId: string; radiant: boolean; instanceId?: string };

/**
 * The card a line is about, or null when it names none: the one `describe` names (for a Transform,
 * the card it became; for an attack, the attacker). It reads the same sources `Naming` does — the
 * event's own `defId`, else the instance on the board or in the window's public events — so it can
 * never open a card the line does not print (R97: the sentinel is none).
 */
function cardOf(event: GameEvent, view: PlayerView, remembered: ReadonlyMap<string, string>): LineCard | null {
  const face = (instanceId: string | undefined): boolean =>
    instanceId === undefined ? false : (cardInView(view, instanceId)?.radiant ?? false);
  const byDef = (defId: string, instanceId?: string): LineCard | null =>
    defId === HIDDEN_CARD
      ? null
      : { defId, radiant: face(instanceId), ...(instanceId === undefined || instanceId === HIDDEN_CARD ? {} : { instanceId }) };
  const byInstance = (instanceId: string): LineCard | null => {
    if (instanceId === HIDDEN_CARD) return null;
    const defId = defIdOfInstance(view, instanceId) ?? remembered.get(instanceId);
    return defId === undefined ? null : byDef(defId, instanceId);
  };
  switch (event.type) {
    case "cardPlayed":
    case "summoned":
    case "destroyed":
    case "enteredGraveyard":
    case "exiled":
    case "bounced":
    case "burned":
    case "libraryOverflow":
    case "discarded":
    case "trapFired":
      return event.instanceId === HIDDEN_CARD ? null : byDef(event.defId, event.instanceId);
    case "radiantSet": {
      const card = event.instanceId === HIDDEN_CARD ? null : byDef(event.defId, event.instanceId);
      return card === null ? null : { ...card, radiant: true };
    }
    case "deradianted": {
      // MD-D6: the mirror of `radiantSet`'s card.
      const card = event.instanceId === HIDDEN_CARD ? null : byDef(event.defId, event.instanceId);
      return card === null ? null : { ...card, radiant: false };
    }
    case "manaSpent":
      return null;
    case "transformed":
      return byDef(event.toDefId, event.newInstanceId);
    case "fused":
      return byDef(event.defId, event.resultInstanceId);
    case "damage":
    case "damageAbsorbed":
    case "healed":
      return byInstance(event.targetId);
    case "divineShieldLost":
    case "buffed":
    case "keywordGranted":
    case "counterChanged":
    case "costChanged":
    case "positionSwitched":
    case "controlChanged":
    case "questProgressed":
    case "questCompleted":
      return byInstance(event.instanceId);
    case "attackDeclared":
    case "attackCancelled":
      return byInstance(event.attackerId);
    default:
      return null;
  }
}

/**
 * R745: an event's line as the log keeps it (useLogHistory.ts): this view's words, and the
 * definition's face built without the instance, so it carries no instance id.
 */
export function keptLineFor(view: PlayerView, lookup: CardLookup | null): (event: GameEvent) => LoggedLine | null {
  const remembered = publicNames(view.events);
  const name = namingFor(view, lookup, remembered);
  return (event) => {
    const text = describe(event, view, name);
    if (text === null) return null;
    const card = cardOf(event, view, remembered);
    return {
      type: event.type,
      text,
      face: card === null ? null : namedFace(lookup, view, { defId: card.defId, radiant: card.radiant }),
    };
  };
}

type Line = KeptLine;

/** One line; a line about a card is a button that opens it (see the header). Kept lines are the same objects from render to render, so they do not redraw. */
const LogLine = memo(function LogLine({ line, touchHold }: { line: Line; touchHold?: "sheet" | "preview" }): ReactElement {
  const face = line.face;
  // Lines of code is a hidden stat in matches.
  const inspect = useInspectTrigger(face === null ? null : { key: `log-${line.key}`, face }, { touchHold, showLoc: false });
  if (face === null) {
    return (
      <li className="log-line" data-event={line.type}>
        {line.text}
      </li>
    );
  }
  // A button: Enter and Space click it, so the keyboard needs nothing of its own.
  const open = (event: MouseEvent<HTMLElement>): void => {
    inspect.openSheet(event.currentTarget);
  };
  return (
    <li className="log-line" data-event={line.type} data-inspectable="true">
      <button
        {...inspect.handlers}
        type="button"
        className="log-card"
        data-testid={LOG_CARD_TESTID}
        data-def-id={face.defId}
        aria-haspopup="dialog"
        onClick={open}
      >
        {line.text}
      </button>
      {inspect.overlay}
    </li>
  );
});

export default function Log({ view, revealed = false }: LogProps): ReactElement {
  useLocale();
  // R243: a match-made card is named from the definition the view carries for it.
  const lookup = withMatchDefs(useContext(CatalogContext), view.defs);
  const remembered = publicNames(view.events);
  const name = namingFor(view, lookup, remembered);
  // R745: the lines that have left the window, and the key each window event keeps as it slides.
  const history = useLogHistory(view, keptLineFor(view, lookup));

  // The newest line is the one worth reading, so a log taller than its box keeps its end in view
  // (polish task 7: on a desktop the log is a fixed box beside your seat and hand). A log that was
  // hidden (the phone's popover) has no height to scroll, so it scrolls again as it opens.
  const listRef = useRef<HTMLOListElement>(null);
  useLayoutEffect(() => {
    const list = listRef.current;
    if (list !== null) list.scrollTop = list.scrollHeight;
  }, [view.events, revealed]);

  const windowLines = view.events.flatMap((event, index): Line[] => {
    const text = describe(event, view, name);
    if (text === null) return [];
    const card = cardOf(event, view, remembered);
    const face = card === null ? null : namedFace(lookup, view, card);
    return [{ key: history.keys[index] ?? `${view.viewer}-window-${String(index)}`, type: event.type, text, face }];
  });
  const lines = [...history.kept, ...windowLines].slice(-LOG_HISTORY_LIMIT);
  const touchHold = touchHoldMode(view);

  return (
    <ol ref={listRef} className="log" data-testid={testid.log} aria-label={t("Game log")}>
      {lines.map((line, index) => (
        <Fragment key={line.key}>
          {/* #757: drawn only; a list holds list items, and the turn's own line names the turn. */}
          {index > 0 && line.type === "turnStarted" ? <li className="log-divider" aria-hidden="true" /> : null}
          <LogLine line={line} touchHold={touchHold} />
        </Fragment>
      ))}
    </ol>
  );
}
