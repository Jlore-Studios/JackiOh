// The board (BUILD M5-T1): five lanes, two seats, two hands, the control bar and the log, all drawn from one `PlayerView`
// (CLAUDE.md rule 7, SPEC §10.8), oriented by `viewFor`. Nothing here decides anything: every clickable element takes
// `data-legal` from `props.highlight.legal` (`actions.ts`, from `legalActions`) and fires `props.onClick` only when true (or,
// R979000, to show why a hand card cannot be played, sending nothing). `data-glow` (docs/polish/7-mobile-ux.md S6, S11) is a subset
// of `legal`; `dragToPlay` becomes `data-drag`; `confirmEndTurn` makes End turn ask twice while something is playable. board.css's
// grid keeps the children in this order. #2645: on a phone (`usePhoneBar`) the bar keeps to one row; Offer draw, Concede and Friends
// are ⋯'s menu rows (`MatchMenu.tsx`). A hovered or selected hand card rings the cards its preview names (#991, SPEC §10.10).
// R1297000: the viewer's half wears the wearer's playmat and backdrop. R1834004: an online board's other half wears its seat's frozen loadout (`loadouts`, game/wearer.ts).

import { useCallback, useContext, useRef, useState, type KeyboardEvent, type MouseEvent, type ReactElement, type ReactNode } from "react";

import type { CardView, CreditView, GameEvent, GameEventType, LibraryView, PlayerId, PlayerView, Row } from "@jackioh/shared";

import { animTestid } from "./animations.ts";
import Card, { cx, isLegal, isSelected, legalAttr, type Pops } from "./Card.tsx";
import { CatalogContext, MatchCardsProvider, type CardLookup } from "./catalog.ts";
import {
  LANES,
  NO_HIGHLIGHT,
  playerOf,
  sideOf,
  sideView,
  testid,
  type AnimatingMap,
  type AnimationFrames,
  type BoardControl,
  type BoardProps,
  type Highlight,
  type Side,
} from "./contract.ts";
import Hand from "./Hand.tsx";
import Hero from "./Hero.tsx";
import Log from "./Log.tsx";
import Marketplace from "./Marketplace.tsx";
import MatchMenu from "./MatchMenu.tsx";
import { usePhoneBar } from "./phoneBar.ts";
import { offerDrawControlText } from "./DrawOffer.tsx";
import SandSurface from "./SandSurface.tsx";
import StarscapeMap from "./StarscapeMap.tsx";
import { BurnNotice, PileNotice, noticesFrom, noticeTestid, type OverflowNotices } from "./OverflowNotices.tsx";
import Zone from "./Zone.tsx";
import { listedFace, namedFace } from "./faces.ts";
import { countedTestids, glowAttr, hasMovesLeft } from "./glow.ts";
import { revealedOpponentHand } from "./reveal.ts";
import { CardListPreview, CardListSheet, useInspectTrigger, type CardListEntry, type InspectOverlayState } from "../cards/index.ts";
import { SettingsButton, useSetting } from "../settings/index.ts";
import AudioToggle from "../audio/AudioToggle.tsx";
import { getAudioEngine } from "../audio/engine.ts";
import LanguageButton from "../i18n/LanguageButton.tsx";
import { FriendsButton } from "../together/FriendsButton.tsx";
import { formatSeconds, t } from "../i18n/i18n.ts";
import { ActingProvider } from "./spent.ts";
import { KeywordLossProvider, keywordLossesFrom } from "./keywordLoss.ts";
import { AwakeningProvider, awakeningsFrom } from "./awakening.ts";
import { WearerProvider, seatDeckCosmetics, useSeatDeckCosmetics, type Wear } from "./wearer.ts";
import Backdrop from "./Backdrop.tsx";
import { DEFAULT_CARD_COSMETICS, useCosmetics, useOwnsAccountCosmetic } from "../cards/cosmetics.ts";
import { useCardSettings } from "../cards/settings.ts";

// Order matters: highlights.css paints the glow over board.css's borders (S7); table.css restyles
// both halves of board.css's field and so comes after it (R1297000, R1834004).
import "./board.css";
import "./table.css";
import "./highlights.css";
import "./inspectable.css";

