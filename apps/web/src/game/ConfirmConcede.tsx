// "Concede this game?": what the board's Concede control opens, in every mode. A concede ends the
// game at once and cannot be taken back, and the control sits one stray tap from End turn, so it
// asks first. Only "Concede" sends the action; "Keep playing" is the default: it takes the focus,
// and Escape, Back (R2127000) or a click outside the panel means the same. It decides nothing
// (CLAUDE.md rule 7): the caller dispatches `{ type: "concede" }`. It draws through `Dialog.tsx`
// (R3220000): Tab and Shift+Tab stay between the two buttons, and the caller puts the focus back on
// the Concede control once it closes. A route whose concede costs more than the game (a dungeon fight
// ends its run, R2972000) passes its own body.

import type { ReactElement } from "react";

import { testid } from "./contract.ts";
import { Dialog } from "./Dialog.tsx";
import { t, useLocale } from "../i18n/i18n.ts";

export type ConfirmConcedeProps = {
  /** "Concede": send the concede. */
  onConfirm(): void;
  /** "Keep playing", Escape, Back or a click outside the panel. */
  onCancel(): void;
  /** What the dialog says, already translated; absent, `CONCEDE_BODY`. */
  body?: string;
};

export const CONCEDE_TITLE = "Concede this game?";
export const CONCEDE_BODY = "Your opponent wins and the game ends here.";

export default function ConfirmConcede({ onConfirm, onCancel, body }: ConfirmConcedeProps): ReactElement {
  useLocale();
  return (
    <Dialog
      role="alertdialog"
      testid={testid.concedeDialog}
      title={t(CONCEDE_TITLE)}
      description={body ?? t(CONCEDE_BODY)}
      onClose={onCancel}
      cancel={{ label: t("Keep playing"), kind: "primary", testid: testid.concedeCancel }}
      actions={[{ label: t("Concede"), kind: "danger", onClick: onConfirm, testid: testid.concedeConfirm }]}
    />
  );
}
