// R2118000: every board is Game.tsx, which marks itself here, so the shell mounts no command
// menu over a practice game or a replay, which share paths with menus.

import { useEffect, useSyncExternalStore } from "react";

let boards = 0;
const listeners = new Set<() => void>();

function emit(): void {
  for (const listener of [...listeners]) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function shown(): boolean {
  return boards > 0;
}

/** R2118000: marks a mounted board, so the shell knows one is up. One mark per mount. */
export function useMarkBoardShown(): void {
  useEffect(() => {
    boards += 1;
    emit();
    return () => {
      boards -= 1;
      emit();
    };
  }, []);
}

export function useBoardShown(): boolean {
  return useSyncExternalStore(subscribe, shown, shown);
}
