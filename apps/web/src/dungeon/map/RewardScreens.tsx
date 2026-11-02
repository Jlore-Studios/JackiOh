// The dungeon run's rewards (DR6, DR14 part F, #2707): the relic a fight dropped, taken or left
// before the four packs; one opened pack, keeping exactly `keep` of its cards; and both
// extractions, a card after a mini-boss and a relic after the final boss. Players read "relic";
// the code keeps the run's word, artifact (WRITING.md's jargon). Presentation only (CLAUDE.md
// rule 7): the run answers every rule, and every word goes through `t()` at render.

import { useState, type ReactElement } from "react";

import type { RunContent, RunPackKind, RunPhase, RunState } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
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

/** A pack's label: its kind, translated. */
export function packLabel(kind: RunPackKind): string {
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
export function RewardScreen({ content, phase, send }: RewardScreenProps): ReactElement {
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
  return (
    <section className="dungeon-run-reward" data-testid={dungeonMapTestid.reward}>
      <h2 className="dungeon-run-title">{t("Choose a pack")}</h2>
      <ul className="dungeon-run-choices" data-testid={dungeonMapTestid.packs}>
        {phase.packs.map((kind, index) => (
          <li key={index}>
            <button
              type="button"
              className="dungeon-run-choice"
              data-testid={dungeonMapTestid.pack(index)}
              onClick={() => {
                send({ type: "openPack", index });
              }}
            >
              {packLabel(kind)}
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
  const toggle = (id: string): void => {
    setSelected((was) => {
      if (was.includes(id)) return was.filter((kept) => kept !== id);
      if (was.length >= phase.keep) return was;
      return [...was, id];
    });
  };
  const skip = hasSkipPassive(run, content);
  return (
    <section className="dungeon-run-pack" data-testid={dungeonMapTestid.packScreen}>
      <h2 className="dungeon-run-title">{t("Keep {count} cards", { count: String(phase.keep) })}</h2>
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
