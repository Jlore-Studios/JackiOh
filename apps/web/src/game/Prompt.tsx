// The choice pickers (BUILD M5-T2, SPEC §10.6). One modal, a picker for every prompt kind, no rules.
//
// Two things open this modal, and telling them apart is R81:
//
//  1. `view.pending` — a `PendingChoice` the engine opened during resolution (Discover, a chained
//     step, an Echo repeat, a cast, a trigger, the mulligan). Resolution is paused until it is
//     answered, and the answer is an `answer` action (or, for the mulligan, §10.2's own `mulligan`
//     action). The opponent sees only that a prompt is open, never its options (§10.6, §10.8).
//
//  2. `props.interaction` — a play the player is still building. R81: "Zone, X, embiggen, Tribute
//     and the targets and modes a card's script declares travel in the `play` action, which
//     `legalActions` enumerates; the client builds them with the prompt pickers." Those five
//     pickers therefore have to work with NO `view.pending` at all: nothing is paused, nothing is
//     waiting on an answer, and what they submit is a `play`, never an `answer`. §10.6 keeps the
//     `x`, `embiggen`, `zone`, `tribute` and `direction` prompt kinds for later sets, so both
//     routes render the same picker with the same `data-prompt-kind`. An activation (R384) is
//     built on this route too — its targets, modes and Tribute — and submits an `activate`; so is
//     a play's payment: the Plague Counters paying a graveyard play (Classic #74, a `number` picker
//     of chips). A targeting cost's discards (Classic #89) are random at pay time (R682) and need
//     no picker.
//
// A card option is drawn as the card in play (faces.ts, SPEC §10.10): a card the view lists — a hand
// card in a mulligan or a hand pick, a unit a target reaches — as it stands, and a Discover's card as
// the game shows it. An option that names no card is its label; a Discover of numbers (#82 KY's
// Trial, R247) offers exactly that, and each number is drawn on a card back, with no face to read.
//
// #492: one card picker (`CardChoice`) holds every picker whose options are cards, in the middle of
// the screen: the mulligan, a Discover, a hand pick, a pick, a short "Choose one" (Appropriations'),
// and two play choices that used to be small controls docked beside the board, an embiggen price
// (the card at its normal price and at its embiggen price) and an X of X_CARD_LIMIT values or fewer
// (the card once per X). Those two keep their `data-prompt-kind`; `data-prompt-layout="cards"` says
// which layout a picker has.
//
// `data-prompt-source` says which route opened the modal ("engine" or "play"), for layout only:
// prompt.css turns a play's board picks into a slim bar on a phone while drag to play is on
// (polish task 7), because their answers already glow on the board.
//
// The mulligan is both seats' at once (R265): each answers its own, in either order, and an answer
// is sealed until the other is in. So the mulligan picker says whether the opponent has answered
// yet (`mulligan-opponent-status`, from `view.mulligan`), its confirm reads "Ready", and once the
// viewer has answered the picker gives way to `mulligan-waiting`: the hand, with the cards going
// back marked from the view's own `kept`, until the opponent is ready too and the game begins.
//
// No rule is applied here either. `min` and `max` gate the confirm button, and both came from the
// engine — from `PendingView` on route 1 and from the candidate `play`s on route 2. The options
// likewise: a prompt's options are the engine's, and a play's choices are read off the
// `legalActions` array by `actions.ts`.

