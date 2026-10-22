// R1563006: Meditative+ #20 KY's Competition's sequence clock, as the client keeps it. A sequence has
// `KY_COMPETITION_SECONDS` from its first question; then its host answers Stop. The server's match
// clock is the authority online (`crates/server/src/actor/clock.rs`); the practice host is this
// client, so its controller answers Stop itself (practice/controller.ts). The prompt shows the seconds
// left either way, counted from the moment this client first saw the sequence's first question.

import { useEffect, useState } from "react";

import type { ActionBody, PlayerView } from "@jackioh/shared";

import { KY_COMPETITION_SECONDS } from "@jackioh/engine/config";

/** A sequence's window in milliseconds. */
export const SEQUENCE_WINDOW_MS = KY_COMPETITION_SECONDS * 1000;

/** The question of a sequence the viewer must answer now, or null. */
export function sequenceQuestionOf(view: PlayerView): { choiceId: string; streak: number } | null {
  const pending = view.pending;
  if (pending === null || !pending.forYou || pending.kind !== "answer" || pending.streak === undefined) return null;
  return { choiceId: pending.choiceId, streak: pending.streak };
}

/** R1184001: the answer that stops the open sequence (its Stop option, key `none`), or null. */
export function stopAnswerOf(view: PlayerView): ActionBody | null {
  const pending = view.pending;
  if (sequenceQuestionOf(view) === null || pending === null || !pending.forYou) return null;
  if (!pending.options.some((option) => option.key === "none")) return null;
  return { type: "answer", choiceId: pending.choiceId, selection: [{ pick: "none" }] };
}

/** The sequence this client has seen begin: its first question's id and when it was first seen. */
let open: { first: string; at: number } | null = null;

/**
 * When the open sequence began, in `now`'s terms: a first question (streak 0) this client has not
 * started on begins a new one; a later question keeps the one it is part of, or begins one if this
 * client came in mid-sequence.
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
