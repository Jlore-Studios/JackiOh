// R1379000: a socket's sign-in travels as an offered WebSocket subprotocol, never in its address.
// Every socket (`/ws/match`, `/ws/duos`, `/ws/watch`) offers `jackioh.v1` and, beside it, the token
// as `jackioh.bearer.<base64url of its UTF-8, no padding>`; the server reads the second and answers
// with the first alone (`WS_SUBPROTOCOL`, `WS_BEARER_PROTOCOL_PREFIX` and `bearer_protocol` in
// `crates/server/src/actor/ws_server.rs`). The encoding keeps any token a valid protocol name.

/** The one subprotocol the server echoes; a browser fails a handshake whose offer gets no name back. */
export const WS_SUBPROTOCOL = "jackioh.v1";

/** The offered subprotocol that carries the token, read by the server and never echoed. */
export const WS_BEARER_PROTOCOL_PREFIX = "jackioh.bearer.";

function base64url(text: string): string {
  let binary = "";
  for (const byte of new TextEncoder().encode(text)) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/gu, "-").replace(/\//gu, "_").replace(/=+$/u, "");
}

/** What a socket offers on its handshake for `token`: `jackioh.v1` first, then the credential. */
export function socketProtocols(token: string): string[] {
  return [WS_SUBPROTOCOL, WS_BEARER_PROTOCOL_PREFIX + base64url(token)];
}

/**
 * The token `protocols` carry, read as the server's `upgrade_token` reads it: the first credential,
 * or null when there is none or it does not decode to a non-empty token. The tests read a socket's
 * offer back with it.
 */
export function tokenFromProtocols(protocols: readonly string[]): string | null {
  const offered = protocols.find((protocol) => protocol.startsWith(WS_BEARER_PROTOCOL_PREFIX));
  if (offered === undefined) return null;
  const encoded = offered.slice(WS_BEARER_PROTOCOL_PREFIX.length).replace(/-/gu, "+").replace(/_/gu, "/");
  try {
    const bytes = Uint8Array.from(atob(encoded), (char) => char.charCodeAt(0));
    const token = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    return token.length > 0 ? token : null;
  } catch {
    return null;
  }
}
