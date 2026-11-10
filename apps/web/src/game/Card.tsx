// The face of a card, and the interaction vocabulary every other board component shares (BUILD M5-T1).
// A card renders `CardView` / `UnitView` and nothing else (CLAUDE.md rule 7, SPEC §10.8); `card: null` draws
// a back (`BackrowView { faceDown: true }`, the opponent's hand): no identity to leak or key a testid off.
// No rule lives here: `legal` comes from the engine's `legalActions`; `canAct` / `usedThisTurn` are state.
// The face (`.cf`, which ignores the pointer, so the root and badges take every click) is the card
// presentation module's (apps/web/src/cards, docs/polish/6-cards.md, faces.ts, SPEC §10.10): the card in
// play, a hand Unit's grown stats (#89) included; a match-made card reads its definition from the view (R243).
// Marks: R667 (hand card `counteredOnPlay`, #87), R437 (the view's marks, #50; a back too, R33), #2270 Brittle
// `BRITTLE_LAST_COUNT` (R385, R638) wears Allure's aura (R963, R1045000) in gray, a mark of its own outranks it;
// #2437 Temporary `TEMPORARY_LAST_COUNT` (R637, R2179001) fuchsia. R1295000: a stolen unit keeps its owner's
// look, the resolving strip wears defaults. R1655000: the viewer's own card wears its own picks. R384, R510:
// `activations` are Activate controls beside the face. R2321000: `tallies` pills (cards/Tallies.tsx). R3338002: a
// Ceaseless Void in the viewer's own hand drains its gem as its cost falls (cards/gemDrain.ts); no other card's does.

import { isValidElement, memo, useMemo, type DragEvent, type KeyboardEvent, type MouseEvent, type ReactElement, type ReactNode } from "react";

import type { CardMark, CardType, CardView, PlayerId, RevealAt, UnitView } from "@jackioh/shared";

import {
  CardBack,
  CardFace,
  FACE_DOWN_TAG,
  FaceDownPreview,
  FaceDownSheet,
  INSPECT_KEY_SHORTCUT,
  Icon,
  MinionFace,
  PileDepth,
  REVEAL_AT_LABEL,
  REVEAL_AT_NOTE,
  TermTips,
  UNREVEALED_NOTE,
  useCanOpenFullScreen,
  faceDownLabel,
  isHeld,
  isInspectKey,
  openFullScreenArt,
  useCardSettings,
  useInspectTrigger,
  useIsHeld,
  type FaceModel,
  type InspectRenderSubject,
  type InspectSubject,
} from "../cards/index.ts";
import { cx } from "../cards/CardFace.tsx";
import { gemDrainOf } from "../cards/gemDrain.ts";
import CardMarks from "../cards/CardMarks.tsx";
import Tallies from "../cards/Tallies.tsx";
import { OwnCopy } from "../cards/fullArt.ts";
import { BRITTLE_LAST_COUNT, TEMPORARY_LAST_COUNT } from "../cards/keywordVisuals.ts";
import { TheirCopy } from "../cards/loadouts.ts";
import { BRITTLE_LAST_COLOR, TEMPORARY_LAST_COLOR, marksOf } from "../cards/marks.ts";
import ActivateControls from "./ActivateControl.tsx";
import { AttackTakenPreview, AttackTargetPreview } from "./AttackOutcome.tsx";
import { outcomeFor } from "./attackOutcome.ts";
import FieldBadges from "./FieldBadges.tsx";
import { usePhonePortrait } from "./phoneBar.ts";
import { useCardInfo, useCopiedDef, useFieldPower, useTallies } from "./catalog.ts";
import { useLiveFace } from "./faces.ts";
import { printedCost } from "./decks.ts";
import { isSpent, useActingSeat } from "./spent.ts";
import { useKeywordLost } from "./keywordLoss.ts";
import { useAwakening } from "./awakening.ts";
import { useSeatDeckCosmetics, useTheirLoadout, useWearerSeat } from "./wearer.ts";
import { NO_HIGHLIGHT, testid, type AnimatingMap, type ClickTarget, type Highlight } from "./contract.ts";
import { COUNTERED_NOTE, conditionAttr, conditionTargetAttr, counteredAttr, countedAttr, glowAttr, paretoAttr } from "./glow.ts";
import { useSetting } from "../settings/store.ts";
import { t } from "../i18n/i18n.ts";
import { SHORTCUTS } from "../shortcuts/shortcuts.ts";

