// R3013001: a signed-in account's card proficiency (§14.14) lives on the account alone: the server counts the plays
// and levels (R3013000), and this page holds a copy for the faces and the Collection (`stamps.ts`). The sync loads
// it once an active account is on a gated screen, sends each stamp picked here (`PUT /api/proficiency/stamp`) and
// puts the account's answer in place of the copy; a pick the server refuses, or one that never reached it, loads
// the account's copy again, so the page never shows a stamp the account does not wear. Nothing is uploaded: the
// device holds no proficiency of its own. Stopping clears the copy, so no other account sees it. A failed request
// throws nothing into React.

import { getProficiency, putProficiencyStamp, type ProficiencyResponse, type ProficiencyStampPut } from "../net/api.ts";
import type { Account } from "../net/gate.ts";
import { requestQueue, SETTINGS_SYNC_GRACE_MS, useAccountSync } from "../settings/accountSync.ts";
import { isRecord } from "../wire/json.ts";
import { replaceOwnProficiency, subscribeStampWrites } from "./stamps.ts";

/** The two requests, injectable so the tests never touch the network. */
export type ProficiencyAccountApi = {
  load(token: string): Promise<ProficiencyResponse>;
  put(token: string, change: ProficiencyStampPut): Promise<ProficiencyResponse>;
};

export const proficiencyAccountApi: ProficiencyAccountApi = {
  load: getProficiency,
  put: putProficiencyStamp,
};

export type ProficiencyAccountSync = {
  /** Stop listening and clear the page's copy; an answer still in flight is ignored. */
  stop(): void;
  /** Resolves once no request is in flight (tests; nothing in the app waits on it). */
  settled(): Promise<void>;
};

/** The account's answer's cards; an answer of another shape is a failed request. */
function cardsOf(answer: unknown): readonly unknown[] {
  if (!isRecord(answer) || !Array.isArray(answer.cards)) throw new Error("not a proficiency answer");
  return answer.cards as unknown[];
}

/** R3013001: keeps this page's copy of the proficiency of the account of `token()` level with the account's. */
export function startProficiencyAccountSync(options: { token: () => string; api: ProficiencyAccountApi }): ProficiencyAccountSync {
  const { token, api } = options;
  let stopped = false;
  /** One request after those before it; a failure is swallowed, so the next still runs. */
  const { enqueue, settled } = requestQueue();

  const replace = (answer: unknown): void => {
    const cards = cardsOf(answer);
    if (!stopped) replaceOwnProficiency(cards);
  };

  const reload = async (): Promise<void> => {
    replace(await api.load(token()));
  };

  const unsubscribe = subscribeStampWrites((defId, stamp) => {
    if (stopped) return;
    enqueue(async () => {
      try {
        replace(await api.put(token(), { cardId: defId, stamp }));
      } catch {
        // Refused (a stamp the card's level has not unlocked here or on another device) or never sent.
        await reload();
      }
    });
  });

  enqueue(reload);

  return {
    stop: () => {
      stopped = true;
      unsubscribe();
      replaceOwnProficiency([]);
    },
    settled,
  };
}

// Shared page sync, as `cardPicksSync.ts` shares its own: every gated screen mounts the hook.

type Shared = { profileId: string; sync: ProficiencyAccountSync; users: number; setToken: (token: () => string) => void; stopTimer: ReturnType<typeof setTimeout> | null };

let shared: Shared | null = null;

function attach(profileId: string, token: () => string, api: () => ProficiencyAccountApi): () => void {
  if (shared !== null && shared.profileId !== profileId) {
    if (shared.stopTimer !== null) clearTimeout(shared.stopTimer);
    shared.sync.stop();
    shared = null;
  }
  if (shared === null) {
    let readToken = token;
    const sync = startProficiencyAccountSync({
      token: () => readToken(),
      api: {
        load: (bearer) => api().load(bearer),
        put: (bearer, change) => api().put(bearer, change),
      },
    });
    shared = {
      profileId,
      sync,
      users: 0,
      setToken: (next) => {
        readToken = next;
      },
      stopTimer: null,
    };
  }
  const mine = shared;
  if (mine.stopTimer !== null) clearTimeout(mine.stopTimer);
  mine.stopTimer = null;
  mine.users += 1;
  mine.setToken(token);
  return () => {
    mine.users -= 1;
    if (mine.users > 0 || shared !== mine) return;
    mine.stopTimer = setTimeout(() => {
      if (shared !== mine || mine.users > 0) return;
      mine.sync.stop();
      shared = null;
    }, SETTINGS_SYNC_GRACE_MS);
  };
}

/** R3013001: an active account's proficiency and stamps are loaded and kept level, on every gated screen. */
export function useProficiencyAccountSync(account: Account, api: ProficiencyAccountApi = proficiencyAccountApi): void {
  useAccountSync(account, api, attach);
}

/** Test seam: a new page, with no sync running. */
export function resetProficiencySyncForTests(): void {
  if (shared?.stopTimer !== null && shared?.stopTimer !== undefined) clearTimeout(shared.stopTimer);
  shared?.sync.stop();
  shared = null;
}
