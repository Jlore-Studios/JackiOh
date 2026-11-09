// The dungeon run's screens between fights (DR14 part F, #2707): the map, the rewards after a
// fight, the pack, the extraction after a boss, the shop, the campfire, the event, the shrine and
// the run-over screen. Part D's page renders it in every phase but `fight`. Players read "relic";
// the code keeps the run's word, artifact (WRITING.md's jargon). Presentation only (CLAUDE.md
// rule 7): every control sends the frozen `RunAction` through `Dungeon.act` and shows its refusal,
// and every word goes through `t()` at render.

import { useState, type ReactElement } from "react";

import type { RunAction, RunContent, RunState } from "@jackioh/run";

import { t, useLocale } from "../../i18n/i18n.ts";
import { paths } from "../../net/navigate.ts";
import { BackLink } from "../../routes/nav.tsx";
import { runMaxHp, runScore } from "../../wasm/index.ts";
import { RunTimer } from "../fight/RunTimer.tsx";
import { runFinished, type Dungeon } from "../store.ts";
import { MapScreen } from "./MapScreen.tsx";
import {
  ExtractArtifactScreen,
  ExtractCardScreen,
  PackScreen,
  RewardScreen,
} from "./RewardScreens.tsx";
import { dungeonMapTestid } from "./testids.ts";
import { CampfireScreen, EventScreen, ShopScreen, ShrineScreen } from "./TileScreens.tsx";
import { RunOverScreen } from "./RunOverScreen.tsx";
import { passiveName, type SendRunAction } from "./RunDisplay.tsx";
import "../../auth/tavern.css";
import "./run.css";

/**
 * The HUD, on every screen: HP, Coins, Tickets, the deck's size, the relics held and the score, and
 * the run's time until the run is over (B24), whose record then holds it.
 */
function RunHud({
  run,
  content,
  elapsedMs,
  resumedAt,
}: {
  run: RunState;
  content: RunContent;
  elapsedMs: number | null;
  resumedAt: number | null;
}): ReactElement {
  const max = runMaxHp(run, content);
  const score = runScore(run, content);
  return (
    <header className="dungeon-run-hud" data-testid={dungeonMapTestid.hud}>
      {runFinished(run) ? null : <RunTimer elapsedMs={elapsedMs} resumedAt={resumedAt} />}
      <span className="dungeon-run-hud__stat" data-testid={dungeonMapTestid.hp}>
        {t("HP")}: {run.hp} / {max}
      </span>
      <span className="dungeon-run-hud__stat" data-testid={dungeonMapTestid.coins}>
        {t("Coins")}: {run.coins}
      </span>
      <span className="dungeon-run-hud__stat" data-testid={dungeonMapTestid.tickets}>
        {t("Tickets")}: {run.tickets}
      </span>
      <span className="dungeon-run-hud__stat" data-testid={dungeonMapTestid.deckSize}>
        {t("Deck")}: {run.deck.length}
      </span>
      <span className="dungeon-run-hud__relics" data-testid={dungeonMapTestid.artifacts}>
        <span className="dungeon-run-hud__label">{t("Relics")}</span>
        {run.artifacts.length === 0 ? (
          <span className="dungeon-run-hud__empty">{t("No relics")}</span>
        ) : (
          run.artifacts.map((artifact, index) => (
            <span
              className="dungeon-run-hud__relic"
              data-testid={dungeonMapTestid.hudArtifact(index)}
              key={`${String(index)}-${artifact.id}`}
            >
              {passiveName(content, artifact.id)}{" "}
              {t("Level {level}", { level: String(artifact.level) })}
            </span>
          ))
        )}
      </span>
      <span className="dungeon-run-hud__stat" data-testid={dungeonMapTestid.score}>
        {t("Score")}: {score}
      </span>
    </header>
  );
}

/** Every non-fight phase's screen: the HUD, the run's latest refusal, and the phase's body. */
export function RunScreen({ dungeon }: { dungeon: Dungeon }): ReactElement {
  // Re-render on a live language change, so every `t()` and card face follows it.
  useLocale();
  const [refusal, setRefusal] = useState<string | null>(null);
  const run = dungeon.run;
  const content = dungeon.content;
  // Impossible while part D's page stands guard (it renders the start screen with no run), but a
  // screen that crashes on null is a screen that crashes.
  if (run === null) {
    return (
      <div className="app-shell tavern dungeon-run-screen" data-testid={dungeonMapTestid.screen}>
        <p className="notice" role="alert">
          {t("There is no run to show.")}
        </p>
      </div>
    );
  }
  const send = (action: RunAction): void => {
    setRefusal(dungeon.act(action));
  };
  return (
    <div className="app-shell tavern dungeon-run-screen" data-testid={dungeonMapTestid.screen}>
      {/* R2972000: back to the Play menu; the run stays saved, and the start screen offers Continue. */}
      <BackLink to={paths.play} title={t("Dungeon run")} />
      <RunHud run={run} content={content} elapsedMs={dungeon.elapsedMs} resumedAt={dungeon.resumedAt} />
      {refusal === null ? null : (
        <p className="notice" role="alert" data-testid={dungeonMapTestid.refusal}>
          {t(refusal)}
        </p>
      )}
      <div className="dungeon-run-body" data-testid={dungeonMapTestid.body}>
        <PhaseBody dungeon={dungeon} run={run} content={content} send={send} />
      </div>
    </div>
  );
}

type PhaseBodyProps = {
  dungeon: Dungeon;
  run: RunState;
  content: RunContent;
  send: SendRunAction;
};

/** The phase's screen. Every `RunPhase` has its case; the switch below is total. */
function PhaseBody({ dungeon, run, content, send }: PhaseBodyProps): ReactElement {
  const phase = run.phase;
  switch (phase.phase) {
    case "map":
      return <MapScreen run={run} content={content} send={send} />;
    case "fight":
      // Part E's screen owns fights; the page never renders this screen for one.
      return <></>;
    case "reward":
      return <RewardScreen run={run} content={content} phase={phase} send={send} />;
    case "pack":
      return <PackScreen run={run} content={content} phase={phase} send={send} />;
    case "extractCard":
      return <ExtractCardScreen run={run} content={content} send={send} />;
    case "extractArtifact":
      return <ExtractArtifactScreen run={run} content={content} send={send} />;
    case "shop":
      return <ShopScreen run={run} content={content} phase={phase} send={send} />;
    case "campfire":
      return <CampfireScreen run={run} content={content} send={send} />;
    case "event":
      return <EventScreen run={run} content={content} phase={phase} send={send} />;
    case "shrine":
      return <ShrineScreen run={run} content={content} phase={phase} send={send} />;
    case "won":
      return <RunOverScreen dungeon={dungeon} run={run} content={content} won={true} />;
    case "dead":
      return <RunOverScreen dungeon={dungeon} run={run} content={content} won={false} />;
  }
}
