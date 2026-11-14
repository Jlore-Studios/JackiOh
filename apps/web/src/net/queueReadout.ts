// What the queue search says about the wait (R640002, R3729001): the expected wait per mode, as the server
// counted it over the last hour, read for the shell's queue banner (`shell/QueueBanner.tsx`) and `/play`'s
// tiles. Informational only (CLAUDE.md rule 7): a read that fails leaves the readout as it was, and nothing
// waits on it.

import { useCallback, useEffect, useState } from "react";

import { QUEUE_POPULATION_POLL_SECONDS } from "@jackioh/server-config";
import { pollWhileVisible } from "../game/net.ts";
import { t } from "../i18n/i18n.ts";
import { getPopulation, type PopulationResponse } from "./api.ts";
import { attempt } from "./errors.ts";

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;

/** R640002: an expected wait the server counted, in seconds, as its tile says it. */
export function waitWords(seconds: number): string {
  if (seconds < SECONDS_PER_MINUTE) return t("Usually under {n} s", { n: String(seconds) });
  return t("Usually under {n} min", { n: String(Math.ceil(seconds / SECONDS_PER_MINUTE)) });
}

/**
 * R3729001: the population readout, read once when the caller mounts, then every
 * `QUEUE_POPULATION_POLL_SECONDS` while `active` (a hidden tab reads nothing: the read holds no ticket, R3205000).
 * Null until the first read answers, and after a failed one with nothing before it.
 */
export function useQueueReadout(token: string, active: boolean): PopulationResponse | null {
  const [population, setPopulation] = useState<PopulationResponse | null>(null);
  const read = useCallback(
    (canceled: () => boolean): void => {
      attempt(() => getPopulation(token)).then(
        (answer) => {
          if (!canceled()) setPopulation(answer);
        },
        () => undefined,
      );
    },
    [token],
  );

  useEffect(() => {
    let done = false;
    read(() => done);
    return () => {
      done = true;
    };
  }, [read]);

  useEffect(() => {
    if (!active) return;
    let done = false;
    const stop = pollWhileVisible(() => {
      read(() => done);
    }, QUEUE_POPULATION_POLL_SECONDS * MS_PER_SECOND);
    return () => {
      done = true;
      stop();
    };
  }, [read, active]);

  return population;
}
