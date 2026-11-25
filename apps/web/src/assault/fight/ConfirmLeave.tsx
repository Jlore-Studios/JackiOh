// "Leave this fight?": what leaving an Infinite Assault fight in progress asks, since the fight
// waits where it stood and is resumed when the attempt starts it again (R4582001). Staying is the
// default: it has the focus, and Escape, Back (R2127000) or a click outside the panel means stay.
// Leaving makes the move the leave guard held (R1754000). It decides nothing (CLAUDE.md rule 7).
// Modeled on the dungeon fight's question (ConfirmLeaveFight.tsx), which it copies sentence for
// sentence, and drawn, as it is, through `Dialog.tsx` (R3220000).

import type { ReactElement } from "react";

import { Dialog } from "../../game/Dialog.tsx";
import { t } from "../../i18n/i18n.ts";
import { assaultFightTestid } from "./testids.ts";

export type ConfirmLeaveProps = {
  /** "Stay", Escape, Back or a click outside the panel. */
  onStay(): void;
  /** "Leave": make the held move. */
  onLeave(): void;
};

export default function ConfirmLeave({ onStay, onLeave }: ConfirmLeaveProps): ReactElement {
  return (
    <Dialog
      role="alertdialog"
      testid={assaultFightTestid.leaveDialog}
      title={t("Leave this fight?")}
      description={t("It waits here and picks up where you left it.")}
      onClose={onStay}
      cancel={{ label: t("Stay"), kind: "primary", testid: assaultFightTestid.leaveStay }}
      actions={[{ label: t("Leave"), kind: "danger", onClick: onLeave, testid: assaultFightTestid.leaveConfirm }]}
    />
  );
}
