// The viewer's hovered hand card (#3478): the `string | null` hover state lives here instead of in
// `Board`'s own state, so sliding the pointer across the hand re-renders only the field's ringed
// cards (#991, R583, SPEC §10.10) instead of the whole board. Only the field `Zone`s read the value;
// `Hand` reports through the setter. The defaults (`null` / noop) keep bare `<Hand>` and `<Zone>`
// mounts working: hover reporting silently no-ops there, rings fall back to selected-only.

import { createContext, useContext, useState, type ReactElement, type ReactNode } from "react";

const HoveredHandCardContext = createContext<string | null>(null);

const SetHoveredHandCardContext = createContext<(hovered: string | null) => void>(() => {});

export function HoveredHandCardProvider({ children }: { children?: ReactNode }): ReactElement {
  const [hovered, setHovered] = useState<string | null>(null);
  return (
    <HoveredHandCardContext.Provider value={hovered}>
      <SetHoveredHandCardContext.Provider value={setHovered}>{children}</SetHoveredHandCardContext.Provider>
    </HoveredHandCardContext.Provider>
  );
}

/** The hand card under the pointer, or null. Read only where the #991 rings are drawn. */
export function useHoveredHandCard(): string | null {
  return useContext(HoveredHandCardContext);
}

/** Report the hand card under the pointer. Stable, so reporters never re-render on a hover. */
export function useSetHoveredHandCard(): (hovered: string | null) => void {
  return useContext(SetHoveredHandCardContext);
}
