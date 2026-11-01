// The choice pickers (BUILD M5-T2, SPEC §10.6). One modal, a picker for every prompt kind, no rules.
// Two things open it, and telling them apart is R81: (1) `view.pending`, a `PendingChoice` the engine opened
// during resolution, answered by an `answer` action (or, for the mulligan, §10.2's own `mulligan` action);
// (2) `props.interaction`, a play the player is still building, with NO `view.pending`: nothing is paused and
// it submits a `play`, never an `answer`. Both render the same picker with the same `data-prompt-kind` (§10.6
// keeps `x`, `embiggen`, `zone`, `tribute` and `direction` for later sets). A targeting cost's discards
// (Classic #89) are random at pay time (R682) and need no picker. A card option is drawn as the card in play
// (faces.ts, SPEC §10.10). `data-prompt-source` ("engine" or "play") is for layout only (prompt.css). #1204: a
// new prompt is a new `PromptModal` (its `key`), so a folded picker opens unfolded. #2646: in a choice of
// DISCOVER_OPTION_LIMIT options or fewer, every option naming no card is a choice card (ChoiceCard.tsx), art,
// label and what it does, no cost, stats, id, inspect overlay or sheet: route 2's `mode`, `ability`, `equip`,
// `faceDown` and `plague`, route 1's `mode`, `answer` and `reward`. No rule is applied here: `min`, `max` and the
// options come from the engine, from `PendingView` (route 1) and the `legalActions` candidates (`actions.ts`, route 2).

import {
  Fragment,
  useContext,
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
  CardDef,
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
  isPlaguePlacementPrompt,
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
  type FaceDownChoice,
  type Interaction,
  type PlagueChoice,
  type PlayBuild,
  type PlayNeed,
} from "./actions.ts";
import { CardBack, CardFace, faceModel, useInspectTrigger, type FaceModel } from "../cards/index.ts";
import { OwnCopy } from "../cards/fullArt.ts";
import { chineseName, drawnInChinese } from "../cards/chinese.ts";
import { setMarkOf } from "../cards/setMark.ts";
import { isZhCn, t, useLocale } from "../i18n/i18n.ts";
import {
  CatalogContext,
  MatchCardsProvider,
  useCardInfo,
  useCopiedDef,
  useFieldPower,
  type CardLookup,
} from "./catalog.ts";
import { CraftEditor } from "./craft/CraftEditor.tsx";
import { useSequenceSecondsLeft } from "./sequenceClock.ts";
import MarketPrompt from "./MarketPrompt.tsx";
import { liveFace } from "./faces.ts";
import { DISCOVER_OPTION_LIMIT, X_CARD_LIMIT, sideOf, testid } from "./contract.ts";
import { abilityLabel, usesWords } from "./ActivateControl.tsx";
import { ChoiceCard, choiceSeed } from "./ChoiceCard.tsx";
import { modeText } from "./modeText.ts";
import { useWearerSeat } from "./wearer.ts";
import "./prompt.css";

export type PromptProps = {
  view: PlayerView;
  /** The play or activation in flight, for the inline R81 pickers. */
  interaction?: Interaction;
  legal?: readonly ActionBody[];
  onAction: (body: ActionBody) => void;
  onInteraction?: (next: Interaction) => void;
  onCancel?: () => void;
  /** The event type animating the open modal (`promptAnswered` fades it out), for `data-animating`. */
  animating?: string;
  /** A tutorial lesson's board: the mulligan picker keeps the engine's instruction and its count. */
  lesson?: boolean;
  /** R2211000: a replay's board: the open prompt as offered, folded and inert; nothing calls `onAction`. */
  readOnly?: boolean;
};

/**
 * The seat only watching gets NO `data-prompt-kind`: §10.8 gives that view the fact that a choice is open and
 * nothing else. Its own flag keeps "a prompt is open for me" and "somebody is choosing" apart in the DOM.
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
/**
 * R1184000: a question of a sequence (Meditative+ #20) offers Stop after its answers, a `none` pick
 * under no letter, as the engine keys it (`ANSWER_STOP_KEY`).
 */
const ANSWER_STOP = "none";
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
  defId?: string;
  radiant?: boolean;
  /** R1301: the option says the card is shown in Chinese (`PendingOption.chinese`). */
  chinese?: boolean;
  cost?: number;
  card?: CardView;
  where?: string;
  /** The row a zone option sits in, so the zone grid can group by it without parsing keys. */
  group?: string;
  /** Which arrow a direction option draws. Never read off the key, which is the engine's. */
  arrow?: "left" | "right";
  detail?: string;
  /** R831004: the set an option names (#82 KY's Trial's first Discover), whose mark is its picture. */
  set?: string;
  /** R420: an answer's letter, its place in the order offered. */
  letter?: string;
  /** #492: what picking a card option chooses, under its face ("Embiggened", "X = 2"). */
  caption?: string;
  /** #492: an embiggen option, whose face shows the price that form is played for (`pricedFace`). */
  price?: EmbiggenPrice;
  /**
   * R1183005: the option is a card's code block alone (`PendingOption.code`, Meditative+ #25 KY's
   * Intuition): its label is the code, drawn as code, untranslated, and it names no card.
   */
  code?: boolean;
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
  /** #492: drawn as the card picker (`isCardLayout`) whatever its chrome. The chrome stays the `data-prompt-kind`. */
  cards?: boolean;
  title: string;
  /** The card asking, when the picker knows it: its name heads the title ("Pocket Chaos: choose one"). */
  sourceDefId?: string;
  /** #2646: the card whose options the choice cards are, which seeds their art when no title names it. */
  artSource?: string;
  /**
   * #617: the card being played, as the view lists it, when the options are words ("Choose one"): its name in
   * the title opens its preview and long-press sheet, so the card can be read while its choice is open.
   */
  sourceCard?: CardView;
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
  /** R1184000: a sequence question's right answers so far, as the view carries them. */
  streak?: number;
  /** R1563006: the sequence question's choice id, which its countdown keys on. */
  sequenceChoice?: string;
  /** No "Choose N: M chosen" line under the title: the mulligan outside a lesson. */
  uncounted?: boolean;
};

const CARD_CHROMES: ReadonlySet<PromptKind> = new Set(["discover", "hand", "mulligan", "pick"]);

/**
 * #492: the card picker, front and center (`CardChoice`). #2646: a short choice of options naming no card is
 * one too, its options choice cards. `data-prompt-layout="cards"` keeps it out of the small pickers' dock (prompt.css).
 */
function isCardLayout(picker: Picker): boolean {
  return picker.cards === true || CARD_CHROMES.has(picker.chrome);
}

// Labels. Read out of the view, never computed.

type CardRef = { defId: string; radiant: boolean; cost: number; card: CardView };

