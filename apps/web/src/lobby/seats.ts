// What a private lobby's seats show (R3730000), from the server's view alone: who sits where, an empty
// seat a member may invite into, the AI's two seats in a co-op lobby, and whether the host's Start can
// light up (R3730002). CLAUDE.md rule 7: the Start is the server's to refuse; this only lights the button.

import { t } from "../i18n/i18n.ts";
import type { LobbySeat, LobbyView } from "./api.ts";

/** One seat as the screen draws it. */
export type SeatShown =
  /** A player's seat: theirs, the host's crown on it. */
  | { kind: "player"; seat: LobbySeat; host: boolean; you: boolean }
  /** An empty seat a member may invite a friend into. */
  | { kind: "invite" }
  /** An empty seat a visitor sees, theirs to take with Join lobby. */
  | { kind: "open" }
  /** One of the AI team's two seats in a co-op lobby. */
  | { kind: "ai"; label: string };

/** Each team's seats, team A first: two in a duel (host, guest), four in co-op (A1, A2, then B1, B2 the AI). */
export function seatsOf(view: LobbyView): SeatShown[][] {
  const member = view.you !== null;
  const humans: SeatShown[] = view.seats.map((seat) => {
    if (seat.player === null) return member ? { kind: "invite" } : { kind: "open" };
    return { kind: "player", seat, host: seat.role === "host", you: seat.role === view.you };
  });
  if (view.settings.format === "duel") return [humans];
  return [humans, [{ kind: "ai", label: "B1" }, { kind: "ai", label: "B2" }]];
}

/** R3730002: the host's Start lights once every human seat is filled and ready, and the game has not started. */
export function canStart(view: LobbyView): boolean {
  return view.you === "host" && view.startedAt === null && view.seats.every((seat) => seat.player !== null && seat.state === "ready");
}

/** R3730001: a seat's deck state in words: choosing, the deck's name (or a word for a hidden or random one), ready. */
export function deckWords(seat: LobbySeat): string {
  switch (seat.state) {
    case "choosing":
      return t("Choosing a deck…");
    case "ready":
      // The mark is a glyph beside the word, not a word of the sentence.
      return `✓ ${t("Ready")}`;
    case "chosen":
      if (seat.deck !== null) return seat.deck;
      return seat.random ? t("Random deck") : t("Deck chosen");
  }
}