import "./facedown.css";
import "./countered.css";
import "./spent.css";

/** #258: what the "can't act yet" badge says in words (spent.css draws it). */
export const SPENT_NOTE = "No action left this turn";

/** R90 (#992): what a hand card whose listed plays were cut says; informational, never "illegal". */
export const CHOICES_TRUNCATED_NOTE = "Not every combination of this card's choices is listed; each choice is in at least one listed play";

/** B17: the board's class joiner, still exported here, is the card face's own. */
export { cx };

export function isLegal(highlight: Highlight | undefined, testId: string | undefined): boolean {
  if (testId === undefined) return false;
  return (highlight ?? NO_HIGHLIGHT).legal.has(testId);
}

export function isSelected(highlight: Highlight | undefined, testId: string | undefined): boolean {
  if (testId === undefined) return false;
  return (highlight ?? NO_HIGHLIGHT).selected.has(testId);
}

/** `data-legal` is the only gate on firing `onClick` but R979000's reason; see the drag note below. */
export function legalAttr(legal: boolean): "true" | "false" {
  return legal ? "true" : "false";
}

// Drag-to-attack: a drop reports two clicks, the source then what it landed on, so click-click and
// drag reach `actions.ts` through one path (BUILD M5-T2). Only the source is gated, at `dragstart`: at
// `drop` the target's `data-legal` still describes the board *before* the pickup and would refuse it.

export const DRAG_MIME = "application/x-jackioh-target";

export function encodeTarget(target: ClickTarget): string {
  return JSON.stringify(target);
}

export function decodeTarget(raw: string): ClickTarget | null {
  if (raw === "") return null;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed === "object" && parsed !== null && "on" in parsed) return parsed as ClickTarget;
    return null;
  } catch {
    return null;
  }
}

export function beginDrag(event: DragEvent<HTMLElement>, legal: boolean, target: ClickTarget | null | undefined): void {
  if (!legal || target === null || target === undefined) {
    event.preventDefault();
    return;
  }
  event.dataTransfer.setData(DRAG_MIME, encodeTarget(target));
  event.dataTransfer.effectAllowed = "move";
}

export function allowDrop(event: DragEvent<HTMLElement>): void {
  const types: readonly string[] = Array.from(event.dataTransfer.types);
  if (types.includes(DRAG_MIME)) event.preventDefault();
}

export function completeDrop(
  event: DragEvent<HTMLElement>,
  target: ClickTarget | null | undefined,
  onClick: ((target: ClickTarget) => void) | undefined,
): void {
  if (target === null || target === undefined) return;
  const source = decodeTarget(event.dataTransfer.getData(DRAG_MIME));
  if (source === null) return;
  event.preventDefault();
  event.stopPropagation();
  onClick?.(source);
  onClick?.(target);
}

// Animation hooks (BUILD M5-T4). The runner owns timing; the board only owns the DOM it needs.
// `.damage-pop` text equals the amount in the `damage` event.

export type Pops = { damage?: number; heal?: number; loss?: number };

export function PopLayer({ pops }: { pops?: Pops }): ReactElement | null {
  if (pops === undefined) return null;
  const { damage, heal, loss } = pops;
  if (damage === undefined && heal === undefined && loss === undefined) return null;
  return (
    <span className="pop-layer" aria-hidden="true">
      {damage !== undefined && <span className="damage-pop">{damage}</span>}
      {heal !== undefined && <span className="heal-pop">{heal}</span>}
      {loss !== undefined && <span className="loss-pop">{loss}</span>}
    </span>
  );
}

