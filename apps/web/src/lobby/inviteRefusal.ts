// R3874000: a friend's Lobby on Play together (`together/TogetherRoute.tsx`) makes a lobby, invites the
// friend and opens the lobby even when the invite is refused; the refusal waits here for that lobby's
// screen (`LobbyScreen.tsx`), which says it once. Tab memory only: a reload says nothing again, as
// `auth/redirect.ts`'s `takeLinkResult` has it. CLAUDE.md rule 7: the words are the server's.

let held: { code: string; message: string } | null = null;

/** The server's words for a refused invite into the lobby `code`; a later one replaces it. */
export function holdInviteRefusal(code: string, message: string): void {
  held = { code, message };
}

/** Read once, by the screen of the lobby under `code`; another lobby's screen leaves it held. */
export function takeInviteRefusal(code: string): string | null {
  if (held === null || held.code !== code) return null;
  const { message } = held;
  held = null;
  return message;
}
