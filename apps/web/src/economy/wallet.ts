// The Jlore wallet as every screen of the tab shows it (#1659, part of #1651; SPEC §14.1, §14.3).
// CLAUDE.md rule 7: the server owns the balance. This keeps the last answer a server gave this
// session's token, from `GET /api/wallet` or from another answer's `jlore` (`announceWallet`), and
// never a sum of its own. The pill (`JlorePill.tsx`) reads it through `useWallet`.
//
// A read starts when `useWallet` is first used, on `jackioh:navigated`, when the page becomes
// visible again and on every `announceWallet()`. A read that fails (a pending account's refusal, a
// network error) leaves what was known: nothing for an account never read, so no pill shows.

import { useEffect, useSyncExternalStore } from "react";

import { getWallet } from "../net/api.ts";
import { isRecord } from "../wire/json.ts";

/** Fired by `announceWallet`; its `detail` is the balance a server answer gave, or absent. */
const WALLET_CHANGED = "jackioh:wallet-changed";

/** `net/navigate.ts` fires it after every `navigate`; the store reads the wallet again then. */
const NAVIGATED = "jackioh:navigated";

/** The last answer, and the session token it was the answer for: another token never sees it. */
let known: { token: string; jlore: number } | null = null;

/** The token a read is out for, and which read it is: only the latest read, or a newer answer, wins. */
let inflight: string | null = null;
let latest = 0;

const listeners = new Set<() => void>();

function subscribe(onChange: () => void): () => void {
  listeners.add(onChange);
  return () => {
    listeners.delete(onChange);
  };
}

function remember(token: string, jlore: number): void {
  latest += 1;
  inflight = null;
  if (known?.token === token && known.jlore === jlore) return;
  known = { token, jlore };
  for (const listener of [...listeners]) listener();
}

/** Reads the wallet; `again` replaces a read already out, which a later change may have outdated. */
function read(token: string, again: boolean): void {
  if (!again && inflight === token) return;
  latest += 1;
  const mine = latest;
  inflight = token;
  const failed = (): void => {
    if (mine === latest) inflight = null;
  };
  // An answer that is not a balance is a failed read, as a refusal is.
  Promise.resolve()
    .then(() => getWallet(token))
    .then((answer: unknown) => {
      if (!isRecord(answer) || typeof answer.jlore !== "number" || !Number.isFinite(answer.jlore)) return failed();
      if (mine === latest) remember(token, answer.jlore);
    }, failed);
}

/**
 * Tells every pill the balance changed. With `jlore`, a server answer's balance (as
 * `POST /api/packs/buy` answers it), every pill shows it at once and nothing is requested; with
 * none (a level or a quest may have paid some), each reads the wallet again.
 */
export function announceWallet(jlore?: number): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent<number | undefined>(WALLET_CHANGED, { detail: jlore }));
}

/** The balance the server last gave `token`, or null before the first answer, with no session, or when it refuses. */
export function useWallet(token: string | null): number | null {
  const jlore = useSyncExternalStore(
    subscribe,
    () => (known !== null && known.token === token ? known.jlore : null),
    () => null,
  );

  useEffect(() => {
    if (token === null) return;
    read(token, false);
    const again = (): void => {
      read(token, true);
    };
    const onChanged = (event: Event): void => {
      const detail: unknown = (event as CustomEvent<unknown>).detail;
      if (typeof detail === "number" && Number.isFinite(detail)) remember(token, detail);
      else again();
    };
    const onVisible = (): void => {
      if (document.visibilityState === "visible") again();
    };
    window.addEventListener(NAVIGATED, again);
    window.addEventListener(WALLET_CHANGED, onChanged);
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.removeEventListener(NAVIGATED, again);
      window.removeEventListener(WALLET_CHANGED, onChanged);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [token]);

  return jlore;
}

/** Forgets every answer and read, so one test's balance is never the next one's. */
export function __resetWalletForTests(): void {
  known = null;
  inflight = null;
  latest += 1;
}