export type CardProps = {
  /** Absent for a face-down back: the view gives it no instance id, so it has no testid. */
  testId?: string;
  /** `null` draws a back and renders no name, no def id and no stats (SPEC §10.8). */
  card: CardView | null;
  /** Present for a unit. */
  unit?: UnitView | null;
  /** A face-up backrow card's type, straight off `BackrowView`. */
  type?: CardType;
  /** R33: a public card names its owner and its controller; they differ after a steal or a swap. */
  owner?: PlayerId;
  controller?: PlayerId;
  /** R1295000: a card in the viewer's own hand, which wears the device's cosmetics: `CardView` names no owner. */
  inOwnHand?: boolean;
  /** R1834004: the seat whose hand the card lies in, whose loadout it wears where it is not `worn`: `CardView` names no owner. */
  inHandOf?: PlayerId;
  /**
   * R1297000, R1834004: the seat whose card back a back wears, the same for every back of a seat (a
   * back names no card, B16): the viewer's own pick on the wearer's side, the seat's loadout elsewhere.
   */
  backOf?: PlayerId;
  /** `gradeLetter` is the letter the view names #93's grade by (R372); the badge prints it. */
  counters?: { plague?: number; grade?: number; gradeLetter?: string };
  /**
   * R370: a face-down backrow trap's back; absent for every other back. `at` names its zone ("opponent-3"),
   * which keys its inspect overlay (a back has no instance id). R437, R33: `marks`. R471: Plague Counters, public.
   */
  faceDown?: { cost?: number; at: string; marks?: readonly CardMark[]; plague?: number };
  /** R371: the viewer's own face-down trap (`BackrowView.unrevealed`), veiled: the other sees its back. */
  unrevealed?: boolean;
  /**
   * R1046: when the viewer's own face-down play reveals (`BackrowView.revealAt`). Shown beside the
   * "Face down" tag; the opponent's view never carries it, so their card shows no badge.
   */
  revealAt?: RevealAt;
  /** What a click reports. Absent means the element is not clickable and clicks bubble. */
  target?: ClickTarget | null;
  /** Units carry the switch-position button (BUILD M5-T2). */
  switchTarget?: boolean;
  /**
   * #2932: a face-up card standing in a field zone (a unit in the units row, a unit or a card in the
   * backrow). On the phone portrait layout its Activate control and keyword marks stand in its zone's
   * top-left corner (`FieldBadges.tsx`); anywhere else, and in a hand or an inspect view, they sit on the face.
   */
  onField?: boolean;
  draggable?: boolean;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
  pops?: Pops;
  className?: string;
  /** #2267: a hand card's hint row (HandHints.tsx), drawn as a child of the root so that a click on it is the card's. A back draws none. */
  hintRow?: ReactNode;
};

/** Which face a card draws, straight from its props (docs/polish/6-cards.md, "Card.tsx"). */
type Form = "back" | "minion" | "compact" | "full";

function formOf(card: CardView | null, unit: UnitView | null | undefined, type: CardType | undefined): Form {
  if (card === null) return "back";
  if (unit !== undefined && unit !== null) return "minion";
  if (type !== undefined) return "compact";
  return "full";
}

/** R849000: a card's native tooltip, its name and why your own unit cannot attack now; R979000: a card in hand that cannot be played says why instead. */
function cardTitle(name: string, unit: UnitView | null | undefined, card: CardView | null): string {
  const reason = unit?.attackReason ?? (unit == null ? card?.playReason : undefined);
  return reason === undefined ? name : t("{name}: {reason}", { name, reason: t(reason) });
}