/** Top to bottom in every lane column (BUILD M5-T1). */
const FIELD_ROWS: readonly { side: Side; row: Row }[] = [
  { side: "opponent", row: "backrow" },
  { side: "opponent", row: "units" },
  { side: "you", row: "units" },
  { side: "you", row: "backrow" },
];

/**
 * ASSUMPTION, recorded because `contract.ts` has no entry for them: the per-side pile, hand and
 * mana counters use the ids `e2e/support/testids.ts` already assumes, so the specs and the board agree.
 */
function countId(pile: "library" | "graveyard" | "exile", side: Side): string {
  return `${pile}-count-${side}`;
}

/** R1639000: the fatigue an empty deck has dealt so far, which it shows at rest; null while it holds cards or has dealt none. */
function restingFatigue(count: number, fatigue: number | undefined): number | null {
  return count === 0 && fatigue !== undefined && fatigue > 0 ? fatigue : null;
}

// Animation hooks: `.damage-pop`, `.heal-pop` and `.loss-pop` take their text from the event the
// runner is currently animating on that element (BUILD M5-T4). The board does not time anything.

const POP_EVENTS: readonly GameEventType[] = ["damage", "healed", "healthLost"];

function heroTargetId(view: PlayerView, testId: string): PlayerId | null {
  if (testId === testid.hero("you")) return playerOf(view, "you");
  if (testId === testid.hero("opponent")) return playerOf(view, "opponent");
  return null;
}

function amountFor(
  view: PlayerView,
  events: readonly GameEvent[],
  testId: string,
  type: GameEventType,
): number | undefined {
  const player = heroTargetId(view, testId);
  const instanceId = testId.startsWith("card-") ? testId.slice("card-".length) : null;
  // The last matching event wins: the runner animates them in order.
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];
    if (event === undefined || event.type !== type) continue;
    if (event.type === "healthLost") {
      if (player !== null && event.player === player) return event.amount;
      continue;
    }
    if (event.type === "damage" || event.type === "healed") {
      if (instanceId !== null && event.targetId === instanceId) return event.amount;
      if (player !== null && event.targetId === `hero-${player}`) return event.amount;
    }
  }
  return undefined;
}

/**
 * The numbers to pop come from the burst's events, not `view`: the runner is still HOLDING BACK that
 * view (BUILD M5-T4), so it lacks the event being animated. No burst: what animates against the shown view.
 */
export function popsFrom(
  view: PlayerView,
  animating: AnimatingMap | undefined,
  animated?: readonly AnimationFrames[],
): ReadonlyMap<string, Pops> {
  const pops = new Map<string, Pops>();
  const sources: readonly AnimationFrames[] =
    animated ?? (animating === undefined ? [] : [{ frames: animating, events: view.events }]);
  for (const source of sources) {
    for (const [testId, type] of source.frames) {
      if (!POP_EVENTS.includes(type)) continue;
      const amount = amountFor(view, source.events, testId, type);
      if (amount === undefined) continue;
      const current = pops.get(testId) ?? {};
      pops.set(
        testId,
        type === "damage"
          ? { ...current, damage: amount }
          : type === "healed"
            ? { ...current, heal: amount }
            : { ...current, loss: amount },
      );
    }
  }
  return pops;
}

function ManaTray({
  side,
  mana,
  credit,
  animating,
}: {
  side: Side;
  mana: { current: number; max: number };
  credit?: CreditView;
  animating?: AnimatingMap;
}): ReactElement {
  const testId = `mana-${side}`;
  const crystals = Math.max(mana.max, mana.current);
  // R1224: the instalment the last refresh took locks that many of the tray's top crystals.
  const locked = credit?.locked ?? 0;
  return (
    <span
      className="mana"
      data-testid={testId}
      data-current={mana.current}
      data-max={mana.max}
      data-animating={animating?.get(testId)}
      title={t("Mana {current}/{max}", { current: mana.current, max: mana.max })}
    >
      <span className="crystals">
        {Array.from({ length: crystals }, (_unused, index) => (
          // `.mana-crystal` is the class the M5-T4 acceptance row and e2e read; `.crystal` is the
          // shorter alias the animation table's prose uses. Both sit on every crystal.
          <span
            key={index}
            className="mana-crystal crystal"
            data-filled={index < mana.current ? "true" : "false"}
            data-locked={index >= mana.current && index >= crystals - locked ? "true" : undefined}
          />
        ))}
      </span>
      <span className="mana-text">
        {mana.current}/{mana.max}
      </span>
      {credit !== undefined && (
        <span
          className="mana-credit"
          data-testid={`credit-${side}`}
          data-available={credit.available === undefined ? "none" : String(credit.available)}
          data-used={credit.used === undefined ? "none" : String(credit.used)}
        >
          {credit.available !== undefined && (
            <span>
              +{credit.available} {t("credit")}
            </span>
          )}
          {(credit.owed?.length ?? 0) > 0 && (
            <span>
              {t("owed")} {(credit.owed ?? []).join("·")}
            </span>
          )}
          {credit.used !== undefined && <span>{credit.used ? t("used") : t("unused")}</span>}
        </span>
      )}
    </span>
  );
}