function refOf(card: CardView): CardRef {
  return { defId: card.defId, radiant: card.radiant, cost: card.cost, card };
}

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
    // One whole sentence per side, so "Your" and "Enemy" are the reader's words too.
    const yours = side === view.you;
    const unitAt = side.units.findIndex((pile) => pile !== null && pile.instanceId === instanceId);
    if (unitAt >= 0) {
      const lane = String(unitAt + 1);
      return yours ? t("Your unit, lane {lane}", { lane }) : t("Enemy unit, lane {lane}", { lane });
    }
    const backrowAt = side.backrow.findIndex(
      (slot) => slot !== null && slot.faceDown === false && slot.instanceId === instanceId,
    );
    if (backrowAt >= 0) {
      const lane = String(backrowAt + 1);
      return yours ? t("Your backrow, lane {lane}", { lane }) : t("Enemy backrow, lane {lane}", { lane });
    }
    const hand = Array.isArray(side.hand) ? side.hand : [];
    if (hand.some((card) => card.instanceId === instanceId)) return yours ? t("Your hand") : t("Enemy hand");
    if (side.graveyard.some((card) => card.instanceId === instanceId)) return yours ? t("Your graveyard") : t("Enemy graveyard");
    if (side.exile.some((card) => card.instanceId === instanceId)) return yours ? t("Your exile") : t("Enemy exile");
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
  return row === "units" ? t("Unit lane {lane}", { lane }) : t("Backrow lane {lane}", { lane });
}

function rowName(row: string): string {
  return row === "units" ? t("units") : row === "backrow" ? t("backrow") : row;
}

function selectionLabel(view: PlayerView, selection: Selection): string {
  switch (selection.pick) {
    case "instance":
      return cardRefFor(view, selection.instanceId)?.defId ?? selection.instanceId;
    case "hero":
      return sideOf(view, selection.player) === "you" ? t("Your hero") : t("Enemy hero");
    case "zone": {
      const { lane } = selection;
      const yours = sideOf(view, selection.player) === "you";
      if (selection.row === "units") return yours ? t("Your Unit lane {lane}", { lane }) : t("Enemy Unit lane {lane}", { lane });
      return yours ? t("Your Backrow lane {lane}", { lane }) : t("Enemy Backrow lane {lane}", { lane });
    }
    case "mode":
      return t(selection.option);
    case "craft":
      return `${selection.recipe.adjective} ${selection.recipe.noun}`;
    case "none":
      return t("Nothing");
  }
}

// Route 1: an open `PendingChoice`.

