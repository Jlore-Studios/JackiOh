// The Dungeon run's start screen (DR1–DR3, DR8, DR9; DR14 part D, #2705): the mode, the seed, the
// starting deck, its relics and curses, the Embers that raise its level, and the run under way that
// can be picked up again or given up (R2972000: giving it up asks first).
//
// Presentation only (CLAUDE.md rule 7): the run's rules are `crates/run`'s. This screen never judges a
// starting deck: it offers the unlocked cards, counts the picks, and asks `runCheckSetup` whether the
// setup stands, showing its refusal word for word. The numbers it shows (the level's slots, the
// deck's size, the relics' limits and points, a level's cost) are `runConstants()`'s and
// `runLevelCost()`'s, never stated here (rule 9).

import { useId, useMemo, useState, type KeyboardEvent, type ReactElement } from "react";

import type { RunSetup, RunTier } from "@jackioh/run";

import { t, useLocale } from "../../i18n/i18n.ts";
import { paths } from "../../net/navigate.ts";
import { BackLink } from "../../routes/nav.tsx";
import { Wordmark } from "../../routes/Wordmark.tsx";
import { runCheckSetup, runConstants, runDailySeed, runLevelCost } from "../../wasm/index.ts";
import { DUNGEON_FIRST_LEVEL, DUNGEON_LEVEL_STEP } from "../config.ts";
import { isTypedSeed, randomSeed, seedLength, todayDate } from "../seed.ts";
import { cursePointsOf, dailyRunNumber, runFinished, type Dungeon } from "../store.ts";
import { dungeonTestid } from "../testids.ts";
import "../../game/deckbuilder/deckbuilder.css";
import "../dungeon.css";
import { DeckBuilder, SLOT_IDS, slotName } from "./DeckBuilder.tsx";
import { PresetDecks } from "./PresetDecks.tsx";

export type StartScreenProps = {
  dungeon: Dungeon;
  /** The run is under way (a started one, or the one kept on the device picked up again): the page shows it. */
  onEnter: () => void;
};

type SeedChoice = "daily" | "random" | "typed";

function toggled(held: readonly string[], id: string): string[] {
  return held.includes(id) ? held.filter((one) => one !== id) : [...held, id];
}

function Progress({ dungeon }: { dungeon: Dungeon }): ReactElement {
  const { progress } = dungeon;
  const [refusal, setRefusal] = useState<string | null>(null);
  const { level } = progress.unlocks;
  const cost = runLevelCost(level);
  return (
    <section className="dungeon-block dungeon-progress">
      <h2>{t("Progress")}</h2>
      <p data-testid={dungeonTestid.embers}>{t("Embers: {embers}", { embers: String(progress.embers) })}</p>
      <p data-testid={dungeonTestid.level}>{t("Starting deck level {level}", { level: String(level) })}</p>
      {cost === null ? (
        <p>{t("Your starting deck is at its top level.")}</p>
      ) : (
        <button
          type="button"
          className="button-secondary"
          data-testid={dungeonTestid.buyLevel}
          onClick={() => {
            setRefusal(dungeon.buyLevel());
          }}
        >
          {t("Raise to level {level} for {cost} Embers", { level: String(level + DUNGEON_LEVEL_STEP), cost: String(cost) })}
        </button>
      )}
      {refusal === null ? null : (
        <p className="notice" role="alert" data-testid={dungeonTestid.buyRefusal}>
          {refusal}
        </p>
      )}
    </section>
  );
}

