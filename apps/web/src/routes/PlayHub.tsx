// The Play hub (#3729, R3729000, SPEC §9.5): every way to play as a card in one of four groups, so a new player can
// tell at a glance what each is, how long a game takes and what it needs. The words are `playModes.ts`'s; a card
// decides nothing (CLAUDE.md rule 7): an Online card opens its panel, a link card is a link, the server
// matches and the engine plays.

import { useId, type ReactElement } from "react";

import { DailyBadge } from "../daily/DailyBadge.tsx";
import { t } from "../i18n/i18n.ts";
import { ladderOfMode, type Ladder, type OwnRankResponse, type PopulationResponse, type QueueMode } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { waitWords } from "../net/queueReadout.ts";
import { rankCaption } from "../rank/rank.ts";
import RankCrest from "../rank/RankCrest.tsx";
import { followInApp } from "./nav.tsx";
import { PLAY_GROUPS, groupLabel, type PlayCard, type PlayCardId, type PlayGroup } from "./playModes.ts";
import { playCardRankTestid, playGroupTestid, playTestid, playWaitTestid } from "./playTestids.ts";

export type PlayHubProps = {
  cards: readonly PlayCard[];
  /** The mode the player used last, which carries a mark; none for a player who has not picked one. */
  lastMode: QueueMode | null;
  /** Both ladders' ranks, as the reads answered; null where one has not (or cannot be read). */
  ranks: Readonly<Record<Ladder, OwnRankResponse | null>>;
  expectedWait: PopulationResponse["expectedWaitSeconds"] | undefined;
  /** #3727: the daily games waiting on the player. */
  dailyWaiting: number;
  /** A wait or a pairing in hand: what would start another game waits. */
  locked: boolean;
  onPick(mode: QueueMode): void;
  onLobby(): void;
};

const ICON_PROPS = { viewBox: "0 0 32 32", "aria-hidden": true, className: "play-card__icon" } as const;

/** One emblem a card wears, in the brass the hub's tiles have always had. */
function CardIcon({ id }: { id: PlayCardId }): ReactElement {
  switch (id) {
    case "bo1":
      return (
        <svg {...ICON_PROPS}>
          <path d="M22 4h6v6L14 24l-6-6z" />
          <path d="M9 19l4 4-3 3-1.5-1.5L5 28l-1-1 3.5-3.5L6 22z" />
        </svg>
      );
    case "bo3":
      return (
        <svg {...ICON_PROPS}>
          <path d="M4 7l6-2 6 2v6c0 4-2.6 6.7-6 8-3.4-1.3-6-4-6-8z" />
          <path d="M16 11l6-2 6 2v6c0 4-2.6 6.7-6 8-3.4-1.3-6-4-6-8z" />
          <path d="M10 15l6-2 6 2v6c0 4-2.6 6.7-6 8-3.4-1.3-6-4-6-8z" />
        </svg>
      );
    case "random":
      return (
        <svg {...ICON_PROPS}>
          <rect x="5" y="5" width="22" height="22" rx="5" />
          <circle cx="11" cy="11" r="2" className="play-card__pip" />
          <circle cx="16" cy="16" r="2" className="play-card__pip" />
          <circle cx="21" cy="21" r="2" className="play-card__pip" />
        </svg>
      );
    case "daily":
      return (
        <svg {...ICON_PROPS}>
          <rect x="4" y="7" width="24" height="21" rx="3" />
          <path d="M4 13h24M10 4v6M22 4v6" />
          <circle cx="16" cy="20" r="2" className="play-card__pip" />
        </svg>
      );
    case "lobby":
      return (
        <svg {...ICON_PROPS}>
          <path d="M6 28V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v22z" />
          <path d="M3 28h26" />
          <circle cx="19" cy="17" r="1.6" className="play-card__pip" />
        </svg>
      );
    case "challenge":
      return (
        <svg {...ICON_PROPS}>
          <path d="M6 5l12 12M26 5L14 17" />
          <path d="M16 16l-3 3 6 6 3-3z" />
          <path d="M6 5h5v1.5M26 5h-5v1.5" />
        </svg>
      );
    case "practice":
      return (
        <svg {...ICON_PROPS}>
          <circle cx="16" cy="16" r="11" />
          <circle cx="16" cy="16" r="6" />
          <circle cx="16" cy="16" r="1.8" className="play-card__pip" />
        </svg>
      );
    case "dungeon":
      return (
        <svg {...ICON_PROPS}>
          <path d="M6 28V13a10 10 0 0 1 20 0v15z" />
          <path d="M12 28V16a4 4 0 0 1 8 0v12" />
        </svg>
      );
    case "tutorial":
      return (
        <svg {...ICON_PROPS}>
          <path d="M16 8C13 5.5 8 5 4 6v19c4-1 9-.5 12 2 3-2.5 8-3 12-2V6c-4-1-9-.5-12 2z" />
          <path d="M16 8v19" />
        </svg>
      );
  }
}

