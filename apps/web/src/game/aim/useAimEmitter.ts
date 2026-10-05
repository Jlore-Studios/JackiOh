// R660: this seat's aim, sent to the opponent as it changes.
//
// It reads the board's own `interaction` — what a drag lifts and what a click-select picks are the
// same state — and the target under the pointer, read off the DOM with the drag layer's own reader
// (`pickDropSpot`) against `aimTargets`, so a drag and a hover over click-selected targets aim alike.
// Only a change is sent (`aimKey`), the server coalesces the rest (`AIM_RELAY_INTERVAL_MS`), and an
// aim that ends, or a board that unmounts mid-aim, sends `null`. Nothing here decides a rule.

import { useEffect, useMemo, useRef, useState } from "react";

import { aimKey, type ActionBody, type Aim, type PlayerView } from "@jackioh/shared";

import type { Interaction } from "../actions.ts";
import type { ClickTarget } from "../contract.ts";
import { pickDropSpot } from "../drag/targets.ts";
import { aimFor, aimTargets } from "./aim.ts";

type Hovered = { target: ClickTarget; testid: string };

const NONE: ReadonlySet<string> = new Set();

function stackAt(x: number, y: number): readonly Element[] {
  try {
    return document.elementsFromPoint(x, y);
  } catch {
    // jsdom has no elementsFromPoint: nothing is under the pointer.
    return [];
  }
}

/** Sends `emit` this seat's aim on every change; does nothing when `emit` is undefined (hotseat). */
export function useAimEmitter(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
  emit: ((aim: Aim | null) => void) | undefined,
): void {
  const targets = useMemo(
    () => (emit === undefined ? NONE : aimTargets(view, legal, interaction)),
    [emit, view, legal, interaction],
  );
  const [hovered, setHovered] = useState<Hovered | null>(null);
  const latestTargets = useRef(targets);
  latestTargets.current = targets;

  const aiming = targets.size > 0;
  useEffect(() => {
    if (!aiming) return undefined;
    function onMove(event: PointerEvent): void {
      const spot = pickDropSpot(stackAt(event.clientX, event.clientY), latestTargets.current);
      const next = spot.at === "target" ? { target: spot.target, testid: spot.testid } : null;
      setHovered((prev) => (prev?.testid === next?.testid ? prev : next));
    }
    window.addEventListener("pointermove", onMove, { capture: true, passive: true });
    return () => {
      window.removeEventListener("pointermove", onMove, { capture: true });
      setHovered(null);
    };
  }, [aiming]);

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
}
