// "Leave this fight?": what leaving a dungeon run's fight in progress asks, since the fight
// waits where it stood and is resumed when the run starts it again (R3476000). Staying is the
// default: it has the focus, and Escape, Back (R2127000) or a click outside the panel means stay.
// Leaving makes the move the leave guard held (R1754000). It decides nothing (CLAUDE.md rule 7).
// Modeled on practice's question (PracticeLeave.tsx), minus the Save and leave a dungeon fight
// never has (R2972000), and drawn, as it is, through `Dialog.tsx` (R3220000).

import type { ReactElement } from "react";

import { Dialog } from "../../game/Dialog.tsx";
import { t } from "../../i18n/i18n.ts";
import { fightTestid } from "./testids.ts";

export type ConfirmLeaveFightProps = {
  /** "Stay", Escape, Back or a click outside the panel. */
  onStay(): void;
  /** "Leave": make the held move. */
  onLeave(): void;
};

export default function ConfirmLeaveFight({ onStay, onLeave }: ConfirmLeaveFightProps): ReactElement {
  return (
    <Dialog
      role="alertdialog"
      testid={fightTestid.leaveDialog}
      title={t("Leave this fight?")}
      description={t("It waits here and picks up where you left it.")}
      onClose={onStay}
      cancel={{ label: t("Stay"), kind: "primary", testid: fightTestid.leaveStay }}
      actions={[{ label: t("Leave"), kind: "danger", onClick: onLeave, testid: fightTestid.leaveConfirm }]}
    />
  );
}
