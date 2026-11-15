// R3838000: the Almanac's and the Statistics page's "Show card text" button, an on/off button in the Segmented's look
// (R3324003). While it is pressed every face on the page prints its normal face, its words on it (`FullArtAllowed`
// false, R1108000), whatever the Full-art cards setting or a pick says. It is the pages' own choice, one for both, kept
// on the device: it never writes the Full-art cards setting, so a game's faces are as they were. CLAUDE.md rule 7.
// R3939000: the Statistics page's card breakdown, a dialog that covers the header, holds it too, below View in Almanac.

import { useState, type ReactElement, type ReactNode } from "react";

import { FullArtAllowed } from "../cards/fullArt.ts";
import { t } from "../i18n/i18n.ts";
import { getStored, setStored } from "../net/storage.ts";

import "../tabs/tabs.css";

/** R3838000: the localStorage key that holds "1" while the Almanac and the Statistics page print every card's text. */
export const CARD_TEXT_KEY = "jackioh.cardText";

export const cardTextTestid = {
  /** R3838000: the button in the page's header. */
  toggle: "card-text-toggle",
  /** R3939000: the button in the Statistics breakdown, below View in Almanac. */
  dialog: "card-text-toggle-dialog",
} as const;

/** R3838000: whether the pages print every card's text, as the device keeps it (off when it keeps nothing). */
export function readCardText(): boolean {
  return getStored(CARD_TEXT_KEY) === "1";
}

/** R3838000: the pages' choice and its toggle. */
export type CardText = { shown: boolean; toggle: () => void };

/** R3838000: the pages' choice and its toggle, which keeps the new choice on the device (in memory where it cannot). */
export function useCardText(): CardText {
  const [shown, setShown] = useState(readCardText);
  return {
    shown,
    toggle: () => {
      const next = !shown;
      setStored(CARD_TEXT_KEY, next ? "1" : "0");
      setShown(next);
    },
  };
}

/**
 * R3838000: the button, pressed while every card prints its text, in the Segmented's look. R3939000: given a
 * `className`, the bare button wearing it, as the Statistics breakdown draws its other buttons.
 */
export function CardTextButton({
  shown,
  onToggle,
  testid = cardTextTestid.toggle,
  className,
}: {
  shown: boolean;
  onToggle: () => void;
  testid?: string;
  className?: string;
}): ReactElement {
  const button = (
    <button
      type="button"
      className={className ?? `segmented__item ${shown ? "segmented__item--selected" : ""}`}
      data-testid={testid}
      aria-pressed={shown}
      title={t("Every card prints its text on its face, with no full art")}
      onClick={onToggle}
    >
      {t("Show card text")}
    </button>
  );
  return className === undefined ? <div className="segmented">{button}</div> : button;
}

/** R3838000: the faces below it print no full art while `shown`. */
export function CardTextFaces({ shown, children }: { shown: boolean; children?: ReactNode }): ReactElement {
  return <FullArtAllowed allowed={!shown}>{children}</FullArtAllowed>;
}