function SavedRun({ dungeon, onEnter }: StartScreenProps): ReactElement {
  // R2972000: giving up a run cannot be undone, so the first press asks in place, as a token's
  // redemption asks first (R2480000); `asked` puts the focus back on the offer when Cancel or
  // Escape answers.
  const [asking, setAsking] = useState(false);
  const [asked, setAsked] = useState(false);
  const questionId = useId();
  const cancel = (): void => {
    setAsking(false);
  };
  // Escape answers Cancel and nothing else.
  const cancelOnEscape = (event: KeyboardEvent<HTMLDivElement>): void => {
    if (event.key !== "Escape") return;
    event.stopPropagation();
    cancel();
  };
  return (
    <section className="dungeon-block dungeon-saved" data-testid={dungeonTestid.saved}>
      <p>{t("You have a run in progress.")}</p>
      <div className="dungeon-actions">
        <button type="button" className="button-primary" data-testid={dungeonTestid.continueRun} onClick={onEnter}>
          {t("Continue the run")}
        </button>
        {asking ? (
          <div
            className="dungeon-abandon__question"
            role="alertdialog"
            aria-labelledby={questionId}
            data-testid={dungeonTestid.abandonAsk}
            onKeyDown={cancelOnEscape}
          >
            <p id={questionId}>{t("Abandon this run? It ends here and cannot be undone.")}</p>
            {/* R3221000: the safe choice first, holding the focus, and the act last. */}
            <div className="dungeon-actions">
              <button type="button" data-testid={dungeonTestid.abandonCancel} autoFocus onClick={cancel}>
                {t("Cancel")}
              </button>
              <button type="button" data-testid={dungeonTestid.abandonConfirm} onClick={dungeon.abandon}>
                {t("Confirm")}
              </button>
            </div>
          </div>
        ) : (
          <button
            type="button"
            className="button-secondary"
            data-testid={dungeonTestid.abandonRun}
            autoFocus={asked}
            onClick={() => {
              setAsked(true);
              setAsking(true);
            }}
          >
            {t("Abandon the run")}
          </button>
        )}
      </div>
    </section>
  );
}

