// A popover that hangs off a workshop top-bar button: the deck's history (`DeckHistory.tsx`) and its
// cosmetics (`DeckCosmeticsPicker.tsx`). An outside press and Escape close it; Back does too, through
// the panel's own BackLayer (R2127000).

import { useCallback, useEffect, useRef, useState, type Dispatch, type RefObject, type SetStateAction } from "react";

export function useWorkshopPopover(): {
  open: boolean;
  setOpen: Dispatch<SetStateAction<boolean>>;
  /** Closes the popover, handing the focus back to its button. */
  close: () => void;
  buttonRef: RefObject<HTMLButtonElement | null>;
  /** The button and its panel: a press outside it closes the popover. */
  root: RefObject<HTMLDivElement | null>;
} {
  const [open, setOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const root = useRef<HTMLDivElement>(null);

  // Focus moves before the panel unmounts, so it never falls back to <body> on the way.
  const close = useCallback((): void => {
    setOpen(false);
    buttonRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!open) return undefined;
    const onPointerDown = (event: PointerEvent): void => {
      if (root.current !== null && !root.current.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") close();
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [close, open]);

  return { open, setOpen, close, buttonRef, root };
}
