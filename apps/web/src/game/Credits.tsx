// A seat's Credits (issue #1840, R1840005): a public count on both seats, which the Marketplace spends
// (R1853003) and a Resource played or discarded from the hand earns (R2250001). Absent until first
// gained, so a board without them renders nothing. A Resource is a hand card since #2250 (R2250000), so
// the hand draws it; the Resource tray this component once drew is gone.

import type { ReactElement } from "react";

import type { SideView } from "@jackioh/shared";

import type { Side } from "./contract.ts";
import { t } from "../i18n/i18n.ts";

export type CreditsProps = {
  side: Side;
  credits: SideView["marketCredits"];
};

export default function Credits({ side, credits }: CreditsProps): ReactElement | null {
  if (credits === undefined) return null;
  return (
    <span className="resource-tray" data-testid={`credits-${side}`}>
      <span className="market-credits" data-testid={`market-credits-${side}`} data-credits={credits} title={t("Credits")}>
        {credits}
      </span>
    </span>
  );
}
