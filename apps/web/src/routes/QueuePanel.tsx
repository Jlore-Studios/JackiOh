// The Online cards' one panel (#3729, R3729000): the rank of the mode's ladder, the deck or trio, its verdict and Find
// a match, in a dialog over the hub, where three numbered steps stood. Verdicts are UX only (CLAUDE.md rule 7): the
// server validates and matches. It decides nothing: `routes/play.tsx` owns the picks and the queue.

import type { ReactElement, ReactNode } from "react";

import { DECK_SIZE } from "@jackioh/engine/config";
import type { LoadoutResult } from "@jackioh/validator";
import type { LeanPick } from "@jackioh/shared";

import { EmptyState } from "../empty/EmptyState.tsx";
import { Dialog } from "../game/Dialog.tsx";
import { LeanSetPicker } from "../game/LeanNewest.tsx";
import { t } from "../i18n/i18n.ts";
import { ladderPath, type Ladder, type OwnRankResponse, type QueueMode, type SavedDeck, type SavedTrio } from "../net/api.ts";
import { MODE_LABEL } from "../net/liveGame.ts";
import { paths } from "../net/navigate.ts";
import { rankCaption } from "../rank/rank.ts";
import RankCrest from "../rank/RankCrest.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { ArrowIcon } from "./ArrowIcon.tsx";
import { followInApp } from "./nav.tsx";
import { playTestid } from "./playTestids.ts";

export function Verdict({ result }: { result: LoadoutResult | null }): ReactElement | null {
  if (result === null) return null;
  if (result.ok) {
    return (
      <p className="lobby-verdict" data-testid={playTestid.verdict} data-ready="true">
        <span className="lobby-verdict__mark" aria-hidden="true">
          ✓
        </span>
        {t("Ready to queue")}
      </p>
    );
  }
  return (
    <div className="lobby-verdict" data-testid={playTestid.verdict} data-ready="false">
      <p className="lobby-verdict__lead">{t("Not ready yet. The server checks again when you queue:")}</p>
      <ul>
        {result.errors.map((error, index) => (
          // The same message can repeat for two cards; the index keeps the keys apart.
          <li key={`${String(index)}-${error.message}`}>{t(error.message)}</li>
        ))}
      </ul>
    </div>
  );
}

function DeckSummary({ deck }: { deck: SavedDeck }): ReactElement {
  const full = deck.cards.length === DECK_SIZE;
  return (
    <p className="play-pick-summary" data-full={full ? "true" : "false"}>
      {t("{count} / {size} cards", { count: String(deck.cards.length), size: String(DECK_SIZE) })}
    </p>
  );
}

function TrioSummary({ trio, decks }: { trio: SavedTrio; decks: readonly SavedDeck[] }): ReactElement {
  return (
    <ul className="play-trio-decks" aria-label={t("Decks in {name}", { name: trio.name })}>
      {trio.deckIds.map((id, slot) => {
        const deck = id === null ? undefined : decks.find((saved) => saved.id === id);
        return (
          // Slot identity distinguishes duplicate empty entries.
          <li key={`${String(slot)}-${id ?? "empty"}`} className="play-trio-deck" data-empty={deck === undefined}>
            {deck === undefined ? t("Empty slot") : deck.name}
          </li>
        );
      })}
    </ul>
  );
}

/** R2778000: the rank of the ladder the mode plays for, and a link to that ladder's board. */
function RankPanel({ rank, ladder }: { rank: OwnRankResponse | null; ladder: Ladder }): ReactElement {
  const board = ladderPath(paths.leaderboard, ladder);
  return (
    <section className="play-panel play-panel--rank" aria-labelledby="play-rank-heading">
      <h2 id="play-rank-heading" className="play-panel__heading">
        {t("Your rank")}
      </h2>
      {rank === null ? null : (
        <p data-testid={playTestid.rank} data-ladder={ladder}>
          {t(ladder === "random" ? "All random" : "Constructed")} · <RankCrest rank={rank.rank} size="large" />{" "}
          {t("{rank} · Season {season}", { rank: rankCaption(rank.rank), season: rank.season })}
        </p>
      )}
      <a href={board} data-testid={playTestid.leaderboard} onClick={followInApp(board)}>
        {t("Leaderboard")}
        <ArrowIcon direction="forward" />
      </a>
    </section>
  );
}

/** What the picks read: the decks while they load, a failed read, or the saved decks and the ones shown. */
export type QueuePicks =
  | { kind: "loading" }
  | { kind: "failed"; message: string; retry: () => void }
  | { kind: "ready"; decks: readonly SavedDeck[]; trios: readonly SavedTrio[]; deck: SavedDeck | null; trio: SavedTrio | null };

