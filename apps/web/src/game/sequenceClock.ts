// R1563006: Meditative+ #20 KY's Competition's sequence clock, as the prompt shows it. A sequence has
// `KY_COMPETITION_SECONDS` from its first question; then its host answers Stop. The host decides that:
// the server's match clock online and the practice session's machine in practice, both from
// `crates/shared/src/host/clock.rs`; nothing here answers. The prompt only shows the seconds left, counted
// from the moment this client first saw the sequence's first question.

import { useEffect, useState } from "react";

import { KY_COMPETITION_SECONDS } from "@jackioh/engine/config";

export const SEQUENCE_WINDOW_MS = KY_COMPETITION_SECONDS * 1000;

/** The sequence this client has seen begin: its first question's id and when it was first seen. */
let open: { first: string; at: number } | null = null;

/**
 * When the open sequence began, in `now`'s terms: a later question keeps the one it is part of, or
 * begins one if this client came in mid-sequence.
 */
export function sequenceStartedAt(choiceId: string, streak: number, now: number): number {
  if (open === null || (streak === 0 && open.first !== choiceId)) open = { first: choiceId, at: now };
  return open.at;
}

/** The whole seconds the open sequence has left, ticking while a question is shown. */
export function useSequenceSecondsLeft(choiceId: string | undefined, streak: number | undefined): number | null {
  const [now, setNow] = useState(() => Date.now());
  const showing = choiceId !== undefined && streak !== undefined;
  useEffect(() => {
    if (!showing) return undefined;
    const tick = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(tick);
  }, [showing]);
  if (choiceId === undefined || streak === undefined) return null;
  const left = sequenceStartedAt(choiceId, streak, now) + SEQUENCE_WINDOW_MS - now;
  return Math.max(0, Math.ceil(left / 1000));
}