function Card(props: CardProps): ReactElement {
  const { card, unit, target, testId } = props;
  const info = useCardInfo(card?.defId ?? "", card?.radiant ?? false);
  const fieldPower = useFieldPower(card?.instanceId);
  const copied = useCopiedDef(card);
  const tallies = useTallies(unit);
  const settings = useCardSettings();
  // The preview opens only while the panel's "Hover previews" is on too (useInspectTrigger.tsx).
  const panelHover = useSetting("hoverPreviews");
  const form = formOf(card, unit, props.type);
  // #2932: hooks run on every render, backs included. Not cached: a test swaps `matchMedia`.
  const portrait = usePhonePortrait();
  const offFace = props.onField === true && portrait;
  const acting = useActingSeat();
  const wearer = useWearerSeat();
  const { cardBack } = useSeatDeckCosmetics(props.backOf ?? null);
  // R1295000: the viewer's own card wears the device's cosmetics, on a board that wears them; R1834004:
  // any other card its owner's loadout, where the board carries one.
  const owner = props.owner ?? unit?.owner ?? props.inHandOf;
  const worn = wearer !== null && (props.inOwnHand === true || owner === wearer);
  const theirs = useTheirLoadout(card === null || worn ? null : owner);

  // The card in play: the unit's view when it is one, else the card's own. A back has no face at all.
  const shown = unit ?? card;
  // #3478: the face memoized on the view object, the definition, the live facts and the language.
  const model: FaceModel = useLiveFace(info, shown, {
    ...(props.type === undefined ? {} : { type: props.type }),
    ...(fieldPower === undefined ? {} : { fieldPower }),
    ...(copied === undefined ? {} : { copied }),
    ...(tallies.length === 0 ? {} : { tallies }),
    ...(props.inHandOf === undefined ? {} : { inHand: true }),
  });
  // A face-up backrow card is drawn as the type its `BackrowView` names: the view is what the
  // client renders (CLAUDE.md rule 7), and the catalog only fills in what the view leaves out.
  // #3633: one object while its parts are, so the face (memoized, CardFace.tsx) is not drawn again.
  const face: FaceModel = useMemo(() => (props.type === undefined ? model : { ...model, type: props.type }), [model, props.type]);

  // Hooks run on every render, backs included. A back in the backrow (R370) opens what the view
  // says of it; any other back opens nothing. A full face sits in a row, so its preview rises over
  // it rather than covering the neighbour the pointer is heading for.
  const faceDown = props.faceDown;
  const subject: InspectSubject | InspectRenderSubject | null =
    card !== null
      ? {
          key: testId ?? card.instanceId,
          face,
          ...(props.unrevealed === true
            ? { note: t(UNREVEALED_NOTE) }
            : card.counteredOnPlay === true
              ? { note: t(COUNTERED_NOTE) }
              : card.choicesTruncated === true
                ? { note: t(CHOICES_TRUNCATED_NOTE) }
                : {}),
        }
      : faceDown !== undefined
        ? {
            key: `facedown-${faceDown.at}`,
            render: ({ mode, anchor, close }) =>
              mode === "hover" ? (
                <FaceDownPreview cost={faceDown.cost} plague={faceDown.plague} anchor={anchor} />
              ) : (
                <FaceDownSheet cost={faceDown.cost} plague={faceDown.plague} onClose={close} />
              ),
          }
        : null;
  // R3345000: Mythic / full-art view trigger. On the board, only own cards and not while the queue drains: `draining`
  // is subscribed, so the hold follows the queue, and the callback asks again as the hold fires.
  const inHand = props.inOwnHand === true;
  const opensFullScreen = useCanOpenFullScreen(card === null ? null : face, card?.defId ?? "", worn);
  // #3633: only a worn unit of the viewer's that could open full screen reads the hold, so the queue
  // starting or stopping draws no other card again.
  const draining = useIsHeld(card !== null && opensFullScreen && !inHand && unit !== undefined && unit !== null && worn);
  const canFullScreen =
    card !== null &&
    opensFullScreen &&
    (inHand || (unit !== undefined && unit !== null && worn && !draining));

  // Lines of code is a hidden stat in matches.
  const inspect = useInspectTrigger(subject, {
    prefer: form === "full" ? "above" : "beside",
    showLoc: false,
    keyHint: SHORTCUTS.inspect,
    holdAnyPointer: canFullScreen ? true : undefined,
    onLongPress: canFullScreen
      ? (pointerId) => {
          if (!inHand && isHeld()) return;
          openFullScreenArt({
            defId: card.defId,
            radiant: card.radiant,
            name: face.name,
            heldPointerId: pointerId,
          });
        }
      : undefined,
  });

  const legal = isLegal(props.highlight, testId);
  const selected = isSelected(props.highlight, testId);
  const clickable = target !== null && target !== undefined;
  const animatingEvent = testId === undefined ? undefined : props.animating?.get(testId);
  // R3615000: the hovered attack's markers. The card wears the blow it takes when it is the hovered
  // target, and what it takes back when it is the hovering attacker (a strike back of 0 shows none).
  const hovered = props.highlight?.hoveredAttack;
  const targetOutcome = hovered?.target === testId ? outcomeFor(props.highlight, testId) : undefined;
  const takenOutcome =
    hovered !== undefined && hovered.attacker === testId && hovered.strikesBack
      ? outcomeFor(props.highlight, hovered.target)
      : undefined;
  // #1598: a `keywordGranted` that takes the keyword away plays as the icon going (keywordLoss.ts).
  const keywordLost = useKeywordLost(testId);
  // #2435: an `awakened` in flight breaks the card's Dormant seal until the board catches up
  // (awakening.ts, R1839002, R1852003).
  const awakening = useAwakening(testId);
  // R979000: a card in your hand the view says cannot be played stays greyed out, as `legal` offers
  // it nothing, but its click still reaches the board, which shows `playReason` and sends nothing.
  const explains = card?.playReason !== undefined;

  function fire(event: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>): void {
    if (!clickable || !(legal || explains)) return;
    event.stopPropagation();
    props.onClick?.(target);
  }

  function onKeyDown(event: KeyboardEvent<HTMLElement>): void {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    fire(event);
  }

  // #258: the inspect keys open the sheet; every card with something to inspect takes focus for it,
  // legal or not; Enter and Space play or pick only a legal one. A right-click stays the drag cancel.
  const keyInspect = subject !== null;
  function onInspectKey(event: KeyboardEvent<HTMLElement>): void {
    if (isInspectKey(event)) {
      event.preventDefault();
      event.stopPropagation();
      inspect.openSheet(event.currentTarget);
      return;
    }
    if (clickable) onKeyDown(event);
  }

  const shared = {
    "data-testid": testId,
    "data-legal": clickable ? legalAttr(legal) : undefined,
    "data-selected": selected ? "true" : undefined,
    "data-animating": animatingEvent,
    "data-lost": keywordLost ? "true" : undefined,
    "data-awakening": awakening ? "true" : undefined,
    "data-glow": glowAttr(props.highlight, testId),
    "aria-disabled": clickable && !legal ? ("true" as const) : undefined,
    onClick: clickable ? fire : undefined,
    onKeyDown: keyInspect ? onInspectKey : clickable ? onKeyDown : undefined,
    tabIndex: (clickable && legal) || keyInspect ? 0 : undefined,
    "aria-keyshortcuts": keyInspect ? INSPECT_KEY_SHORTCUT : undefined,
    draggable: props.draggable === true && legal ? true : undefined,
    onDragStart: props.draggable === true ? (event: DragEvent<HTMLElement>) => beginDrag(event, legal, target) : undefined,
    onDragOver: clickable ? allowDrop : undefined,
    onDrop: clickable ? (event: DragEvent<HTMLElement>) => completeDrop(event, target, props.onClick) : undefined,
  };

  if (card === null) {
    // A back: no name, def id or instance id, as the view has none. A face-down trap shows its cost.
    if (faceDown === undefined) {
      return (
        // WCAG 4.1.2 (#757): a name needs a role; a back is a picture, or a button when it is clickable.
        <div
          {...shared}
          className={cx("card", "card-back", props.className)}
          data-face-down="true"
          role={clickable ? "button" : "img"}
          aria-label={t("Face-down card")}
        >
          <CardBack back={cardBack} />
        </div>
      );
    }
    // R471: the back's Plague Counters are public, so its label says them too.
    const label =
      faceDown.plague === undefined
        ? t(faceDownLabel(faceDown.cost))
        : t("{label}, Plague Counters: {count}", { label: t(faceDownLabel(faceDown.cost)), count: faceDown.plague });
    return (
      <>
        <div
          {...shared}
          {...inspect.handlers}
          className={cx("card", "card-back", "card-facedown", props.className)}
          data-face-down="true"
          data-facedown-cost={faceDown.cost}
          data-facedown-plague={faceDown.plague}
          data-marks={faceDown.marks !== undefined && faceDown.marks.length > 0 ? faceDown.marks.map((entry) => entry.mark).join(" ") : undefined}
          role="img"
          aria-label={label}
          title={settings.hoverPreviews && panelHover ? undefined : label}
        >
          <CardBack back={cardBack} />
          {faceDown.cost === undefined ? null : (
            <span className="facedown-cost" data-cost={faceDown.cost} aria-hidden="true">
              {faceDown.cost}
            </span>
          )}
          {faceDown.plague === undefined ? null : (
            // R471: the badge a face-up card's Plague Counters wear (board.css `.counter-plague`).
            <span className="counter counter-plague" data-counter="plague" title={t("Plague counters")} aria-hidden="true">
              {faceDown.plague}
            </span>
          )}
          <CardMarks marks={faceDown.marks ?? []} instanceId={`facedown-${faceDown.at}`} />
        </div>
        {inspect.overlay}
      </>
    );
  }

  const position = unit?.position;
  const marks = marksOf(unit ?? card);
  // #2270: the view's Brittle count, never the keyword's printed number (CLAUDE.md rule 7), on the field only.
  const brittleLast = (form === "minion" || form === "compact") && (unit?.brittle ?? card.brittle) === BRITTLE_LAST_COUNT;
  // #2437: the face's Temporary count, which only a hand card's carries, at its last.
  const temporaryLast = face.temporary === TEMPORARY_LAST_COUNT;
  const counters: CardProps["counters"] = props.counters ?? unit?.counters;
  const buried = unit?.buried ?? 0;
  const cardType = face.type;
  const spent = unit !== undefined && unit !== null && isSpent(unit, acting);
  // R3338002: the card's own cost against its printed one, in its owner's own hand only (R202).
  const printed = info.def === undefined ? undefined : printedCost(info.def.cost);
  const gemDrain = props.inOwnHand === true ? gemDrainOf(card.defId, card.cost, printed) : null;

  const root = (
    <div
      {...shared}
      {...inspect.handlers}
      className={cx(
        "card",
        unit ? "card-unit" : "card-spell",
        card.radiant && "radiant",
        "cf-host",
        `cf-host--${form}`,
        props.className,
      )}
      data-face={form}
      data-def-id={card.defId}
      data-radiant={card.radiant ? "true" : undefined}
      data-rarity={face.rarity ?? undefined}
      data-card-type={cardType}
      // animations.css keeps a fired Field Trap on the board with its own flip (trapFired).
      data-field-trap={cardType === "Field Trap" ? "true" : undefined}
      data-condition-active={conditionAttr(card) ?? conditionTargetAttr(props.highlight, testId)}
      // R667: the engine says a card on the field would counter this hand card if played now.
      data-countered-on-play={counteredAttr(card)}
      // R1844001: the engine says the judge would call this hand card's play optimal (the Pareto hint).
      data-pareto-optimal={paretoAttr(card)}
      // #991, R583: a hovered or selected hand card's preview counts this card (highlights.css rings it).
      data-counted={countedAttr(props.highlight, testId)}
      data-owner={props.owner ?? unit?.owner}
      data-controller={props.controller ?? unit?.controller}
      data-vanilla={unit?.vanilla === true ? "true" : undefined}
      data-marks={marks.length > 0 ? marks.map((entry) => entry.mark).join(" ") : undefined}
      data-unrevealed={props.unrevealed === true ? "true" : undefined}
      data-position={position}
      // `canAct` is drawn as state, never read as permission: legality is `props.highlight`.
      data-can-act={unit === undefined || unit === null ? undefined : unit.canAct ? "true" : "false"}
      // #258: no exertion left on the acting side's turn (spent.ts), drawn dimmed with a "Zz".
      data-spent={spent ? "true" : undefined}
      // DEF is a sideways card: the rotation is inline so a test can read `rotate(90deg)` off the
      // style (BUILD M5-T4 `positionSwitched`); the lane's `--def-scale` (board.css) keeps it in its lane.
      style={position === "DEF" ? { transform: "rotate(90deg) scale(var(--def-scale, 1))" } : undefined}
      // The hover preview replaces the native tooltip; with previews off, the name comes back, and
      // with it (R849000) the engine's reason a unit of yours cannot attack now.
      title={settings.hoverPreviews && panelHover ? undefined : cardTitle(face.name, unit, card)}
    >
      {unit !== undefined && unit !== null ? (
        <MinionFace face={face} unit={unit} worn={worn} keywordsOffFace={offFace} attacking={animatingEvent === "attackDeclared"} />
      ) : (
        // Issue #814: a Summit in play shortens to its header and next Ascent line; the overlays keep it all.
        // R2098000: its terms show their rule on hover and on a long press (cards/TermTip.tsx); a tap on one still reaches this root.
        // R2723000: a full-art card in the viewer's own hand draws its art alone; its inspect overlays draw it whole.
        <TermTips>
          <CardFace
            face={face}
            layout={form === "compact" ? "compact" : "full"}
            ascentLevel={face.ascent}
            worn={worn}
            ownHand={worn && props.inOwnHand === true}
            onBoard={form === "compact"}
            gemDrain={gemDrain}
            tilt="host"
          />
        </TermTips>
      )}

      {/* Everything below is a sibling of `.cf`, not inside it, so it stays clickable while the
          face has `pointer-events: none`. */}
      <CardMarks marks={marks} instanceId={card.instanceId} bareAura={brittleLast ? BRITTLE_LAST_COLOR : temporaryLast ? TEMPORARY_LAST_COLOR : undefined} />
      {/* R2321000: the counts cards on the field keep on the unit (Level Up!'s attacks). */}
      <Tallies tallies={tallies} instanceId={card.instanceId} />
      {card.counteredOnPlay === true && (
        // R667: "don't play this" by shape as well as color. The film takes no pointer, and a click
        // or drag on the badge still reaches the card it sits in (countered.css).
        <span className="countered-warning" data-testid={testid.countered(card.instanceId)}>
          <span className="countered-warning-badge" aria-hidden="true" title={t(COUNTERED_NOTE)} />
          <span className="countered-warning-sr">{t(COUNTERED_NOTE)}</span>
        </span>
      )}
      {card.choicesTruncated === true && (
        // R90 (#992): legalActions lists a slice of this card's choice combinations, every pick among
        // them. A child of the root, so a click or drag on it reaches the card.
        <span className="choices-truncated-badge" data-testid={testid.choicesTruncated(card.instanceId)} title={t(CHOICES_TRUNCATED_NOTE)}>
          <span aria-hidden="true">⋯</span>
          <span className="choices-truncated-sr">{t(CHOICES_TRUNCATED_NOTE)}</span>
        </span>
      )}
      {/* #2267: the hint row takes no pointer event (handHints.css). */}
      {props.hintRow}
      {position !== undefined && <span className="position-tag">{t(position)}</span>}

      {counters?.plague !== undefined && (
        <span className="counter counter-plague" data-counter="plague" title={t("Plague counters")}>
          {counters.plague}
        </span>
      )}
      {counters?.grade !== undefined && (
        // R372: the grade by the letter the view names it with (E to S); the number is a fallback
        // for a view that names none.
        <span
          className="counter counter-grade"
          data-counter="grade"
          data-grade={counters.grade}
          data-grade-letter={counters.gradeLetter}
          title={counters.gradeLetter === undefined ? t("Grade counters") : t("Grade {letter}", { letter: counters.gradeLetter })}
        >
          {counters.gradeLetter ?? counters.grade}
        </span>
      )}

      {props.unrevealed === true && (
        // R371: the tag says it in words and with the struck-through eye, never by color alone.
        <span className="unrevealed-tag" data-testid={testid.unrevealed(card.instanceId)} title={t(UNREVEALED_NOTE)}>
          <Icon name="eyeOff" />
          <span className="unrevealed-tag-text">{t(FACE_DOWN_TAG)}</span>
          <span className="unrevealed-tag-sr">{t(": your opponent can't see this card")}</span>
        </span>
      )}

      {props.unrevealed === true && props.revealAt !== undefined && (
        // R1046: the owner's own reveal timing, in the same words the picker used.
        <span className="reveal-at-badge" data-testid={testid.revealAt(card.instanceId)} title={t(REVEAL_AT_NOTE)}>
          <span className="reveal-at-badge-text">{t(REVEAL_AT_LABEL[props.revealAt] ?? "")}</span>
        </span>
      )}

      {spent && (
        // #258: the "can't act yet" cue (spent.ts says which units wear it, spent.css draws it).
        <span className="spent-badge" title={t(SPENT_NOTE)}>
          <span aria-hidden="true">Zz</span>
          <span className="spent-badge-sr">{t(SPENT_NOTE)}</span>
        </span>
      )}

      {/* §3.2: the cards beneath a Stack pile are face-down and out of play. The view gives their
          number only, so the pile shows a depth badge and no second card; a press opens the pile
          as a wheel, the top with its face and every buried card as a back. */}
      {buried > 0 && <PileDepth buried={buried} top={face} className="buried-badge" />}

      {/* R384, R510: the card's Activate abilities, which the view lists on its controller's own
          view of a card acting on the field (ActivateControl.tsx). Nothing when it lists none.
          #2932: on a phone's field they stand in the zone's corner instead (FieldBadges, below). */}
      {offFace ? null : <ActivateControls card={unit ?? card} highlight={props.highlight} animating={props.animating} onClick={props.onClick} />}

      <PopLayer pops={props.pops} />
      {targetOutcome !== undefined && testId !== undefined && (
        <AttackTargetPreview outcome={targetOutcome} target={testId} />
      )}
      {takenOutcome !== undefined && takenOutcome.taken > 0 && testId !== undefined && (
        <AttackTakenPreview outcome={takenOutcome} attacker={testId} />
      )}
    </div>
  );

  // The overlay is a sibling of the root, never its child, so the root's handlers never see an event from
  // inside a preview or a sheet. #258: so is the switch (⟳): a full 44 px target (board.css), never rotated by DEF.
  // #2932: and so, on a phone's field, are the Activate control and the keyword marks (FieldBadges.tsx).
  // R1313000, R1655000: the viewer's own card's overlay prints its full art and cosmetics; R1834004: another
  // seat's card wears that seat's loadout.
  return (
    <TheirCopy loadout={theirs}>
      {root}
      {offFace && (
        <FieldBadges
          card={unit ?? card}
          unit={unit}
          words={face.keywordWords}
          join={face.keywordJoin}
          highlight={props.highlight}
          animating={props.animating}
          onClick={props.onClick}
        />
      )}
      {props.switchTarget === true && unit !== undefined && unit !== null && (
        <SwitchButton instanceId={unit.instanceId} highlight={props.highlight} animating={props.animating} onClick={props.onClick} />
      )}
      <OwnCopy own={worn}>{inspect.overlay}</OwnCopy>
    </TheirCopy>
  );
}