function GroupSection({
  group,
  cards,
  children,
}: {
  group: PlayGroup;
  cards: readonly PlayCard[];
  children: (card: PlayCard) => ReactElement;
}): ReactElement {
  const headingId = useId();
  return (
    <section className="play-group" data-group={group} data-testid={playGroupTestid(group)} aria-labelledby={headingId}>
      <h2 id={headingId} className="play-group__heading">
        {groupLabel(group)}
      </h2>
      <ul className="play-group__cards">
        {cards.map((card) => (
          <li key={card.id}>{children(card)}</li>
        ))}
      </ul>
    </section>
  );
}

export function PlayHub({
  cards,
  lastMode,
  ranks,
  expectedWait,
  dailyWaiting,
  locked,
  onPick,
  onLobby,
}: PlayHubProps): ReactElement {
  /** What every card says of itself: its words, and for an Online card its rank and wait. */
  const body = (card: PlayCard): ReactElement => {
    const mode = card.action.kind === "queue" ? card.action.mode : null;
    const rank = mode === null ? null : ranks[ladderOfMode(mode)];
    const wait = mode === null ? null : (expectedWait?.[mode] ?? null);
    return (
      <>
        <CardIcon id={card.id} />
        <span className="play-card__text">
          <span className="play-card__name">
            {card.name}
            {card.id === "daily" ? <DailyBadge count={dailyWaiting} testid={playTestid.dailyBadge} /> : null}
            {mode !== null && mode === lastMode ? (
              <span className="play-card__last" data-testid={playTestid.lastUsed}>
                {t("Last used")}
              </span>
            ) : null}
          </span>
          <span className="play-card__line">{card.line}</span>
          <span className="play-card__facts">
            <span className="play-card__fact" data-field="time">
              {card.time}
            </span>
            <span className="play-card__fact" data-field="needs">
              {card.needs}
            </span>
            {mode === null || rank === null ? null : (
              <span className="play-card__fact play-card__rank" data-testid={playCardRankTestid(mode)}>
                <RankCrest rank={rank.rank} size="small" /> {rankCaption(rank.rank)}
              </span>
            )}
            {mode === null || wait === null ? null : (
              <span className="play-card__fact play-card__waiting" data-testid={playWaitTestid(mode)}>
                {waitWords(wait)}
              </span>
            )}
          </span>
        </span>
      </>
    );
  };

  const draw = (card: PlayCard): ReactElement => {
    const { action } = card;
    if (action.kind === "link") {
      return (
        <a className="play-card" href={action.href} data-testid={card.testid} data-card={card.id} onClick={followInApp(action.href)}>
          {body(card)}
        </a>
      );
    }
    const mode = action.kind === "queue" ? action.mode : null;
    return (
      <button
        type="button"
        className="play-card"
        data-testid={card.testid}
        data-card={card.id}
        data-mode={mode ?? undefined}
        data-last={mode !== null && mode === lastMode ? "true" : undefined}
        aria-haspopup="dialog"
        disabled={locked}
        onClick={() => {
          if (action.kind === "queue") onPick(action.mode);
          else onLobby();
        }}
      >
        {body(card)}
      </button>
    );
  };

  return (
    <div className="play-cards" data-testid={playTestid.hub}>
      {PLAY_GROUPS.map((group) => {
        const inGroup = cards.filter((card) => card.group === group);
        return inGroup.length === 0 ? null : (
          <GroupSection key={group} group={group} cards={inGroup}>
            {draw}
          </GroupSection>
        );
      })}
      <nav className="play-more" data-testid={playTestid.more} aria-label={t("More ways to play")}>
        <a href={paths.replays} data-testid={playTestid.replays} onClick={followInApp(paths.replays)}>
          {t("Replays")}
        </a>
        <a href={paths.together} data-testid={playTestid.together} onClick={followInApp(paths.together)}>
          {t("Play together")}
        </a>
      </nav>
    </div>
  );
}
