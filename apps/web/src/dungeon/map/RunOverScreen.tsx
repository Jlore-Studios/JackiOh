// The dungeon run's end (DR9, DR14 part F, #2707): the score with the run's time and turns beside it
// (§15.10), the Embers it pays, its seed (#3501), what the run extracted, the run's stats, and the ways out, a new run or the statistics page. Players read
// "relic"; the code keeps the run's word, artifact (WRITING.md's jargon). Presentation only
// (CLAUDE.md rule 7): the numbers come from part C's bindings, and every word goes through `t()`
// at render, never at module scope.

import type { ReactElement } from "react";

import type { RunContent, RunState, RunStats } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { paths } from "../../net/navigate.ts";
import { followInApp } from "../../routes/nav.tsx";
import { runEmbers, runScore } from "../../wasm/index.ts";
import { formatRunTime } from "../fight/RunTimer.tsx";
import type { Dungeon } from "../store.ts";
import { dungeonMapTestid } from "./testids.ts";
import { cardName, PassiveChoice } from "./RunDisplay.tsx";

/**
 * Every `RunStats` key's English label, stored untranslated and passed through `t()` while
 * rendering, never at module scope.
 */
export const RUN_STAT_LABELS: Readonly<Record<keyof RunStats, string>> = {
  nodesVisited: "Nodes visited",
  fightsWon: "Fights won",
  elitesWon: "Elites won",
  ghostsWon: "Ghosts won",
  miniBossesWon: "Mini-bosses won",
  bossesWon: "Bosses won",
  damageDealt: "Damage dealt",
  healingDone: "Healing done",
  cardsDrawn: "Cards drawn",
  turns: "Turns played",
  damageTaken: "Damage taken",
  healed: "Healed",
  coinsEarned: "Coins earned",
  coinsSpent: "Coins spent",
  ticketsEarned: "Tickets earned",
  ticketsSpent: "Tickets spent",
  cardsKept: "Cards kept",
  cardsBought: "Cards bought",
  cardsRemoved: "Cards removed",
  cardsMadeRadiant: "Cards made Radiant",
  cardsBuffed: "Cards buffed",
  packsSkipped: "Packs skipped",
  artifactsTaken: "Relics taken",
  artifactsLeveled: "Relics leveled",
  blessingsTaken: "Blessings taken",
  eventsSeen: "Events seen",
  chestsOpened: "Chests opened",
};

/** Every stat's key, in the order the screen lists them. */
const STAT_KEYS = Object.keys(RUN_STAT_LABELS) as (keyof RunStats)[];

type RunOverScreenProps = {
  dungeon: Dungeon;
  run: RunState;
  content: RunContent;
  /** Whether the run was won; dead otherwise. */
  won: boolean;
};

/** The run's end: its score, its pay, its extracts, its stats, and its exits. */
export function RunOverScreen({ dungeon, run, content, won }: RunOverScreenProps): ReactElement {
  const score = runScore(run, content);
  const embers = runEmbers(score);
  return (
    <section className="dungeon-run-over" data-testid={dungeonMapTestid.runOver}>
      <h2 className="dungeon-run-title">{won ? t("Victory") : t("Defeat")}</h2>
      <p className="dungeon-run-end__score" data-testid={dungeonMapTestid.endScore}>
        {t("Score: {score}", { score: String(score) })}
      </p>
      {dungeon.timeMs === null ? null : (
        <p className="dungeon-run-end__time" data-testid={dungeonMapTestid.endTime}>
          {t("Time: {time}", { time: formatRunTime(dungeon.timeMs) })}
        </p>
      )}
      <p className="dungeon-run-end__turns" data-testid={dungeonMapTestid.endTurns}>
        {t("Turns: {turns}", { turns: String(run.stats.turns) })}
      </p>
      <p className="dungeon-run-end__embers" data-testid={dungeonMapTestid.endEmbers}>
        {t("Embers: {embers}", { embers: String(embers) })}
      </p>
      <p className="dungeon-run-end__seed" data-testid={dungeonMapTestid.endSeed}>
        {t("Seed: {seed}", { seed: run.seed })}
      </p>
      <h3 className="dungeon-run-section__title">{t("Cards extracted")}</h3>
      {run.extractedCards.length === 0 ? (
        <p className="dungeon-run-note" data-testid={dungeonMapTestid.extractedCards}>
          {t("No cards extracted.")}
        </p>
      ) : (
        <ul className="dungeon-run-choices" data-testid={dungeonMapTestid.extractedCards}>
          {run.extractedCards.map((id, index) => (
            <li key={`${String(index)}-${id}`}>{cardName(content, id)}</li>
          ))}
        </ul>
      )}
      <h3 className="dungeon-run-section__title">{t("Relics extracted")}</h3>
      {run.extractedArtifacts.length === 0 ? (
        <p className="dungeon-run-note" data-testid={dungeonMapTestid.extractedArtifacts}>
          {t("No relics extracted.")}
        </p>
      ) : (
        <ul className="dungeon-run-choices" data-testid={dungeonMapTestid.extractedArtifacts}>
          {run.extractedArtifacts.map((id, index) => (
            <li key={`${String(index)}-${id}`}>
              <PassiveChoice id={id} content={content} />
            </li>
          ))}
        </ul>
      )}
      <h3 className="dungeon-run-section__title">{t("Run statistics")}</h3>
      <dl className="dungeon-run-stats">
        {STAT_KEYS.map((key) => (
          <div className="dungeon-run-stats__row" data-testid={dungeonMapTestid.stat(key)} key={key}>
            <dt className="dungeon-run-stats__key">{t(RUN_STAT_LABELS[key])}</dt>
            <dd className="dungeon-run-stats__value">{String(run.stats[key])}</dd>
          </div>
        ))}
      </dl>
      <div className="dungeon-run-actions">
        <button
          type="button"
          className="button-primary"
          data-testid={dungeonMapTestid.newRun}
          onClick={() => {
            dungeon.abandon();
          }}
        >
          {t("Start a new run")}
        </button>
        <a href={paths.dungeonStats} data-testid={dungeonMapTestid.statsLink} onClick={followInApp(paths.dungeonStats)}>
          {t("Statistics")}
        </a>
      </div>
    </section>
  );
}
