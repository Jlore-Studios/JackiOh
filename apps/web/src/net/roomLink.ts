// Play room links (SPEC §9.5, R767) and co-op room links (R2094000).
// They only fill the lobby form; the server validates the mode (R264, CLAUDE.md rule 7).
// At boot, valid `room` and `mode` leave the URL for per-tab storage (R632); codes use `canonicalCode`
// and `ROOM_CODE_FORMAT` (R191). Sign-in keeps only code and mode (B35); unavailable storage means no link.

import { canonicalCode } from "@jackioh/shared";
import { ROOM_CODE_FORMAT } from "@jackioh/server-config";
import { t } from "../i18n/i18n.ts";
import type { QueueMode } from "./api.ts";
import { currentPath, paths } from "./navigate.ts";
import { getStored, removeStored, sessionStorageOrNull, setStored } from "./storage.ts";

const ROOM_PARAM = "room";
const MODE_PARAM = "mode";
const COOP_PARAM = "coop";

export const ROOM_LINK_STORAGE_KEY = "jackioh.play.roomLink";

export const ROOM_LINK_SHARE_TITLE = "Join my JackiOh room";

export type RoomLink = { code: string; mode: QueueMode | null };

function isQueueMode(value: unknown): value is QueueMode {
  return value === "bo1" || value === "bo3" || value === "random";
}

export function roomLinkPath(code: string, mode: QueueMode): string {
  const query = new URLSearchParams({ [ROOM_PARAM]: code, [MODE_PARAM]: mode });
  return `${paths.play}?${query.toString()}`;
}

export function roomLinkUrl(code: string, mode: QueueMode): string {
  return `${window.location.origin}${roomLinkPath(code, mode)}`;
}

/** R191: invalid room codes yield no link; unrecognized modes are only omitted hints. */
export function roomLinkOf(search: string): RoomLink | null {
  const params = new URLSearchParams(search);
  const raw = params.get(ROOM_PARAM);
  if (raw === null) return null;
  const code = canonicalCode(raw, ROOM_CODE_FORMAT);
  if (code === null) return null;
  const mode = params.get(MODE_PARAM);
  return { code, mode: isQueueMode(mode) ? mode : null };
}

/** Before routing, adopt only a `/play` link and remove its room parameters without touching others. */
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
  // Blocked or full storage: the link serves this page load but not a trip through sign-in.
  setStored(ROOM_LINK_STORAGE_KEY, JSON.stringify(link), sessionStorageOrNull());
  return true;
}

/** Read URL then untrusted tab storage without consuming either, so StrictMode gets the same result. */
export function readRoomLink(): RoomLink | null {
  adoptRoomLink();
  try {
    const raw = getStored(ROOM_LINK_STORAGE_KEY, sessionStorageOrNull());
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

export function forgetRoomLink(): void {
  removeStored(ROOM_LINK_STORAGE_KEY, sessionStorageOrNull());
}

export const COOP_LINK_STORAGE_KEY = "jackioh.together.coopLink";

/** R2094000: the co-op room link, R767's twin for a co-op room: `/together?coop=<CODE>`. */
export function coopLinkPath(code: string): string {
  const query = new URLSearchParams({ [COOP_PARAM]: code });
  return `${paths.together}?${query.toString()}`;
}

/** R2094000: the co-op link as a shareable URL on this origin. */
export function coopLinkUrl(code: string): string {
  return `${window.location.origin}${coopLinkPath(code)}`;
}

/** R2094000: R191-style: an invalid co-op code yields no link. */
export function coopLinkOf(search: string): string | null {
  const params = new URLSearchParams(search);
  const raw = params.get(COOP_PARAM);
  if (raw === null) return null;
  return canonicalCode(raw, ROOM_CODE_FORMAT);
}

/** R2094000: before routing, adopt only a `/together` link and remove its coop parameter without touching others. */
export function adoptCoopLink(): boolean {
  if (typeof window === "undefined") return false;
  if (currentPath() !== paths.together) return false;
  const url = new URL(window.location.href);
  if (!url.searchParams.has(COOP_PARAM)) return false;
  const code = coopLinkOf(url.search);
  url.searchParams.delete(COOP_PARAM);
  try {
    window.history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
  } catch {
    // A sandboxed frame may refuse; the link is still read and kept below.
  }
  if (code === null) return false;
  // Blocked or full storage: the link serves this page load but not a trip through sign-in.
  setStored(COOP_LINK_STORAGE_KEY, JSON.stringify({ code }), sessionStorageOrNull());
  return true;
}

/** R2094000: read URL then untrusted tab storage without consuming either, so StrictMode gets the same result. */
export function readCoopLink(): string | null {
  adoptCoopLink();
  try {
    const raw = getStored(COOP_LINK_STORAGE_KEY, sessionStorageOrNull());
    if (raw === null) return null;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null) return null;
    const { code } = parsed as { code?: unknown };
    return typeof code === "string" ? canonicalCode(code, ROOM_CODE_FORMAT) : null;
  } catch {
    return null;
  }
}

export function forgetCoopLink(): void {
  removeStored(COOP_LINK_STORAGE_KEY, sessionStorageOrNull());
}

/**
 * R2094000: the first room link of this site a message holds, built as a path, never markup.
 * Accepts this origin's `/play?room=…` (as `roomLinkOf` reads it) and `/together?coop=…`;
 * a link to any other origin, or a malformed one, draws no button.
 */
export function findRoomLinkInText(text: string, origin: string): { path: string } | null {
  const candidates = text.match(/https?:\/\/[^\s"'<>]+|\/(?:play\?room=|together\?coop=)[^\s"'<>]*/g);
  if (candidates === null) return null;
  for (let candidate of candidates) {
    // Trailing punctuation the sentence added is not the link's: "here: <link> (room code …)."
    candidate = candidate.replace(/[.,;:!?)\]]+$/u, "");
    let url: URL;
    try {
      url = new URL(candidate, origin);
    } catch {
      continue;
    }
    if (url.origin !== origin) continue;
    const bare = url.pathname.replace(/\/+$/u, "") || "/";
    if (bare === paths.play) {
      const room = roomLinkOf(url.search);
      if (room !== null) {
        return { path: room.mode === null ? `${paths.play}?room=${room.code}` : roomLinkPath(room.code, room.mode) };
      }
    }
    if (bare === paths.together) {
      const coop = coopLinkOf(url.search);
      if (coop !== null) return { path: coopLinkPath(coop) };
    }
  }
  return null;
}

/** A declined share must not silently fall back to the clipboard. */
export async function sendRoomLink(url: string): Promise<"shared" | "copied"> {
  if (typeof navigator.share === "function") {
    await navigator.share({ title: t(ROOM_LINK_SHARE_TITLE), url });
    return "shared";
  }
  await navigator.clipboard.writeText(url);
  return "copied";
}
