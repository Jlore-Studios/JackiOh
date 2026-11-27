// The verdict's list of what keeps a deck or a trio from queueing, as the deck editor and the trio
// editor show it under their verdict's title, and the hint that ownership went unchecked when the
// collection could not be read.

import type { ReactElement } from "react";

import type { LoadoutError } from "@jackioh/validator";

import { t } from "../../i18n/i18n.ts";
import { LOADOUT_ERRORS, loadoutErrorId } from "./testids.ts";

export function LoadoutErrorList({
  errors,
  labelledBy,
  collectionRead,
}: {
  errors: readonly LoadoutError[];
  /** The verdict title's id, which names the list. */
  labelledBy: string;
  /** False when `GET /api/collection` could not be read: the list then flags no card as unowned. */
  collectionRead: boolean;
}): ReactElement {
  return (
    <>
      <ul className="db-errors" data-testid={LOADOUT_ERRORS} data-count={String(errors.length)} data-tone="hint" aria-labelledby={labelledBy}>
        {errors.map((issue, position) => (
          <li
            key={`${issue.rule}:${String(issue.deck ?? "")}:${issue.cardId ?? ""}:${String(position)}`}
            data-testid={loadoutErrorId(issue.rule)}
            data-rule={issue.rule}
            data-deck={issue.deck === undefined ? undefined : String(issue.deck)}
            data-card={issue.cardId}
            data-source="client"
          >
            {t(issue.message)}
          </li>
        ))}
      </ul>
      {collectionRead ? null : (
        <p className="ws-hint">{t("Your collection couldn't be loaded, so cards you don't own aren't flagged here.")}</p>
      )}
    </>
  );
}
