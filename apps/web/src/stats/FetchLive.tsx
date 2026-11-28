// R5274000: `/stats`' Fetch live, for a dev account. The server keeps the card statistics six hours
// (`CARD_STATS_CACHE_TTL_SECONDS`); this asks it to let them go and reloads the page, whose reads then wait
// for figures loaded now behind the progress bar. The server decides (CLAUDE.md rule 7): the button shows
// when the tab's kept `/api/auth/me` says `isDev`, and the route checks `DEV_PROFILE_IDS` again.

import { type ReactElement, useState } from "react";

import { t } from "../i18n/i18n.ts";
import { devStatsRefresh } from "../net/api.ts";
import { keptAccount } from "../net/gate.ts";
import { readSession } from "../net/session.ts";
import { statsTestid } from "./testids.ts";

type Asked = "no" | "asking" | "failed";

/** The page's reload once the server has let its figures go; a test hands its own. */
function reloadPage(): void {
  window.location.reload();
}

export function FetchLive({ reload = reloadPage }: { reload?: () => void }): ReactElement | null {
  const [asked, setAsked] = useState<Asked>("no");
  const token = readSession()?.accessToken ?? null;
  if (token === null || keptAccount()?.isDev !== true) return null;
  const fetchLive = (): void => {
    setAsked("asking");
    void devStatsRefresh(token).then(reload, () => {
      setAsked("failed");
    });
  };
  return (
    <button
      type="button"
      className="stats-retry-btn"
      data-testid={statsTestid.fetchLive}
      disabled={asked === "asking"}
      onClick={fetchLive}
    >
      {asked === "asking" ? t("Fetching live…") : asked === "failed" ? t("Couldn't fetch live. Try again") : t("Fetch live")}
    </button>
  );
}
