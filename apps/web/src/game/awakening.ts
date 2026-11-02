// #2435: which cards the burst has awakened, their Dormant seal broken. The board shows the
// pre-burst view until the burst ends, so an awakened card is still drawn Dormant; its `awakened`
// entry breaks the seal (animations.css) and `data-awakening` keeps it broken until the board
// catches up (R1839002, R1852003). It is drawn state only, never a rule (CLAUDE.md rule 7).

import { createContext, createElement, useContext, type ReactElement, type ReactNode } from "react";

import type { PlayerView } from "@jackioh/shared";

import { testid, type AnimatingMap, type AnimationFrames } from "./contract.ts";

const NONE: ReadonlySet<string> = new Set();

/** The testids of the cards an `awakened` entry of the burst played on (the entry in flight included). Without a burst, the shown view's events against `animating`, as `keywordLossesFrom` reads them. */
export function awakeningsFrom(
  view: PlayerView,
  animating: AnimatingMap | undefined,
  animated?: readonly AnimationFrames[],
): ReadonlySet<string> {
  if (animating === undefined) return NONE;
  const sources: readonly AnimationFrames[] = animated ?? [{ frames: animating, events: view.events }];
  const awake = new Set<string>();
  for (const source of sources) {
    for (const event of source.events) {
      if (event.type !== "awakened") continue;
      const card = testid.card(event.instanceId);
      if (source.frames.get(card) === "awakened") awake.add(card);
    }
  }
  return awake.size === 0 ? NONE : awake;
}

const AwakeningContext = createContext<ReadonlySet<string>>(NONE);

export function AwakeningProvider({ awake, children }: { awake: ReadonlySet<string>; children?: ReactNode }): ReactElement {
  return createElement(AwakeningContext.Provider, { value: awake }, children);
}

export function useAwakening(testId: string | undefined): boolean {
  const awake = useContext(AwakeningContext);
  return testId !== undefined && awake.has(testId);
}
