// R768000: a replay whose source is not in hand yet: the online match a result screen has just
// shown, whose length is known only once its first page is in (`openOnlineMatchReplay`). It loads
// the source once, then hands it to the player; a failure says why, with the way back.

import { useEffect, useState } from "react";
import type { ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import ReplayPlayer, { replayTestid } from "./ReplayPlayer.tsx";
import type { ReplayPlayerProps } from "./ReplayPlayer.tsx";
import { replayFailureOf, replayFailureText } from "./source.ts";
import type { ReplayFailure, ReplaySource } from "./source.ts";

export type OpenedReplayProps = Omit<ReplayPlayerProps, "source"> & {
  /** Called once, when the component mounts. */
  load: () => Promise<ReplaySource>;
};

export default function OpenedReplay({ load, onLeave, ...player }: OpenedReplayProps): ReactElement {
  const [source, setSource] = useState<ReplaySource | null>(null);
  const [failure, setFailure] = useState<ReplayFailure | null>(null);

  useEffect(() => {
    let cancelled = false;
    load().then(
      (loaded) => {
        if (!cancelled) setSource(loaded);
      },
      (cause: unknown) => {
        if (!cancelled) setFailure(replayFailureOf(cause));
      },
    );
    return () => {
      cancelled = true;
    };
    // One load per mount: a different replay is a different `OpenedReplay`, keyed by the route.
  }, []);

  if (source !== null) return <ReplayPlayer source={source} onLeave={onLeave} {...player} />;
  return (
    <div className="app-shell">
      <button type="button" className="link-button" data-testid={replayTestid.back} onClick={onLeave}>
        {t("← Back")}
      </button>
      {failure === null ? (
        <p className="notice" role="status" data-testid={replayTestid.loading}>
          {t("Loading the replay…")}
        </p>
      ) : (
        <p className="notice" role="alert" data-testid={replayTestid.failure}>
          {replayFailureText(failure)}
        </p>
      )}
    </div>
  );
}