function pickerForPending(
  pending: Extract<PendingView, { forYou: true }>,
  view: PlayerView,
  legal: readonly ActionBody[],
  lesson: boolean,
): Picker {
  const items = pending.options.map((option): PickerItem => {
    const base: PickerItem = { key: option.key, label: t(option.label) };
    // R1183005: a code option is its code as the engine wrote it, never a sentence to translate.
    if (option.code === true) {
      base.code = true;
      base.label = option.label;
    }
    if (option.defId !== undefined) base.defId = option.defId;
    // B5 E17, E18: a card the view lists nowhere (the opponent's hand, C #11) is drawn from the
    // option itself, Radiant and at its cost where the option says so.
    if (option.radiant === true) base.radiant = true;
    if (option.chinese === true) base.chinese = true;
    if (option.cost !== undefined) base.cost = option.cost;
    // A face-down card's option names its zone too (R821000), but it is a card here, not a zone to group.
    if (option.row !== undefined && option.instanceId === undefined) base.group = rowName(option.row);
    // R514: a `cell` is a zone of either side, so its list groups by side and row, lane by lane.
    if (pending.kind === "cell" && option.player !== undefined && option.row !== undefined && option.lane !== undefined) {
      const yours = sideOf(view, option.player) === "you";
      base.group =
        option.row === "units"
          ? yours ? t("Your units") : t("Enemy units")
          : yours ? t("Your backrow") : t("Enemy backrow");
      base.label = t("Lane {lane}", { lane: String(option.lane) });
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
    // #2646, R2646000: a card's Choose one reads each option's words off that card (modeText.ts) when
    // the view names the card asking. The label stays the engine's, which can carry a rolled number.
    if (pending.kind === "mode") {
      const selection = selectionForOption(option);
      const detail = selection.pick === "mode" ? modeText(pending.sourceDefId, selection.option, pending.sourceRadiant === true).detail : undefined;
      if (detail !== undefined) base.detail = detail;
      // R831004: an option that names a set (#82 KY's Trial's first Discover) wears the set's mark (R503).
      if (option.defId === undefined && setMarkOf(option.label).kind !== "unknown") base.set = option.label;
    }
    return base;
  });

  const short = pending.options.length <= DISCOVER_OPTION_LIMIT;
  // R1184000: a sequence's Stop is a button under the answers, never one of them.
  const answers = pending.options.filter((option) => option.key !== ANSWER_STOP).length;
  const chrome = pending.kind === "mode" && short ? "discover" : pending.kind;
  // #492: an embiggen price and a short X are cards here too, as on a play (`pickerForNeed`); #2646:
  // so are a few answers (C+ #42) and a quest's rewards (Classic #90), each a choice card.
  const cards =
    pending.kind === "embiggen" ||
    (pending.kind === "x" && pending.options.length <= X_CARD_LIMIT) ||
    (pending.kind === "answer" && answers <= DISCOVER_OPTION_LIMIT) ||
    (pending.kind === "reward" && short);
  // Outside a lesson the mulligan's heading says who goes first, which bears on what to keep, rather
  // than how to answer it. §2.1: Player 1 takes the first turn, and is `active` until then.
  const turnOrder = pending.kind === "mulligan" && !lesson;
  const picker: Picker = {
    chrome,
    ...(cards ? { cards } : {}),
    ...(turnOrder ? { uncounted: true } : {}),
    title: turnOrder
      ? view.active === view.viewer
        ? t("You are going first")
        : t("You are going second")
      : t(pending.kind === "reward" ? pending.prompt.replace(QUEST_COMPLETE_PREFIX, "") : pending.prompt),
    // #2646: a reward's choice card is named for its place, as an answer is, and says the reward.
    items: pending.kind === "reward" && cards ? items.map((item, at) => ({ ...item, label: t("Reward {letter}", { letter: ANSWER_LETTERS[at] ?? String(at + 1) }), detail: item.label })) : items,
    min: pending.min,
    max: pending.max,
    // The mulligan is per-card toggles plus a confirm, whatever its max.
    immediate: pending.kind !== "mulligan" && pending.max <= 1,
    // R9's answer names the cards KEPT, and Hearthstone keeps the whole hand until a card is
    // marked to go back, so a mulligan opens with every card kept.
    ...(pending.kind === "mulligan" ? { initial: items.slice(0, pending.max).map((item) => item.key) } : {}),
    // The picks in the order offered, so the same picks always build the same answer.
    submit: (keys) => ({
      action: answerAction(pending, items.map((item) => item.key).filter((key) => keys.includes(key)), view, legal),
    }),
  };
  // R2646000: the card asking, when the view names it, seeds its options' art, and a Choose one's title
  // names it as a play's does. A reward's title stays the quest (R404).
  if (pending.sourceDefId !== undefined) {
    picker.artSource = pending.sourceDefId;
    if (pending.kind === "mode") picker.sourceDefId = pending.sourceDefId;
  }
  if (pending.kind === "answer" && pending.streak !== undefined) {
    picker.streak = pending.streak;
    picker.sequenceChoice = pending.choiceId;
  }
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

// Route 2: an R81 choice of a play or an activation. Submits a `play` or an `activate`, never an `answer`.

function isDirection(options: readonly string[]): boolean {
  return options.length > 0 && options.every((option) => DIRECTIONS.some((d) => d === option));
}

/**
 * R81 puts a declared `hand` pick (#26 Glowy Jelly Bean) in the play action's `targets`, so it arrives as a
 * `target` need, but §10.6 names `hand` as its own prompt kind. Presentation only: the action is the same `play`.
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

function plagueLabel(view: PlayerView, option: PlagueChoice, nameSource: boolean): string {
  if (option === "none") return t("Pay in mana only");
  const n = String(option.tokens);
  const where = nameSource ? whereOf(view, option.from) : null;
  if (where === null) return option.tokens === 1 ? t("Spend {n} Plague Counter", { n }) : t("Spend {n} Plague Counters", { n });
  return option.tokens === 1
    ? t("Spend {n} Plague Counter ({where})", { n, where })
    : t("Spend {n} Plague Counters ({where})", { n, where });
}

function playedRef(interaction: Interaction, view: PlayerView): CardRef | null {
  return isBuilding(interaction) ? cardRefFor(view, interaction.instanceId) : null;
}

function playedFace(played: CardRef): Pick<PickerItem, "defId" | "radiant" | "cost" | "card"> {
  return { defId: played.defId, radiant: played.radiant, cost: played.cost, card: played.card };
}

function pickerForNeed(need: PlayNeed, interaction: Interaction, view: PlayerView, lookup: CardLookup | null): Picker {
  const play = (patch: Partial<PlayBuild>): Submitted => {
    const result = pickInPlay(interaction, patch);
    return result.action === undefined
      ? { interaction: result.interaction }
      : { interaction: result.interaction, action: result.action };
  };
  const common = { min: need.min, max: need.max, immediate: need.max <= 1 };

  switch (need.kind) {
    case "ability": {
      // R1751000: picking an ability when a card has several.
      const played = isBuilding(interaction) ? cardRefFor(view, interaction.instanceId) : null;
      const source = played?.defId ?? undefined;
      const info = played && lookup ? lookup(played.defId, played.radiant) : undefined;
      const activations = played?.card.activations ?? [];
      const actMap = new Map(activations.map((a) => [a.ability, a]));
      return {
        ...common,
        // #2646: a few abilities are choice cards; more keep the plain list.
        chrome: need.abilities.length <= DISCOVER_OPTION_LIMIT ? "discover" : "mode",
        title: t("Choose an ability"),
        ...(source !== undefined ? { sourceDefId: source } : {}),
        ...(played !== null ? { sourceCard: played.card } : {}),
        items: need.abilities.map((abilityId) => {
          const act = actMap.get(abilityId);
          const rawLabel = act?.label ?? abilityId;
          const label = info && played?.card ? abilityLabel(info, played.card, rawLabel) : rawLabel;
          // A choice card says what it does (#2646): how often the ability may still be used.
          const detail = act === undefined ? t("Use this ability.") : usesWords(act.usesLeft);
          return { key: abilityId, label: t(label), detail };
        }),
        submit: (keys) => (keys[0] === undefined ? {} : play({ ability: keys[0] })),
      };
    }
    case "zone":
      return {
        ...common,
        chrome: "zone",
        title: t("Choose a zone"),
        items: zonesInBoardOrder(need.zones).map((zone) => ({
          key: zoneKey(zone),
          label: zoneLabel(zone.row, zone.lane),
          group: rowName(zone.row),
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
        title: t("Choose X"),
        items: need.values.map((value) => {
          const item: PickerItem = { key: String(value), label: String(value) };
          return played === null ? item : { ...item, ...playedFace(played), caption: `X = ${String(value)}` };
        }),
        submit: (keys) => (keys[0] === undefined ? {} : play({ x: Number(keys[0]) })),
      };
    }
    case "embiggen": {
      // #492: the two prices are two cards. R1563004: a card of one price has a dearer price only
      // under Mountaintop Garden, which plays it Radiant, so its two cards are the card as it is and
      // the card Radiant.
      const played = playedRef(interaction, view);
      const printed = played === null ? undefined : lookup?.(played.defId, false)?.def?.cost;
      const radiantOnly = typeof printed === "number" || printed === "X";
      return {
        ...common,
        chrome: "embiggen",
        cards: true,
        ...(played === null ? {} : { sourceDefId: played.defId }),
        title: radiantOnly ? t("Play it as it is, or Radiant?") : t("Play the normal or the Embiggened version?"),
        items: need.values.map((value) => {
          const label = value ? (radiantOnly ? t("Radiant") : t("Embiggened")) : t("Normal");
          const item: PickerItem = { key: String(value), label };
          const price: EmbiggenPrice = value ? "embiggen" : "normal";
          if (played === null) return item;
          const face = playedFace(played);
          return { ...item, ...face, ...(radiantOnly && value ? { radiant: true } : {}), caption: label, price };
        }),
        submit: (keys) => (keys[0] === undefined ? {} : play({ embiggen: keys[0] === "true" })),
      };
    }
    case "tribute":
      return {
        ...common,
        chrome: "tribute",
        title: t("Choose Tributes"),
        items: need.instanceIds.map((id) => itemForInstance(view, id, id)),
        submit: (keys) => play({ tributes: [...keys] }),
      };
    case "equip": {
      // R1086: one zone with both a plain and an Equip play asks which: two choice cards (#2646), each
      // saying what it does in its keyword's words (glossary.ts).
      const cards = need.values.length <= DISCOVER_OPTION_LIMIT;
      const played = playedRef(interaction, view);
      return {
        ...common,
        chrome: "mode",
        ...(cards ? { cards } : {}),
        ...(played === null ? {} : { artSource: played.defId }),
        title: t("Equip or Stack?"),
        items: need.values.map((value) => ({
          key: String(value),
          label: value ? t("Equip") : t("Stack"),
          ...(cards ? { detail: value ? t("Fuse it into the unit in this zone.") : t("Play it on top of the unit in this zone.") } : {}),
        })),
        submit: (keys) => (keys[0] === undefined ? {} : play({ equip: keys[0] === "true" })),
      };
    }
    case "target": {
      const byKey = new Map(need.selections.map((selection) => [selectionKey(selection), selection]));
      // R81: a declared `target` pick whose selections are all hand cards is rendered with the hand
      // chrome, so the player sees the pick where the cards are; e2e spec 02 asserts it.
      const inHand = isHandPick(need, view);
      return {
        ...common,
        chrome: inHand ? "hand" : "target",
        title: inHand ? t("Choose a card in your hand") : t("Choose a target"),
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
      // #2646: a few ways to pay are choice cards; more keep the row of chips.
      const cards = need.options.length <= DISCOVER_OPTION_LIMIT;
      const played = playedRef(interaction, view);
      return {
        ...common,
        chrome: "number",
        ...(cards ? { cards } : { variant: "plague" as const }),
        ...(played === null ? {} : { artSource: played.defId }),
        title: t("Pay with Plague Counters?"),
        items: need.options.map((option) => ({
          key: plagueKey(option),
          label: plagueLabel(view, option, sources.size > 1),
          ...(cards
            ? { detail: option === "none" ? t("The whole price is paid in mana.") : t("The counters pay that much of the price; mana pays the rest.") }
            : {}),
        })),
        submit: (keys) => {
          const choice: PlagueChoice | undefined = keys[0] === undefined ? undefined : byKey.get(keys[0]);
          return choice === undefined ? {} : play({ plague: choice });
        },
      };
    }
    case "faceDown": {
      // ME-ALTPLAY (R1040, R1044): how the card is played — face-up, or set as a Trap. R783004: a
      // Cloaked set has no timing; it reveals when its controller uses its Activate: Reveal.
      const labels: Record<FaceDownChoice, string> = {
        up: t("Play it face-up"),
        endOfThisTurn: t("Set face-down: reveal at the end of this turn"),
        startOfNextTurn: t("Set face-down: reveal at the start of your next turn"),
        endOfNextTurn: t("Set face-down: reveal at the end of your next turn"),
        cloaked: t("Set face-down: reveal it with its Activate"),
      };
      // #2646: a few ways are choice cards, each named for how the card lies and saying when it
      // reveals; more keep the plain list of sentences.
      const names: Record<FaceDownChoice, string> = {
        up: t("Face-up"),
        endOfThisTurn: t("Face-down"),
        startOfNextTurn: t("Face-down"),
        endOfNextTurn: t("Face-down"),
        cloaked: t("Cloaked"),
      };
      const cards = need.options.length <= DISCOVER_OPTION_LIMIT;
      const played = playedRef(interaction, view);
      return {
        ...common,
        chrome: "mode",
        ...(cards ? { cards } : {}),
        ...(played === null ? {} : { artSource: played.defId }),
        title: t("Play it how?"),
        items: need.options.map((option) => (cards ? { key: option, label: names[option], detail: labels[option] } : { key: option, label: labels[option] })),
        submit: (keys) => {
          const choice: FaceDownChoice | undefined =
            keys[0] === undefined ? undefined : (keys[0] as FaceDownChoice);
          return choice === undefined || !Object.hasOwn(labels, choice) ? {} : play({ faceDown: choice });
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
      const discover = !numbers && !isDirection(need.options) && need.options.length <= DISCOVER_OPTION_LIMIT;
      const picker: Picker = {
        ...common,
        chrome: numbers ? "number" : isDirection(need.options) ? "direction" : discover ? "discover" : "mode",
        title: numbers ? t("Choose a number") : isDirection(need.options) ? t("Choose a direction") : t("Choose one"),
        items: need.options.map((option): PickerItem => {
          const arrow = DIRECTIONS.find((d) => d === option);
          if (arrow !== undefined) return { key: option, label: t(option), arrow };
          const text = modeText(source, option, radiant);
          return text.detail === undefined
            ? { key: option, label: text.label }
            : { key: option, label: text.label, detail: text.detail };
        }),
        submit: (keys) => play({ modes: [...keys] }),
      };
      if (source !== undefined) picker.sourceDefId = source;
      if (played !== null) picker.sourceCard = played.card;
      return picker;
    }
  }
}

// The modal.

/** R247: a Discover option that is a number rather than a card (#82 KY's Trial). */
const NUMBER_OPTION = /^\d+$/;

/** #492: the gem of an embiggened option whose price nothing vouches for (`pricedFace`). */
const UNKNOWN_PRICE = "?";

/**
 * #492: the embiggened form is at the view's `embiggenCost`, which the engine reads with the function it
 * charges that play by. Without it (a card not in its owner's hand) fall back to the price beside the face's
 * gem (`FaceCost.alt`, cards/model.ts), else "?": the client never works out an embiggen price (CLAUDE.md rule 7).
 */
function pricedFace(face: FaceModel, price: EmbiggenPrice, card: CardView | undefined, def: CardDef | undefined): FaceModel {
  if (price === "normal") return { ...face, cost: { ...face.cost, alt: null } };
  const live = card?.embiggenCost;
  if (live === undefined) {
    const text = face.cost.alt ?? UNKNOWN_PRICE;
    return { ...face, cost: { text, value: text, tone: "base", alt: null } };
  }
  const printed = typeof def?.cost === "object" ? def.cost.embiggen : live;
  const tone = live < printed ? "down" : live > printed ? "up" : "base";
  return { ...face, cost: { text: String(live), value: String(live), tone, alt: null } };
}

/** What an embiggen option pays, in R432's words: "Pay (4)", or "Pay the Embiggened cost" when nothing vouches for it (R1175000). */
function priceLine(face: FaceModel): string {
  return face.cost.text === UNKNOWN_PRICE ? t("Pay the Embiggened cost") : t("Pay ({cost})", { cost: face.cost.text });
}

/** R1563006: the seconds a question sequence has left, ticking; at 0 its host answers Stop. */
function SequenceCountdown(props: { choiceId: string; streak: number }) {
  useLocale();
  const left = useSequenceSecondsLeft(props.choiceId, props.streak);
  if (left === null) return null;
  return (
    <p className="prompt-sequence-clock" data-testid="prompt-sequence-clock" data-seconds={left} aria-live="off">
      {t("Time left: {seconds} s", { seconds: left })}
    </p>
  );
}

function isFlagged(item: PickerItem): boolean {
  return item.chinese === true || item.card?.chinese === true;
}

/** A card option's name: the catalog's, or the Chinese table's for a Chinese card (R1301). */
function nameOf(defId: string, info: { name: string; def?: CardDef }, chinese: boolean): string {
  return chinese ? chineseName(defId, info.name, info.def) : info.name;
}

// A number option is a card back (R247), a code option its code (R1183005), and any other option that
// names no card a `ChoiceOption` (#2646).
function CardOption(props: {
  item: PickerItem;
  pressed: boolean;
  /** A mulligan's options say what Confirm will do to each card. */
  verdicts?: boolean;
  /** R1655000: the option is the viewer's own card (a mulligan's, on a board that wears cosmetics): it wears its picks. */
  own?: boolean;
  /** R515: the card would take a pick past its budget, so it is greyed and cannot be added. */
  over?: boolean;
  /** R2211000: a replay's option: greyed, and a click picks nothing. */
  readOnly?: boolean;
  onPick: () => void;
}) {
  useLocale();
  const info = useCardInfo(props.item.defId ?? "", props.item.radiant === true);
  const fieldPower = useFieldPower(props.item.card?.instanceId);
  const copied = useCopiedDef(props.item.card);
  // faceModel reads the reader's language itself (R711000), so the face gets the raw flag.
  const flagged = isFlagged(props.item);
  const name = props.item.defId === undefined ? props.item.label : nameOf(props.item.defId, info, drawnInChinese(flagged));
  const radiant = props.item.radiant === true;
  const cost = props.item.cost;
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
            chinese: flagged,
            ...(cost === undefined ? {} : { liveCost: cost }),
            inPlay: {},
          });
  const face =
    drawn === null || props.item.price === undefined ? drawn : pricedFace(drawn, props.item.price, props.item.card, info.def);
  const caption = face === null ? undefined : props.item.caption;
  const pays = face === null || props.item.price === undefined ? undefined : priceLine(face);
  // R247: a number names no card here, so it is drawn on a card back and nothing opens it.
  const number = face === null && NUMBER_OPTION.test(props.item.label) ? props.item.label : null;
  const code = face === null && props.item.code === true ? props.item.label : null;
  const testId = `prompt-option-${props.item.key}`;
  // Lines of code is a hidden stat in matches.
  const inspect = useInspectTrigger(face === null ? null : { key: testId, face }, { prefer: "above", showLoc: false });
  const verdict = props.verdicts === true ? (props.pressed ? "keep" : "redraw") : undefined;
  // The name, then the cost the gem shows, so a screen reader hears what a sighted player reads, in
  // R432's words ("costs (3)").
  const faceLabel = t("{name}, costs ({cost})", { name, cost: face?.cost.text ?? "" });
  const named =
    face === null ? (number === null ? undefined : t("Number {n}", { n: number })) : caption === undefined ? faceLabel : `${caption}: ${faceLabel}`;
  // R515: OVER_BUDGET's words in lower case after the name.
  const label = props.over === true && named !== undefined ? t("{name}, over the budget", { name: named }) : named;
  return (
    <>
      <button
        type="button"
        className={number !== null ? "prompt-card prompt-card--number" : code !== null ? "prompt-card prompt-card--code" : "prompt-card"}
        data-testid={testId}
        data-verdict={verdict}
        data-number={number ?? undefined}
        data-code={code === null ? undefined : "true"}
        data-price={face === null ? undefined : props.item.price}
        data-over-budget={props.over === true ? "true" : undefined}
        aria-pressed={props.pressed}
        aria-disabled={props.over === true || props.readOnly === true ? "true" : undefined}
        aria-label={label}
        title={props.over === true ? t(OVER_BUDGET) : undefined}
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
        ) : code !== null ? (
          <pre className="prompt-code">
            <code>{code}</code>
          </pre>
        ) : face === null ? null : (
          <span className="cf-option">
            <CardFace face={face} layout="full" worn={props.own === true} />
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
            {verdict === "keep" ? t("Keep") : t("Redraw")}
          </span>
        )}
      </button>
      <OwnCopy own={props.own === true}>{inspect.overlay}</OwnCopy>
    </>
  );
}

