// #1598: which cards are animating a keyword they lose rather than one they gain.
//
// R46's knock-down reports the Taunt it takes as a `keywordGranted` with `lost` set. The animation
// table keys its rows by event type, so a loss plays on the grant's row and the board writes the same
// `data-animating="keywordGranted"` (contract.ts). The board reads the events being animated, as its
// number pops do (`popsFrom`, Board.tsx), and a card whose `keywordGranted` in flight is a loss also
// carries `data-lost`, which animations.css plays as the icon going. Drawn state only, never a rule
// (CLAUDE.md rule 7).

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { testid, type AnimatingMap, type AnimationFrames } from "./contract.ts";

type KeywordGranted = Extract<GameEvent, { type: "keywordGranted" }>;

const NO_LOSSES: ReadonlySet<string> = new Set();

/**
 * The testids of the cards animating a `keywordGranted` with `lost` set. `animated` is the burst the
 * runner has played since the board last caught up: the newest entry marking a card is the one on it
 * now, and within it the last matching event wins. Without a burst it reads the shown view's events,
 * as `popsFrom` does for a board rendered straight out of a fixture.
 */
export function keywordLossesFrom(
  view: PlayerView,
  animating: AnimatingMap | undefined,
  animated?: readonly AnimationFrames[],
): ReadonlySet<string> {
  if (animating === undefined) return NO_LOSSES;
  const sources: readonly AnimationFrames[] = animated ?? [{ frames: animating, events: view.events }];
  const lost = new Set<string>();
  for (const [testId, type] of animating) {
    if (type !== "keywordGranted") continue;
    const source = sources.findLast((candidate) => candidate.frames.get(testId) === "keywordGranted");
    const event = source?.events.findLast(
      (candidate): candidate is KeywordGranted =>
        candidate.type === "keywordGranted" &&
        (testId === testid.card(candidate.instanceId) || testId === testid.handCard(candidate.instanceId)),
    );
    if (event?.lost === true) lost.add(testId);
  }
  return lost;
}

const KeywordLossContext = createContext<ReadonlySet<string>>(NO_LOSSES);

export function KeywordLossProvider({ lost, children }: { lost: ReadonlySet<string>; children?: ReactNode }): ReactElement {
  return createElement(KeywordLossContext.Provider, { value: lost }, children);
}

/** Whether the card under this testid is animating a keyword it loses. */
export function useKeywordLost(testId: string | undefined): boolean {
  const lost = useContext(KeywordLossContext);
  return testId !== undefined && lost.has(testId);
}
