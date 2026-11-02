// R1834000: a signed-in account's per-card picks (R1655000) live on the account, and the device's store
// (`cards/cosmetics.ts`) is their cache. The device's own picks are sent up once, by the first active
// profile to sign in on it while it holds some (`POST /api/card-picks/upload` never overwrites a card
// the account holds); any other time the account's copy is read. Either answer replaces the device's
// copy, but for a write made here meanwhile, and lists the profile under CARD_PICKS_UPLOADED_KEY: an
// account's copy is never sent up again, so a pick cleared on another device never comes back from this
// one's cache. A failed request throws nothing into React, changes nothing here and lists no profile,
// so the next sign-in tries again.

import { getCardPicks, putCardPick, uploadCardPicks, type CardPicksEntry, type CardPicksResponse, type CardPickPut } from "../net/api.ts";
import type { Account } from "../net/gate.ts";
import { SETTINGS_SYNC_GRACE_MS, useAccountSync } from "../settings/accountSync.ts";
import { isRecord } from "../wire/json.ts";
import {
  readAllCardPicks,
  replaceAllCardPicks,
  subscribeCardPickWrites,
  type CardPickChange,
  type StoredCardPicks,
} from "./cosmetics.ts";
import { getStored, setStored } from "../net/storage.ts";

/** R1834000: the profiles whose account copy has replaced this device's picks, a JSON list of profile ids. */
export const CARD_PICKS_UPLOADED_KEY = "jackioh.cosmetics.cards.uploaded.v1";

/** The three requests, injectable so the tests never touch the network. */
export type CardPicksAccountApi = {
  load(token: string): Promise<CardPicksResponse>;
  upload(token: string, cards: CardPicksEntry[]): Promise<CardPicksResponse>;
  put(token: string, change: CardPickPut): Promise<CardPicksResponse>;
};

export const cardPicksAccountApi: CardPicksAccountApi = {
  load: getCardPicks,
  upload: uploadCardPicks,
  put: putCardPick,
};

export type CardPicksAccountSync = {
  /** Stop listening; an answer still in flight is ignored. */
  stop(): void;
  /** Resolves once no request is in flight (tests; nothing in the app waits on it). */
  settled(): Promise<void>;
};

/** The profiles listed under CARD_PICKS_UPLOADED_KEY; none where storage is missing, blocked or unparsable. */
function uploadedProfiles(): string[] {
  try {
    const raw: unknown = JSON.parse(getStored(CARD_PICKS_UPLOADED_KEY) ?? "[]");
    return Array.isArray(raw) ? raw.filter((id): id is string => typeof id === "string") : [];
  } catch {
    return [];
  }
}

function markUploaded(profileId: string): void {
  // Storage that refuses it: the next sign-in may send the picks again, which overwrites nothing on the account.
  setStored(CARD_PICKS_UPLOADED_KEY, JSON.stringify([...uploadedProfiles(), profileId]));
}

/** The account's answer as the store's input, card id to picks; an answer of another shape is a failed request. */
function accountCopy(answer: unknown): Record<string, unknown> {
  if (!isRecord(answer) || !Array.isArray(answer.cards)) throw new Error("not a card picks answer");
  const picks: Record<string, unknown> = {};
  for (const entry of answer.cards as unknown[]) {
    if (isRecord(entry) && typeof entry.cardId === "string") picks[entry.cardId] = entry;
  }
  return picks;
}

/** Applies `change` to `picks[defId]` in place: a null field is dropped. */
function withChange(picks: Record<string, unknown>, defId: string, change: CardPickChange): void {
  const held = picks[defId];
  const next: Record<string, unknown> = isRecord(held) ? { ...held } : {};
  for (const [field, value] of Object.entries(change)) {
    if (value === null) delete next[field];
    else next[field] = value;
  }
  picks[defId] = next;
}

/** R1834000: keeps `profileId`'s per-card picks level between this device and the account of `token()`. */
export function startCardPicksAccountSync(options: {
  profileId: string;
  token: () => string;
  api: CardPicksAccountApi;
}): CardPicksAccountSync {
  const { profileId, token, api } = options;
  let stopped = false;
  /** The writes made while the account's copy was on its way, which it must not undo; null once it came. */
  let early: [string, CardPickChange][] | null = [];
  let queue: Promise<void> = Promise.resolve();
  let pending = 0;

  /** One request after those before it; a failure is swallowed, so the next still runs. */
  const enqueue = (request: () => Promise<unknown>): void => {
    pending += 1;
    queue = queue
      .then(request)
      .then(
        () => undefined,
        () => undefined,
      )
      .finally(() => {
        pending -= 1;
      });
  };

  const send = (defId: string, change: CardPickChange): void => {
    enqueue(() => api.put(token(), { cardId: defId, ...change }));
  };

  const unsubscribe = subscribeCardPickWrites((defId, change) => {
    if (stopped) return;
    if (early === null) send(defId, change);
    else early.push([defId, change]);
  });

  enqueue(async () => {
    let copy: Record<string, unknown> | null = null;
    try {
      const device = readAllCardPicks();
      // Once any profile's copy has replaced the store, what it holds is an account's, not the device's own.
      const upload = Object.keys(device).length > 0 && uploadedProfiles().length === 0;
      if (upload) {
        const cards = Object.entries(device).map(([cardId, picks]: [string, StoredCardPicks]): CardPicksEntry => ({ cardId, ...picks }));
        copy = accountCopy(await api.upload(token(), cards));
      } else {
        copy = accountCopy(await api.load(token()));
      }
      if (!uploadedProfiles().includes(profileId)) markUploaded(profileId);
    } finally {
      const owed = early ?? [];
      early = null;
      if (!stopped) {
        if (copy !== null) {
          for (const [defId, change] of owed) withChange(copy, defId, change);
          replaceAllCardPicks(copy);
        }
        for (const [defId, change] of owed) send(defId, change);
      }
    }
  });

  return {
    stop: () => {
      stopped = true;
      unsubscribe();
    },
    settled: async () => {
      while (pending > 0) await queue;
    },
  };
}

// Shared page sync, as `settings/accountSync.ts` shares its own: every gated screen mounts the hook.

type Shared = { profileId: string; sync: CardPicksAccountSync; users: number; setToken: (token: () => string) => void; stopTimer: ReturnType<typeof setTimeout> | null };

let shared: Shared | null = null;

function attach(profileId: string, token: () => string, api: () => CardPicksAccountApi): () => void {
  if (shared !== null && shared.profileId !== profileId) {
    if (shared.stopTimer !== null) clearTimeout(shared.stopTimer);
    shared.sync.stop();
    shared = null;
  }
  if (shared === null) {
    let readToken = token;
    const sync = startCardPicksAccountSync({
      profileId,
      token: () => readToken(),
      api: {
        load: (bearer) => api().load(bearer),
        upload: (bearer, cards) => api().upload(bearer, cards),
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

/** R1834000: an active account's per-card picks are kept level with this device's, on every gated screen. */
export function useCardPicksAccountSync(account: Account, api: CardPicksAccountApi = cardPicksAccountApi): void {
  useAccountSync(account, api, attach);
}

/** Test seam: a new page, with no sync running. */
export function resetCardPicksSyncForTests(): void {
  if (shared?.stopTimer !== null && shared?.stopTimer !== undefined) clearTimeout(shared.stopTimer);
  shared?.sync.stop();
  shared = null;
}
