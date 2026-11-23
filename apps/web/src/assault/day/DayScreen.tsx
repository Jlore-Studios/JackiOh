// The UTC day's public Assault deal, record, ladder, deck choices and history. Every transition is
// handed to the store; the screen derives no legal move (CLAUDE.md rule 7).

import { useMemo, useState, type ReactElement } from "react";

import { useSecondsUntil } from "../../auth/cooldown.ts";
import { t, useLocale } from "../../i18n/i18n.ts";
import { paths } from "../../net/navigate.ts";
import type { SavedDecks } from "../../practice/PracticeSetup.tsx";
import { BackLink } from "../../routes/nav.tsx";
import { Wordmark } from "../../routes/Wordmark.tsx";
import {
  ASSAULT_CLOCK_DIGITS,
  ASSAULT_FIRST_LEVEL,
  ASSAULT_HOURS_PER_DAY,
  ASSAULT_MILLISECONDS_PER_SECOND,
  ASSAULT_MINUTES_PER_HOUR,
  ASSAULT_RARITIES,
  ASSAULT_SECONDS_PER_MINUTE,
} from "../config.ts";
import type { Assault } from "../store.ts";
import { assaultTestid } from "../testids.ts";
import { DeckPicker, dayDeckChoice, type AssaultDeckChoice } from "./DeckPicker.tsx";

type DayScreenProps = {
  assault: Assault;
  saved: SavedDecks;
  /** R4582001: the open attempt's fight was left by its Menu; Resume picks it up where it stood. */
  onResume?: () => void;
};

function highestCheckpoint(checkpoints: readonly number[]): number {
  return checkpoints.length === 0 ? ASSAULT_FIRST_LEVEL : Math.max(...checkpoints);
}

function nextUtcDay(date: string): number {
  const dayStart = Date.parse(`${date}T00:00:00.000Z`);
  return dayStart
    + ASSAULT_HOURS_PER_DAY
      * ASSAULT_MINUTES_PER_HOUR
      * ASSAULT_SECONDS_PER_MINUTE
      * ASSAULT_MILLISECONDS_PER_SECOND;
}

function countdownWords(seconds: number): string {
  const hours = Math.floor(seconds / (ASSAULT_MINUTES_PER_HOUR * ASSAULT_SECONDS_PER_MINUTE));
  const minutes = Math.floor((seconds / ASSAULT_SECONDS_PER_MINUTE) % ASSAULT_MINUTES_PER_HOUR);
  const rest = seconds % ASSAULT_SECONDS_PER_MINUTE;
  const two = (value: number): string => String(value).padStart(ASSAULT_CLOCK_DIGITS, "0");
  return `${String(hours)}:${two(minutes)}:${two(rest)}`;
}

