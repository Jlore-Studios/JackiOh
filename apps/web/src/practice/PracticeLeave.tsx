// "Leave this game?": what "New game" and "Menu" ask while a practice game is still being played, since
// it is minutes of play one stray tap from gone. Staying is the default: it has the focus, and Escape,
// Back (R2127000) or a click outside the panel means stay. R765: a free game's question offers Save and
// leave; a tutorial lesson is never kept (R668), so its question keeps the one way out. It draws through
// `Dialog.tsx` (R3220000): Stay first, Save and leave between, the way out last.

import type { ReactElement } from "react";

import { Dialog, type DialogAction } from "../game/Dialog.tsx";
import { t } from "../i18n/i18n.ts";
import { practiceTestid } from "./testids.ts";

/** Where leaving goes: "New game", "Menu", or from a tutorial lesson "Exit tutorial". */
export type PracticeLeaveTo = "setup" | "menu" | "lessons";

type PracticeLeaveProps = {
  to: PracticeLeaveTo;
  onStay(): void;
  /** Leave and give the game up. */
  onLeave(): void;
  /** R765: leave and keep the game; offered only when given (a free game, never a lesson). */
  onSaveAndLeave?: () => void;
};

/** Where the player lands, in the question's own words. */
const LEAVE_TO: Record<PracticeLeaveTo, string> = {
  setup: "choosing a difficulty and a deck",
  menu: "the main menu",
  lessons: "the lessons",
};

const LEAVE_BODY: Record<PracticeLeaveTo, string> = {
  setup: `It ends here and you go back to ${LEAVE_TO.setup}.`,
  menu: `It ends here and you go back to ${LEAVE_TO.menu}.`,
  lessons: `It ends here and you go back to ${LEAVE_TO.lessons}. You can start it again any time.`,
};

/** R765: a free game's question, which can keep it. */
function saveBody(to: PracticeLeaveTo): string {
  return `You go back to ${LEAVE_TO[to]}. Save it to pick it up later from the practice menu, or leave without saving and it ends here.`;
}

export function PracticeLeave({ to, onStay, onLeave, onSaveAndLeave }: PracticeLeaveProps): ReactElement {
  const canSave = onSaveAndLeave !== undefined;
  const actions: DialogAction[] = [
    ...(canSave
      ? [{ label: t("Save and leave"), kind: "secondary" as const, onClick: onSaveAndLeave, testid: practiceTestid.leaveSave }]
      : []),
    {
      label: canSave ? t("Leave without saving") : t("Leave"),
      kind: "danger",
      onClick: onLeave,
      testid: practiceTestid.leaveConfirm,
    },
  ];

  return (
    <Dialog
      role="alertdialog"
      testid={practiceTestid.leave}
      data={{ "data-can-save": canSave ? "true" : "false" }}
      title={t("Leave this game?")}
      description={t(canSave ? saveBody(to) : LEAVE_BODY[to])}
      onClose={onStay}
      cancel={{ label: t("Stay"), kind: "primary", testid: practiceTestid.leaveStay }}
      actions={actions}
    />
  );
}
