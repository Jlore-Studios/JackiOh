// R738: this seat's aim, sent to the opponent as it changes.
//
// It reads the board's own `interaction` — what a drag lifts and what a click-select picks are the
// same state — and the target under the pointer comes from the shared hovered-target reader
// (`useHoveredAimTarget`: the drag layer's own `pickDropSpot` against `aimTargets`), so a drag and a
// hover over click-selected targets aim alike. Only a change is sent (`aimKey`), the server coalesces
// the rest (`AIM_RELAY_INTERVAL_MS`). Nothing here decides a rule. R3615000: the same hovered target
// feeds the attack preview, so this returns it (tracked even where `emit` is undefined and nothing is
// sent, as in hotseat).

import { useEffect, useMemo, useRef } from "react";

import { aimKey, type ActionBody, type Aim, type PlayerView } from "@jackioh/shared";

import type { Interaction } from "../actions.ts";
import { aimFor, aimTargets } from "./aim.ts";
import { useHoveredAimTarget, type HoveredAimTarget } from "./useHoveredAimTarget.ts";

const NONE: ReadonlySet<string> = new Set();

/**
 * Sends `emit` this seat's aim on every change; does nothing when `emit` is undefined (hotseat).
 * Returns the legal target under the pointer while `interaction` aims one, for the attack preview.
 */
export function useAimEmitter(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
  emit: ((aim: Aim | null) => void) | undefined,
): HoveredAimTarget | null {
  const targets = useMemo(
    () => (emit === undefined ? NONE : aimTargets(view, legal, interaction)),
    [emit, view, legal, interaction],
  );
  const hovered = useHoveredAimTarget(view, legal, interaction);

  const over = hovered !== null && targets.has(hovered.testid) ? hovered.target : null;
  const aim = useMemo(() => aimFor(view, interaction, targets, over), [view, interaction, targets, over]);
  const key = aimKey(aim);

  const sent = useRef<{ key: string; emit: typeof emit }>({ key: aimKey(null), emit });
  useEffect(() => {
    if (emit === undefined || sent.current.key === key) return;
    sent.current = { key, emit };
    emit(aim);
    // Keyed on `key`, not `aim`: an equal aim rebuilt by a re-render is never sent twice.
  }, [emit, key, aim]);

  useEffect(
    () => () => {
      const last = sent.current;
      if (last.emit !== undefined && last.key !== aimKey(null)) last.emit(null);
    },
    [],
  );

  return hovered;
}