/**
 * A pile: `regionId` is what the M5-T4 animation table moves, `testId` what e2e reads (`e2e/support/testids.ts`). Graveyards and
 * exiles are public (§10.8), as is the viewer's own library (R313, `SideView.ownLibrary` R310; R311 faces, R312 backs): `browse`. Not the opponent's (CLAUDE.md rule 7).
 */
type PileBrowse =
  | { kind: "pile"; title: string; cards: readonly CardView[]; view: PlayerView }
  | { kind: "library"; title: string; library: LibraryView; view: PlayerView; back?: string };

/** R313: what the library's list says of its order. */
const LIBRARY_ORDER = "Order hidden";

/**
 * B5 E11: what the viewer's graveyard pile needs to offer "Play": the highlight (`pile-play-<id>` is
 * in `legal` exactly when `legalActions` lists a play for it) and the board's click.
 */
type PilePlays = { highlight: Highlight; onClick?: BoardProps["onClick"] };

/** The entry's "Play", when the highlight lists one for this card (CLAUDE.md rule 7). */
function playFor(plays: PilePlays | undefined, instanceId: string): CardListEntry["play"] {
  if (plays === undefined) return undefined;
  const testId = testid.pilePlay(instanceId);
  if (!isLegal(plays.highlight, testId)) return undefined;
  return {
    testId,
    glow: glowAttr(plays.highlight, testId) !== undefined,
    onPlay: () => plays.onClick?.({ on: "graveyard", instanceId }),
  };
}

/** A browsable pile's accessible name: "Your graveyard: 2 cards. Show them". */
function pileLabel(title: string, count: number, order: string | undefined): string {
  if (order === undefined) {
    return count === 1 ? t("{title}: {count} card. Show them", { title, count }) : t("{title}: {count} cards. Show them", { title, count });
  }
  const lower = order.toLowerCase();
  return count === 1
    ? t("{title}: {count} card, {order}. Show them", { title, count, order: lower })
    : t("{title}: {count} cards, {order}. Show them", { title, count, order: lower });
}

function browseEntries(
  lookup: CardLookup | null,
  browse: PileBrowse,
  plays?: PilePlays,
): { entries: CardListEntry[]; order?: string } {
  const entries: CardListEntry[] = [];
  if (browse.kind === "library") {
    for (const entry of browse.library.cards) {
      const face = namedFace(lookup, browse.view, { defId: entry.defId, radiant: entry.radiant });
      if (face !== null) {
        if (entry.created === true) face.created = true;
        const created = entry.created === true ? ":created" : "";
        entries.push({ key: `${entry.defId}:${entry.radiant ? "radiant" : "base"}${created}`, face, count: entry.count });
      }
    }
    if (browse.library.unknown > 0) {
      entries.push({ key: "unknown", face: null, count: browse.library.unknown, ...(browse.back === undefined ? {} : { back: browse.back }) });
    }
    return { entries, order: t(LIBRARY_ORDER) };
  }
  // Newest first: a pile grows at its end (the engine appends each card that lands in it).
  for (let at = browse.cards.length - 1; at >= 0; at -= 1) {
    const card = browse.cards[at];
    const face = card === undefined ? null : listedFace(lookup, browse.view, card);
    if (card === undefined || face === null) continue;
    const play = playFor(plays, card.instanceId);
    entries.push(play === undefined ? { key: card.instanceId, face } : { key: card.instanceId, face, play });
  }
  return { entries };
}

