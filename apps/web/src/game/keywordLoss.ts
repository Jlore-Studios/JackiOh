// #1598: marks cards losing a keyword (R46 knock-down) with data-lost (CLAUDE.md rule 7).

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { testid, type AnimatingMap, type AnimationFrames } from "./contract.ts";

type KeywordGranted = Extract<GameEvent, { type: "keywordGranted" }>;

const NO_LOSSES: ReadonlySet<string> = new Set();

/** Testids of cards animating a keywordGranted event with lost set. */
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
