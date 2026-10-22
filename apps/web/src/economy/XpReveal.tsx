// The XP reveal (#1664, part of #1651, SPEC §14.7–§14.9): what a finished game did to the set tracks,
// shown after every game that paid (the 1v1 result panel, the co-op result and a paid practice
// game's result dialog). Each track the game moved is a bar that fills from where it stood to where
// it stands, level by level; each level it reaches pops its number and flies out what it paid, one
// reward at a time with the sting; each quest the game completed is listed with its pay. A quest
// only ever completes inside a game's payout (§14.9), so this is where a completion shows.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The tracks and the quests are the answer's (`xp.ts`): the
// client computes no curve and picks no reward, it plays the answer. Levels span different XP, so
// a track is played as one bar for each level it passes through (`stagesOf`), each of the size the
// answer gives; the level a bar fills to, and so the rewards that fly out, follow its bursts.
//
// REDUCED MOTION (the panel's setting or the system's) draws every track's end state and every
// reward at once, one sting, as the bars and the reveal do under it.

import { useEffect, useState, type CSSProperties, type ReactElement } from "react";

import { SET_TRACK_LEVELS } from "@jackioh/server-config";
import type { SetName } from "@jackioh/shared";

import { rewardWords } from "../collection/collection.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import type { TrackRewards } from "../net/api.ts";
import type { RevealItem, RevealKind } from "./bars.ts";
import FillBar, { useReducedMotion } from "./FillBar.tsx";
import RewardReveal from "./RewardReveal.tsx";
import { LEVEL_POP_MS, stagesOf, type LevelReached, type QuestPaid, type TrackGain } from "./xp.ts";
import "./economy.css";
import "./xp.css";

export type XpRevealProps = {
  /** The tracks a game moved and the quests it completed; a game's whole answer is accepted. */
  economy: { tracks: readonly TrackGain[]; quests: readonly QuestPaid[] };
  testid?: string;
};

/** What each part of a level's reward is, in the reveal's kinds and the answer's fields. */
const REWARD_PARTS: readonly (readonly [RevealKind, keyof TrackRewards])[] = [
  ["jlore", "jlore"],
  ["setPack", "setPacks"],
  ["cardToken", "cardTokens"],
];

/** What a level paid as the reveal's items, one for each part above 0, in `rewardWords`' words. */
function rewardItems({ level, rewards }: LevelReached, setId: SetName): RevealItem[] {
  return REWARD_PARTS.flatMap(([kind, field]) =>
    rewards[field] > 0
      ? [
          {
            key: `level-${String(level)}-${kind}`,
            kind,
            label: rewardWords({ jlore: 0, setPacks: 0, cardTokens: 0, [field]: rewards[field] }, setId),
          },
        ]
      : [],
  );
}

const levelWords = (level: number): string =>
  t("Level {level} of {levels}", { level: formatNumber(level), levels: formatNumber(SET_TRACK_LEVELS) });

/** The tracks a game moved and the quests it completed, played; nothing when it moved and completed none. */
export default function XpReveal({ economy, testid = "xp-reveal" }: XpRevealProps): ReactElement | null {
  const { tracks, quests } = economy;
  if (tracks.length === 0 && quests.length === 0) return null;
  return (
    <div className="xp-reveal" data-testid={testid}>
      {tracks.length === 0 ? null : (
        <ul className="xp-reveal__tracks">
          {tracks.map((gain) => (
            // A new answer plays from its start; the same one, sent again, plays on.
            <TrackReveal key={`${gain.setId}:${JSON.stringify([gain.before, gain.after])}`} gain={gain} testid={testid} />
          ))}
        </ul>
      )}
      {quests.length === 0 ? null : <Quests quests={quests} testid={testid} />}
    </div>
  );
}

function TrackReveal({ gain, testid }: { gain: TrackGain; testid: string }): ReactElement {
  const { setId, before, after, levels } = gain;
  const reduced = useReducedMotion();
  const stages = stagesOf(gain);
  const [at, setAt] = useState(0);
  const [reached, setReached] = useState(before.level);
  const [done, setDone] = useState(false);

  // Motion off: the end state at once, whatever the bars have played, and kept if motion comes back.
  useEffect(() => {
    if (!reduced) return;
    setAt(stages.length - 1);
    setReached(after.level);
    setDone(true);
  }, [reduced, stages.length, after.level]);
  const index = reduced ? stages.length - 1 : at;
  const stage = stages[index]!;
  const level = reduced ? after.level : reached;
  const final = index === stages.length - 1;
  // A bar that does not move (a track already on its last level) is at rest from the start.
  const settled = reduced || done || (final && stage.from === null);
  // The levels the track stood past before the game (a track the game unlocked pays the one it starts
  // on) show at once; each other level shows as its bar bursts.
  const items = levels.filter((paid) => settled || paid.level <= level).flatMap((paid) => rewardItems(paid, setId));
  const part = (name: string): string => `${testid}-${name}-${setId}`;

  return (
    <li className="xp-reveal__track" data-testid={part("track")} data-state={settled ? "done" : "filling"}>
      <FillBar
        // Two bars of one answer can look alike (a level passed through whole): each plays.
        key={index}
        label={t("{set} track", { set: t(setId) })}
        size={stage.size}
        value={stage.value}
        {...(stage.from === null ? {} : { from: stage.from })}
        fills={stage.fills}
        caption={!final ? "" : after.levelXp === null ? t("Last level reached.") : undefined}
        onFill={() => {
          setReached(stage.level + 1);
        }}
        onDone={() => {
          if (final) setDone(true);
          else setAt(index + 1);
        }}
        testid={part("bar")}
      />
      <span
        key={level}
        className={level === before.level ? "xp-reveal__level" : "xp-reveal__level xp-reveal__level--pop"}
        data-testid={part("level")}
        data-level={level}
        style={{ "--pop-ms": `${String(LEVEL_POP_MS)}ms` } as CSSProperties}
      >
        {levelWords(level)}
      </span>
      <RewardReveal items={items} testid={part("rewards")} />
    </li>
  );
}

function Quests({ quests, testid }: { quests: readonly QuestPaid[]; testid: string }): ReactElement {
  return (
    <div className="xp-reveal__quests">
      <h3 className="xp-reveal__heading">{t("Quests")}</h3>
      <ul className="xp-reveal__quest-list" data-testid={`${testid}-quests`}>
        {quests.map((quest) => (
          <li key={quest.questId} className="xp-reveal__quest" data-testid={`${testid}-quest-${quest.questId}`}>
            <span className="xp-reveal__quest-title">{t(quest.title)}</span>
            <span className="xp-reveal__quest-pay">
              {t("{jlore} Jlore · {xp} XP", { jlore: formatNumber(quest.jlore), xp: formatNumber(quest.xp) })}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