function Pile({
  label,
  regionId,
  testId,
  count,
  fatigue,
  animating,
  browse,
  plays,
  back,
  children,
}: {
  label: string;
  regionId: string;
  testId: string;
  count: number;
  fatigue?: number;
  animating?: AnimatingMap;
  browse?: PileBrowse;
  /** B5 E11: the viewer's graveyard offers "Play" on the cards `legalActions` lists, and glows then. */
  plays?: PilePlays;
  /** R1297000: the viewer's card back on their own deck pile; absent where it wears the default. */
  back?: string;
  /** R318: the pile's overflow notice (OverflowNotices.tsx), drawn inside it. */
  children?: ReactNode;
}): ReactElement {
  const lookup = useContext(CatalogContext);
  const hoverPreviews = useSetting("hoverPreviews");
  const { entries, order } = browse === undefined ? { entries: [] } : browseEntries(lookup, browse, plays);
  const title = browse?.title ?? label;
  const browsable = entries.length > 0;
  const orderNote = order === undefined ? {} : { order };
  // R1639000: an empty deck that has dealt fatigue says how much in its label as well as its badge.
  const fatigued = restingFatigue(count, fatigue);
  const named = fatigued === null ? label : t("{label}, Fatigue {count}", { label, count: fatigued });
  const inspect = useInspectTrigger(
    browsable
      ? {
          key: `pile-${regionId}`,
          render: ({ mode, anchor, close }: InspectOverlayState) =>
            mode === "hover" ? (
              <CardListPreview title={title} entries={entries} anchor={anchor} {...orderNote} />
            ) : (
              <CardListSheet title={title} entries={entries} onClose={close} {...orderNote} />
            ),
        }
      : null,
  );

  const open = (event: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>): void => {
    inspect.openSheet(event.currentTarget);
  };

  return (
    <>
      <span
        {...(browsable ? inspect.handlers : {})}
        className={cx("pile", browsable && "pile--browsable")}
        data-card-back={back}
        // The preview replaces the native tooltip; with previews off, the label comes back.
        title={browsable && hoverPreviews ? undefined : named}
        data-testid={regionId}
        data-fatigue={fatigue}
        data-animating={animating?.get(regionId)}
        data-glow={plays === undefined ? undefined : glowAttr(plays.highlight, regionId)}
        data-browsable={browsable ? "true" : undefined}
        role={browsable ? "button" : undefined}
        tabIndex={browsable ? 0 : undefined}
        aria-haspopup={browsable ? "dialog" : undefined}
        aria-label={browsable ? pileLabel(title, count, order) : undefined}
        onClick={browsable ? open : undefined}
        onKeyDown={
          browsable
            ? (event) => {
                if (event.key !== "Enter" && event.key !== " ") return;
                event.preventDefault();
                open(event);
              }
            : undefined
        }
      >
        {/* WCAG 2.5.3 (#757): the shown words read "Deck 17", not "Deck17", so the name holds them;
            a flex box draws no space between its items, so the layout is unchanged. */}
        <span className="pile-label">{label}</span>{" "}
        <span className="pile-n" data-testid={testId}>
          {count}
        </span>
        {children}
      </span>
      {inspect.overlay}
    </>
  );
}

