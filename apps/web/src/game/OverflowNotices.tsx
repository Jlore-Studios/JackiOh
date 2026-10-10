// Notices for §2.4's overflows (SPEC §10.10, R318) use the runner's entries: the newest wins per pile or hand.
// #33 Unstable Clone Machine is the library-overflow fixture.
// `data-playing="true"` follows its entry; reduced motion starts none.
// No rule lives here (CLAUDE.md rule 7); only redacted event data is read (R97, R202), showing a back for the sentinel.

import { useContext, type ReactElement } from "react";

import type { GameEvent, GameEventType, LibraryOverflowOutcome, PlayerView } from "@jackioh/shared";

import { CardBack, CardFace } from "../cards/index.ts";
import { animTestid } from "./animations.ts";
import { CatalogContext } from "./catalog.ts";
import { sideOf, type AnimatingMap, type AnimationFrames, type Side } from "./contract.ts";
import { HIDDEN_CARD, namedFace } from "./faces.ts";
import "./overflow.css";

const NOTICE_EVENTS: readonly GameEventType[] = ["fatigue", "libraryOverflow", "burned"];

/** Event card data preserves the R97 sentinel; R316 radiant status accompanies viewer-readable cards. */
export type NoticeCard = { instanceId: string; defId: string; radiant?: boolean };

export type PileNoticeModel =
  | { kind: "fatigue"; count: number; playing: boolean; entry: number }
  | { kind: "libraryFull"; card: NoticeCard; outcome: LibraryOverflowOutcome; playing: boolean; entry: number };

export type BurnNoticeModel = { card: NoticeCard; playing: boolean; entry: number };

export type OverflowNotices = {
  pile: ReadonlyMap<Side, PileNoticeModel>;
  burn: ReadonlyMap<Side, BurnNoticeModel>;
};

export const NO_NOTICES: OverflowNotices = { pile: new Map(), burn: new Map() };

/** e2e contract (R318). */
export const noticeTestid = {
  pile: (side: Side): string => `pile-notice-${side}`,
  overflowCard: (side: Side): string => `overflow-card-${side}`,
  burn: (side: Side): string => `burn-notice-${side}`,
  burnCard: (side: Side): string => `burn-card-${side}`,
} as const;

function regionOf(view: PlayerView, event: GameEvent): { side: Side; region: string } | null {
  if (event.type === "fatigue" || event.type === "libraryOverflow") {
    const side = sideOf(view, event.player);
    return { side, region: animTestid.library(side) };
  }
  if (event.type === "burned") {
    const side = sideOf(view, event.owner);
    return { side, region: animTestid.hand(side) };
  }
  return null;
}

export function noticesFrom(
  view: PlayerView,
  animating: AnimatingMap | undefined,
  animated?: readonly AnimationFrames[],
): OverflowNotices {
  const sources: readonly AnimationFrames[] =
    animated ?? (animating === undefined ? [] : [{ frames: animating, events: view.events }]);
  if (sources.length === 0) return NO_NOTICES;
  const pile = new Map<Side, PileNoticeModel>();
  const burn = new Map<Side, BurnNoticeModel>();
  const last = sources.length - 1;
  sources.forEach((source, index) => {
    for (const [region, type] of source.frames) {
      if (!NOTICE_EVENTS.includes(type)) continue;
      // Last matching event; the runner animates in order.
      const event = [...source.events].reverse().find((e) => e.type === type && regionOf(view, e)?.region === region);
      if (event === undefined) continue;
      const playing =
        animating !== undefined && (source.frames === animating || (index === last && animating.get(region) === type));
      const at = regionOf(view, event);
      if (at === null) continue;
      const entry = index;
      if (event.type === "fatigue") pile.set(at.side, { kind: "fatigue", count: event.count, playing, entry });
      if (event.type === "libraryOverflow") {
        pile.set(at.side, {
          kind: "libraryFull",
          card: { instanceId: event.instanceId, defId: event.defId, ...(event.radiant === true ? { radiant: true } : {}) },
          outcome: event.outcome,
          playing,
          entry,
        });
      }
      if (event.type === "burned") burn.set(at.side, { card: { instanceId: event.instanceId, defId: event.defId }, playing, entry });
    }
  });
  return pile.size === 0 && burn.size === 0 ? NO_NOTICES : { pile, burn };
}

/** R97: render a back for the hidden-card sentinel. */
function NoticeCardFace({
  card,
  view,
  className,
  testId,
  outcome,
  as: Tag = "span",
}: {
  card: NoticeCard;
  view: PlayerView;
  className: string;
  testId: string;
  outcome?: LibraryOverflowOutcome;
  as?: "span" | "div";
}): ReactElement {
  const lookup = useContext(CatalogContext);
  const face =
    card.defId === HIDDEN_CARD
      ? null
      : namedFace(lookup, view, {
          defId: card.defId,
          radiant: card.radiant === true,
          ...(card.instanceId === HIDDEN_CARD ? {} : { instanceId: card.instanceId }),
        });
  return (
    <Tag className={className} data-testid={testId} data-face={face === null ? "back" : "face"} data-outcome={outcome}>
      {face === null ? <CardBack /> : <CardFace face={face} layout="compact" />}
    </Tag>
  );
}

/** R373: deck-pile notice. */
export function PileNotice({
  notice,
  side,
  view,
}: {
  notice: PileNoticeModel | undefined;
  side: Side;
  view: PlayerView;
}): ReactElement | null {
  if (notice === undefined) return null;
  const playing = notice.playing ? "true" : undefined;
  if (notice.kind === "fatigue") {
    return (
      <span className="pile-notice" data-testid={noticeTestid.pile(side)} data-kind="fatigue" data-side={side} data-playing={playing}>
        <span className="pile-notice-tag">Fatigue {notice.count}</span>
      </span>
    );
  }
  return (
    <span
      className="pile-notice"
      data-testid={noticeTestid.pile(side)}
      data-kind="libraryFull"
      data-side={side}
      data-outcome={notice.outcome}
      data-playing={playing}
    >
      <span className="pile-notice-tag">Deck full</span>
      <NoticeCardFace
        card={notice.card}
        view={view}
        className="overflow-card"
        testId={noticeTestid.overflowCard(side)}
        outcome={notice.outcome}
      />
    </span>
  );
}

export function BurnNotice({
  notice,
  side,
  view,
}: {
  notice: BurnNoticeModel | undefined;
  side: Side;
  view: PlayerView;
}): ReactElement | null {
  if (notice === undefined) return null;
  return (
    <div className="burn-notice" data-testid={noticeTestid.burn(side)} data-side={side} data-playing={notice.playing ? "true" : undefined}>
      <span className="burn-tag">Hand full</span>
      <NoticeCardFace card={notice.card} view={view} className="burn-card" testId={noticeTestid.burnCard(side)} as="div" />
    </div>
  );
}
