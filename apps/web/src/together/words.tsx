// Words Play together's screens share: the modes as the lobby names them (R330 calls `bo3` Conquest),
// a friend's status, and a sentence with a player's username set into it.

import type { ReactElement } from "react";

import Username from "../auth/Username.tsx";
import type { QueueMode } from "../net/api.ts";
import type { Friend } from "../net/social.ts";

/** The modes a challenge can be made in, as `/play` offers them. */
export const CHALLENGE_MODES: readonly QueueMode[] = ["bo1", "bo3", "random"];

/** As `MODE_LABEL` in `routes/play.tsx` words them; wrapped in `t()` where shown. */
export const MODE_WORDS: Readonly<Record<QueueMode, string>> = {
  bo1: "Best of 1",
  bo3: "Conquest",
  random: "All Random",
};

/** A friend's status (B47) in a player's words; wrapped in `t()` where shown. */
export const STATUS_WORDS: Readonly<Record<Friend["status"], string>> = {
  online: "Online",
  inMatch: "In a game",
  offline: "Offline",
};

/**
 * A translated sentence holding `{name}`, with the username drawn through `auth/Username.tsx` in its
 * place (R1436), so the sentence stays one dictionary entry.
 */
export function Named({ text, name }: { text: string; name: string }): ReactElement {
  const [before = "", after = ""] = text.split("{name}");
  return (
    <>
      {before}
      <Username name={name} />
      {after}
    </>
  );
}
