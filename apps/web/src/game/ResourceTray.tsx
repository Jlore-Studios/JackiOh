// A seat's Resource tray and Credits (issue #1840, R1840000, R1840003, R1840005).
//
// `SideView.resourceTray` is a hand's union, `CardView[] | { count: number }`, and the same privacy
// boundary (SPEC §10.8): the viewer's own tray is full cards with `tray-card-<instanceId>` testids,
// the opponent's a number. The narrowing is `Array.isArray`, so a count is never drawn as cards. Once
// the game is over the view shows both trays as cards (R434); only the viewer's own reports a click,
// which `actions.ts` turns into a `play` that `legalActions` lists (R1840003): nothing here decides
// whether a Resource can be played (CLAUDE.md rule 7). `SideView.marketCredits` is public on both
// seats. Each is absent until there is something to show, so a board without either renders nothing.

import type { ReactElement } from "react";

import type { SideView } from "@jackioh/shared";

import Card, { type CardProps } from "./Card.tsx";
import { testid, type Side } from "./contract.ts";
import { t } from "../i18n/i18n.ts";

export type ResourceTrayProps = {
  side: Side;
  tray: SideView["resourceTray"];
  credits: SideView["marketCredits"];
} & Pick<CardProps, "touchHold" | "highlight" | "animating" | "onClick">;

export default function ResourceTray({ side, tray, credits, ...card }: ResourceTrayProps): ReactElement | null {
  if (tray === undefined && credits === undefined) return null;
  return (
    <span className="resource-tray" data-testid={`tray-${side}`}>
      {tray === undefined ? null : Array.isArray(tray) ? (
        tray.map((resource) => (
          <Card
            key={resource.instanceId}
            testId={testid.trayCard(resource.instanceId)}
            card={resource}
            className="card-tray"
            // R434: the opponent's tray, revealed once the game is over, is read and never played.
            target={side === "you" ? { on: "tray", instanceId: resource.instanceId } : null}
            {...card}
          />
        ))
      ) : (
        <span className="pile-n" data-testid={`tray-count-${side}`} title={t("Resources")}>
          {tray.count}
        </span>
      )}
      {credits === undefined ? null : (
        <span className="market-credits" data-testid={`market-credits-${side}`} data-credits={credits} title={t("Credits")}>
          {credits}
        </span>
      )}
    </span>
  );
}
