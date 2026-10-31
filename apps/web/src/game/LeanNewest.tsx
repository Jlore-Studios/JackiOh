// "More cards from" (R1372, R1373, R924000): the picker beside every random deck a player is dealt
// (R2031000). It deals nothing (CLAUDE.md rule 7): with a pick, the dealer leans on that set, at least
// `AI_DECK.leanMinShare` of the deck (R1370). "Newest" names the set from `newestShippedSet()` (R1371)
// and the dealer resolves it when it deals; the other choices are the sets that ship (R1420).
// The online pick (every one but practice's, which `routes/practice.tsx` keeps) is kept on the device
// (`playLean.ts`), so a pick made in one carries to the next. The share is read from WASM (rule 9), so
// a screen that may paint before the engine loads shows the picker through `LeanSetPickerLazy.tsx` (#778).

import { useId, type ReactElement } from "react";

import { AI_DECK } from "@jackioh/ai";
import { DECK_SIZE } from "@jackioh/engine/config";
import { LEAN_NEWEST, isLeanPick, leanPicks, leanSetOf, newestShippedSet, type LeanPick } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import { NO_LEAN } from "./playLean.ts";

import "./leanNewest.css";

export { PLAY_LEAN_NEWEST_KEY, PLAY_LEAN_SET_KEY, readPlayLean, writePlayLean } from "./playLean.ts";

export function leanLabel(): string {
  return t("More cards from");
}

/** R924000: "newest" names the set it is now. */
export function leanChoiceLabel(pick: LeanPick | null): string {
  if (pick === null) return t("No set in particular");
  if (pick === LEAN_NEWEST) return t("The newest set ({set})", { set: t(newestShippedSet()) });
  return t(pick);
}

/** The cards a deck of `size` holds from the set at least (R1370: rounded up, as the deck builder rounds). */
export function leanFloor(size: number = DECK_SIZE): number {
  return Math.ceil(size * AI_DECK.leanMinShare);
}

/** The line under the picker when it holds a pick: what it asks of the deal. */
export function leanNote(pick: LeanPick, size: number = DECK_SIZE): string {
  return t("At least {floor} of your {size} cards come from {set}.", {
    floor: leanFloor(size),
    size,
    set: t(leanSetOf(pick)),
  });
}

export type LeanSetPickerProps = {
  value: LeanPick | null;
  onChange(pick: LeanPick | null): void;
  /** On the `<select>` itself, as the lobby's mode radios carry theirs. */
  testid: string;
  disabled?: boolean;
};

export function LeanSetPicker({ value, onChange, testid, disabled = false }: LeanSetPickerProps): ReactElement {
  const noteId = useId();
  return (
    <label className="lean-newest" data-pick={value ?? "none"}>
      <span className="lean-newest__text">
        <span className="lean-newest__label">{leanLabel()}</span>
        <select
          className="lean-newest__input"
          data-testid={testid}
          value={value ?? NO_LEAN}
          disabled={disabled}
          aria-describedby={value === null ? undefined : noteId}
          onChange={(event) => {
            const picked = event.currentTarget.value;
            onChange(isLeanPick(picked) ? picked : null);
          }}
        >
          <option value={NO_LEAN}>{leanChoiceLabel(null)}</option>
          {leanPicks().map((pick) => (
            <option key={pick} value={pick}>
              {leanChoiceLabel(pick)}
            </option>
          ))}
        </select>
        {value === null ? null : (
          <span className="lean-newest__note" id={noteId}>
            {leanNote(value)}
          </span>
        )}
      </span>
    </label>
  );
}