/**
 * #2646: an option that names no card, in a short choice: a choice card (ChoiceCard.tsx), which is not
 * a card, so it opens no inspect overlay or long-press sheet. Read out as its label and what it does.
 */
function ChoiceOption(props: { item: PickerItem; seed: string; pressed: boolean; readOnly?: boolean; onPick: () => void }) {
  const { item } = props;
  const detail = item.detail === undefined || item.detail === "" ? undefined : item.detail;
  return (
    <button
      type="button"
      className="prompt-card prompt-card--choice"
      data-testid={`prompt-option-${item.key}`}
      data-letter={item.letter}
      data-set={item.set}
      aria-pressed={props.pressed}
      aria-disabled={props.readOnly === true ? "true" : undefined}
      aria-label={detail === undefined ? item.label : `${item.label}: ${detail}`}
      onClick={props.onPick}
    >
      <ChoiceCard
        seed={props.seed}
        label={item.label}
        {...(detail === undefined ? {} : { detail })}
        {...(item.set === undefined ? {} : { set: item.set })}
      />
    </button>
  );
}

function ListOption(props: { item: PickerItem; pressed: boolean; readOnly?: boolean; onPick: () => void }) {
  const info = useCardInfo(props.item.defId ?? "", props.item.radiant === true);
  const name =
    props.item.defId === undefined
      ? props.item.label
      : `${nameOf(props.item.defId, info, drawnInChinese(isFlagged(props.item)))} — ${props.item.where ?? props.item.label}`;
  return (
    <li>
      <button
        type="button"
        data-testid={`prompt-option-${props.item.key}`}
        aria-pressed={props.pressed}
        aria-disabled={props.readOnly === true ? "true" : undefined}
        onClick={props.onPick}
      >
        {name}
      </button>
    </li>
  );
}

