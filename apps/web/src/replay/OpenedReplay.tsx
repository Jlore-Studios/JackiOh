// R768000: a replay whose source is not in hand yet: the online match a result screen has just
// shown, whose length is known only once its first page is in (`openOnlineMatchReplay`). One load
// per mount and per Try again (#1568), then it hands the source to the player; a failure says why,
// with the way back.

import { useEffect, useState } from "react";
import type { ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import ReplayPlayer, { replayTestid } from "./ReplayPlayer.tsx";
import type { ReplayPlayerProps } from "./ReplayPlayer.tsx";
import { replayFailureOf, replayFailureText } from "./source.ts";
import type { ReplayFailure, ReplaySource } from "./source.ts";

export type OpenedReplayProps = Omit<ReplayPlayerProps, "source"> & {
  /** Called when the component mounts, and again on each Try again. */
  load: () => Promise<ReplaySource>;
};

export default function OpenedReplay({ load, onLeave, ...player }: OpenedReplayProps): ReactElement {
  const [source, setSource] = useState<ReplaySource | null>(null);
  const [failure, setFailure] = useState<ReplayFailure | null>(null);
  const [attempt, setAttempt] = useState(0);

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
    // One load per mount and per Try again.
  }, [attempt]);

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
        <>
          <p className="notice" role="alert" data-testid={replayTestid.failure}>
            {replayFailureText(failure)}
          </p>
          {failure.reason === "error" ? (
            <div className="row">
              <button
                type="button"
                className="button-primary"
                data-testid={replayTestid.retry}
                onClick={() => {
                  setFailure(null);
                  setAttempt((before) => before + 1);
                }}
              >
                {t("Try again")}
              </button>
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}