function Setup({ dungeon, onEnter }: StartScreenProps): ReactElement {
  const { content, progress } = dungeon;
  const { unlocks } = progress;
  const constants = useMemo(() => runConstants(), []);
  const length = useMemo(() => seedLength(), []);

  // The seed: the day's (DR2), a random one, or one typed.
  const [seedChoice, setSeedChoice] = useState<SeedChoice>("daily");
  const [random, setRandom] = useState(randomSeed);
  const [typed, setTyped] = useState("");
  const date = todayDate();
  const seed = seedChoice === "daily" ? runDailySeed(date, dailyRunNumber(date)) : seedChoice === "random" ? random : typed;
  const typedWrong = seedChoice === "typed" && typed !== "" && !isTypedSeed(typed);
  const seedUsable = seedChoice !== "typed" || isTypedSeed(typed);

  // The starting deck's slots and the cards a deck may hold: the unlocked ones, in the pool's order.
  const slots = constants.slotLadder[unlocks.level - DUNGEON_FIRST_LEVEL] ?? [];
  const offered = useMemo(() => {
    const allowed = new Set([...content.starterAccess, ...unlocks.cards]);
    const seen = new Set<string>();
    return content.pool.filter((card) => {
      if (!allowed.has(card.id) || seen.has(card.id)) return false;
      seen.add(card.id);
      return true;
    });
  }, [content, unlocks.cards]);
  const relics = useMemo(
    () =>
      content.passives.filter(
        (passive) =>
          (passive.kind === "combat" || passive.kind === "economic") &&
          !passive.unobtainable &&
          (passive.starter || unlocks.artifacts.includes(passive.id)),
      ),
    [content, unlocks.artifacts],
  );

  const [pickedCards, setPickedCards] = useState<string[]>([]);
  const [pickedRelics, setPickedRelics] = useState<string[]>([]);
  const [pickedCurses, setPickedCurses] = useState<string[]>([]);
  // A pick the pool no longer offers is no pick.
  const cards = useMemo(() => pickedCards.filter((id) => offered.some((card) => card.id === id)), [pickedCards, offered]);
  const taken = useMemo(() => pickedRelics.filter((id) => relics.some((relic) => relic.id === id)), [pickedRelics, relics]);
  const curses = useMemo(
    () => pickedCurses.filter((id) => content.curses.some((curse) => curse.id === id)),
    [pickedCurses, content.curses],
  );

  const pointsOf = (tier: RunTier | null): number => constants.tierPoints.find(([name]) => name === tier)?.[1] ?? 0;
  const relicPoints = taken.reduce((sum, id) => sum + pointsOf(relics.find((relic) => relic.id === id)?.tier ?? null), 0);
  const cursePoints = cursePointsOf(curses, content);

  const setup = useMemo<RunSetup>(() => ({ seed, starter: cards, artifacts: taken, curses }), [seed, cards, taken, curses]);
  const refusal = useMemo(() => (seedUsable ? runCheckSetup(setup, unlocks, content) : null), [seedUsable, setup, unlocks, content]);
  const picked = cards.length > 0 || taken.length > 0 || curses.length > 0;
  const [startRefusal, setStartRefusal] = useState<string | null>(null);
  const shownRefusal = startRefusal ?? (picked ? refusal : null);

  function pick(change: () => void): void {
    setStartRefusal(null);
    change();
  }

  return (
    <>
      <section className="dungeon-block">
        <h2>{t("Mode")}</h2>
        <div className="dungeon-choices">
          <label className="dungeon-choice" data-selected="true">
            <input type="radio" name="dungeon-mode" checked readOnly data-testid={dungeonTestid.modeSandbox} />
            <span>
              <span className="dungeon-choice__name">{t("Sandbox")}</span>
              <span className="dungeon-choice__note">{t("Offline, on this device.")}</span>
            </span>
          </label>
          <label className="dungeon-choice" data-selected="false" data-disabled="true">
            <input type="radio" name="dungeon-mode" disabled checked={false} readOnly data-testid={dungeonTestid.modeCampaign} />
            <span>
              <span className="dungeon-choice__name">{t("Campaign")}</span>
              <span className="dungeon-choice__note" data-testid={dungeonTestid.campaignNote}>
                {t("Sign in to play the Campaign: coming with accounts.")}
              </span>
            </span>
          </label>
        </div>
      </section>

      <section className="dungeon-block">
        <h2>{t("Starting deck")}</h2>
        <ul className="dungeon-slots">
          {slots.map((count, column) =>
            count === 0 ? null : (
              <li key={column} data-testid={dungeonTestid.slot(SLOT_IDS[column] ?? String(column))}>
                {t("{slot} × {count}", { slot: slotName(column), count: String(count) })}
              </li>
            ),
          )}
        </ul>
        <PresetDecks
          presets={constants.presets}
          offered={offered}
          picked={cards}
          onPick={(ids) => {
            pick(() => {
              setPickedCards(ids.filter((id) => offered.some((card) => card.id === id)));
            });
          }}
        />
        <DeckBuilder
          offered={offered}
          picked={cards}
          onToggle={(id) => {
            pick(() => {
              setPickedCards(toggled(cards, id));
            });
          }}
          slots={slots}
          starterSize={constants.starterSize}
        />
      </section>

      <section className="dungeon-block">
        <h2>{t("Relics")}</h2>
        <p data-testid={dungeonTestid.relicCount}>
          {t("Relics: {chosen} of {max}", { chosen: String(taken.length), max: String(constants.starterArtifactsMax) })}
        </p>
        <p data-testid={dungeonTestid.relicPoints}>
          {t("Relic points: {points} of {max}", { points: String(relicPoints), max: String(constants.starterArtifactPoints) })}
        </p>
        <ul className="dungeon-relics">
          {relics.map((relic) => {
            const chosen = taken.includes(relic.id);
            return (
              <li key={relic.id}>
                <button
                  type="button"
                  className="dungeon-relic"
                  aria-pressed={chosen}
                  disabled={!chosen && taken.length >= constants.starterArtifactsMax}
                  data-testid={dungeonTestid.relic(relic.id)}
                  data-tier={relic.tier ?? "none"}
                  onClick={() => {
                    pick(() => {
                      setPickedRelics(toggled(taken, relic.id));
                    });
                  }}
                >
                  <span className="dungeon-relic__name">{relic.name}</span>
                  <span className="dungeon-relic__text">{relic.text}</span>
                  <span className="dungeon-relic__points">{t("Points: {points}", { points: String(pointsOf(relic.tier)) })}</span>
                </button>
              </li>
            );
          })}
        </ul>
      </section>

      <section className="dungeon-block">
        <h2>{t("Curses")}</h2>
        <p data-testid={dungeonTestid.cursePoints}>{t("Curse points: {points}", { points: String(cursePoints) })}</p>
        <ul className="dungeon-relics">
          {content.curses.map((curse) => {
            const chosen = curses.includes(curse.id);
            return (
              <li key={curse.id}>
                <button
                  type="button"
                  className="dungeon-relic"
                  aria-pressed={chosen}
                  data-testid={dungeonTestid.curse(curse.id)}
                  onClick={() => {
                    pick(() => {
                      setPickedCurses(toggled(curses, curse.id));
                    });
                  }}
                >
                  <span className="dungeon-relic__name">{curse.name}</span>
                  <span className="dungeon-relic__text">{curse.text}</span>
                  <span className="dungeon-relic__points">{t("Points: {points}", { points: String(curse.points) })}</span>
                </button>
              </li>
            );
          })}
        </ul>
      </section>

      <section className="dungeon-block">
        <h2>{t("Choose a seed")}</h2>
        <div className="dungeon-choices">
          <label className="dungeon-choice" data-selected={seedChoice === "daily" ? "true" : "false"}>
            <input
              type="radio"
              name="dungeon-seed"
              checked={seedChoice === "daily"}
              data-testid={dungeonTestid.seedDaily}
              onChange={() => {
                pick(() => {
                  setSeedChoice("daily");
                });
              }}
            />
            <span className="dungeon-choice__name">{t("Today's run")}</span>
          </label>
          <label className="dungeon-choice" data-selected={seedChoice === "random" ? "true" : "false"}>
            <input
              type="radio"
              name="dungeon-seed"
              checked={seedChoice === "random"}
              data-testid={dungeonTestid.seedRandom}
              // A press on the chosen one draws another, which a change event never reports.
              onChange={() => undefined}
              onClick={() => {
                pick(() => {
                  setSeedChoice("random");
                  setRandom(randomSeed());
                });
              }}
            />
            <span className="dungeon-choice__name">{t("Random")}</span>
          </label>
          <label className="dungeon-choice" data-selected={seedChoice === "typed" ? "true" : "false"}>
            <input
              type="radio"
              name="dungeon-seed"
              checked={seedChoice === "typed"}
              data-testid={dungeonTestid.seedTyped}
              onChange={() => {
                pick(() => {
                  setSeedChoice("typed");
                });
              }}
            />
            <span className="dungeon-choice__name">{t("Type one")}</span>
          </label>
        </div>
        {seedChoice === "typed" ? (
          <input
            className="dungeon-seed-input"
            type="text"
            value={typed}
            maxLength={length}
            autoComplete="off"
            spellCheck={false}
            aria-label={t("Seed")}
            aria-invalid={typedWrong}
            data-testid={dungeonTestid.seedInput}
            onChange={(event) => {
              pick(() => {
                setTyped(event.target.value);
              });
            }}
          />
        ) : null}
        {typedWrong ? (
          <p className="notice" role="alert" data-testid={dungeonTestid.seedError}>
            {t("A seed is {length} capital letters, A to Z.", { length: String(length) })}
          </p>
        ) : null}
        {seedUsable ? <p data-testid={dungeonTestid.seedValue}>{t("Seed: {seed}", { seed })}</p> : null}
      </section>

      {shownRefusal === null ? null : (
        <p className="notice" role="alert" data-testid={dungeonTestid.setupRefusal}>
          {shownRefusal}
        </p>
      )}
      <button
        type="button"
        className="button-primary dungeon-start"
        data-testid={dungeonTestid.startRun}
        disabled={!seedUsable || refusal !== null}
        onClick={() => {
          const refused = dungeon.start(setup);
          setStartRefusal(refused);
          if (refused === null) onEnter();
        }}
      >
        {t("Start the run")}
      </button>
    </>
  );
}

export function StartScreen({ dungeon, onEnter }: StartScreenProps): ReactElement {
  useLocale();
  const unfinished = dungeon.run !== null && !runFinished(dungeon.run);
  return (
    <div className="app-shell tavern dungeon-screen" data-testid={dungeonTestid.start}>
      <BackLink to={paths.play} title={t("Dungeon run")} />
      <section className="panel panel--auth dungeon-panel">
        <Wordmark />
        {dungeon.notice === null ? null : (
          <p className="notice" role="status" data-testid={dungeonTestid.notice}>
            {dungeon.notice}
          </p>
        )}
        <Progress dungeon={dungeon} />
        {unfinished ? <SavedRun dungeon={dungeon} onEnter={onEnter} /> : <Setup dungeon={dungeon} onEnter={onEnter} />}
      </section>
    </div>
  );
}
