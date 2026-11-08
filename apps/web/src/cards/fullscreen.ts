// R3345000: full-screen art view state and trigger hooks for Mythics and full-art levels.
// Presentation only (CLAUDE.md rule 7).

import { useCallback, useRef, useSyncExternalStore } from "react";
import type { Rarity } from "@jackioh/shared";

import { FULLSCREEN_HOLD_MS, FULLSCREEN_SLOP_PX } from "./constants.ts";
import { useCardLook } from "./fullArt.ts";
import { isFullBleed } from "./versions.ts";

export type FullScreenState = {
  defId: string;
  radiant?: boolean;
  name?: string;
  heldPointerId?: number | null;
} | null;

let currentState: FullScreenState = null;
const listeners = new Set<() => void>();

export function openFullScreenArt(state: NonNullable<FullScreenState>): void {
  currentState = state;
  for (const listener of listeners) listener();
}

export function closeFullScreenArt(): void {
  if (currentState === null) return;
  currentState = null;
  for (const listener of listeners) listener();
}

export function useFullScreenArt(): FullScreenState {
  return useSyncExternalStore(
    (onStoreChange) => {
      listeners.add(onStoreChange);
      return () => listeners.delete(onStoreChange);
    },
    () => currentState,
  );
}

/**
 * R3345000: whether a card can open the full-screen view: any Mythic (printed or current rarity),
 * or any card wearing a full-art-shaped level/version.
 */
export function canOpenFullScreen(card: {
  rarity?: Rarity | string | null;
  printedRarity?: Rarity | string | null;
  version?: string | null;
  level?: string | null;
}): boolean {
  if (card.rarity === "Mythic" || card.printedRarity === "Mythic") return true;
  if (isFullBleed(card.level ?? null) || isFullBleed(card.version ?? null)) return true;
  return false;
}

/**
 * R3345000: `canOpenFullScreen` for a copy of `defId` as it prints: the level it wears (`useCardLook`, a viewer's own
 * copy when `worn`) counts, so a Common wearing a full-art level opens as a Mythic does. A level or version the
 * caller names itself wins over the worn one.
 */
export function useCanOpenFullScreen(
  card: Parameters<typeof canOpenFullScreen>[0] | null,
  defId: string,
  worn: boolean = false,
): boolean {
  const look = useCardLook(defId, worn);
  return card !== null && canOpenFullScreen({ ...card, level: card.level ?? look.level, version: card.version ?? look.version });
}

export type FullScreenTriggerOptions = {
  /** The card is the viewer's own copy, so it wears the viewer's picks (R3345000). */
  worn?: boolean;
  enabled?: boolean;
  onOpen?: () => void;
};

/**
 * R3345000: long-press (phone) or click-and-hold (desktop) trigger to open full-screen view.
 */
export function useFullScreenTrigger(
  card: {
    defId: string;
    radiant?: boolean;
    name?: string;
    rarity?: Rarity | string | null;
    printedRarity?: Rarity | string | null;
    version?: string | null;
    level?: string | null;
  } | null,
  options: FullScreenTriggerOptions = {},
) {
  const wearsIt = useCanOpenFullScreen(card, card?.defId ?? "", options.worn);
  const allowed = wearsIt && (options.enabled ?? true);
  const timerRef = useRef<number | null>(null);
  const startRef = useRef<{ x: number; y: number; pointerId: number } | null>(null);
  const cardRef = useRef(card);
  cardRef.current = card;

  const clear = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    startRef.current = null;
  }, []);

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (!allowed || (e.pointerType === "mouse" && e.button !== 0)) return;
      clear();
      startRef.current = { x: e.clientX, y: e.clientY, pointerId: e.pointerId };
      timerRef.current = window.setTimeout(() => {
        timerRef.current = null;
        const current = cardRef.current;
        if (current) {
          openFullScreenArt({
            defId: current.defId,
            radiant: current.radiant,
            name: current.name,
            heldPointerId: e.pointerId,
          });
          options.onOpen?.();
        }
      }, FULLSCREEN_HOLD_MS);
    },
    [allowed, clear, options],
  );

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      const start = startRef.current;
      if (start === null || timerRef.current === null) return;
      if (Math.hypot(e.clientX - start.x, e.clientY - start.y) > FULLSCREEN_SLOP_PX) {
        clear();
      }
    },
    [clear],
  );

  const onPointerUp = useCallback(() => {
    clear();
  }, [clear]);

  const onPointerCancel = useCallback(() => {
    clear();
  }, [clear]);

  return {
    handlers: allowed
      ? {
          onPointerDown,
          onPointerMove,
          onPointerUp,
          onPointerCancel,
        }
      : {},
  };
}
