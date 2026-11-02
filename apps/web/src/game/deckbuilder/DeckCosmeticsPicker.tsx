// R1297000: the open deck's own playmat, backdrop and card back, behind a top-bar button. Three
// inline selects in the sidebar push the verdict and the pool's first row off the first screen
// (BUILD M5-T1), so the picks live in a popover like the settings gear's (SettingsButton.tsx).
// R1833000: each select offers the starters and the account cosmetics the account owns while
// ownership decides (`useOwnsAccountCosmetic`), and a stored pick it does not own shows as the
// kind's default, which is what the board draws.

import { useCallback, useEffect, useRef, useState, type ReactElement } from "react";

import { DEFAULT_CARD_COSMETICS, cosmeticsOf, resolveCosmetic, useOwnsAccountCosmetic, type DeckSlotKind } from "../../cards/cosmetics.ts";
import { t, useLocale } from "../../i18n/i18n.ts";
import { BackLayer } from "../../net/backLayers.ts";
import { DECK_COSMETICS_BUTTON } from "./testids.ts";
import type { DeckItem } from "./sync.ts";

const DECK_COSMETIC_FIELDS: readonly { kind: DeckSlotKind; key: "tableCosmetic" | "fieldCosmetic" | "cardBackCosmetic"; label: string }[] = [
  { kind: "table", key: "tableCosmetic", label: "Playmat" },
  { kind: "field", key: "fieldCosmetic", label: "Backdrop" },
  { kind: "cardBack", key: "cardBackCosmetic", label: "Card back" },
];

/** A four-point sparkle: deck cosmetics, drawn like the settings gear (24 px, current color). */
const SPARKLE_PATH =
  "M12 2.5 C12.9 8.4 15.6 11.1 21.5 12 C15.6 12.9 12.9 15.6 12 21.5 C11.1 15.6 8.4 12.9 2.5 12 C8.4 11.1 11.1 8.4 12 2.5 Z";

export default function DeckCosmeticsPicker({
  deck,
  onCosmetic,
}: {
  deck: DeckItem;
  /** R1297000: a playmat, backdrop or card back pick is saved through the deck upsert; null clears it. */
  onCosmetic: (kind: DeckSlotKind, id: string | null) => void;
}): ReactElement {
  useLocale();
  const owns = useOwnsAccountCosmetic();
  const [open, setOpen] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const root = useRef<HTMLDivElement>(null);

  // Focus moves before the panel unmounts, so it never falls back to <body> on the way.
  const close = useCallback((): void => {
    setOpen(false);
    buttonRef.current?.focus();
  }, []);

  // Outside press and Escape close the picker, like the portrait picker's; Back does too (R2127000, the panel's BackLayer).
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

  return (
    <div className="ws-cosmetics" ref={root}>
      <button
        ref={buttonRef}
        type="button"
        className="ws-cosmetics-button"
        data-testid={DECK_COSMETICS_BUTTON}
        aria-label={t("Deck cosmetics")}
        title={t("Deck cosmetics")}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => {
          if (open) close();
          else setOpen(true);
        }}
      >
        <span className="ws-cosmetics-button__icon" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path d={SPARKLE_PATH} fill="currentColor" />
          </svg>
        </span>
      </button>
      {open ? (
        <div className="ws-cosmetics-panel" role="dialog" aria-label={t("Deck cosmetics")}>
          <BackLayer onClose={close} />
          {/* R1297000: the deck's own playmat, backdrop and card back; "As equipped in Collection" leaves
              the pick to the account's, equipped in the collection's Cosmetics section (R1579000), then
              the default. */}
          {DECK_COSMETIC_FIELDS.map((slot) => {
            const pick = deck[slot.key] ?? null;
            const resolved = pick === null ? null : resolveCosmetic(slot.kind, pick);
            return (
              <label className="ws-compare-pick" key={slot.kind}>
                <span className="ws-field-label">{t(slot.label)}</span>
                <select
                  className="ws-select"
                  data-testid={`deck-cosmetic-${slot.kind}`}
                  value={resolved === null ? "" : owns(resolved) ? resolved : DEFAULT_CARD_COSMETICS[slot.kind]}
                  onChange={(event) => {
                    onCosmetic(slot.kind, event.target.value === "" ? null : event.target.value);
                  }}
                >
                  <option value="">{t("As equipped in Collection")}</option>
                  {cosmeticsOf(slot.kind)
                    .filter((cosmetic) => owns(cosmetic.id))
                    .map((cosmetic) => (
                      <option key={cosmetic.id} value={cosmetic.id}>
                        {t(cosmetic.name)}
                      </option>
                    ))}
                </select>
              </label>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
