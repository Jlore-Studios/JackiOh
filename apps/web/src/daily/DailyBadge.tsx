// The count of daily games waiting on the player (#3727), on the Play screen's Daily games: the social
// read's `dailyWaiting` (`together/socialPoll.ts`), drawn only when one waits.

import type { ReactElement } from "react";

import { t } from "../i18n/i18n.ts";

import "./daily.css";

export function DailyBadge({ count, testid }: { count: number; testid: string }): ReactElement | null {
  if (count <= 0) return null;
  return (
    <span className="daily-badge" data-testid={testid} aria-label={t("{count} waiting on you", { count })}>
      {count}
    </span>
  );
}