function Seat({
  view,
  side,
  highlight,
  animating,
  onClick,
  onControl,
  pops,
  notices,
  emotes,
}: {
  view: PlayerView;
  side: Side;
  highlight: Highlight;
  animating?: AnimatingMap;
  onClick?: BoardProps["onClick"];
  onControl?: BoardProps["onControl"];
  pops: ReadonlyMap<string, Pops>;
  notices: OverflowNotices;
  emotes?: BoardProps["emotes"];
}): ReactElement {
  const seat = sideView(view, side);
  // R1297000: the viewer's deck wears their card back; R1834004: the other seat's its loadout's, else
  // the default. A default back adds no attribute, so a board wearing nothing is the board it was before.
  const { cardBack } = useSeatDeckCosmetics(seat.player);
  const back = cardBack === DEFAULT_CARD_COSMETICS.cardBack ? undefined : cardBack;
  // R1639000: the empty deck's resting badge, from the view alone; it stands down while R318's notice
  // is on the pile (the board then shows the view from before the burst, whose count is a draw behind).
  const restingCount = notices.pile.get(side) === undefined ? restingFatigue(seat.libraryCount, seat.fatigueCount) : null;
  return (
    <div className={cx("seat", `seat-${side}`)} data-side={side} data-player={seat.player}>
      <Hero
        view={view}
        side={side}
        highlight={highlight}
        animating={animating}
        onClick={onClick}
        onControl={onControl}
        pops={pops.get(testid.hero(side))}
        emotes={emotes?.(side)}
      />
      <ManaTray side={side} mana={seat.mana} credit={seat.credit} animating={animating} />
      {/* R1847000: the seat's starscape map, both seats' once its player has first traveled. */}
      {seat.starscape !== undefined && <StarscapeMap side={side} starscape={seat.starscape} />}
      <span className="piles">
        {/* R373: the rules' library is shown to players as the Deck. */}
        <Pile
          label={t("Deck")}
          regionId={animTestid.library(side)}
          testId={countId("library", side)}
          count={seat.libraryCount}
          fatigue={seat.fatigueCount}
          animating={animating}
          // R310, R313: only the viewer's own side carries the list; the opponent's is a count.
          {...(side === "you" && seat.ownLibrary !== undefined
            ? { browse: { kind: "library" as const, title: t("Your deck"), library: seat.ownLibrary, view, ...(back === undefined ? {} : { back }) } }
            : {})}
          back={back}
        >
          <PileNotice key={notices.pile.get(side)?.entry} notice={notices.pile.get(side)} side={side} view={view} />
          {restingCount !== null && (
            <span className="pile-fatigue" data-testid={noticeTestid.fatigue(side)}>
              {t("Fatigue {count}", { count: restingCount })}
            </span>
          )}
        </Pile>
        <Pile
          label={t("Graveyard")}
          regionId={animTestid.graveyard(side)}
          testId={countId("graveyard", side)}
          count={seat.graveyard.length}
          animating={animating}
          browse={{ kind: "pile", title: side === "you" ? t("Your graveyard") : t("Opponent's graveyard"), cards: seat.graveyard, view }}
          {...(side === "you" ? { plays: { highlight, onClick } } : {})}
        />
        <Pile
          label={t("Exile")}
          regionId={animTestid.exile(side)}
          testId={countId("exile", side)}
          count={seat.exile.length}
          animating={animating}
          browse={{ kind: "pile", title: side === "you" ? t("Your exile") : t("Opponent's exile"), cards: seat.exile, view }}
        />
      </span>
      {/* R98: a card mid-resolution is public and still itself, so it is shown rather than
          vanishing between its play and its graveyard. It is not a click target — `ClickTarget`
          has no member for the resolving zone, and nothing in `legalActions` points at one. */}
      {(seat.resolving ?? []).length > 0 && (
        <span className="resolving" data-testid={`resolving-${side}`}>
          {(seat.resolving ?? []).map((card) => (
            <Card
              key={card.instanceId}
              testId={testid.card(card.instanceId)}
              card={card}
              className="card-resolving"
              highlight={highlight}
              animating={animating}
            />
          ))}
        </span>
      )}
    </div>
  );
}

function ControlButton({
  control,
  testId,
  label,
  highlight,
  animating,
  confirm,
  turnState,
  pressed = false,
  title,
  onPress,
}: {
  control: BoardControl;
  testId: string;
  label: string;
  highlight: Highlight;
  animating?: AnimatingMap;
  /** `"armed"` while End turn is waiting for its confirming second click (B25). */
  confirm?: "armed";
  /** End-turn's physical housing state; this is presentation over engine-provided legality. */
  turnState?: "ready" | "waiting" | "enemy" | "locked";
  /** A dispatched turn-end stays visibly depressed until the next rendered view takes over. */
  pressed?: boolean;
  /** A tooltip, where the label is a short form of it (Offer draw while R36 blocks it). */
  title?: string;
  onPress: () => void;
}): ReactElement {
  const legal = isLegal(highlight, testId);
  return (
    <button
      type="button"
      className={cx("control", `control-${control}`)}
      data-testid={testId}
      title={title}
      data-legal={legalAttr(legal)}
      data-glow={glowAttr(highlight, testId)}
      data-confirm={confirm}
      data-turn-state={turnState}
      data-pressed={pressed ? "true" : undefined}
      data-selected={isSelected(highlight, testId) ? "true" : undefined}
      data-animating={animating?.get(testId)}
      aria-disabled={legal ? undefined : "true"}
      // BUILD M5-T4 `turnEnded` asserts the end-turn button is disabled, so the real attribute
      // goes on alongside `aria-disabled`. Either way an illegal control fires nothing.
      disabled={!legal}
      onClick={() => {
        if (!legal) return;
        onPress();
      }}
    >
      {label}
    </button>
  );
}