export function DayScreen({ assault, saved, onResume }: DayScreenProps): ReactElement {
  useLocale();
  const { constants, content, day, record, records } = assault;
  const checkpoint = highestCheckpoint(record.checkpoints);
  const [startLevel, setStartLevel] = useState(checkpoint);
  const [choice, setChoice] = useState<AssaultDeckChoice>(() => dayDeckChoice(day.date));
  const [refusal, setRefusal] = useState<string | null>(null);
  const secondsToNextDay = useSecondsUntil(nextUtcDay(day.date));
  const starts = checkpoint === ASSAULT_FIRST_LEVEL ? [ASSAULT_FIRST_LEVEL] : [ASSAULT_FIRST_LEVEL, checkpoint];
  const strength = day.levels.find((level) => level.level === startLevel) ?? day.levels[0];
  const history = useMemo(
    () => Object.values(records).sort((a, b) => b.date.localeCompare(a.date)),
    [records],
  );

  return (
    <div className="app-shell tavern assault-screen">
      <BackLink to={paths.play} title={t("Infinite Assault")} />
      <section className="panel panel--auth assault-panel">
        <Wordmark />
        <p>{t("Today's Assault · {date}", { date: day.date })}</p>
        <p>{t("Next Assault in {time}", { time: countdownWords(secondsToNextDay) })}</p>
        <section className="assault-block">
          <h2>{t("Today's record")}</h2>
          <p data-testid={assaultTestid.record}>
            {t("Best: level {best} · Attempts: {attempts} · Checkpoints: {checkpoints}", {
              best: String(record.best),
              attempts: String(record.attempts),
              checkpoints: record.checkpoints.length === 0 ? t("none") : record.checkpoints.join(t(", ")),
            })}
          </p>
        </section>

        <section className="assault-block">
          <h2>{t("The ladder")}</h2>
          <ol className="assault-ladder">
            {day.levels.map((level) => {
              const doctrine = content.doctrines.find((candidate) => candidate.id === level.doctrine);
              return (
                <li
                  key={level.level}
                  data-testid={assaultTestid.level(level.level)}
                  data-beaten={level.level <= record.best ? "true" : undefined}
                >
                  <span>{t("Level {level}: {enemy}", { level: String(level.level), enemy: t(level.enemy) })}</span>
                  {doctrine === undefined ? null : <span>{t(doctrine.name)}</span>}
                  {constants.commanderLevels.includes(level.level) ? <span>{t("Commander")}</span> : null}
                  {constants.checkpoints.includes(level.level) ? <span>{t("Checkpoint")}</span> : null}
                </li>
              );
            })}
          </ol>
        </section>

        {onResume === undefined || assault.state === null ? (
          <>
            <section className="assault-block">
              <h2>{t("Start level")}</h2>
              <div className="assault-start-levels">
                {starts.map((level) => (
                  <label key={level}>
                    <input
                      type="radio"
                      name="assault-start-level"
                      checked={startLevel === level}
                      data-testid={assaultTestid.startLevel(level)}
                      onChange={() => {
                        setStartLevel(level);
                      }}
                    />
                    {t("Level {level}", { level: String(level) })}
                  </label>
                ))}
              </div>
              {strength === undefined ? null : (
                <div className="assault-strength">
                  <h3>{t("Assault strength at level {level}", { level: String(strength.level) })}</h3>
                  <p>
                    {ASSAULT_RARITIES.map((rarity, index) =>
                      t("{rarity}: {count}", { rarity: t(rarity), count: String(strength.rarities[index] ?? 0) }),
                    ).join(t(" · "))}
                  </p>
                  <p>{t("{count} upgrades", { count: String(strength.upgrades.length) })}</p>
                </div>
              )}
            </section>

            <DeckPicker date={day.date} saved={saved} choice={choice} onChange={setChoice} />
            {refusal === null ? null : <p className="notice" role="alert">{refusal}</p>}
            <button
              type="button"
              className="button-primary assault-start"
              data-testid={assaultTestid.start}
              onClick={() => {
                setRefusal(assault.start({
                  date: day.date,
                  deck: choice.cards,
                  source: choice.source,
                  startLevel,
                }));
              }}
            >
              {t("Start at level {level}", { level: String(startLevel) })}
            </button>
          </>
        ) : (
          <section className="assault-block">
            <p>{t("Your attempt waits at level {level}.", { level: String(assault.state.level) })}</p>
            <button type="button" className="button-primary assault-start" data-testid={assaultTestid.resume} onClick={onResume}>
              {t("Resume the fight")}
            </button>
          </section>
        )}

        <section className="assault-block" data-testid={assaultTestid.history}>
          <h2>{t("History")}</h2>
          {history.length === 0 ? <p>{t("No earlier Assaults on this device.")}</p> : (
            <ol className="assault-history">
              {history.map((past) => (
                <li key={past.date}>
                  {t("{date} · Best level {best} · {attempts} attempts", {
                    date: past.date,
                    best: String(past.best),
                    attempts: String(past.attempts),
                  })}
                </li>
              ))}
            </ol>
          )}
        </section>
      </section>
    </div>
  );
}
