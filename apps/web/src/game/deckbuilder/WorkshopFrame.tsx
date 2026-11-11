// R3228000: the workshop's frame, one component for every state of `/decks`. The route draws it around
// the skeleton, the error and the banned notice, and the loaded workshop draws it around the editor, so
// the bar, the title and the background stand where they are when the decks land and only the body
// changes. Callers leave `testId`, `view` and `bar` out when they have none, never passing `undefined`.

import type { ReactElement, ReactNode } from "react";

import "../../auth/tavern.css";
import "./deckbuilder.css";
import "./workshop.css";
import { t, useLocale } from "../../i18n/i18n.ts";
import { paths } from "../../net/navigate.ts";
import { BackLink } from "../../routes/nav.tsx";

export type WorkshopFrameProps = {
  /** The bar's extras, drawn after the title (the open deck's history, cosmetics and sync line). */
  bar?: ReactNode;
  view?: "list" | "editor";
  testId?: string;
  children: ReactNode;
};

export function WorkshopFrame({ bar, view, testId, children }: WorkshopFrameProps): ReactElement {
  useLocale();
  return (
    <div className="app-shell app-shell--wide tavern deckbuilder workshop" data-testid={testId} data-view={view}>
      <header className="db-header">
        <BackLink to={paths.collection} title={t("Decks")}>
          {bar}
        </BackLink>
      </header>
      {children}
    </div>
  );
}
