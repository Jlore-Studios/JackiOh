// R3887000: the Statistics page's "Plain text" button, an on/off button in the Segmented's look (R3324003), and a card
// as plain text. While it is pressed the Cards tab prints each card it lists as readable text instead of a face: its
// name, cost, type and tags, its Attack and Health, its whole rules text and its Radiant text, all from the face
// model (jackioh-shared `face`, #3106) through `RulesText`, so terms, Radiant marks and reference names print as they
// do everywhere else. The page's own choice, kept on the device apart from "Show card text" (cardText.tsx): it never
// writes the Full-art cards setting or touches a game. CLAUDE.md rule 7: presentation of the public catalog only.

import { useMemo, useState, type ReactElement } from "react";

import type { CardDef } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { faceModel, type FaceModel } from "../cards/model.ts";
import { RulesText } from "../cards/RulesText.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { getStored, setStored } from "../net/storage.ts";
import { statsTestid } from "./testids.ts";

import "./plainText.css";

/** R3887000: the localStorage key that holds "1" while the Statistics page prints its cards as plain text. */
export const PLAIN_TEXT_KEY = "jackioh.plainText";

/** R3887000: whether the page prints its cards as plain text, as the device keeps it (off when it keeps nothing). */
export function readPlainText(): boolean {
  return getStored(PLAIN_TEXT_KEY) === "1";
}

/** R3887000: the page's choice and its toggle. */
export type PlainText = { shown: boolean; toggle: () => void };

/** R3887000: the page's choice and its toggle, which keeps the new choice on the device (in memory where it cannot). */
export function usePlainText(): PlainText {
  const [shown, setShown] = useState(readPlainText);
  return {
    shown,
    toggle: () => {
      const next = !shown;
      setStored(PLAIN_TEXT_KEY, next ? "1" : "0");
      setShown(next);
    },
  };
}

/** R3887000: the button, pressed while the page's cards print as plain text. */
export function PlainTextButton({ shown, onToggle }: { shown: boolean; onToggle: () => void }): ReactElement {
  return (
    <div className="segmented">
      <button
        type="button"
        className={`segmented__item stats-view-btn ${shown ? "segmented__item--selected" : ""}`}
        data-testid={statsTestid.plainTextToggle}
        aria-pressed={shown}
        title={t("Every card prints as plain text, with no face")}
        onClick={onToggle}
      >
        {t("Plain text")}
      </button>
    </div>
  );
}

/** One face's lines: a label and a type line when given, its Attack and Health, its whole text, a Resource's Credits. */
function FaceLines({ face, label, typeLine }: { face: FaceModel; label?: string; typeLine?: string }): ReactElement {
  return (
    <div className="plain-card__face" data-face={face.radiant ? "radiant" : "base"}>
      {label === undefined ? null : <p className="plain-card__label">{label}</p>}
      {typeLine === undefined ? null : <p className="plain-card__line">{typeLine}</p>}
      {face.stats === null ? null : (
        <p className="plain-card__stats">
          {`${t("Attack")} ${face.stats.attack} · ${t("Health")} ${face.stats.health}`}
        </p>
      )}
      {face.text.full === "" ? null : (
        <p className="plain-card__text">
          <RulesText rich={face.text} />
        </p>
      )}
      {face.creditsLine === "" ? null : <p className="plain-card__text">{face.creditsLine}</p>}
    </div>
  );
}

/**
 * R3887000: a catalog card as plain text: its name, cost, type and tags, then its base face (Attack and Health, whole
 * text) and its Radiant face (marked, its own type when that differs). A printed face has no gained line or grants.
 */
export function PlainCard({ def }: { def: CardDef }): ReactElement {
  const locale = useLocale();
  const { base, radiant } = useMemo(() => {
    // The faces are the reader's language's: a language switch rebuilds them.
    void locale;
    return {
      base: faceModel({ defId: def.id, def, radiant: false }),
      radiant: faceModel({ defId: def.id, def, radiant: true }),
    };
  }, [def, locale]);
  return (
    <div className="plain-card" data-testid={statsTestid.plainCard} data-card-id={def.id}>
      <p className="plain-card__head">
        <strong className="plain-card__name">{readerName(def)}</strong>
        <span className="plain-card__cost">{`${t("Cost")} ${base.cost.text}`}</span>
      </p>
      <p className="plain-card__line">{[base.typeLine, ...base.tagChips.map((chip) => chip.text)].join(" · ")}</p>
      <FaceLines face={base} />
      <FaceLines
        face={radiant}
        label={t("Radiant")}
        typeLine={radiant.typeLine !== base.typeLine ? radiant.typeLine : undefined}
      />
    </div>
  );
}
