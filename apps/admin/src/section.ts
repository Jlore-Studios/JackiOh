// What every dashboard section shares: the API, one request at a time through `run` (whose answer the
// status line shows), and `revision`, which each successful request moves so every section reads again.

import { useEffect, useState, type DependencyList } from "react";

import { messageOf, type AdminApi } from "./api.ts";

/** Sends one request and puts the text it resolves to (or the server's refusal) in the status line. */
export type Run = (action: () => Promise<string>) => void;

export type SectionProps = { api: AdminApi; revision: number; working: boolean; run: Run };

/** A read's last answer, kept while the next one is out, and the server's message when one failed. */
export type Read<T> = { value: T | null; error: string | null };

export function useRead<T>(load: () => Promise<T>, keys: DependencyList): Read<T> {
  const [state, setState] = useState<Read<T>>({ value: null, error: null });
  useEffect(() => {
    let live = true;
    load().then(
      (value) => {
        if (live) setState({ value, error: null });
      },
      (cause: unknown) => {
        if (live) setState((previous) => ({ value: previous.value, error: messageOf(cause) }));
      },
    );
    return () => {
      live = false;
    };
    // `keys` names what the read depends on; `load` is a new closure each render.
  }, keys);
  return state;
}

/** How many decimals a rating or a deviation shows, and a volatility (whose scale is far smaller). */
export const SHOWN_DECIMALS = 2;
export const VOLATILITY_DECIMALS = 4;

/** A number as the dashboard shows it: rounded to `decimals`, with no grouping and no trailing zeros. */
export function shown(value: number, decimals: number = SHOWN_DECIMALS): string {
  return String(Number(value.toFixed(decimals)));
}

/** An epoch-ms time in the dev's own locale and zone. */
export function when(ms: number): string {
  return new Date(ms).toLocaleString();
}

/** A JSON value as plain text (`JSON.stringify`), never markup; an absent one is empty. */
export function json(value: unknown): string {
  return JSON.stringify(value) ?? "";
}

/** The status line's words for a request the server answered with data, not a message. */
export function answered(answer: unknown): string {
  return `Done: ${json(answer)}`;
}
