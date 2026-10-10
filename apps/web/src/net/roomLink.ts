// Play room links (SPEC §9.5, R767).
// They only fill the lobby form; the server validates the mode (R264, CLAUDE.md rule 7).
// At boot, valid `room` and `mode` leave the URL for per-tab storage (R632); codes use `canonicalCode`
// and `ROOM_CODE_FORMAT` (R191). Sign-in keeps only code and mode (B35); unavailable storage means no link.

import { canonicalCode } from "@jackioh/shared";
import { ROOM_CODE_FORMAT } from "@jackioh/server-config";
import type { QueueMode } from "./api.ts";
import { currentPath, paths } from "./navigate.ts";

const ROOM_PARAM = "room";
const MODE_PARAM = "mode";

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

function tabStorage(): Storage | null {
  try {
    return window.sessionStorage;
  } catch {
    return null;
  }
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
  try {
    tabStorage()?.setItem(ROOM_LINK_STORAGE_KEY, JSON.stringify(link));
  } catch {
    // Blocked or full storage: the link serves this page load but not a trip through sign-in.
  }
  return true;
}

/** Read URL then untrusted tab storage without consuming either, so StrictMode gets the same result. */
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

export function forgetRoomLink(): void {
  try {
    tabStorage()?.removeItem(ROOM_LINK_STORAGE_KEY);
  } catch {
    // Blocked storage held nothing to forget.
  }
}

/** A declined share must not silently fall back to the clipboard. */
export async function sendRoomLink(url: string): Promise<"shared" | "copied"> {
  if (typeof navigator.share === "function") {
    await navigator.share({ title: ROOM_LINK_SHARE_TITLE, url });
    return "shared";
  }
  await navigator.clipboard.writeText(url);
  return "copied";
}
