// The hooks that pace the AI at a practice table (SPEC §9.9): the practice route
// (`routes/practice.tsx`) and a dungeon fight's screen (`dungeon/fight/FightScreen.tsx`, SPEC §15, #2925)
// both use them. The AI's next step waits while the board animates (`data-animating`), while the card it
// just played is held up (`data-showcase`) and while a voice line plays (`data-speaking`).

import { useEffect, useSyncExternalStore } from "react";

import { PRACTICE_SHOWCASE_HOLD_MAX_MS, PRACTICE_VOICE_HOLD_MAX_MS } from "./config.ts";
import { PRACTICE_IDLE_STATE, type PracticeController, type PracticeControllerState } from "./controller.ts";

/**
 * Tell the controller whether the board is still animating, so the AI's next step waits for the player to
 * see the last one. An element carries `data-animating` for as long as the runner holds a view back (BUILD
 * M5-T4), which is also what `cy.settled()` waits on.
 */
export function useBoardBusy(root: HTMLElement | null, controller: PracticeController | null): void {
  useEffect(() => {
    if (controller === null || root === null || typeof MutationObserver !== "function") return;
    const report = (): void => {
      controller.setBoardBusy(root.querySelector("[data-animating]") !== null);
    };
    const observer = new MutationObserver(report);
    observer.observe(root, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-animating"] });
    report();
    return () => {
      observer.disconnect();
    };
  }, [root, controller]);
}

/**
 * Hold the AI's next step while an element anywhere in the page carries `attribute` (SPEC §9.9: AI turns
 * are paced so the player can follow them), for at most `maxMs` per mark, so a stuck mark can slow a turn
 * but never stop the game; it is ignored from then on until the page drops it.
 */
export function useMarkHold(controller: PracticeController | null, reason: string, attribute: string, maxMs: number): void {
  useEffect(() => {
    if (controller === null || typeof MutationObserver !== "function") return;
    const root = document.documentElement;
    const selector = `[${attribute}]`;
    let cap: ReturnType<typeof setTimeout> | null = null;
    /** The cap ran out on this mark; it is ignored until the page drops it. */
    let expired = false;
    const release = (): void => {
      if (cap !== null) clearTimeout(cap);
      cap = null;
      controller.setHold(reason, false);
    };
    const report = (): void => {
      const marked = root.matches(selector) || root.querySelector(selector) !== null;
      if (!marked) {
        expired = false;
        release();
        return;
      }
      if (expired || cap !== null) return;
      controller.setHold(reason, true);
      cap = setTimeout(() => {
        expired = true;
        release();
      }, maxMs);
    };
    const observer = new MutationObserver(report);
    observer.observe(root, { subtree: true, childList: true, attributes: true, attributeFilter: [attribute] });
    report();
    return () => {
      observer.disconnect();
      release();
    };
  }, [controller, reason, attribute, maxMs]);
}

/** Hold the AI's next step while a voice line plays (`data-speaking`, polish task 2). */
export function useVoiceHold(controller: PracticeController | null): void {
  useMarkHold(controller, "voice", "data-speaking", PRACTICE_VOICE_HOLD_MAX_MS);
}

/** Hold the AI's next step while its last card is held up (`data-showcase`), so it never plays over it. */
export function useShowcaseHold(controller: PracticeController | null): void {
  useMarkHold(controller, "showcase", "data-showcase", PRACTICE_SHOWCASE_HOLD_MAX_MS);
}

const noSubscribe = (): (() => void) => () => {};
const idleState = (): PracticeControllerState => PRACTICE_IDLE_STATE;

export function useControllerState(controller: PracticeController | null): PracticeControllerState {
  return useSyncExternalStore(
    controller === null ? noSubscribe : controller.subscribe,
    controller === null ? idleState : controller.getState,
    controller === null ? idleState : controller.getState,
  );
}