function PlainOption(props: { item: PickerItem; pressed: boolean; readOnly?: boolean; onPick: () => void }) {
  return (
    <button
      type="button"
      className={props.item.detail === undefined ? undefined : "prompt-mode"}
      data-testid={`prompt-option-${props.item.key}`}
      aria-pressed={props.pressed}
      aria-disabled={props.readOnly === true ? "true" : undefined}
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

/**
 * Arrow keys walk a grid of option buttons `columns` wide, the options within the element that
 * listens; Enter or Space presses the focused one. A key pressed off the options is left alone.
 */
function arrowFocus(columns: number) {
  const steps: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -columns, ArrowDown: columns };
  return (event: ReactKeyboardEvent<HTMLElement>): void => {
    const step = steps[event.key];
    if (step === undefined) return;
    const buttons = [...event.currentTarget.querySelectorAll<HTMLElement>('button[data-testid^="prompt-option-"]')];
    const at = event.target instanceof HTMLElement ? buttons.indexOf(event.target) : -1;
    if (at < 0) return;
    event.preventDefault();
    buttons[at + step]?.focus();
  };
}

function PickerTitle(props: { title: string; sourceDefId: string | undefined; sourceCard: CardView | undefined }) {
  const info = useCardInfo(props.sourceDefId ?? "", props.sourceCard?.radiant === true);
  const face = props.sourceCard === undefined ? null : liveFace(info, props.sourceCard);
  const inspect = useInspectTrigger(face === null ? null : { key: `prompt-source:${face.defId}`, face }, { prefer: "above", showLoc: false });
  if (props.sourceDefId === undefined) return <p className="prompt-title">{props.title}</p>;
  // R711000: the asking card's name as a reader of Chinese reads it.
  const name = nameOf(props.sourceDefId, info, isZhCn());
  // R2646000: an engine prompt that names its card already ("Pickle: choose one") says it once.
  const ask = props.title.startsWith(`${name}: `) ? props.title.slice(name.length + 2) : props.title;
  return (
    <p className="prompt-title">
      {face === null ? (
        <span className="prompt-title-source">{name}</span>
      ) : (
        <span className="prompt-title-source prompt-title-source--inspect" data-testid="prompt-title-source" {...inspect.handlers}>
          {name}
        </span>
      )}
      <span className="prompt-title-ask">{ask}</span>
      {inspect.overlay}
    </p>
  );
}

/**
 * A board pick's hint. On an upright phone with drag to play on, a play's zone, target and Tribute picks fold
 * to a bar with no options (prompt.css): the answers glow on the board and are tapped there, and this says so.
 */
const BOARD_HINTS: Partial<Record<PromptKind, string>> = {
  zone: "Tap a highlighted zone on the board.",
  target: "Tap a highlighted target on the board.",
  tribute: "Tap a highlighted unit on the board.",
};

function BoardHint(props: { chrome: PromptKind }) {
  const hint = BOARD_HINTS[props.chrome];
  return hint === undefined ? null : <p className="prompt-board-hint">{t(hint)}</p>;
}

/**
 * R689, R821000: "Place N Plague Counters" names one of every permanent on the field; a list of them covered
 * the board. The pick is a click on the highlighted card or zone (`answerPlaguePlacement`, Game.tsx), so the
 * panel is only the question: no Confirm (a click answers), no Cancel (an engine prompt must be answered).
 */
function BoardOnlyPrompt(props: {
  pending: Extract<PendingView, { forYou: true }>;
  boardTestids: readonly string[];
  animating: string | undefined;
}) {
  const title = t(props.pending.prompt);
  return (
    <div className="prompt-scrim" data-testid="prompt-scrim">
      <div
        className="prompt prompt-target"
        data-testid="prompt-modal"
        data-prompt-kind="target"
        data-prompt-source="engine"
        data-board-only="true"
        data-animating={props.animating}
        data-board-testids={props.boardTestids.join(" ")}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <p className="prompt-title">{title}</p>
        <p className="prompt-board-note">{t("Tap a highlighted target on the board.")}</p>
      </div>
    </div>
  );
}

function CardChoice(props: {
  picker: Picker;
  /** The row's name, when it is a group of its own (the answers). */
  label?: string;
  spent: number;
  pressed: (key: string) => boolean;
  fits: (key: string) => boolean;
  pick: (key: string) => void;
  readOnly: boolean;
}) {
  const { picker } = props;
  const budget = picker.budget;
  // R1655000: a mulligan's options are the viewer's own hand.
  const wearer = useWearerSeat();
  const own = picker.chrome === "mulligan" && wearer !== null;
  // R247: a Discover of numbers offers no card, and each is drawn on a card back. A Choose one whose
  // options are numbers with words of their own (#51's cost brackets) is choice cards instead.
  const numbers = picker.items.every((item) => item.defId === undefined && item.detail === undefined && NUMBER_OPTION.test(item.label));
  const artSource = picker.artSource ?? picker.sourceDefId;
  return (
    <>
      {budget === undefined ? null : (
        <p className="prompt-budget" data-testid="prompt-budget" role="status">
          {t("({spent}) of ({limit}) spent", { spent: props.spent, limit: budget.limit })}
        </p>
      )}
      <div
        className="prompt-cards"
        data-testid="prompt-cards"
        data-choice={picker.chrome}
        {...(props.label === undefined ? {} : { role: "group", "aria-label": props.label })}
      >
        {picker.items.map((item) =>
          item.defId !== undefined || item.code === true || numbers ? (
            <CardOption
              key={item.key}
              item={item}
              pressed={props.pressed(item.key)}
              verdicts={picker.chrome === "mulligan"}
              own={own}
              over={!props.pressed(item.key) && !props.fits(item.key)}
              readOnly={props.readOnly}
              onPick={() => props.pick(item.key)}
            />
          ) : (
            <ChoiceOption
              key={item.key}
              item={item}
              seed={choiceSeed(artSource, picker.chrome, item.key)}
              pressed={props.pressed(item.key)}
              readOnly={props.readOnly}
              onPick={() => props.pick(item.key)}
            />
          ),
        )}
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
  /** R2211000: a replay's picker: folded as it opens, its options greyed, and nothing it draws sends. */
  readOnly?: boolean;
}) {
  useLocale();
  const { picker } = props;
  const readOnly = props.readOnly === true;
  const [selected, setSelected] = useState<readonly string[]>(() => picker.initial ?? []);
  /**
   * What is typed in the X field, apart from what is chosen: a cleared field, or a number the engine did not
   * offer, stages nothing. `null`: nothing typed since the last stepper press, so the field follows the stepper.
   */
  const [typedX, setTypedX] = useState<string | null>(null);
  // R515: what the picks cost together, and whether they fit the budget (a hint; the engine rules).
  const spent = (keys: readonly string[]): number => keys.reduce((sum, key) => sum + (picker.budget?.costs.get(key) ?? 0), 0);
  const fits = (keys: readonly string[]): boolean => picker.budget === undefined || spent(keys) <= picker.budget.limit;
  const inRange = selected.length >= picker.min && selected.length <= picker.max && fits(selected);
  const panel = useRef<HTMLDivElement>(null);
  /**
   * #1204: the card picker folded to a bar so the board shows. Never sent; `selected` outlasts it.
   * R2211000: a read-only one opens folded.
   */
  const [folded, setFolded] = useState(readOnly);

  function send(keys: readonly string[]): void {
    if (readOnly) return;
    const out = picker.submit(keys);
    if (out.interaction !== undefined) props.onInteraction?.(out.interaction);
    if (out.action !== undefined) props.onAction(out.action);
  }

  /** Stages a value without sending it: the X stepper's − and +. */
  function stage(key: string): void {
    if (readOnly) return;
    setSelected([key]);
  }

  function pick(key: string): void {
    if (readOnly) return;
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
    if (readOnly || !FOCUS_ON_OPEN.has(picker.chrome)) return;
    const first = panel.current?.querySelector<HTMLElement>('[data-testid^="prompt-option-"]:not([aria-disabled="true"])');
    first?.focus({ preventScroll: true });
  }, [picker.chrome, readOnly]);

  // R514: a click on a zone of the board a `cell` prompt offers, or Enter or Space on it, picks that
  // cell. The board's own handler builds nothing from a zone while a prompt is open, so this is the
  // one answer. maxi-tail: document listeners for the one board-answered prompt kind; move them into
  // `onClickTarget` (actions.ts) if target prompts are ever answered on the board too.
  const live = useRef({ pick, keys: picker.boardKeys });
  live.current = { pick, keys: picker.boardKeys };
  const answersOnBoard = picker.boardKeys !== undefined && !readOnly;
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
        <div className="prompt-chips prompt-plague" role="group" aria-label={t("Plague Counters")}>
          {items.map((item) => (
            <PlainOption key={item.key} item={item} pressed={pressed(item.key)} readOnly={readOnly} onPick={() => pick(item.key)} />
          ))}
        </div>
      );
    }

    if (picker.chrome === "answer") {
      // R1184000: a question of a sequence says how many it has right so far and offers Stop, under
      // no letter, after the lettered answers; a lone problem (C+ #42) has neither.
      const answers = items.filter((item) => item.key !== ANSWER_STOP);
      const stop = items.find((item) => item.key === ANSWER_STOP);
      const stopButton =
        stop === undefined ? null : (
          <button
            key={stop.key}
            type="button"
            className="prompt-answer-stop"
            data-testid={`prompt-option-${stop.key}`}
            aria-pressed={pressed(stop.key)}
            aria-disabled={readOnly ? "true" : undefined}
            onClick={() => pick(stop.key)}
          >
            {stop.label}
          </button>
        );
      return (
        <>
          {picker.streak === undefined ? null : (
            <p className="prompt-streak" data-testid="prompt-streak" data-streak={picker.streak}>
              {t("Right answers in a row: {count}", { count: picker.streak })}
            </p>
          )}
          {readOnly || picker.streak === undefined || picker.sequenceChoice === undefined ? null : (
            <SequenceCountdown choiceId={picker.sequenceChoice} streak={picker.streak} />
          )}
          {isCardLayout(picker) ? (
            // #2646: a few answers are choice cards, each named by its letter (R420) and saying its
            // answer; Stop stays a button under them.
            <>
              <CardChoice
                picker={{
                  ...picker,
                  items: answers.map((item, at) => {
                    const letter = ANSWER_LETTERS[at] ?? String(at + 1);
                    return { ...item, label: letter, detail: item.label, letter };
                  }),
                }}
                label={t("Answers")}
                spent={spent(selected)}
                pressed={pressed}
                fits={(key) => fits([...selected, key])}
                pick={pick}
                readOnly={readOnly}
              />
              {stopButton}
            </>
          ) : (
            <div className="prompt-answers" role="group" aria-label={t("Answers")}>
              {answers.map((item, at) => {
                const letter = ANSWER_LETTERS[at] ?? String(at + 1);
                return (
                  <button
                    key={item.key}
                    type="button"
                    data-testid={`prompt-option-${item.key}`}
                    data-letter={letter}
                    aria-pressed={pressed(item.key)}
                    aria-label={`${letter}: ${item.label}`}
                    aria-disabled={readOnly ? "true" : undefined}
                    onClick={() => pick(item.key)}
                  >
                    <span className="prompt-answer-letter" aria-hidden="true">
                      {letter}
                    </span>
                    <span>{item.label}</span>
                  </button>
                );
              })}
              {stopButton}
            </div>
          )}
        </>
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
          readOnly={readOnly}
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
              aria-disabled={readOnly ? "true" : undefined}
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
        if (readOnly) return;
        setTypedX(raw);
        const match = items.find((item) => item.key === raw);
        setSelected(match === undefined ? [] : [match.key]);
      };
      return (
        <>
          <div className="prompt-stepper" data-testid="x-stepper" role="group" aria-label={t("Choose X")}>
            <button type="button" data-testid="x-minus" aria-label={t("Lower X")} aria-disabled={readOnly ? "true" : undefined} onClick={() => step(-1)}>
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
              readOnly={readOnly}
              onChange={(event) => typeX(event.target.value)}
            />
            <span className="prompt-stepper-value" data-testid="x-value">
              {items[at]?.label ?? ""}
            </span>
            <button type="button" data-testid="x-plus" aria-label={t("Raise X")} aria-disabled={readOnly ? "true" : undefined} onClick={() => step(1)}>
              +
            </button>
          </div>
          <div className="prompt-chips">
            {items.map((item) => (
              <PlainOption key={item.key} item={item} pressed={pressed(item.key)} readOnly={readOnly} onPick={() => pick(item.key)} />
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
                  <PlainOption key={item.key} item={item} pressed={pressed(item.key)} readOnly={readOnly} onPick={() => pick(item.key)} />
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
          aria-label={t("Numbers")}
          style={{ "--pad-columns": String(NUMBER_PAD_COLUMNS) } as CSSProperties}
          onKeyDown={arrowFocus(NUMBER_PAD_COLUMNS)}
        >
          {items.map((item) => (
            <PlainOption key={item.key} item={item} pressed={pressed(item.key)} readOnly={readOnly} onPick={() => pick(item.key)} />
          ))}
        </div>
      );
    }

    if (picker.chrome === "reward") {
      return (
        <div className="prompt-rewards" role="group" aria-label={t("Rewards")}>
          {items.map((item) => (
            <PlainOption key={item.key} item={item} pressed={pressed(item.key)} readOnly={readOnly} onPick={() => pick(item.key)} />
          ))}
        </div>
      );
    }

    if (picker.chrome === "target" || picker.chrome === "tribute") {
      return (
        <ul className="prompt-list">
          {items.map((item) => (
            <ListOption key={item.key} item={item} pressed={pressed(item.key)} readOnly={readOnly} onPick={() => pick(item.key)} />
          ))}
        </ul>
      );
    }

    return (
      <div className="prompt-modes">
        {items.map((item) => (
          <PlainOption key={item.key} item={item} pressed={pressed(item.key)} readOnly={readOnly} onPick={() => pick(item.key)} />
        ))}
      </div>
    );
  }

  const range = picker.min === picker.max ? `${picker.min}` : `${picker.min}–${picker.max}`;

  return (
    <div
      className="prompt-scrim"
      data-testid="prompt-scrim"
      data-collapsed={folded ? "true" : undefined}
      data-read-only={readOnly ? "true" : undefined}
    >
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
        data-collapsed={folded ? "true" : undefined}
        data-read-only={readOnly ? "true" : undefined}
        role="dialog"
        aria-modal={!folded && !readOnly}
        aria-label={picker.title}
        // The answers, as cards or as a list, and Stop after them, walked by the arrows (R420).
        onKeyDown={picker.chrome === "answer" ? arrowFocus(ANSWER_COLUMNS) : undefined}
      >
        {picker.chrome === "reward" ? (
          <p className="prompt-quest-banner" data-testid="prompt-quest-complete">
            {t(QUEST_COMPLETE)}
          </p>
        ) : null}
        <PickerTitle title={picker.title} sourceDefId={picker.sourceDefId} sourceCard={picker.sourceCard} />
        {folded ? null : (
          <>
            {picker.uncounted === true ? null : (
              <p className="prompt-count">
                {t("Choose {range}: {count} chosen", { range, count: selected.length })}
              </p>
            )}
            {props.status}
            {body()}
            {(picker.chrome === "target" || picker.chrome === "cell") && props.boardTestids.length > 0 ? (
              <p className="prompt-board-note">{t("Highlighted on the board as well.")}</p>
            ) : null}
            {/* Shown only where the picker folds to a bar with no options in it (prompt.css). */}
            {props.source === "play" ? <BoardHint chrome={picker.chrome} /> : null}
          </>
        )}
        <div className="prompt-actions">
          {folded || readOnly ? null : (
            <button
              type="button"
              data-testid="prompt-submit"
              aria-disabled={!inRange}
              onClick={() => {
                if (inRange) send(selected);
              }}
            >
              {/* R265: the mulligan's answer is sealed until both seats have given theirs. */}
              {picker.chrome === "mulligan" ? t("Ready") : t("Confirm")}
            </button>
          )}
          {folded || readOnly || props.onCancel === undefined ? null : (
            <button type="button" data-testid="prompt-cancel" onClick={props.onCancel}>
              {t("Cancel")}
            </button>
          )}
          {isCardLayout(picker) || readOnly ? (
            <button type="button" data-testid="prompt-peek" aria-pressed={folded} onClick={() => setFolded((was) => !was)}>
              {folded ? t("Back to the choice") : t("View the board")}
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

/**
 * §10.6, §10.8: the other seat learns that a choice is open and nothing else. The chooser is always the
 * viewer's opponent (a prompt for the viewer is `forYou`), so the line names the opponent. R2211000: on a
 * replay's board, the viewer's own craft, market or board-only placement is this panel, under the prompt's words.
 */
function Waiting(props: { pendingFor: PlayerId; prompt?: string }) {
  useLocale();
  const title = props.prompt ?? t("Waiting for choice");
  return (
    <div className="prompt-scrim" data-testid="prompt-scrim">
      <div
        className="prompt prompt-waiting"
        data-testid="prompt-modal"
        data-prompt-waiting={WAITING_FLAG}
        data-pending-for={props.pendingFor}
        data-read-only={props.prompt === undefined ? undefined : "true"}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <p className="prompt-title">{title}</p>
        <p className="prompt-sub" role="status">
          {props.prompt === undefined ? t("Your opponent is choosing…") : t("A replay doesn't show this choice.")}
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
  useLocale();
  const ready = props.mulligan.opponentReady;
  return (
    <p
      className="prompt-sub mulligan-status"
      data-testid={testid.mulliganOpponentStatus}
      data-ready={ready ? "true" : "false"}
      role="status"
    >
      {ready ? <span data-testid={testid.mulliganOpponentReady}>{t("Opponent is ready")}</span> : t("Opponent is choosing…")}
    </p>
  );
}

function WaitingCard(props: { card: CardView; keep: boolean }) {
  useLocale();
  const info = useCardInfo(props.card.defId, props.card.radiant);
  const fieldPower = useFieldPower(props.card.instanceId);
  const copied = useCopiedDef(props.card);
  const face = liveFace(info, props.card, {
    ...(fieldPower === undefined ? {} : { fieldPower }),
    ...(copied === undefined ? {} : { copied }),
  });
  const verdict = props.keep ? "keep" : "redraw";
  // R1655000: the viewer's own hand, on a board that wears cosmetics.
  const own = useWearerSeat() !== null;
  // R711000: the card's name as a reader of Chinese reads it.
  const name = isZhCn() ? chineseName(props.card.defId, info.name, info.def) : info.name;
  return (
    <span
      className="prompt-card"
      data-testid={`mulligan-waiting-card-${props.card.instanceId}`}
      data-verdict={verdict}
      aria-label={props.keep ? t("{name}: kept", { name }) : t("{name}: going back", { name })}
      role="img"
    >
      <span className="cf-option">
        <CardFace face={face} layout="full" worn={own} />
      </span>
      <span className="prompt-card-verdict" aria-hidden="true">
        {props.keep ? t("Keep") : t("Redraw")}
      </span>
    </span>
  );
}

/**
 * R265, R266: the viewer has answered its mulligan and the opponent has not. The answer is sealed (the hand
 * changes once both are in), so the panel shows the hand as it is, each card stamped with what will happen
 * to it (`kept` is the view's record).
 */
function MulliganWaiting(props: { view: PlayerView; mulligan: MulliganView }) {
  useLocale();
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
        // A labeled region, not a modal dialog: there is nothing in it to answer, and a modal
        // would hold a screen reader inside static text while Concede stays live on the board.
        role="region"
        aria-labelledby={titleId}
      >
        <p className="prompt-title" id={titleId}>
          {t("Waiting for your opponent…")}
        </p>
        <p className="prompt-sub" role="status">
          {back === 0
            ? t("You're ready and keeping your whole hand.")
            : back === 1
              ? t("You're ready. {count} card goes back and is redrawn once your opponent is ready too.", { count: back })
              : t("You're ready. {count} cards go back and are redrawn once your opponent is ready too.", { count: back })}
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
  useLocale();
  const lookup = useContext(CatalogContext);
  const interaction = props.interaction ?? IDLE;
  const readOnly = props.readOnly === true;
  // R2211000: a replay lights nothing, so no prompt says "Highlighted on the board as well."
  const boardTestids: string[] = readOnly ? [] : [...highlightFor(props.view, props.legal ?? [], interaction).legal].sort();
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
    // R2211000: a form only an answer fills is not drawn on a replay's board; the waiting panel says so.
    if (readOnly && (pending.kind === "craft" || pending.kind === "market" || isPlaguePlacementPrompt(pending))) {
      return <Waiting pendingFor={props.view.viewer} prompt={t(pending.prompt)} />;
    }
    // ME-CRAFT (Meditative #17, R880): a craft prompt opens the block editor, not the picker.
    if (pending.kind === "craft") {
      return (
        <MatchCardsProvider view={props.view}>
          <CraftEditor key={pending.choiceId} pending={pending} onAction={props.onAction} />
        </MatchCardsProvider>
      );
    }
    // ME-MARKET (Meditative #42, R1000): a night market opens its stall, not the picker.
    if (pending.kind === "market") {
      return (
        <MatchCardsProvider view={props.view}>
          <MarketPrompt
            key={pending.choiceId}
            view={props.view}
            pending={pending}
            legal={props.legal ?? []}
            onAction={props.onAction}
            {...(props.animating === undefined ? {} : { animating: props.animating })}
          />
        </MatchCardsProvider>
      );
    }
    if (isPlaguePlacementPrompt(pending)) {
      return <BoardOnlyPrompt key={pending.choiceId} pending={pending} boardTestids={boardTestids} animating={props.animating} />;
    }
    return (
      <MatchCardsProvider view={props.view}>
        <PromptModal
          key={pending.choiceId}
          source="engine"
          picker={pickerForPending(pending, props.view, props.legal ?? [], props.lesson === true)}
          {...(pending.kind === "mulligan" && mulligan !== undefined
            ? { status: <OpponentMulliganStatus mulligan={mulligan} /> }
            : {})}
          {...(props.animating === undefined ? {} : { animating: props.animating })}
          boardTestids={boardTestids}
          onAction={props.onAction}
          {...(props.onInteraction === undefined ? {} : { onInteraction: props.onInteraction })}
          {...(props.onCancel === undefined ? {} : { onCancel: props.onCancel })}
          readOnly={readOnly}
        />
      </MatchCardsProvider>
    );
  }

  // R81: no prompt is open and nothing is paused, but the play in flight still needs a choice.
  if (readOnly) return null;
  const need = outstandingNeed(interaction);
  if (need === null) return null;

  return (
    <MatchCardsProvider view={props.view}>
      <PromptModal
        key={needKey(need)}
        source="play"
        picker={pickerForNeed(need, interaction, props.view, lookup)}
        boardTestids={boardTestids}
        onAction={props.onAction}
        {...(props.onInteraction === undefined ? {} : { onInteraction: props.onInteraction })}
        {...(props.onCancel === undefined ? {} : { onCancel: props.onCancel })}
      />
    </MatchCardsProvider>
  );
}
