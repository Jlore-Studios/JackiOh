// A room shared as a link (SPEC §9.5, R767).
//
// A room's host can hand over `/play?room=CODE&mode=<mode>` instead of a bare code. This module
// builds that link and reads one back; the lobby (`routes/play.tsx`) does the filling in.
//
// IT ENFORCES NOTHING (CLAUDE.md rule 7). A link only fills in a form: the code goes in the join
// input and the mode is a hint. The server's refusal still decides (R264: a room plays the mode it
// was made with), and the joiner still picks a deck and presses Join, so a link never seats anyone.
//
//   * THE LINK IS READ ONCE. `adoptRoomLink` takes `room` and `mode` out of the address bar with
//     `history.replaceState` before anything renders, so a reload or Back never refills a stale
//     code, and keeps what it read for this tab only, in `sessionStorage` (R632). The code is read
//     as a typed one is (R191: `canonicalCode` against `ROOM_CODE_FORMAT`); one that fails is no link
//     and says nothing.
//   * IT SURVIVES A SIGN-IN. A visitor who is signed out is sent to `/login` by the gate, and a
//     sign-in lands on the main menu (B35, #479): the link waits in the tab, and the next lobby uses
//     it once (`forgetRoomLink`). Only the code and the mode are kept, never a path (B35).
//
// Storage is untrusted and may be missing: a read that throws or finds anything else is "no link",
// and a write that throws only costs the wait through a sign-in.

import { canonicalCode } from "@jackioh/shared";
import { ROOM_CODE_FORMAT } from "@jackioh/server-config";
import type { QueueMode } from "./api.ts";
import { currentPath, paths } from "./navigate.ts";

/** The link's query parameters. */
const ROOM_PARAM = "room";
const MODE_PARAM = "mode";

/** Where this tab keeps a link it opened, until a lobby has used it. */
export const ROOM_LINK_STORAGE_KEY = "jackioh.play.roomLink";

/** The title the share sheet shows beside the link. */
export const ROOM_LINK_SHARE_TITLE = "Join my JackiOh room";

/** What a link carries: a room code in its canonical form, and the room's mode if the link named one. */
export type RoomLink = { code: string; mode: QueueMode | null };

function isQueueMode(value: unknown): value is QueueMode {
  return value === "bo1" || value === "bo3" || value === "random";
}

/** The link's path and query, for the room `code` of `mode`. */
export function roomLinkPath(code: string, mode: QueueMode): string {
  const query = new URLSearchParams({ [ROOM_PARAM]: code, [MODE_PARAM]: mode });
  return `${paths.play}?${query.toString()}`;
}

/** The link to hand over: this site's address and `roomLinkPath`. */
export function roomLinkUrl(code: string, mode: QueueMode): string {
  return `${window.location.origin}${roomLinkPath(code, mode)}`;
}

/**
 * The link a query string carries, or null when it has no `room` or its code is no room code (R191).
 * A mode that is not one of the three is dropped and the code kept: the mode is only a hint.
 */
export function roomLinkOf(search: string): RoomLink | null {
  const params = new URLSearchParams(search);
  const raw = params.get(ROOM_PARAM);
  if (raw === null) return null;
  const code = canonicalCode(raw, ROOM_CODE_FORMAT);
  if (code === null) return null;
  const mode = params.get(MODE_PARAM);
  return { code, mode: isQueueMode(mode) ? mode : null };
}

function tabStorage(): Storage | null {
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
}

/**
 * Called once at boot, before the route switch: if this is `/play` and the address bar holds a
 * `room`, take `room` and `mode` out of it (other parameters stay) and keep a valid link for this
 * tab. Returns whether a link was kept. Any other path is left alone: `room` there is some other
 * page's.
 */
export function adoptRoomLink(): boolean {
  if (typeof window === "undefined") return false;
  if (currentPath() !== paths.play) return false;
  const url = new URL(window.location.href);
  if (!url.searchParams.has(ROOM_PARAM)) return false;
  const link = roomLinkOf(url.search);
  url.searchParams.delete(ROOM_PARAM);
  url.searchParams.delete(MODE_PARAM);
  try {
    window.history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
  } catch {
    // A sandboxed frame may refuse; the link is still read and kept below.
  }
  if (link === null) return false;
  try {
    tabStorage()?.setItem(ROOM_LINK_STORAGE_KEY, JSON.stringify(link));
  } catch {
    // Blocked or full storage: the link serves this page load but not a trip through sign-in.
  }
  return true;
}

/**
 * The link this tab opened, or null: whatever the address bar carries now, taken in first, then
 * what the tab kept. The stored value is read as untrusted input, code and mode alike. It removes
 * nothing, so React StrictMode's second call gets the same answer; the lobby forgets it once it is
 * mounted (`forgetRoomLink`).
 */
export function readRoomLink(): RoomLink | null {
  adoptRoomLink();
  try {
    const raw = tabStorage()?.getItem(ROOM_LINK_STORAGE_KEY) ?? null;
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return null;
    const { code, mode } = parsed as { code?: unknown; mode?: unknown };
    const canonical = typeof code === "string" ? canonicalCode(code, ROOM_CODE_FORMAT) : null;
    if (canonical === null) return null;
    return { code: canonical, mode: isQueueMode(mode) ? mode : null };
  } catch {
    return null;
  }
}

/** The link has been used: this tab keeps nothing of it. */
export function forgetRoomLink(): void {
  try {
    tabStorage()?.removeItem(ROOM_LINK_STORAGE_KEY);
  } catch {
    // Blocked storage held nothing to forget.
  }
}

/**
 * Hands `url` over: the share sheet where the browser has one, else the clipboard. Either refusal
 * rejects, and a share that failed does not fall back to the clipboard: a person who closed the
 * sheet did not ask for a copy.
 */
export async function sendRoomLink(url: string): Promise<"shared" | "copied"> {
  if (typeof navigator.share === "function") {
    await navigator.share({ title: ROOM_LINK_SHARE_TITLE, url });
    return "shared";
  }
  await navigator.clipboard.writeText(url);
  return "copied";
}
