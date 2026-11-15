// R3910000: the words BUILD M5-T4 promises for `attackCancelled` ("Canceled") and `countered` ("Countered"), drawn by the
// board itself on the element the entry animates, so they show with the effects Off (the effects layer is `aria-hidden`
// and draws nothing there, R200). The tag holds only the event's type, already in the animating map: no hidden card is
// named (R97). It mounts while the entry plays and goes with its `data-animating`.

import type { ReactElement } from "react";

import type { GameEventType } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import "./overflow.css";

/** e2e contract (R3910000): the tag on the element `host` names (`card-<id>`, `hand-card-<id>`, `graveyard-<side>`). */
export const eventTagTestid = (host: string): string => `event-tag-${host}`;

export function EventTag({ host, animating }: { host: string | undefined; animating: GameEventType | undefined }): ReactElement | null {
  if (host === undefined) return null;
  if (animating === "attackCancelled") {
    return (
      <span className="cancel-tag" data-testid={eventTagTestid(host)}>
        {t("Canceled")}
      </span>
    );
  }
  if (animating === "countered") {
    return (
      <span className="counter-tag" data-testid={eventTagTestid(host)}>
        {t("Countered")}
      </span>
    );
  }
  return null;
}
