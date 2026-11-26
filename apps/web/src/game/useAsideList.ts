// A list that opens from a button beside a hero, as a fight's trays do: a dungeon run's relics
// (`dungeon/fight/ArtifactTray.tsx`) and an Infinite Assault level's boosts (`assault/fight/UpgradeTray.tsx`).
// Each tray reads its own Escape; Back closes it through the list's own BackLayer (R2127000).

import { useCallback, useEffect, useId, useRef, useState, type Dispatch, type RefObject, type SetStateAction } from "react";

export function useAsideList(): {
  open: boolean;
  setOpen: Dispatch<SetStateAction<boolean>>;
  /** The list's id, for the button's `aria-controls`. */
  listId: string;
  /** The tray: a press outside it closes the list. */
  root: RefObject<HTMLDivElement | null>;
  button: RefObject<HTMLButtonElement | null>;
  close: () => void;
} {
  const [open, setOpen] = useState(false);
  const listId = useId();
  const root = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);

  /** Escape and Back close the list and put the focus back on the button that opened it. */
  const close = useCallback(() => {
    setOpen(false);
    button.current?.focus();
  }, []);

  // A press anywhere else closes the list, as the board's other menus do (ModifierList.tsx).
  useEffect(() => {
    if (!open) return;
    function onPointer(event: PointerEvent): void {
      if (event.target instanceof Node && root.current?.contains(event.target) === true) return;
      setOpen(false);
    }
    window.addEventListener("pointerdown", onPointer);
    return () => {
      window.removeEventListener("pointerdown", onPointer);
    };
  }, [open]);

  return { open, setOpen, listId, root, button, close };
}