import {
  Fragment,
  useEffect,
  useId,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";

import type {
  ActionBody,
  CardView,
  MulliganView,
  PendingView,
  PlayerId,
  PlayerView,
  PromptKind,
  Selection,
} from "@jackioh/shared";

import {
  IDLE,
  highlightFor,
  isBuilding,
  outstandingNeed,
  pickInPlay,
  answerAction,
  plagueKey,
  selectionForOption,
  selectionKey,
  selectionTestid,
  zoneKey,
  zonesInBoardOrder,
  parseZoneKey,
  type Interaction,
  type PlagueChoice,
  type PlayBuild,
  type PlayNeed,
} from "./actions.ts";
import { CardBack, CardFace, faceModel, useInspectTrigger, type FaceModel } from "../cards/index.ts";
import { MatchCardsProvider, useCardInfo, useCopiedDef, useFieldPower } from "./catalog.ts";
import { liveFace } from "./faces.ts";
import { DISCOVER_OPTION_LIMIT, X_CARD_LIMIT, sideOf, testid } from "./contract.ts";
import { modeText } from "./modeText.ts";
import "./prompt.css";

export type PromptProps = {
  view: PlayerView;
  /** The play or activation in flight, for the inline R81 pickers (x, embiggen, zone, tribute, direction, …). */
  interaction?: Interaction;
  legal?: readonly ActionBody[];
  onAction: (body: ActionBody) => void;
  onInteraction?: (next: Interaction) => void;
  onCancel?: () => void;
  /** The event type animating the open modal (`promptAnswered` fades it out), for `data-animating`. */
  animating?: string;
};

/**
 * The seat that is only watching gets NO `data-prompt-kind` at all: §10.8 gives that view the
 * fact that a choice is open and nothing else, and `data-prompt-kind` is the attribute M5-T4
 * animates the open picker on. It is marked with its own flag instead, so "a prompt is open for
 * me" and "somebody is choosing" never read the same in the DOM.
 */
const WAITING_FLAG = "waiting";

/** R14 / §3.1: the only two directions a rotation can take (`RotationDirection` in the engine). */
const DIRECTIONS = ["left", "right"] as const;
const ARROWS: Record<string, string> = { left: "←", right: "→" };

/**
 * R420: an `answer` prompt's options are lettered in the order offered, as the engine names them
 * (`ANSWER_OPTION_IDS`), so a letter is a place and says nothing about which answer is right.
 */
const ANSWER_LETTERS: readonly string[] = ["A", "B", "C", "D", "E", "F", "G", "H"];
/** The answers stand in one column, so the arrows walk them up and down. */
const ANSWER_COLUMNS = 1;
/** C #18's 0 to 10 sit as 0–5 over 6–10: the arrows move by this, and prompt.css lays the pad out by it. */
const NUMBER_PAD_COLUMNS = 6;
/** B5 E18's kinds take the focus as they open, so a keyboard player can answer at once. */
const FOCUS_ON_OPEN: ReadonlySet<PromptKind> = new Set(["number", "answer", "cell", "reward", "pick"]);
/** R404: the reward picker's header; the engine's prompt names the quest after one of its own. */
const QUEST_COMPLETE = "Quest complete!";
const QUEST_COMPLETE_PREFIX = /^quest complete[:!]?\s*/i;
/** R515: what a card that would go over a pick's budget says in its tooltip and its label. */
const OVER_BUDGET = "Over the budget";

type PickerItem = {
  key: string;
  label: string;
  /** Set when the option names a card, so the picker can show its real name (`catalog.ts`). */
  defId?: string;
  radiant?: boolean;
  /** The card's cost as the view carries it (`CardView.cost`), for its face's gem. */
  cost?: number;
  /** The card as the view lists it, when it lists it: its face is then the card as it stands. */
  card?: CardView;
  /** Where the card sits ("Enemy unit, lane 2"), which tells two copies of one card apart. */
  where?: string;
  /** The row a zone option sits in, so the zone grid can group by it without parsing keys. */
  group?: string;
  /** Which arrow a direction option draws. Never read off the key, which is the engine's. */
  arrow?: "left" | "right";
  /** A "Choose one" option's line of detail under its label (modeText.ts). */
  detail?: string;
  /** #492: what picking a card option chooses, under its face ("Embiggened", "X = 2"). */
  caption?: string;
  /** #492: an embiggen option, whose face shows the price that form is played for (`pricedFace`). */
  price?: EmbiggenPrice;
};

type EmbiggenPrice = "normal" | "embiggen";

type Submitted = { action?: ActionBody; interaction?: Interaction };

type Picker = {
  /** Which picker to draw, and the `data-prompt-kind` value M5-T4 animates on. */
  chrome: PromptKind;
  /**
   * A play's payment drawn its own way whatever its chrome: `plague` is the Plague Counter count of a
   * graveyard play (Classic #74), a `number` prompt to the DOM and a row of chips to the eye.
   */
  variant?: "plague";
  /**
   * #492: drawn as the card picker (`isCardLayout`) whatever its chrome: an embiggen price, or an X
   * from X_CARD_LIMIT values or fewer. The chrome stays the `data-prompt-kind`.
   */
  cards?: boolean;
  title: string;
  /** The card asking, when the picker knows it: its name heads the title ("Pocket Chaos: choose one"). */
  sourceDefId?: string;
  items: PickerItem[];
  min: number;
  max: number;
  /** A one-of-N picker sends as soon as an option is clicked. */
  immediate: boolean;
  /** What is chosen before the player touches anything: every card, for a mulligan. */
  initial?: readonly string[];
  submit: (keys: readonly string[]) => Submitted;
  /** R515: a `pick` prompt's budget, and what each option counts against it (its `cost`). */
  budget?: { limit: number; costs: ReadonlyMap<string, number> };
  /** R514: the board zones that answer a `cell` prompt, by testid, to the option each answers with. */
  boardKeys?: ReadonlyMap<string, string>;
};

/** The pickers whose options are cards by nature. */
const CARD_CHROMES: ReadonlySet<PromptKind> = new Set(["discover", "hand", "mulligan", "pick"]);

/**
 * #492: the card picker, front and centre: every option a card in the middle of the screen, the way
 * a Discover and Appropriations' "Choose one" are laid out (`CardChoice`). A short "Choose one" is
 * one (its chrome is `discover`), and so are a play's embiggen price and an X from X_CARD_LIMIT
 * values or fewer, which keep their own chrome. It is `data-prompt-layout="cards"`, which prompt.css
 * reads to keep it out of the small pickers' dock beside the board.
 */
function isCardLayout(picker: Picker): boolean {
  return picker.cards === true || CARD_CHROMES.has(picker.chrome);
}

// ---------------------------------------------------------------------------------------------
// Labels. Read out of the view, never computed.
// ---------------------------------------------------------------------------------------------

type CardRef = { defId: string; radiant: boolean; cost: number; card: CardView };

function refOf(card: CardView): CardRef {
  return { defId: card.defId, radiant: card.radiant, cost: card.cost, card };
}

/** Where an instance id sits in the viewer's own view, for a picker label and the option's face. */
function cardRefFor(view: PlayerView, instanceId: string): CardRef | null {
  for (const side of [view.you, view.opponent]) {
    for (const pile of side.units) {
      if (pile !== null && pile.instanceId === instanceId) return refOf(pile);
    }
    for (const slot of side.backrow) {
      if (slot !== null && slot.faceDown === false && slot.instanceId === instanceId) return refOf(slot);
    }
    const piles = [Array.isArray(side.hand) ? side.hand : [], side.graveyard, side.exile];
    for (const pile of piles) {
      for (const card of pile) {
        if (card.instanceId === instanceId) return refOf(card);
      }
    }
  }
  return null;
}

/**
 * Where an instance sits, as a player would say it, for a list picker's label: a target or a
 * Tribute is told apart from another copy of the same card by its place, never by its instance id.
 */
function whereOf(view: PlayerView, instanceId: string): string | null {
  for (const side of [view.you, view.opponent]) {
    const whose = side === view.you ? "Your" : "Enemy";
    const unitAt = side.units.findIndex((pile) => pile !== null && pile.instanceId === instanceId);
    if (unitAt >= 0) return `${whose} unit, lane ${String(unitAt + 1)}`;
    const backrowAt = side.backrow.findIndex(
      (slot) => slot !== null && slot.faceDown === false && slot.instanceId === instanceId,
    );
    if (backrowAt >= 0) return `${whose} backrow, lane ${String(backrowAt + 1)}`;
    const hand = Array.isArray(side.hand) ? side.hand : [];
    if (hand.some((card) => card.instanceId === instanceId)) return `${whose} hand`;
    if (side.graveyard.some((card) => card.instanceId === instanceId)) return `${whose} graveyard`;
    if (side.exile.some((card) => card.instanceId === instanceId)) return `${whose} exile`;
  }
  return null;
}

function itemForInstance(view: PlayerView, instanceId: string, fallback: string): PickerItem {
  const ref = cardRefFor(view, instanceId);
  const where = whereOf(view, instanceId);
  const item: PickerItem = ref === null
    ? { key: instanceId, label: fallback }
    : { key: instanceId, label: fallback, defId: ref.defId, radiant: ref.radiant, cost: ref.cost, card: ref.card };
  if (where !== null) item.where = where;
  return item;
}

/** Lanes are 1-based, as `contract.ts` and the engine's `zones.ts` have them. */
function zoneLabel(row: string, lane: number): string {
  return `${row === "units" ? "Unit" : "Backrow"} lane ${lane}`;
}

function selectionLabel(view: PlayerView, selection: Selection): string {
  switch (selection.pick) {
    case "instance":
      return cardRefFor(view, selection.instanceId)?.defId ?? selection.instanceId;
    case "hero":
      return `${sideOf(view, selection.player) === "you" ? "Your" : "Enemy"} hero`;
    case "zone":
      return `${sideOf(view, selection.player) === "you" ? "Your" : "Enemy"} ${zoneLabel(selection.row, selection.lane)}`;
    case "mode":
      return selection.option;
    case "none":
      return "Nothing";
  }
}

// ---------------------------------------------------------------------------------------------
// Route 1: an open `PendingChoice`.
// ---------------------------------------------------------------------------------------------

function pickerForPending(
  pending: Extract<PendingView, { forYou: true }>,
  view: PlayerView,
  legal: readonly ActionBody[],
): Picker {
  const items = pending.options.map((option): PickerItem => {
    const base: PickerItem = { key: option.key, label: option.label };
    if (option.defId !== undefined) base.defId = option.defId;
    // B5 E17, E18: a card the view lists nowhere (the opponent's hand, C #11) is drawn from the
    // option itself, Radiant and at its cost where the option says so.
    if (option.radiant === true) base.radiant = true;
    if (option.cost !== undefined) base.cost = option.cost;
    if (option.row !== undefined) base.group = option.row;
    // R514: a `cell` is a zone of either side, so its list groups by side and row, lane by lane.
    if (pending.kind === "cell" && option.player !== undefined && option.row !== undefined && option.lane !== undefined) {
      base.group = `${sideOf(view, option.player) === "you" ? "Your" : "Enemy"} ${option.row}`;
      base.label = `Lane ${String(option.lane)}`;
    }
    if (option.instanceId !== undefined) {
      const ref = cardRefFor(view, option.instanceId);
      if (ref !== null) {
        base.defId = ref.defId;
        base.radiant = ref.radiant;
        base.cost = ref.cost;
        base.card = ref.card;
      }
      const where = whereOf(view, option.instanceId);
      if (where !== null) base.where = where;
    }
    // The engine prefixes a direction key (`mode:left`), so the arrow comes off the label or the
    // key's tail, never off the whole key: `data-testid="direction-left"` is the M5-T2 contract.
    const arrow = DIRECTIONS.find((d) => option.label === d || option.key === d || option.key.endsWith(`:${d}`));
    if (arrow !== undefined) base.arrow = arrow;
    return base;
  });

  // A short "Choose one" menu is a Discover pop-up; more options keep the plain mode list.
  const chrome = pending.kind === "mode" && pending.options.length <= DISCOVER_OPTION_LIMIT ? "discover" : pending.kind;
  // #492: an embiggen price and a short X are cards here too, as on a play (`pickerForNeed`).
  const cards = pending.kind === "embiggen" || (pending.kind === "x" && pending.options.length <= X_CARD_LIMIT);
  const picker: Picker = {
    chrome,
    ...(cards ? { cards } : {}),
    title: pending.kind === "reward" ? pending.prompt.replace(QUEST_COMPLETE_PREFIX, "") : pending.prompt,
    items,
    min: pending.min,
    max: pending.max,
    // The mulligan is per-card toggles plus a confirm, whatever its max.
    immediate: pending.kind !== "mulligan" && pending.max <= 1,
    // R9's answer names the cards KEPT, and Hearthstone keeps the whole hand until a card is
    // marked to go back, so a mulligan opens with every card kept: Confirm alone keeps the hand,
    // and a tap marks a card for a redraw.
    ...(pending.kind === "mulligan" ? { initial: items.slice(0, pending.max).map((item) => item.key) } : {}),
    // The picks in the order offered, so the same picks always build the same answer.
    submit: (keys) => ({
      action: answerAction(pending, items.map((item) => item.key).filter((key) => keys.includes(key)), view, legal),
    }),
  };
  if (pending.kind === "pick" && pending.budget !== undefined) {
    picker.budget = { limit: pending.budget, costs: new Map(pending.options.map((option) => [option.key, option.cost ?? 0])) };
  }
  if (pending.kind === "cell") {
    picker.boardKeys = new Map(
      pending.options.flatMap((option) => {
        const where = option.row === undefined ? null : selectionTestid(view, selectionForOption(option));
        return where === null ? [] : [[where, option.key] as const];
      }),
    );
  }
  return picker;
}

// ---------------------------------------------------------------------------------------------
// Route 2: an R81 choice of a play or an activation. Submits a `play` or an `activate`, never an
// `answer`.
// ---------------------------------------------------------------------------------------------

function isDirection(options: readonly string[]): boolean {
  return options.length > 0 && options.every((option) => DIRECTIONS.some((d) => d === option));
}

/**
 * R81 puts a declared `hand` pick (#26 Glowy Jelly Bean) in the play action's `targets`, so it
 * arrives here as a `target` need — but §10.6 still names `hand` as its own prompt kind and
 * BUILD M5-T2 draws it as its own picker. The only thing that tells the two apart is where the
 * offered instances live, which `PlayerView` already says: every candidate being a card in the
 * viewer's own hand is a hand pick, anything else (a unit, a hero, a zone) is a target pick.
 * Presentation only — the action built is the same `play` either way.
 */
function isHandPick(need: PlayNeed, view: PlayerView): boolean {
  if (need.kind !== "target") return false;
  const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
  if (hand.length === 0 || need.selections.length === 0) return false;
  return need.selections.every(
    (selection) =>
      selection.pick === "instance" &&
      hand.some((card) => card.instanceId === selection.instanceId),
  );
}

/** "Pay in mana only", "Spend 2 Plague Counters", and where they come from when several cards pay. */
function plagueLabel(view: PlayerView, option: PlagueChoice, nameSource: boolean): string {
  if (option === "none") return "Pay in mana only";
  const tokens = `Spend ${String(option.tokens)} Plague ${option.tokens === 1 ? "Counter" : "Counters"}`;
  const where = nameSource ? whereOf(view, option.from) : null;
  return where === null ? tokens : `${tokens} (${where})`;
}

/** The card the play (or activation) in flight is building, as the view lists it. */
function playedRef(interaction: Interaction, view: PlayerView): CardRef | null {
  return isBuilding(interaction) ? cardRefFor(view, interaction.instanceId) : null;
}

/** #492: a card option drawn as the card being played, as the view lists it. */
function playedFace(played: CardRef): Pick<PickerItem, "defId" | "radiant" | "cost" | "card"> {
  return { defId: played.defId, radiant: played.radiant, cost: played.cost, card: played.card };
}

function pickerForNeed(need: PlayNeed, interaction: Interaction, view: PlayerView): Picker {
  const play = (patch: Partial<PlayBuild>): Submitted => {
    const result = pickInPlay(interaction, patch);
    return result.action === undefined
      ? { interaction: result.interaction }
      : { interaction: result.interaction, action: result.action };
  };
  const common = { min: need.min, max: need.max, immediate: need.max <= 1 };

  switch (need.kind) {
    case "zone":
      return {
        ...common,
        chrome: "zone",
        title: "Choose a zone",
        items: zonesInBoardOrder(need.zones).map((zone) => ({
          key: zoneKey(zone),
          label: zoneLabel(zone.row, zone.lane),
          group: zone.row,
        })),
        submit: (keys) => {
          const zone = keys[0] === undefined ? null : parseZoneKey(keys[0]);
          return zone === null ? {} : play({ zone });
        },
      };
    case "x": {
      // #492: a few values are cards, the card being played once per X; more keep the stepper.
      const cards = need.values.length <= X_CARD_LIMIT;
      const played = cards ? playedRef(interaction, view) : null;
      return {
        ...common,
        chrome: "x",
        ...(cards ? { cards } : {}),
        ...(played === null ? {} : { sourceDefId: played.defId }),
        title: "Choose X",
        items: need.values.map((value) => {
          const item: PickerItem = { key: String(value), label: String(value) };
          return played === null ? item : { ...item, ...playedFace(played), caption: `X = ${String(value)}` };
        }),
        submit: (keys) => (keys[0] === undefined ? {} : play({ x: Number(keys[0]) })),
      };
    }
    case "embiggen": {
      // #492: the two prices are two cards, front and centre: the card being played at the price it
      // shows in hand, and the same card at its embiggen price, each named for the form it plays.
      const played = playedRef(interaction, view);
      return {
        ...common,
        chrome: "embiggen",
        cards: true,
        ...(played === null ? {} : { sourceDefId: played.defId }),
        title: "Pay the embiggen price?",
        items: need.values.map((value) => {
          const label = value ? "Embiggened" : "Normal";
          const item: PickerItem = { key: String(value), label };
          const price: EmbiggenPrice = value ? "embiggen" : "normal";
          return played === null ? item : { ...item, ...playedFace(played), caption: label, price };
        }),
        submit: (keys) => (keys[0] === undefined ? {} : play({ embiggen: keys[0] === "true" })),
      };
    }
    case "tribute":
      return {
        ...common,
        chrome: "tribute",
        title: "Choose Tributes",
        items: need.instanceIds.map((id) => itemForInstance(view, id, id)),
        submit: (keys) => play({ tributes: [...keys] }),
      };
    case "target": {
      const byKey = new Map(need.selections.map((selection) => [selectionKey(selection), selection]));
      // R81: a declared `target` pick whose selections are all hand cards is rendered with the hand
      // chrome, so the player sees the pick where the cards are. (This was left short-circuited to
      // `false` by an abandoned experiment, which made the branch below dead and the hand picker
      // unreachable — e2e spec 02 asserts it for the hand kind.)
      const inHand = isHandPick(need, view);
      return {
        ...common,
        chrome: inHand ? "hand" : "target",
        title: inHand ? "Choose a card in your hand" : "Choose a target",
        items: need.selections.map((selection) => {
          const key = selectionKey(selection);
          const label = selectionLabel(view, selection);
          if (selection.pick !== "instance") return { key, label };
          const ref = cardRefFor(view, selection.instanceId);
          const where = whereOf(view, selection.instanceId);
          const item: PickerItem =
            ref === null
              ? { key, label }
              : { key, label, defId: ref.defId, radiant: ref.radiant, cost: ref.cost, card: ref.card };
          if (where !== null) item.where = where;
          return item;
        }),
        submit: (keys) => {
          const targets = keys.flatMap((key) => {
            const selection = byKey.get(key);
            return selection === undefined ? [] : [selection];
          });
          return play({ targets });
        },
      };
    }
    case "plague": {
      // B5 E11, E19 (Classic #74): how much of the price Plague Counters pay, one option per way the
      // engine listed; the rest is mana. Several paying cards are told apart by where they stand.
      const sources = new Set(need.options.flatMap((option) => (option === "none" ? [] : [option.from])));
      const byKey = new Map(need.options.map((option) => [plagueKey(option), option]));
      return {
        ...common,
        chrome: "number",
        variant: "plague",
        title: "Pay with Plague Counters?",
        items: need.options.map((option) => ({
          key: plagueKey(option),
          label: plagueLabel(view, option, sources.size > 1),
        })),
        submit: (keys) => {
          const choice: PlagueChoice | undefined = keys[0] === undefined ? undefined : byKey.get(keys[0]);
          return choice === undefined ? {} : play({ plague: choice });
        },
      };
    }
    case "mode": {
      // The card being played (or activated, R384) is the one asking; its options read as that
      // card's words, on the face it is played with (#24's radiant 2X, 4X and X).
      const played = isBuilding(interaction) ? cardRefFor(view, interaction.instanceId) : null;
      const source = played?.defId ?? undefined;
      const radiant = played?.radiant === true;
      // B5 E18, R81: C #18's number travels in the play's modes, every option a whole number.
      const numbers = need.options.length > 0 && need.options.every((option) => NUMBER_OPTION.test(option));
      // A short "Choose one" menu is a Discover pop-up; more options keep the plain mode list.
      const discover = !numbers && !isDirection(need.options) && need.options.length <= DISCOVER_OPTION_LIMIT;
      const picker: Picker = {
        ...common,
        chrome: numbers ? "number" : isDirection(need.options) ? "direction" : discover ? "discover" : "mode",
        title: numbers ? "Choose a number" : isDirection(need.options) ? "Choose a direction" : "Choose one",
        items: need.options.map((option): PickerItem => {
          const arrow = DIRECTIONS.find((d) => d === option);
          if (arrow !== undefined) return { key: option, label: option, arrow };
          const text = modeText(source, option, radiant);
          return text.detail === undefined
            ? { key: option, label: text.label }
            : { key: option, label: text.label, detail: text.detail };
        }),
        submit: (keys) => play({ modes: [...keys] }),
      };
      if (source !== undefined) picker.sourceDefId = source;
      return picker;
    }
  }
}

// ---------------------------------------------------------------------------------------------
// The modal.
// ---------------------------------------------------------------------------------------------

/** R247: a Discover option that is a number rather than a card (#82 KY's Trial). */
const NUMBER_OPTION = /^\d+$/;

/** #492: the gem of an embiggened option whose price the face cannot vouch for (`pricedFace`). */
const UNKNOWN_PRICE = "?";

/**
 * #492: an embiggen option's face at the price its form is played for. The normal form is the card
 * as the view shows it, at the price it costs now, without the embiggen price beside the gem. The
 * embiggened form is the same face at the embiggen price the gem shows beside it (`FaceCost.alt`,
 * cards/model.ts), the price the card's own text names ("Paid (4)"). Once a cost change has moved
 * the card off its printed price the face drops that price, and the gem says "?": the client never
 * works out an embiggen price itself (CLAUDE.md rule 7), and the engine charges the price it rules.
 */
function pricedFace(face: FaceModel, price: EmbiggenPrice): FaceModel {
  if (price === "normal") return { ...face, cost: { ...face.cost, alt: null } };
  const text = face.cost.alt ?? UNKNOWN_PRICE;
  return { ...face, cost: { text, value: text, tone: "base", alt: null } };
}

/** What an embiggen option pays, in R432's words: "Pay (4)". */
function priceLine(face: FaceModel): string {
  return face.cost.text === UNKNOWN_PRICE ? "Pay the embiggen price" : `Pay (${face.cost.text})`;
}

// Polish 6 (a minimal edit to task 7's file, flagged in the PR): a card option draws the card's face,
// the one the hand and the deck builder draw, with the same hover preview and long-press sheet, in
// whatever box prompt.css gives it. An option that names no card keeps its name and text.
function CardOption(props: {
  item: PickerItem;
  pressed: boolean;
  /** A mulligan's options say what Confirm will do to each card. */
  verdicts?: boolean;
  /** R515: the card would take a pick past its budget, so it is greyed and cannot be added. */
  over?: boolean;
  onPick: () => void;
}) {
  const info = useCardInfo(props.item.defId ?? "", props.item.radiant === true);
  const fieldPower = useFieldPower(props.item.card?.instanceId);
  const copied = useCopiedDef(props.item.card);
  const name = props.item.defId === undefined ? props.item.label : info.name;
  const radiant = props.item.radiant === true;
  const cost = props.item.cost;
  // The card in play: as the view lists it when it does, else a definition as the game shows it.
  const drawn =
    props.item.defId === undefined
      ? null
      : props.item.card !== undefined
        ? liveFace(info, props.item.card, {
            ...(fieldPower === undefined ? {} : { fieldPower }),
            ...(copied === undefined ? {} : { copied }),
          })
        : faceModel({
            defId: props.item.defId,
            def: info.def,
            name: info.name,
            radiant,
            ...(cost === undefined ? {} : { liveCost: cost }),
            inPlay: {},
          });
  const face = drawn === null || props.item.price === undefined ? drawn : pricedFace(drawn, props.item.price);
  // #492: what this card picks, under its face; an embiggen option also says what it pays.
  const caption = face === null ? undefined : props.item.caption;
  const pays = face === null || props.item.price === undefined ? undefined : priceLine(face);
  // R247: a number names no card here, so it is drawn on a card back and nothing opens it.
  const number = face === null && NUMBER_OPTION.test(props.item.label) ? props.item.label : null;
  const testId = `prompt-option-${props.item.key}`;
  // Lines of code is a hidden stat in matches.
  const inspect = useInspectTrigger(face === null ? null : { key: testId, face }, { prefer: "above", showLoc: false });
  const verdict = props.verdicts === true ? (props.pressed ? "keep" : "redraw") : undefined;
  // The name, then the cost the gem shows, so a screen reader hears what a sighted player reads, in
  // R432's words ("costs (3)").
  const faceLabel = `${name}, costs (${face?.cost.text ?? ""})`;
  const named =
    face === null ? (number === null ? undefined : `Number ${number}`) : caption === undefined ? faceLabel : `${caption}: ${faceLabel}`;
  const label = props.over === true && named !== undefined ? `${named}, ${OVER_BUDGET.toLowerCase()}` : named;
  return (
    <>
      <button
        type="button"
        className={number === null ? "prompt-card" : "prompt-card prompt-card--number"}
        data-testid={testId}
        data-verdict={verdict}
        data-number={number ?? undefined}
        data-price={face === null ? undefined : props.item.price}
        data-over-budget={props.over === true ? "true" : undefined}
        aria-pressed={props.pressed}
        aria-disabled={props.over === true ? "true" : undefined}
        aria-label={label}
        title={props.over === true ? OVER_BUDGET : undefined}
        onClick={props.onPick}
        {...inspect.handlers}
      >
        {number !== null ? (
          <span className="cf-option prompt-number">
            <CardBack />
            <span className="prompt-number-value" aria-hidden="true">
              {number}
            </span>
          </span>
        ) : face === null ? (
          <>
            <span className="prompt-card-name">{name}</span>
            {/* A Discover menu's text option (a card's modes): the effect rides `detail`. */}
            {props.item.detail === undefined ? (
              info.text === "" ? null : <span className="prompt-card-text">{info.text}</span>
            ) : (
              <span className="prompt-card-text">{props.item.detail}</span>
            )}
          </>
        ) : (
          <span className="cf-option">
            <CardFace face={face} layout="full" />
          </span>
        )}
        {caption === undefined ? null : (
          <span className="prompt-card-caption">
            <span className="prompt-card-caption-label">{caption}</span>
            {pays === undefined ? null : <span className="prompt-card-caption-detail">{pays}</span>}
          </span>
        )}
        {verdict === undefined ? null : (
          <span className="prompt-card-verdict" aria-hidden="true">
            {verdict === "keep" ? "Keep" : "Redraw"}
          </span>
        )}
      </button>
      {inspect.overlay}
    </>
  );
}

function ListOption(props: { item: PickerItem; pressed: boolean; onPick: () => void }) {
  const info = useCardInfo(props.item.defId ?? "", props.item.radiant === true);
  const name =
    props.item.defId === undefined ? props.item.label : `${info.name} — ${props.item.where ?? props.item.label}`;
  return (
    <li>
      <button
        type="button"
        data-testid={`prompt-option-${props.item.key}`}
        aria-pressed={props.pressed}
        onClick={props.onPick}
      >
        {name}
      </button>
    </li>
  );
}

function PlainOption(props: { item: PickerItem; pressed: boolean; onPick: () => void }) {
  return (
    <button
      type="button"
      className={props.item.detail === undefined ? undefined : "prompt-mode"}
      data-testid={`prompt-option-${props.item.key}`}
      aria-pressed={props.pressed}
      onClick={props.onPick}
    >
      {props.item.detail === undefined ? (
        props.item.label
      ) : (
        <>
          <span className="prompt-mode-label">{props.item.label}</span>
          <span className="prompt-mode-detail">{props.item.detail}</span>
        </>
      )}
    </button>
  );
}

/** Arrow keys walk a grid of option buttons `columns` wide; Enter or Space presses the focused one. */
function arrowFocus(columns: number) {
  const steps: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -columns, ArrowDown: columns };
  return (event: ReactKeyboardEvent<HTMLElement>): void => {
    const step = steps[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const buttons = [...event.currentTarget.querySelectorAll<HTMLElement>("button")];
    const at = event.target instanceof HTMLElement ? buttons.indexOf(event.target) : -1;
    if (at >= 0) buttons[at + step]?.focus();
  };
}

/** The picker's heading: the asking card's name before it, when the picker knows the card. */
function PickerTitle(props: { title: string; sourceDefId: string | undefined }) {
  const info = useCardInfo(props.sourceDefId ?? "", false);
  if (props.sourceDefId === undefined) return <p className="prompt-title">{props.title}</p>;
  return (
    <p className="prompt-title">
      <span className="prompt-title-source">{info.name}</span>
      <span className="prompt-title-ask">{props.title}</span>
    </p>
  );
}

/**
 * A board pick's hint. On a phone held upright, with drag to play on, a play's zone, target and
 * Tribute picks fold to a bar with no options in it (prompt.css, polish task 7): the answers glow on
 * the board and are tapped there, and this line says so. Every other layout lists the options, and
 * the stylesheet hides it there.
 */
const BOARD_HINTS: Partial<Record<PromptKind, string>> = {
  zone: "Tap a highlighted zone on the board.",
  target: "Tap a highlighted target on the board.",
  tribute: "Tap a highlighted unit on the board.",
};

function BoardHint(props: { chrome: PromptKind }) {
  const hint = BOARD_HINTS[props.chrome];
  return hint === undefined ? null : <p className="prompt-board-hint">{hint}</p>;
}

/**
 * #492: the card picker (`isCardLayout`): a mulligan, a Discover, a card from hand, a pick, a short
 * "Choose one" (Appropriations'), an embiggen price and a short X, each option a card in a row in
 * the middle of the picker. `data-choice` names the picker's chrome.
 */
function CardChoice(props: {
  picker: Picker;
  /** R515: what the picks so far cost together. */
  spent: number;
  pressed: (key: string) => boolean;
  /** R515: whether adding this option keeps the picks within the budget. */
  fits: (key: string) => boolean;
  pick: (key: string) => void;
}) {
  const { picker } = props;
  const budget = picker.budget;
  return (
    <>
      {budget === undefined ? null : (
        <p className="prompt-budget" data-testid="prompt-budget" role="status">
          ({props.spent}) of ({budget.limit}) spent
        </p>
      )}
      <div className="prompt-cards" data-testid="prompt-cards" data-choice={picker.chrome}>
        {picker.items.map((item) => (
          <CardOption
            key={item.key}
            item={item}
            pressed={props.pressed(item.key)}
            verdicts={picker.chrome === "mulligan"}
            over={!props.pressed(item.key) && !props.fits(item.key)}
            onPick={() => props.pick(item.key)}
          />
        ))}
      </div>
    </>
  );
}

function PromptModal(props: {
  picker: Picker;
  /** Which route opened it: an engine prompt (`view.pending`) or a play still being built (R81). */
  source: "engine" | "play";
  /** A line under the count: the mulligan's word on whether the opponent has answered (R265). */
  status?: ReactNode;
  boardTestids: readonly string[];
  onAction: (body: ActionBody) => void;
  onInteraction?: (next: Interaction) => void;
  onCancel?: () => void;
  animating?: string;
}) {
  const { picker } = props;
  const [selected, setSelected] = useState<readonly string[]>(() => picker.initial ?? []);
  /**
   * What is typed in the X field, which is not the same thing as what has been chosen: a field
   * being cleared, or holding a number the engine did not offer, stages nothing. `null` means
   * "nothing typed since the last stepper press", so the field follows the stepper.
   */
  const [typedX, setTypedX] = useState<string | null>(null);
  // R515: what the picks cost together, and whether they fit the budget (a hint; the engine rules).
  const spent = (keys: readonly string[]): number => keys.reduce((sum, key) => sum + (picker.budget?.costs.get(key) ?? 0), 0);
  const fits = (keys: readonly string[]): boolean => picker.budget === undefined || spent(keys) <= picker.budget.limit;
  const inRange = selected.length >= picker.min && selected.length <= picker.max && fits(selected);
  const panel = useRef<HTMLDivElement>(null);

  function send(keys: readonly string[]): void {
    const out = picker.submit(keys);
    if (out.interaction !== undefined) props.onInteraction?.(out.interaction);
    if (out.action !== undefined) props.onAction(out.action);
  }

  /** Stages a value without sending it: the X stepper's − and +. */
  function stage(key: string): void {
    setSelected([key]);
  }

  function pick(key: string): void {
    if (!selected.includes(key) && !fits([...selected, key])) return;
    if (picker.immediate) {
      send([key]);
      return;
    }
    setSelected((prev) => {
      if (prev.includes(key)) return prev.filter((k) => k !== key);
      if (prev.length >= picker.max) return prev;
      return [...prev, key];
    });
  }

  const pressed = (key: string): boolean => selected.includes(key);

  useEffect(() => {
    if (!FOCUS_ON_OPEN.has(picker.chrome)) return;
    const first = panel.current?.querySelector<HTMLElement>('[data-testid^="prompt-option-"]:not([aria-disabled="true"])');
    first?.focus({ preventScroll: true });
  }, [picker.chrome]);

  // R514: a click on a zone of the board a `cell` prompt offers, or Enter or Space on it, picks that
  // cell. The board's own handler builds nothing from a zone while a prompt is open, so this is the
  // one answer. ponytail: document listeners for the one board-answered prompt kind; move them into
  // `onClickTarget` (actions.ts) if target prompts are ever answered on the board too.
  const live = useRef({ pick, keys: picker.boardKeys });
  live.current = { pick, keys: picker.boardKeys };
  const answersOnBoard = picker.boardKeys !== undefined;
  useEffect(() => {
    if (!answersOnBoard) return;
    const zone = '[data-testid^="zone-"]';
    const keyOf = (element: Element | null): string | undefined =>
      live.current.keys?.get(element?.getAttribute("data-testid") ?? "");
    const onClick = (event: MouseEvent): void => {
      const target = event.target;
      if (!(target instanceof Element) || target.closest(".prompt") !== null) return;
      const key = keyOf(target.closest(zone));
      if (key !== undefined) live.current.pick(key);
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      const target = event.target;
      if ((event.key !== "Enter" && event.key !== " ") || !(target instanceof Element) || !target.matches(zone)) return;
      const key = keyOf(target);
      if (key !== undefined) live.current.pick(key);
    };
    document.addEventListener("click", onClick);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("click", onClick);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [answersOnBoard]);

  function body(): ReactNode {
    const items = picker.items;

    if (picker.variant === "plague") {
      return (
        <div className="prompt-chips prompt-plague" role="group" aria-label="Plague Counters">
          {items.map((item) => (
            <PlainOption key={item.key} item={item} pressed={pressed(item.key)} onPick={() => pick(item.key)} />
          ))}
        </div>
      );
    }

    if (isCardLayout(picker)) {
      return (
        <CardChoice
          picker={picker}
          spent={spent(selected)}
          pressed={pressed}
          fits={(key) => fits([...selected, key])}
          pick={pick}
        />
      );
    }

    if (picker.chrome === "direction") {
      return (
        <div className="prompt-direction">
          {items.map((item) => (
            <div
              key={item.key}
              className="prompt-arrow"
              data-testid={item.arrow === undefined ? `direction-${item.key}` : `direction-${item.arrow}`}
              role="button"
              tabIndex={0}
              aria-pressed={pressed(item.key)}
              onClick={() => pick(item.key)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") pick(item.key);
              }}
            >
              <span aria-hidden="true">{item.arrow === undefined ? "•" : ARROWS[item.arrow]}</span>
              <span data-testid={`prompt-option-${item.key}`}>{item.label}</span>
            </div>
          ))}
        </div>
      );
    }

    if (picker.chrome === "x") {
      // The stepper walks the option list the engine offered; it never invents a value.
      const chosen = selected[0];
      const at = chosen === undefined ? 0 : Math.max(items.findIndex((item) => item.key === chosen), 0);
      const step = (delta: number): void => {
        const next = items[Math.min(Math.max(at + delta, 0), items.length - 1)];
        if (next === undefined) return;
        setTypedX(null);
        stage(next.key);
      };
      // Typing stages the value only when the engine offered it; anything else stages nothing, so
      // an X the play cannot take can never be confirmed.
      const typeX = (raw: string): void => {
        setTypedX(raw);
        const match = items.find((item) => item.key === raw);
        setSelected(match === undefined ? [] : [match.key]);
      };
      return (
        <>
          <div className="prompt-stepper" data-testid="x-stepper" role="group" aria-label="Choose X">
            <button type="button" data-testid="x-minus" aria-label="Lower X" onClick={() => step(-1)}>
              −
            </button>
            {/* The number is both typeable and readable: `prompt-x` is the input the value is
                typed into, `x-value` the label the stepper moves. */}
            <input
              className="prompt-x-input"
              data-testid="prompt-x"
              type="number"
              inputMode="numeric"
              aria-label="X"
              min={items[0]?.key}
              max={items[items.length - 1]?.key}
              value={typedX ?? chosen ?? ""}
              onChange={(event) => typeX(event.target.value)}
            />
            <span className="prompt-stepper-value" data-testid="x-value">
              {items[at]?.label ?? ""}
            </span>
            <button type="button" data-testid="x-plus" aria-label="Raise X" onClick={() => step(1)}>
              +
            </button>
          </div>
          <div className="prompt-chips">
            {items.map((item) => (
              <PlainOption key={item.key} item={item} pressed={pressed(item.key)} onPick={() => pick(item.key)} />
            ))}
          </div>
        </>
      );
    }

    if (picker.chrome === "zone" || picker.chrome === "cell") {
      const groups = [...new Set(items.map((item) => item.group ?? ""))];
      return (
        <div className="prompt-zones">
          {groups.map((group) => {
            const inGroup = items.filter((item) => (item.group ?? "") === group);
            return (
              <Fragment key={group}>
                {group === "" ? null : <span className="prompt-zone-row">{group}</span>}
                {inGroup.map((item) => (
                  <PlainOption key={item.key} item={item} pressed={pressed(item.key)} onPick={() => pick(item.key)} />
                ))}
              </Fragment>
            );
          })}
        </div>
      );
    }

    if (picker.chrome === "number") {
      return (
        <div
          className="prompt-numbers"
          role="group"
          aria-label="Numbers"
          style={{ "--pad-columns": String(NUMBER_PAD_COLUMNS) } as CSSProperties}
          onKeyDown={arrowFocus(NUMBER_PAD_COLUMNS)}
        >
          {items.map((item) => (
            <PlainOption key={item.key} item={item} pressed={pressed(item.key)} onPick={() => pick(item.key)} />
          ))}
        </div>
      );
    }

    if (picker.chrome === "answer") {
      return (
        <div className="prompt-answers" role="group" aria-label="Answers" onKeyDown={arrowFocus(ANSWER_COLUMNS)}>
          {items.map((item, at) => {
            const letter = ANSWER_LETTERS[at] ?? String(at + 1);
            return (
              <button
                key={item.key}
                type="button"
                data-testid={`prompt-option-${item.key}`}
                data-letter={letter}
                aria-pressed={pressed(item.key)}
                aria-label={`${letter}: ${item.label}`}
                onClick={() => pick(item.key)}
              >
                <span className="prompt-answer-letter" aria-hidden="true">
                  {letter}
                </span>
                <span>{item.label}</span>
              </button>
            );
          })}
        </div>
      );
    }

    if (picker.chrome === "reward") {
      return (
        <div className="prompt-rewards" role="group" aria-label="Rewards">
          {items.map((item) => (
            <PlainOption key={item.key} item={item} pressed={pressed(item.key)} onPick={() => pick(item.key)} />
          ))}
        </div>
      );
    }

    if (picker.chrome === "target" || picker.chrome === "tribute") {
      return (
        <ul className="prompt-list">
          {items.map((item) => (
            <ListOption key={item.key} item={item} pressed={pressed(item.key)} onPick={() => pick(item.key)} />
          ))}
        </ul>
      );
    }

    return (
      <div className="prompt-modes">
        {items.map((item) => (
          <PlainOption key={item.key} item={item} pressed={pressed(item.key)} onPick={() => pick(item.key)} />
        ))}
      </div>
    );
  }

  const range = picker.min === picker.max ? `${picker.min}` : `${picker.min}–${picker.max}`;

  return (
    <div className="prompt-scrim" data-testid="prompt-scrim">
      <div
        ref={panel}
        className={`prompt prompt-${picker.chrome}`}
        data-testid="prompt-modal"
        data-prompt-kind={picker.chrome}
        data-prompt-layout={isCardLayout(picker) ? "cards" : undefined}
        data-prompt-source={props.source}
        data-animating={props.animating}
        /* The board cells this prompt has blessed, so a `target` pick can be made on the board
           too (BUILD M5-T2). Derived by `highlightFor`, which reads only `legalActions` and the
           prompt's own options. */
        data-board-testids={props.boardTestids.join(" ")}
        role="dialog"
        aria-modal="true"
        aria-label={picker.title}
      >
        {picker.chrome === "reward" ? (
          <p className="prompt-quest-banner" data-testid="prompt-quest-complete">
            {QUEST_COMPLETE}
          </p>
        ) : null}
        <PickerTitle title={picker.title} sourceDefId={picker.sourceDefId} />
        <p className="prompt-count">
          Choose {range}: {selected.length} chosen
        </p>
        {props.status}
        {body()}
        {(picker.chrome === "target" || picker.chrome === "cell") && props.boardTestids.length > 0 ? (
          <p className="prompt-board-note">Highlighted on the board as well.</p>
        ) : null}
        {/* Shown only where the picker folds to a bar with no options in it (prompt.css). */}
        {props.source === "play" ? <BoardHint chrome={picker.chrome} /> : null}
        <div className="prompt-actions">
          <button
            type="button"
            data-testid="prompt-submit"
            aria-disabled={!inRange}
            onClick={() => {
              if (inRange) send(selected);
            }}
          >
            {/* R265: the mulligan's answer is sealed until both seats have given theirs. */}
            {picker.chrome === "mulligan" ? "Ready" : "Confirm"}
          </button>
          {props.onCancel === undefined ? null : (
            <button type="button" data-testid="prompt-cancel" onClick={props.onCancel}>
              Cancel
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * §10.6, §10.8: the other seat learns that a choice is open and nothing else about it. The chooser
 * is always the viewer's opponent (a prompt for the viewer is `forYou`), so the line names the
 * opponent rather than a seat id: in practice that is the AI, which the HUD already shows thinking.
 */
function Waiting(props: { pendingFor: PlayerId }) {
  return (
    <div className="prompt-scrim" data-testid="prompt-scrim">
      <div
        className="prompt prompt-waiting"
        data-testid="prompt-modal"
        data-prompt-waiting={WAITING_FLAG}
        data-pending-for={props.pendingFor}
        role="dialog"
        aria-modal="true"
        aria-label="Waiting for choice"
      >
        <p className="prompt-title">Waiting for choice</p>
        <p className="prompt-sub" role="status">
          Your opponent is choosing…
        </p>
      </div>
    </div>
  );
}

/**
 * R265, R266: in the mulligan picker, whether the opponent has answered its own mulligan. That it
 * has is all the view says — never what it kept (§9.1).
 */
function OpponentMulliganStatus(props: { mulligan: MulliganView }) {
  const ready = props.mulligan.opponentReady;
  return (
    <p
      className="prompt-sub mulligan-status"
      data-testid={testid.mulliganOpponentStatus}
      data-ready={ready ? "true" : "false"}
      role="status"
    >
      {ready ? <span data-testid={testid.mulliganOpponentReady}>Opponent is ready</span> : "Opponent is choosing…"}
    </p>
  );
}

/** A hand card on the waiting panel: its face, and the stamp saying whether it stays or goes back. */
function WaitingCard(props: { card: CardView; keep: boolean }) {
  const info = useCardInfo(props.card.defId, props.card.radiant);
  const fieldPower = useFieldPower(props.card.instanceId);
  const copied = useCopiedDef(props.card);
  const face = liveFace(info, props.card, {
    ...(fieldPower === undefined ? {} : { fieldPower }),
    ...(copied === undefined ? {} : { copied }),
  });
  const verdict = props.keep ? "keep" : "redraw";
  return (
    <span
      className="prompt-card"
      data-testid={`mulligan-waiting-card-${props.card.instanceId}`}
      data-verdict={verdict}
      aria-label={`${info.name}: ${props.keep ? "kept" : "going back"}`}
      role="img"
    >
      <span className="cf-option">
        <CardFace face={face} layout="full" />
      </span>
      <span className="prompt-card-verdict" aria-hidden="true">
        {props.keep ? "Keep" : "Redraw"}
      </span>
    </span>
  );
}

/**
 * R265, R266: the viewer has answered its mulligan and the opponent has not. The answer is sealed —
 * the hand changes only once both are in — so the panel shows the hand as it is, each card stamped
 * with what will happen to it (`kept` is the view's own record of the answer).
 */
function MulliganWaiting(props: { view: PlayerView; mulligan: MulliganView }) {
  const hand = Array.isArray(props.view.you.hand) ? props.view.you.hand : [];
  const kept = new Set(props.mulligan.kept ?? hand.map((card) => card.instanceId));
  const back = hand.filter((card) => !kept.has(card.instanceId)).length;
  const titleId = useId();
  return (
    <div className="prompt-scrim" data-testid="prompt-scrim">
      <div
        className="prompt prompt-mulligan-waiting"
        data-testid={testid.mulliganWaiting}
        data-returning={back}
        // A labelled region, not a modal dialog: there is nothing in it to answer, and a modal
        // would hold a screen reader inside static text while Concede stays live on the board.
        role="region"
        aria-labelledby={titleId}
      >
        <p className="prompt-title" id={titleId}>
          Waiting for your opponent…
        </p>
        <p className="prompt-sub" role="status">
          {back === 0
            ? "You're ready and keeping your whole hand."
            : `You're ready. ${String(back)} ${back === 1 ? "card goes" : "cards go"} back and ${back === 1 ? "is" : "are"} redrawn once your opponent is ready too.`}
        </p>
        <MatchCardsProvider view={props.view}>
          <div className="prompt-cards">
            {hand.map((card) => (
              <WaitingCard key={card.instanceId} card={card} keep={kept.has(card.instanceId)} />
            ))}
          </div>
        </MatchCardsProvider>
      </div>
    </div>
  );
}

function needKey(need: PlayNeed): string {
  return `${need.kind}:${need.min}:${need.max}`;
}

export default function Prompt(props: PromptProps) {
  const interaction = props.interaction ?? IDLE;
  const boardTestids = [...highlightFor(props.view, props.legal ?? [], interaction).legal].sort();
  const pending = props.view.pending;
  const mulligan = props.view.mulligan;

  // R265: the viewer's own answer is in and the opponent's is not; that is not "a choice is open
  // somewhere", it is the viewer's hand waiting to be dealt.
  if (mulligan?.youReady === true && (pending === null || !pending.forYou)) {
    return <MulliganWaiting view={props.view} mulligan={mulligan} />;
  }
  if (pending !== null && !pending.forYou) return <Waiting pendingFor={pending.pendingFor} />;

  // R243: an option naming a match-made card (a crafted card in a hand pick) reads its definition
  // from the view, and a Heroic Power on the field its power.
  if (pending !== null) {
    return (
      <MatchCardsProvider view={props.view}>
        <PromptModal
          key={pending.choiceId}
          source="engine"
          picker={pickerForPending(pending, props.view, props.legal ?? [])}
          {...(pending.kind === "mulligan" && mulligan !== undefined
            ? { status: <OpponentMulliganStatus mulligan={mulligan} /> }
            : {})}
          {...(props.animating === undefined ? {} : { animating: props.animating })}
          boardTestids={boardTestids}
          onAction={props.onAction}
          {...(props.onInteraction === undefined ? {} : { onInteraction: props.onInteraction })}
          {...(props.onCancel === undefined ? {} : { onCancel: props.onCancel })}
        />
      </MatchCardsProvider>
    );
  }

  // R81: no prompt is open and nothing is paused, but the play in flight still needs a choice.
  const need = outstandingNeed(interaction);
  if (need === null) return null;

  return (
    <MatchCardsProvider view={props.view}>
      <PromptModal
        key={needKey(need)}
        source="play"
        picker={pickerForNeed(need, interaction, props.view)}
        boardTestids={boardTestids}
        onAction={props.onAction}
        {...(props.onInteraction === undefined ? {} : { onInteraction: props.onInteraction })}
        {...(props.onCancel === undefined ? {} : { onCancel: props.onCancel })}
      />
    </MatchCardsProvider>
  );
}
