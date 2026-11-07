// "Leave the match?": what every way off a live online match opens (R2479000). It takes its line
// of consequence as a prop (R2973000): the 1v1 loss line unless given. Leaving closes the
// match socket, and the server's disconnect grace runs: back within DISCONNECT_GRACE_SECONDS, or the
// match counts as a loss (§9.5). Stay is the default: it takes the focus, and Escape, Back (R2127000)
// or a click outside the panel means the same. It decides nothing (CLAUDE.md rule 7): Leave makes
// the move the route's leave guard held (R1754000).
//
// An `alertdialog` drawn through `Dialog.tsx` (R3220000), modal, labelled by its question and described
// by its line of consequence. Its keys are the Dialog's (R2126000: a board route takes no key handler of
// its own).

import type { ReactElement } from "react";

import { DISCONNECT_GRACE_SECONDS } from "@jackioh/server-config";

import { Dialog } from "./Dialog.tsx";
import { t, useLocale } from "../i18n/i18n.ts";

export type ConfirmLeaveMatchProps = {
  /** "Stay", Escape, Back or a click outside the panel. */
  onStay(): void;
  /** "Leave": make the held move. */
  onLeave(): void;
  /** The line of consequence under the question; the 1v1 loss line unless given. */
  body?: string;
};

export const leaveMatchTestid = {
  dialog: "leave-match-dialog",
  stay: "leave-match-stay",
  leave: "leave-match-leave",
} as const;

export default function ConfirmLeaveMatch({ onStay, onLeave, body }: ConfirmLeaveMatchProps): ReactElement {
  useLocale();
  return (
    <Dialog
      role="alertdialog"
      testid={leaveMatchTestid.dialog}
      title={t("Leave the match?")}
      description={
        body ??
        t("It goes on without you. Come back within {seconds} seconds or it counts as a loss.", {
          seconds: DISCONNECT_GRACE_SECONDS,
        })
      }
      onClose={onStay}
      cancel={{ label: t("Stay"), kind: "primary", testid: leaveMatchTestid.stay }}
      actions={[{ label: t("Leave"), kind: "danger", onClick: onLeave, testid: leaveMatchTestid.leave }]}
    />
  );
}
