// R3615000: the legal target under the pointer, read the way the aim relay reads it.
//
// Picking or lifting an attacker shows no preview yet; once the pointer (or a drag) is over a legal
// target, the attack preview shows for that one attack, and leaving the target takes it away. This is
// the aim relay's own reader (`pickDropSpot` against `aimTargets`), shared with `useAimEmitter`, so no
// second hit test exists. Tracking is ungated by `emit`: hotseat and practice send no aim but still
// preview. A non-target spot sets null, and the effect cleanup clears it when aiming ends.

import { useEffect, useMemo, useRef, useState } from "react";

import type { ActionBody, PlayerView } from "@jackioh/shared";

import type { Interaction } from "../actions.ts";
import type { ClickTarget } from "../contract.ts";
import { pickDropSpot } from "../drag/targets.ts";
import { aimTargets } from "./aim.ts";

export type HoveredAimTarget = { target: ClickTarget; testid: string };

function stackAt(x: number, y: number): readonly Element[] {
  try {
    return document.elementsFromPoint(x, y);
  } catch {
    // jsdom has no elementsFromPoint: nothing is under the pointer.
    return [];
  }
}

/** The legal target under the pointer while `interaction` aims one, or null. */
export function useHoveredAimTarget(
  view: PlayerView,
  legal: readonly ActionBody[],
  interaction: Interaction,
): HoveredAimTarget | null {
  const targets = useMemo(() => aimTargets(view, legal, interaction), [view, legal, interaction]);
  const [hovered, setHovered] = useState<HoveredAimTarget | null>(null);
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

  return hovered !== null && targets.has(hovered.testid) ? hovered : null;
}
