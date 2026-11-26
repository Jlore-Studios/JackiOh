// The Main menu button of an end-of-game result (issue #4742, R4742000): one label, `t("Main menu")`, for
// every mode's result. It decides nothing (CLAUDE.md rule 7): the caller's `onClick` is where it leaves for,
// and each result puts it after its own ways on, so Continue or Play again keeps the focus and the primary look.
// A bare `<button>` wears the secondary look by itself (buttons.css), and the panels that hold one style it
// as a direct child (`.result-overlay__actions > button`, `.dungeon-run-actions button`), so it takes no class
// and no wrapper. A Dialog result takes `mainMenuAction` for its row of answers instead.

import type { ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import type { DialogAction } from "./Dialog.tsx";

export function MainMenuButton({ testid, onClick }: { testid: string; onClick(): void }): ReactElement {
  useLocale();
  return (
    <button type="button" data-testid={testid} onClick={onClick}>
      {t("Main menu")}
    </button>
  );
}

/** The same button as an answer in a `Dialog`'s row (the practice and the lesson result, R3220000, R3221000). */
export function mainMenuAction(onClick: () => void, testid: string): DialogAction {
  return { label: t("Main menu"), kind: "secondary", onClick, testid };
}