export type QueuePanelProps = {
  mode: QueueMode;
  ladder: Ladder;
  rank: OwnRankResponse | null;
  picks: QueuePicks;
  lean: LeanPick | null;
  verdict: LoadoutResult | null;
  /** A wait or a pairing in hand: nothing here can change. */
  locked: boolean;
  /** Find a match is out. */
  busy: boolean;
  /** The mode's required pick is missing. */
  noChoice: boolean;
  /** The server's refusal, drawn above the picks. */
  notice: ReactNode;
  onDeck(id: string): void;
  onTrio(id: string): void;
  onLean(pick: LeanPick | null): void;
  onFind(): void;
  onClose(): void;
};

export function QueuePanel({
  mode,
  ladder,
  rank,
  picks,
  lean,
  verdict,
  locked,
  busy,
  noChoice,
  notice,
  onDeck,
  onTrio,
  onLean,
  onFind,
  onClose,
}: QueuePanelProps): ReactElement {
  const ready = picks.kind === "ready" ? picks : null;
  return (
    <Dialog
      role="dialog"
      sheet
      title={t(MODE_LABEL[mode])}
      testid={playTestid.queuePanel}
      className="play-queue-panel"
      data={{ "data-mode": mode }}
      onClose={onClose}
      actions={[
        {
          label: t("Find a match"),
          kind: "primary",
          testid: playTestid.queue,
          onClick: onFind,
          disabled: busy || noChoice || locked,
          busy,
        },
      ]}
    >
      <RankPanel rank={rank} ladder={ladder} />
      <h3 className="play-panel__heading play-panel__heading--sub">
        {mode === "bo3" ? t("Bring a trio") : mode === "bo1" ? t("Bring a deck") : t("No deck to bring")}
      </h3>

      {picks.kind === "loading" && mode !== "random" ? <Skeleton variant="panel" label={t("Loading your decks…")} /> : null}
      {picks.kind === "failed" && mode !== "random" ? (
        <>
          <p className="notice" role="alert">
            {t("Your decks could not be loaded: {message}", { message: t(picks.message) })}
          </p>
          <div className="row">
            <button type="button" data-testid={playTestid.decksRetry} onClick={picks.retry}>
              {t("Try again")}
            </button>
          </div>
        </>
      ) : null}

      {ready !== null && mode === "bo1" ? (
        ready.decks.length === 0 ? (
          <EmptyState
            text={t("You have no saved decks yet.")}
            action={{ label: t("Build one in Decks"), to: paths.decks, testId: playTestid.decksLink }}
          />
        ) : (
          <div className="play-pick">
            <label htmlFor="play-deck">{t("Your deck")}</label>
            <select
              id="play-deck"
              className="lobby-select"
              data-testid={playTestid.deckSelect}
              value={ready.deck?.id ?? ""}
              disabled={locked}
              onChange={(event) => {
                onDeck(event.target.value);
              }}
            >
              {ready.decks.map((saved) => (
                <option key={saved.id} value={saved.id}>
                  {saved.name}
                </option>
              ))}
            </select>
            {ready.deck === null ? null : <DeckSummary deck={ready.deck} />}
          </div>
        )
      ) : null}

      {ready !== null && mode === "bo3" ? (
        ready.trios.length === 0 ? (
          <EmptyState
            text={t("You have no saved trios yet.")}
            action={{ label: t("Build one in Decks"), to: paths.decks, testId: playTestid.decksLink }}
          />
        ) : (
          <div className="play-pick">
            <label htmlFor="play-trio">{t("Your trio")}</label>
            <select
              id="play-trio"
              className="lobby-select"
              data-testid={playTestid.trioSelect}
              value={ready.trio?.id ?? ""}
              disabled={locked}
              onChange={(event) => {
                onTrio(event.target.value);
              }}
            >
              {ready.trios.map((saved) => (
                <option key={saved.id} value={saved.id}>
                  {saved.name}
                </option>
              ))}
            </select>
            {ready.trio === null ? null : <TrioSummary trio={ready.trio} decks={ready.decks} />}
          </div>
        )
      ) : null}

      {mode === "random" ? (
        <>
          <p className="lobby-note">{t("No deck needed: the server deals both of you one when the game starts.")}</p>
          <LeanSetPicker value={lean} disabled={locked} testid={playTestid.leanNewest} onChange={onLean} />
        </>
      ) : null}

      <Verdict result={verdict} />
      {notice}
    </Dialog>
  );
}