/**
 * #3633: the keys of the board's `animating` map a card's tree reads: its own testid, its unit's `card-`
 * one (its Activate controls flash on it) and its switch's. A queue step hands every card a new map; a
 * card whose keys all name what they named draws nothing new.
 */
export function cardAnimatingKeys(props: Pick<CardProps, "testId" | "card" | "unit">): string[] {
  const keys = props.testId === undefined ? [] : [props.testId];
  const instanceId = (props.unit ?? props.card)?.instanceId;
  if (instanceId !== undefined) keys.push(testid.card(instanceId), testid.switchPosition(instanceId));
  return keys;
}

/** Equal plain data: what a parent builds inline for a card on each render (its target, pops, back and counters). */
function sameData(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null || Array.isArray(a) !== Array.isArray(b)) return false;
  const left = a as Record<string, unknown>;
  const right = b as Record<string, unknown>;
  const keys = Object.keys(left);
  return keys.length === Object.keys(right).length && keys.every((key) => Object.hasOwn(right, key) && sameData(left[key], right[key]));
}

/** The same element with the same props: a hand card's hint row, built inline on each render. */
function sameElement(a: ReactNode, b: ReactNode): boolean {
  if (Object.is(a, b)) return true;
  if (!isValidElement(a) || !isValidElement(b) || a.type !== b.type || a.key !== b.key) return false;
  const left = a.props as Record<string, unknown>;
  const right = b.props as Record<string, unknown>;
  const keys = Object.keys(left);
  return keys.length === Object.keys(right).length && keys.every((key) => Object.hasOwn(right, key) && Object.is(left[key], right[key]));
}

