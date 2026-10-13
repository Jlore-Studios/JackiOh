// Account tutorial progress (SPEC §9.10, R320, R321) merges by union: the device remains
// authoritative during a session, and failed requests leave it intact for a later retry. Active
// accounts (§9.4) sync one request at a time; signed-out visitors do not sync (R294).

import { useEffect, useRef } from "react";

import { getTutorialProgress, putTutorialProgress, type TutorialAccountProgress } from "../net/api.ts";
import type { Account } from "../net/gate.ts";
import {
  accountLacks,
  adoptTutorialProgress,
  readTutorialProgress,
  subscribeTutorialProgress,
  type TutorialProgressLike,
} from "./progress.ts";

/** The two requests, injectable so the route's tests never touch the network. */
export type TutorialAccountApi = {
  load(token: string): Promise<TutorialAccountProgress>;
  save(token: string, progress: TutorialAccountProgress): Promise<TutorialAccountProgress>;
};

export const tutorialAccountApi: TutorialAccountApi = {
  load: getTutorialProgress,
  save: putTutorialProgress,
};

export type TutorialAccountSync = {
  /** Stop listening; an answer still in flight is ignored. */
  stop(): void;
  /** Resolves once no request is in flight or owed (tests; nothing in the app waits on it). */
  settled(): Promise<void>;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Reject malformed server progress as a failed request. */
function accountCopy(raw: unknown): TutorialProgressLike {
  if (!isRecord(raw) || !Array.isArray(raw.completed)) throw new Error("not a tutorial progress answer");
  const completed = (raw.completed as unknown[]).filter((id): id is string => typeof id === "string");
  const choice = raw.hiddenChoice;
  if (choice === null || choice === undefined) return { completed, hiddenChoice: null };
  if (!isRecord(choice) || typeof choice.hidden !== "boolean" || typeof choice.at !== "number") {
    throw new Error("not a tutorial progress answer");
  }
  return { completed, hiddenChoice: { hidden: choice.hidden, at: choice.at } };
}

/** Sync device progress with `token()`'s account, reading renewed tokens for each request. */
export function startTutorialAccountSync(options: {
  token: () => string;
  api: TutorialAccountApi;
}): TutorialAccountSync {
  let stopped = false;
  let known: TutorialProgressLike | null = null;
  let running: Promise<void> | null = null;
  let owed = false;

  const take = (answer: unknown): void => {
    const copy = accountCopy(answer);
    if (stopped) return;
    known = copy;
    adoptTutorialProgress(copy);
  };

  /** One PUT, if the device holds something the account (as far as this page knows) lacks. */
  const sendIfLacking = async (): Promise<void> => {
    const device = readTutorialProgress();
    if (known === null ? device.completed.length === 0 && device.hiddenChoice === null : !accountLacks(known, device)) {
      return;
    }
    take(
      await options.api.save(options.token(), {
        completed: [...device.completed],
        hiddenChoice: device.hiddenChoice === null ? null : { ...device.hiddenChoice },
      }),
    );
  };

  /** Run again after any change that arrived while a request was in flight. */
  const run = (first: () => Promise<void>): void => {
    running = (async () => {
      try {
        await first();
      } catch {
          // R321: keep device progress; a later load retries.
      }
      while (owed && !stopped) {
        owed = false;
        try {
          await sendIfLacking();
        } catch {
          // Keep device progress; a later load retries.
        }
      }
    })().finally(() => {
      running = null;
    });
  };

  const onDeviceChange = (): void => {
    if (stopped) return;
    if (running !== null) {
      owed = true;
      return;
    }
    run(sendIfLacking);
  };

  const unsubscribe = subscribeTutorialProgress(onDeviceChange);

  run(async () => {
    take(await options.api.load(options.token()));
    if (!stopped) await sendIfLacking();
  });

  return {
    stop: () => {
      stopped = true;
      owed = false;
      unsubscribe();
    },
    settled: async () => {
      while (running !== null) await running;
    },
  };
}

/** R321: sync only active accounts; restart for a different profile and reuse renewed tokens. */
export function useTutorialAccountSync(account: Account, api: TutorialAccountApi = tutorialAccountApi): void {
  const active = account.kind === "ready" && account.me.profile.status === "active";
  const profileId = active ? account.me.profile.id : null;
  const token = useRef<string>("");
  token.current = active ? account.token : "";
  const requests = useRef(api);
  requests.current = api;

  useEffect(() => {
    if (profileId === null) return;
    const sync = startTutorialAccountSync({
      token: () => token.current,
      api: {
        load: (bearer) => requests.current.load(bearer),
        save: (bearer, progress) => requests.current.save(bearer, progress),
      },
    });
    return () => {
      sync.stop();
    };
  }, [profileId]);
}
