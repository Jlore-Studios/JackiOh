// The dungeon run's rewards (DR6, DR14 part F, #2707): the relic a fight dropped, taken or left
// before the four packs; one opened pack, keeping exactly `keep` of its cards; and both
// extractions, a card after a mini-boss and a relic after the final boss. Players read "relic";
// the code keeps the run's word, artifact (WRITING.md's jargon). Presentation only (CLAUDE.md
// rule 7): the run answers every rule, and every word goes through `t()` at render.

import { useEffect, useState, type ReactElement } from "react";

import type { RunContent, RunPackKind, RunPhase, RunState } from "@jackioh/run";

import { useFxSettings } from "../../fx/settings.ts";
import { reducedMotionNow } from "../../game/animations.ts";
import { t } from "../../i18n/i18n.ts";
import {
  DUNGEON_BOSS_PACK,
  DUNGEON_ENEMY_PACK,
  DUNGEON_FILTER_PACK,
  DUNGEON_RANDOM_PACK,
  PACK_TEAR_MS,
} from "../../packs/packs.ts";
import PackObject from "../../packs/PackObject.tsx";
import { useSetting } from "../../settings/store.ts";
import { dungeonMapTestid } from "./testids.ts";
import {
  cardName,
  hasSkipPassive,
  IndexedCardChoices,
  PassiveChoice,
  RunCardFace,
  type SendRunAction,
} from "./RunDisplay.tsx";

type RewardPhase = Extract<RunPhase, { phase: "reward" }>;
type PackPhase = Extract<RunPhase, { phase: "pack" }>;

/**
 * The enemy just beaten, with whether its node was a boss's (#3588): the reward draws the enemy's
 * deck pack in its name, with a boss look on a boss node. Null before the first step, when the run
 * stands on no row yet.
 */
function beatenEnemy(run: RunState): { name: string | null; boss: boolean } | null {
  if (run.row === null) return null;
  const node = run.map[run.row]?.[run.col];
  if (node === undefined) return null;
  return { name: node.enemy?.name ?? null, boss: node.kind === "miniBoss" || node.kind === "boss" };
}

/**
 * The shop-style pack a reward kind draws as (#3588): a set's own pack, the enemy's deck (a boss
 * look on a boss node), or a distinct look for the random and the type, tag or rarity packs.
 */
function packIdFor(kind: RunPackKind, boss: boolean): string {
  switch (kind.type) {
    case "set":
      return kind.set;
    case "boss":
      return boss ? DUNGEON_BOSS_PACK : DUNGEON_ENEMY_PACK;
    case "random":
      return DUNGEON_RANDOM_PACK;
    case "cardType":
    case "tag":
    case "rarity":
      return DUNGEON_FILTER_PACK;
  }
}

/** A pack's label: its kind, translated. */
function packLabel(kind: RunPackKind): string {
  switch (kind.type) {
    case "boss":
      return t("Boss deck");
    case "random":
      return t("Random");
    case "set":
      return t("Set: {set}", { set: kind.set });
    case "cardType":
      return t("Type: {type}", { type: kind.cardType });
    case "tag":
      return t("Tag: {tag}", { tag: kind.tag });
    case "rarity":
      return t("Rarity: {rarity}", { rarity: kind.rarity });
  }
}

type RewardScreenProps = {
  run: RunState;
  content: RunContent;
  phase: RewardPhase;
  send: SendRunAction;
};

/**
 * The fight's reward: the dropped relic first, taken or left, and only after it the four packs,
 * one to open.
 */
