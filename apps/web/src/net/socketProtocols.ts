// R1379000: a socket's sign-in travels as an offered WebSocket subprotocol, never in its address: every
// socket offers `jackioh.v1` and `jackioh.bearer.<base64url of the token's UTF-8, no padding>`, which keeps
// any token a valid protocol name. Must match `crates/server/src/actor/ws_server.rs` (`WS_SUBPROTOCOL`,
// `WS_BEARER_PROTOCOL_PREFIX`, `bearer_protocol`).

/** The one subprotocol the server echoes; a browser fails a handshake whose offer gets no name back. */
export const WS_SUBPROTOCOL = "jackioh.v1";

/** The offered subprotocol that carries the token, read by the server and never echoed. */
export const WS_BEARER_PROTOCOL_PREFIX = "jackioh.bearer.";

function base64url(text: string): string {
  let binary = "";
  for (const byte of new TextEncoder().encode(text)) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/gu, "-").replace(/\//gu, "_").replace(/=+$/u, "");
}

/** `jackioh.v1` first, then the credential. */
export function socketProtocols(token: string): string[] {
  return [WS_SUBPROTOCOL, WS_BEARER_PROTOCOL_PREFIX + base64url(token)];
}

/** The token `protocols` carry, as the server's `upgrade_token` reads it: the first credential, or null. */
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