export default function Board({
  view,
  highlight = NO_HIGHLIGHT,
  animating,
  animated,
  turnClock,
  matchStatus,
  boardRail,
  boardNotices,
  market,
  sandDisabled = false,
  onClick,
  onControl,
  emotes,
  wearCosmetics = false,
  deckCosmetics = null,
  loadouts = null,
}: BoardProps): ReactElement {
  const pops = popsFrom(view, animating, animated);
  // #1598: the cards losing a keyword, read off the burst as the pops are (keywordLoss.ts).
  const keywordLosses = keywordLossesFrom(view, animating, animated);
  // #2435: the cards the burst has awakened, their Dormant seal broken (awakening.ts).
  const awakenings = awakeningsFrom(view, animating, animated);
  // R318: fatigue and a full library on a library pile, a full hand over a hand, as the pops are.
  const notices = noticesFrom(view, animating, animated);
  const burnNotice = (side: Side): ReactElement => (
    <BurnNotice key={notices.burn.get(side)?.entry} notice={notices.burn.get(side)} side={side} view={view} />
  );
  const yourHand: CardView[] | { count: number } = view.you.hand;
  const dragToPlay = useSetting("dragToPlay");
  const confirmEndTurn = useSetting("confirmEndTurn");
  const field = useRef<HTMLDivElement>(null);
  // "Turn 3": split around the number, so English keeps the two text nodes it always had.
  const [turnBefore = "", turnAfter = ""] = t("Turn {n}").split("{n}");

  // B25: the confirm is armed FOR a view. Any new view (the engine moved, the turn changed, an
  // animation caught up) is a different object, so it disarms without an effect.
  const [armedFor, setArmedFor] = useState<PlayerView | null>(null);
  const [pressedFor, setPressedFor] = useState<PlayerView | null>(null);
  const [logOpen, setLogOpen] = useState(false);
  // #991, R583: the hand card under the pointer, whose preview's ids the field rings.
  const [hovered, setHovered] = useState<string | null>(null);
  const counted = countedTestids(view, highlight, hovered);
  const fieldHighlight: Highlight = counted.size === 0 ? highlight : { ...highlight, counted };
  const actionsRemain = hasMovesLeft(highlight);
  const needsConfirm = confirmEndTurn && actionsRemain;
  const armed = needsConfirm && armedFor === view;
  const pressed = pressedFor === view;
  const endTurnLegal = isLegal(highlight, testid.endTurn);
  const ownTurn = view.result === null && view.active === view.viewer;
  const endTurnState = !ownTurn ? "enemy" : !endTurnLegal ? "locked" : actionsRemain ? "ready" : "waiting";
  const offerDrawText = offerDrawControlText(view);
  // #2645: on a phone the bar holds End turn, the Marketplace's toggle, the log's, ⋯ and the gear, and
  // Offer draw, Concede and Friends are rows of ⋯'s menu (MatchMenu.tsx).
  const phoneBar = usePhoneBar();
  // The menu is the seat's that opened it (a hotseat hand-over closes it); it and the shop are never open together.
  const [menuFor, setMenuFor] = useState<PlayerId | null>(null);
  const [marketOpen, setMarketOpen] = useState(false);
  const menuOpen = menuFor === view.viewer;
  const openMenu = useCallback(
    (open: boolean): void => {
      setMenuFor(open ? view.viewer : null);
      if (open) setMarketOpen(false);
    },
    [view.viewer],
  );
  const openMarket = useCallback((open: boolean): void => {
    setMarketOpen(open);
    if (open) setMenuFor(null);
  }, []);
  // R1297000: the playmat and backdrop the viewer's half wears; R1834004: and the other half, from its
  // seat's loadout. A default adds no attribute and no element. R1833000: an unowned pick of the
  // viewer's own draws as the default.
  const wear: Wear = { seat: wearCosmetics ? view.viewer : null, deck: deckCosmetics, loadouts, viewer: view.viewer };
  const account = useCosmetics();
  const owns = useOwnsAccountCosmetic();
  const { opponentCosmetics } = useCardSettings();
  const slots = seatDeckCosmetics(view.you.player, wear, account, opponentCosmetics, owns);
  const theirSlots = seatDeckCosmetics(view.opponent.player, wear, account, opponentCosmetics, owns);
  const backdrop = slots.field === DEFAULT_CARD_COSMETICS.field ? undefined : slots.field;
  const table = slots.table === DEFAULT_CARD_COSMETICS.table ? undefined : slots.table;
  const theirBackdrop = theirSlots.field === DEFAULT_CARD_COSMETICS.field ? undefined : theirSlots.field;
  const theirTable = theirSlots.table === DEFAULT_CARD_COSMETICS.table ? undefined : theirSlots.table;

  const offerDraw = (
    <ControlButton
      control="offer-draw"
      testId={testid.offerDraw}
      label={offerDrawText.label}
      title={offerDrawText.title}
      highlight={highlight}
      animating={animating}
      onPress={() => onControl?.("offer-draw")}
    />
  );
  const concede = (
    <ControlButton
      control="concede"
      testId={testid.concede}
      label={t("Concede")}
      highlight={highlight}
      animating={animating}
      onPress={() => onControl?.("concede")}
    />
  );

  function pressEndTurn(): void {
    if (needsConfirm && !armed) {
      setArmedFor(view);
      return;
    }
    setArmedFor(null);
    setPressedFor(view);
    getAudioEngine().playSfx("endTurn");
    onControl?.("end-turn");
  }

  // For every card drawn below: R243's match-made definitions and field powers, #258's acting seat,
  // R1295000's wearer, R1297000's deck picks, R1834004's loadouts, #1598's keyword losses and #2435's
  // awakenings.
  return (
    <MatchCardsProvider view={view}>
      <ActingProvider view={view}>
      <WearerProvider seat={wear.seat} deck={deckCosmetics} loadouts={loadouts} viewer={view.viewer}>
      <KeywordLossProvider lost={keywordLosses}>
      <AwakeningProvider awake={awakenings}>
      <div
        className="board"
        data-testid={testid.board}
        data-phase={view.phase}
        data-turn={view.turn}
        data-active={sideOf(view, view.active)}
        data-viewer={view.viewer}
        data-backdrop={backdrop}
        data-their-backdrop={theirBackdrop}
        data-drag={dragToPlay ? "on" : "off"}
        data-log={logOpen ? "open" : undefined}
        data-random-targets={view.randomTargets === true ? "" : undefined}
      >
        {backdrop === undefined ? null : <Backdrop field={backdrop} />}
        {theirBackdrop === undefined ? null : <Backdrop field={theirBackdrop} half="theirs" />}
        <Seat
          view={view}
          side="opponent"
          highlight={highlight}
          animating={animating}
          onClick={onClick}
          onControl={onControl}
          pops={pops}
          notices={notices}
          emotes={emotes}
        />
        {/* R434: at the game's end the view shows the opponent's hand, and the row turns it face up. */}
        <Hand side="opponent" seat={view.opponent.player} hand={revealedOpponentHand(view) ?? view.opponent.hand} cap={view.opponent.handCap} marked={view.opponent.handMarked} highlight={highlight} animating={animating} onClick={onClick} notice={burnNotice("opponent")} />

        <div className="field" aria-label={t("Field")} data-table={table} data-their-table={theirTable} ref={field}>
          {theirTable === undefined ? null : <span className="field-their-mat" aria-hidden="true" />}
          <SandSurface field={field} disabled={sandDisabled} table={table} theirTable={theirTable} />
          {LANES.map((lane) => (
            <div className="lane" key={lane} data-lane={lane}>
              {FIELD_ROWS.map((slot) => (
                <Zone
                  key={`${slot.side}-${slot.row}`}
                  view={view}
                  side={slot.side}
                  row={slot.row}
                  lane={lane}
                  highlight={fieldHighlight}
                  animating={animating}
                  onClick={onClick}
                  pops={pops}
                />
              ))}
            </div>
          ))}
        </div>

        <Seat view={view} side="you" highlight={highlight} animating={animating} onClick={onClick} onControl={onControl} pops={pops} notices={notices} emotes={emotes} />
        <Hand side="you" seat={view.you.player} hand={yourHand} cap={view.you.handCap} view={view} highlight={highlight} animating={animating} onClick={onClick} onHover={setHovered} notice={burnNotice("you")} />

        <div className="control-bar" aria-label={t("Controls")}>
          {boardRail === undefined ? null : <div className="board-rail">{boardRail}</div>}
          {boardNotices === undefined ? null : <div className="board-notices">{boardNotices}</div>}
          {/* Whose turn, above End turn wherever the controls have a column of their own (board.css
              hides it on a phone held upright, where the shell's banner says it). The banner is the
              live region, so this copy stays out of the accessibility tree. */}
          <div
            className="turn-plate"
            data-side={view.result !== null ? "over" : sideOf(view, view.active)}
            aria-hidden="true"
          >
            <span className="turn-plate-number">
              {turnBefore}
              {view.turn}
              {turnAfter}
            </span>
            <span className="turn-plate-whose">
              {view.result !== null
                ? t("Game over")
                : view.phase === "mulligan"
                  ? t("Mulligan")
                  : view.active === view.viewer
                    ? t("Your turn")
                    : t("Opponent's turn")}
            </span>
          </div>
          {matchStatus === undefined ? null : <div className="board-status">{matchStatus}</div>}
          {/* R1200: while a Mayor acts every target is drawn at random, so the board says so. */}
          {view.randomTargets === true ? (
            <div className="random-targets" data-testid="random-targets" role="status">
              {t("Targets are random")}
            </div>
          ) : null}
          <div className="turn-mechanism" data-turn-state={endTurnState}>
            <ControlButton
              control="end-turn"
              testId={testid.endTurn}
              label={endTurnState === "enemy" ? t("Enemy turn") : armed ? t("Confirm end turn") : t("End turn")}
              highlight={highlight}
              animating={animating}
              confirm={armed ? "armed" : undefined}
              turnState={endTurnState}
              pressed={pressed}
              onPress={pressEndTurn}
            />
            {turnClock ?? (view.clockMs !== null && <span className="clock turn-mechanism-clock">{formatSeconds(Math.ceil(view.clockMs / 1000))}</span>)}
          </div>
          {market === undefined ? null : (
            <Marketplace view={view} legal={market.legal} onAction={market.onAction} open={marketOpen} onOpenChange={openMarket} />
          )}
          {phoneBar ? null : offerDraw}
          {phoneBar ? null : concede}
          {/* Phones only (board.css): the log's own place on the board is hidden there. */}
          <button
            type="button"
            className="log-toggle"
            data-testid="log-toggle"
            aria-expanded={logOpen}
            aria-label={logOpen ? t("Hide the game log") : t("Show the game log")}
            title={t("Game log")}
            onClick={() => setLogOpen((open) => !open)}
          >
            <span className="log-toggle-icon" aria-hidden="true" />
          </button>
          {phoneBar ? (
            <MatchMenu
              open={menuOpen}
              onOpenChange={openMenu}
              dot={offerDrawText.title !== undefined}
              glow={glowAttr(highlight, testid.offerDraw) ?? glowAttr(highlight, testid.concede)}
              animating={animating?.get(testid.offerDraw) ?? animating?.get(testid.concede)}
            >
              {offerDraw}
              {concede}
              <FriendsButton placement="menu" />
            </MatchMenu>
          ) : (
            <>
              {/* Task 2's mute, beside the gear that holds the rest of its controls: a fixed corner button
                  sat on the practice HUD's Menu and the match bar's clock (integration). */}
              <AudioToggle className="audio-toggle--bar" />
              {/* #1831: friends, and chat with one, from the board, beside the language button and the gear. */}
              <FriendsButton placement="game" />
              <LanguageButton placement="game" />
            </>
          )}
          <SettingsButton placement="game" />
        </div>

        <Log view={view} revealed={logOpen} />
      </div>
      </AwakeningProvider>
      </KeywordLossProvider>
      </WearerProvider>
      </ActingProvider>
    </MatchCardsProvider>
  );
}