export function RewardScreen({ run, content, phase, send }: RewardScreenProps): ReactElement {
  if (phase.artifact !== null) {
    const id = phase.artifact;
    return (
      <section className="dungeon-run-reward" data-testid={dungeonMapTestid.reward}>
        <h2 className="dungeon-run-title">{t("Take the relic?")}</h2>
        <div className="dungeon-run-passive-row" data-testid={dungeonMapTestid.rewardArtifact}>
          <PassiveChoice id={id} content={content} />
        </div>
        <div className="dungeon-run-actions">
          <button
            type="button"
            className="button-primary"
            data-testid={dungeonMapTestid.artifactTake}
            onClick={() => {
              send({ type: "takeArtifact", take: true });
            }}
          >
            {t("Take")}
          </button>
          <button
            type="button"
            data-testid={dungeonMapTestid.artifactLeave}
            onClick={() => {
              send({ type: "takeArtifact", take: false });
            }}
          >
            {t("Leave")}
          </button>
        </div>
      </section>
    );
  }
  const enemy = beatenEnemy(run);
  return (
    <section className="dungeon-run-reward" data-testid={dungeonMapTestid.reward}>
      <h2 className="dungeon-run-title">{t("Choose a pack")}</h2>
      <ul className="dungeon-run-packs" data-testid={dungeonMapTestid.packs}>
        {phase.packs.map((kind, index) => (
          <li key={index}>
            <button
              type="button"
              className="dungeon-run-pack-pick"
              data-testid={dungeonMapTestid.pack(index)}
              onClick={() => {
                send({ type: "openPack", index });
              }}
            >
              <PackObject
                pack={packIdFor(kind, enemy?.boss ?? false)}
                size="shelf"
                testid={dungeonMapTestid.packObject(index)}
              >
                {kind.type === "boss" && enemy?.name !== null && enemy?.name !== undefined ? (
                  <span className="dungeon-run-pack-pick__enemy">{enemy.name}</span>
                ) : undefined}
              </PackObject>
              <span className="dungeon-run-pack-pick__label">{packLabel(kind)}</span>
              {kind.type === "boss" && enemy?.boss === true ? (
                <span className="dungeon-run-pack-pick__boss">{t("Boss")}</span>
              ) : null}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

type PackScreenProps = {
  run: RunState;
  content: RunContent;
  phase: PackPhase;
  send: SendRunAction;
};

/** The opened pack: exactly `keep` cards are kept, or none with a Skip passive. */
export function PackScreen({ run, content, phase, send }: PackScreenProps): ReactElement {
  const [selected, setSelected] = useState<readonly string[]>([]);
  // The shop scene's motion lines (#3588): reduced motion or fx intensity off shows the cards at once.
  const [fx] = useFxSettings();
  const panelReduces = useSetting("reduceMotion");
  const reduced = panelReduces || reducedMotionNow(fx);
  const instant = reduced || fx.intensity === "off";
  const [stage, setStage] = useState<"sealed" | "tearing" | "open">(() => (instant ? "open" : "sealed"));
  useEffect(() => {
    if (stage !== "tearing") return undefined;
    const timer = window.setTimeout(() => {
      setStage("open");
    }, PACK_TEAR_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [stage]);
  const toggle = (id: string): void => {
    setSelected((was) => {
      if (was.includes(id)) return was.filter((kept) => kept !== id);
      if (was.length >= phase.keep) return was;
      return [...was, id];
    });
  };
  const skip = hasSkipPassive(run, content);
  // The run already decided the pack's cards, so the sealed pack is its own tear, not the
  // server-driven `PackScene`: it tears for `PACK_TEAR_MS`, then reveals its cards face up.
  if (stage !== "open") {
    return (
      <section className="dungeon-run-pack" data-testid={dungeonMapTestid.packScreen}>
        <h2 className="dungeon-run-title">{t("Keep {count} cards", { count: String(phase.keep) })}</h2>
        <button
          type="button"
          className="dungeon-run-pack-tear"
          data-testid={dungeonMapTestid.packTear}
          onClick={() => {
            setStage("tearing");
          }}
        >
          <PackObject pack={DUNGEON_FILTER_PACK} size="scene" state={stage === "tearing" ? "tearing" : "idle"}>
            <span className="dungeon-run-pack-tear__hint">{t("Tap to open")}</span>
          </PackObject>
        </button>
      </section>
    );
  }
  return (
    <section className="dungeon-run-pack" data-testid={dungeonMapTestid.packScreen}>
      <h2 className="dungeon-run-title">{t("Keep {count} cards", { count: String(phase.keep) })}</h2>
      <p className="dungeon-run-pack-counter" data-testid={dungeonMapTestid.packCounter}>
        {t("{kept} of {keep} kept", { kept: String(selected.length), keep: String(phase.keep) })}
      </p>
      <ul className="dungeon-run-cards">
        {phase.cards.map((id) => (
          <li className="dungeon-run-cards__item" key={id}>
            <RunCardFace
              id={id}
              radiant={false}
              content={content}
              testid={dungeonMapTestid.packCard(id)}
              label={cardName(content, id)}
              selected={selected.includes(id)}
              onSelect={() => {
                toggle(id);
              }}
            />
          </li>
        ))}
      </ul>
      <div className="dungeon-run-actions">
        <button
          type="button"
          className="button-primary"
          data-testid={dungeonMapTestid.packKeep}
          disabled={selected.length !== phase.keep}
          onClick={() => {
            send({ type: "keep", cards: [...selected] });
          }}
        >
          {t("Keep")}
        </button>
        {skip ? (
          <button
            type="button"
            data-testid={dungeonMapTestid.packSkip}
            onClick={() => {
              send({ type: "keep", cards: [] });
            }}
          >
            {t("Skip this pack")}
          </button>
        ) : null}
      </div>
    </section>
  );
}

type ExtractProps = {
  run: RunState;
  content: RunContent;
  send: SendRunAction;
};

/** After a mini-boss: one of the deck's cards, or none. */
export function ExtractCardScreen({ run, content, send }: ExtractProps): ReactElement {
  return (
    <section className="dungeon-run-extract" data-testid={dungeonMapTestid.extractCard}>
      <h2 className="dungeon-run-title">{t("Extract a card")}</h2>
      <p className="dungeon-run-note">{t("The card leaves the run and is unlocked forever.")}</p>
      <IndexedCardChoices
        run={run}
        content={content}
        testid={(index) => dungeonMapTestid.extractCardChoice(index)}
        label={(card) => cardName(content, card.id)}
        onPick={(index) => {
          send({ type: "extract", index });
        }}
      />
      <div className="dungeon-run-actions">
        <button
          type="button"
          data-testid={dungeonMapTestid.extractNone}
          onClick={() => {
            send({ type: "extract", index: null });
          }}
        >
          {t("Extract nothing")}
        </button>
      </div>
    </section>
  );
}

/**
 * After the final boss: one relic held, or none. The boss's own Unobtainable relic stays a live
 * choice: the run refuses it, and the refusal is shown.
 */
export function ExtractArtifactScreen({ run, content, send }: ExtractProps): ReactElement {
  return (
    <section className="dungeon-run-extract" data-testid={dungeonMapTestid.extractArtifact}>
      <h2 className="dungeon-run-title">{t("Extract a relic")}</h2>
      <p className="dungeon-run-note">{t("The relic leaves the run and is unlocked forever.")}</p>
      <ul className="dungeon-run-choices">
        {run.artifacts.map((artifact, index) => (
          <li key={`${String(index)}-${artifact.id}`}>
            <button
              type="button"
              className="dungeon-run-choice"
              data-testid={dungeonMapTestid.extractArtifactChoice(index)}
              onClick={() => {
                send({ type: "extract", index });
              }}
            >
              <PassiveChoice id={artifact.id} level={artifact.level} content={content} />
            </button>
          </li>
        ))}
      </ul>
      <div className="dungeon-run-actions">
        <button
          type="button"
          data-testid={dungeonMapTestid.extractNone}
          onClick={() => {
            send({ type: "extract", index: null });
          }}
        >
          {t("Extract nothing")}
        </button>
      </div>
    </section>
  );
}
