// What a private lobby's seats show (R3730000), from the server's view alone: who sits where, an empty
// seat a member may invite into, the AI's two seats in a co-op lobby, a 2v2's two teams (R3731000), and
// whether the host's Start can light up (R3730002). CLAUDE.md rule 7: the Start, a team switch and a move
// are the server's to refuse; this only lights the buttons and names the seat a move asks for.

import { t } from "../i18n/i18n.ts";
import type { LobbyMoveSeat, LobbyRole, LobbySeat, LobbyView } from "./api.ts";

/** One seat as the screen draws it. */
export type SeatShown =
  /** A player's seat: theirs, the host's crown on it. */
  | { kind: "player"; seat: LobbySeat; host: boolean; you: boolean }
  /** An empty seat a member may invite a friend into. */
  | { kind: "invite"; role: LobbyRole }
  /** An empty seat a visitor sees, theirs to take with Join lobby. */
  | { kind: "open"; role: LobbyRole }
  /** One of the AI team's two seats in a co-op lobby. */
  | { kind: "ai"; label: string };

/** R3731000: a 2v2's teams by seat, the host's first (0) and the other (1). Structure, not a rule. */
export function lobbyTeamOf(role: LobbyRole): 0 | 1 {
  return role === "host" || role === "guest" ? 0 : 1;
}

/**
 * Each team's seats, team A first: two in a duel (host, guest), four in co-op (A1, A2, then B1, B2 the AI),
 * and a 2v2's four people, the host's team (host, guest) then the other team (guest2, guest3).
 */
export function seatsOf(view: LobbyView): SeatShown[][] {
  const member = view.you !== null;
  const shown = (seat: LobbySeat): SeatShown => {
    if (seat.player === null) return member ? { kind: "invite", role: seat.role } : { kind: "open", role: seat.role };
    return { kind: "player", seat, host: seat.role === "host", you: seat.role === view.you };
  };
  if (view.settings.format === "2v2") {
    return [0, 1].map((team) => view.seats.filter((seat) => lobbyTeamOf(seat.role) === team).map(shown));
  }
  const humans = view.seats.map(shown);
  if (view.settings.format === "duel") return [humans];
  return [humans, [{ kind: "ai", label: "B1" }, { kind: "ai", label: "B2" }]];
}

/** R3731000: whether the team other than `role`'s has an empty seat, which Switch team needs to light. */
export function otherTeamHasRoom(view: LobbyView, role: LobbyRole): boolean {
  return view.seats.some((seat) => lobbyTeamOf(seat.role) !== lobbyTeamOf(role) && seat.player === null);
}

/**
 * R3731000: the seat the host's "Move to the other team" asks for a member at `role`: the other team's
 * first empty seat but the host's, else that team's first seat but the host's, whose member the move
 * swaps across. The host's own seat moves nowhere.
 */
export function moveTargetOf(view: LobbyView, role: LobbyRole): LobbyMoveSeat | null {
  if (role === "host") return null;
  const across = view.seats.filter(
    (seat): seat is LobbySeat & { role: LobbyMoveSeat } => seat.role !== "host" && lobbyTeamOf(seat.role) !== lobbyTeamOf(role),
  );
  return (across.find((seat) => seat.player === null) ?? across[0])?.role ?? null;
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