/**
 * #3633: whether a card would draw what it drew. Every prop is the same object as before, but for the
 * `animating` map, of which only `cardAnimatingKeys` count, and the inline data and hint row a parent
 * builds anew on every render, which count by what they hold. Without it every queue step drew every
 * card on the board again, the slowest work of a throttled phone's frame (R3351000).
 */
export function sameCardProps(prev: Readonly<CardProps>, next: Readonly<CardProps>): boolean {
  const keys = new Set([...Object.keys(prev), ...Object.keys(next)] as (keyof CardProps)[]);
  for (const key of keys) {
    const a = prev[key];
    const b = next[key];
    if (Object.is(a, b)) continue;
    switch (key) {
      case "animating":
        if (cardAnimatingKeys(next).some((id) => prev.animating?.get(id) !== next.animating?.get(id))) return false;
        continue;
      case "hintRow":
        if (!sameElement(prev.hintRow, next.hintRow)) return false;
        continue;
      case "target":
      case "pops":
      case "faceDown":
      case "counters":
        if (!sameData(a, b)) return false;
        continue;
      default:
        return false;
    }
  }
  return true;
}

export default memo(Card, sameCardProps);

function SwitchButton({
  instanceId,
  highlight,
  animating,
  onClick,
}: {
  instanceId: string;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
}): ReactElement {
  const testId = testid.switchPosition(instanceId);
  const legal = isLegal(highlight, testId);
  return (
    <button
      type="button"
      className="switch-button"
      data-testid={testId}
      data-legal={legalAttr(legal)}
      data-selected={isSelected(highlight, testId) ? "true" : undefined}
      data-animating={animating?.get(testId)}
      aria-disabled={legal ? undefined : "true"}
      disabled={!legal}
      title={t("Switch position")}
      aria-label={t("Switch position")}
      onClick={(event) => {
        event.stopPropagation();
        if (!legal) return;
        onClick?.({ on: "switch", instanceId });
      }}
    >
      <span className="switch-glyph" aria-hidden="true">
        ⟳
      </span>
    </button>
  );
}
