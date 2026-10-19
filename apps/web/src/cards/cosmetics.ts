// R1295000: the card cosmetics a player's own cards wear — a foil over the art, a finish on the
// print and a border round the frame — picked in the settings (`settings/controls.tsx`), kept per
// browser under `COSMETICS_SETTINGS_KEY` and on the account as the `cosmetics` group (R633,
// `settings/groups.ts`). The ids are the registry's (`crates/server/cosmetics.json`, generated into
// `wire/generated/cosmetics.ts`), every one unlocked.
//
// The picks are the device's own: nothing here reaches the engine, an action, `PlayerView`, the
// log or a replay, and a face wears them only where the board says it is the viewer's own card in
// a game (`worn`, game/wearer.ts); every other face draws the defaults (CLAUDE.md rule 7). Each
// slot is its own attribute on the face, apart from the rarity foil (`data-foil`, model.ts), so a
// decorative foil never blurs the Mythic and Radiant signal: a Mythic face wears no cosmetic foil,
// and a Radiant face no foil, finish or border. A default adds no attribute at all, so a face
// wearing nothing is the face it was before.
//
// An id this client does not know (one a newer client wrote to the account) is kept in the store,
// so this device never overwrites it, and drawn as its kind's default, never an error. A cosmetic
// foil moves only while one of COSMETIC_FOIL_ANIMATED_MAX slots is the face's; the others wear its
// still version, as every face does under reduced motion (cosmetics.css).

import { useEffect, useState, useSyncExternalStore } from "react";

import { COSMETIC_ID_MAX_LENGTH, COSMETIC_KINDS, COSMETICS } from "../wire/generated/cosmetics.ts";
import { COSMETIC_FOIL_ANIMATED_MAX } from "./constants.ts";
import { foilFor, type FaceModel } from "./model.ts";

export type CosmeticKind = (typeof COSMETIC_KINDS)[number];
export type Cosmetic = (typeof COSMETICS)[number];
export type CosmeticId = Cosmetic["id"];
/** A pick per kind. A stored pick is any short text, an id this client may not know among them. */
export type CardCosmetics = Record<CosmeticKind, string>;

export const COSMETICS_SETTINGS_KEY = "jackioh.cosmetics.v1";

/** The kind's entries, in the registry's order: its default first. */
export function cosmeticsOf(kind: CosmeticKind): readonly Cosmetic[] {
  return COSMETICS.filter((cosmetic) => cosmetic.kind === kind);
}

function defaultOf(kind: CosmeticKind): CosmeticId {
  const found = COSMETICS.find((cosmetic) => cosmetic.kind === kind && cosmetic.default);
  // The registry's check holds one default per kind (crates/server/src/cosmetics.rs).
  if (found === undefined) throw new Error(`the cosmetics registry has no default ${kind}`);
  return found.id;
}

export const DEFAULT_CARD_COSMETICS: Readonly<Record<CosmeticKind, CosmeticId>> = Object.freeze({
  foil: defaultOf("foil"),
  finish: defaultOf("finish"),
  border: defaultOf("border"),
});

/** `id` when it is one of `kind`'s, and the kind's default otherwise: never an error. */
export function resolveCosmetic(kind: CosmeticKind, id: unknown): CosmeticId {
  const found = COSMETICS.find((cosmetic) => cosmetic.kind === kind && cosmetic.id === id);
  return found === undefined ? DEFAULT_CARD_COSMETICS[kind] : found.id;
}

/** Kind by kind: a short text is kept, known or not; anything else is the kind's default. */
export function normalizeCosmetics(raw: unknown): CardCosmetics {
  const record = raw !== null && typeof raw === "object" && !Array.isArray(raw) ? (raw as Record<string, unknown>) : {};
  const out: CardCosmetics = { ...DEFAULT_CARD_COSMETICS };
  for (const kind of COSMETIC_KINDS) {
    const value = record[kind];
    if (typeof value === "string" && value !== "" && value.length <= COSMETIC_ID_MAX_LENGTH) out[kind] = value;
  }
  return out;
}

function defaultStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** Absent, throwing or unparsable storage gives the defaults. */
function load(): CardCosmetics {
  try {
    const text = defaultStorage()?.getItem(COSMETICS_SETTINGS_KEY) ?? null;
    return text === null ? { ...DEFAULT_CARD_COSMETICS } : normalizeCosmetics(JSON.parse(text));
  } catch {
    return { ...DEFAULT_CARD_COSMETICS };
  }
}

function persist(picks: CardCosmetics): void {
  try {
    defaultStorage()?.setItem(COSMETICS_SETTINGS_KEY, JSON.stringify(picks));
  } catch {
    // Quota, private mode or blocked storage: the picks still hold in memory for this page.
  }
}

let current: CardCosmetics | null = null;
const listeners = new Set<() => void>();

/** The in-memory picks, loaded from storage once, on first read. */
export function readCosmetics(): CardCosmetics {
  current ??= load();
  return current;
}

/** Merges `patch` over the picks, normalizes, persists in try/catch and notifies synchronously. */
export function writeCosmetics(patch: Partial<Record<CosmeticKind, unknown>>): CardCosmetics {
  const merged: Record<string, unknown> = { ...readCosmetics() };
  for (const kind of COSMETIC_KINDS) {
    if (patch[kind] !== undefined) merged[kind] = patch[kind];
  }
  const next = normalizeCosmetics(merged);
  current = next;
  persist(next);
  for (const listener of [...listeners]) listener();
  return next;
}

export function subscribeCosmetics(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function useCosmetics(): CardCosmetics {
  return useSyncExternalStore(subscribeCosmetics, readCosmetics, readCosmetics);
}

/**
 * What `face` wears, each kind resolved to a known id: the defaults where `picks` is null (a face
 * that is not the viewer's own), the default foil on a face with its rarity foil (Mythic, Radiant)
 * and the default finish and border on a Radiant face, whose gold is its own.
 */
export function wornCosmetics(face: FaceModel, picks: CardCosmetics | null): Record<CosmeticKind, CosmeticId> {
  if (picks === null) return DEFAULT_CARD_COSMETICS;
  return {
    foil: foilFor(face, true) === "none" ? resolveCosmetic("foil", picks.foil) : DEFAULT_CARD_COSMETICS.foil,
    finish: face.radiant ? DEFAULT_CARD_COSMETICS.finish : resolveCosmetic("finish", picks.finish),
    border: face.radiant ? DEFAULT_CARD_COSMETICS.border : resolveCosmetic("border", picks.border),
  };
}

// ---------------------------------------------------------------------------------------------
// The cap on moving foils. A face that wants its foil to move claims a slot when it mounts and
// gives it back when it unmounts or stops wanting it; past COSMETIC_FOIL_ANIMATED_MAX it waits in
// line, first come first served, and moves once a slot passes to it.
// ---------------------------------------------------------------------------------------------

const moving = new Set<symbol>();
const queued: symbol[] = [];
const slotListeners = new Set<() => void>();

function notifySlots(): void {
  for (const listener of [...slotListeners]) listener();
}

function claimSlot(token: symbol): void {
  if (moving.size < COSMETIC_FOIL_ANIMATED_MAX) moving.add(token);
  else queued.push(token);
  notifySlots();
}

function releaseSlot(token: symbol): void {
  moving.delete(token);
  const waiting = queued.indexOf(token);
  if (waiting >= 0) queued.splice(waiting, 1);
  while (moving.size < COSMETIC_FOIL_ANIMATED_MAX && queued.length > 0) {
    const next = queued.shift();
    if (next !== undefined) moving.add(next);
  }
  notifySlots();
}

function subscribeSlots(listener: () => void): () => void {
  slotListeners.add(listener);
  return () => {
    slotListeners.delete(listener);
  };
}

/** Whether this face's foil may move now: it wants to, and holds one of the slots. */
function useFoilSlot(wanted: boolean): boolean {
  const [token] = useState(() => Symbol("cosmetic-foil"));
  useEffect(() => {
    if (!wanted) return undefined;
    claimSlot(token);
    return () => {
      releaseSlot(token);
    };
  }, [wanted, token]);
  const holds = useSyncExternalStore(
    subscribeSlots,
    () => moving.has(token),
    () => false,
  );
  return wanted && holds;
}

/** The attributes a face wears for its cosmetics: none at all for the defaults. */
export type CosmeticAttributes = {
  "data-cosmetic-foil"?: CosmeticId;
  "data-cosmetic-motion"?: "animated" | "static";
  "data-finish"?: CosmeticId;
  "data-frame"?: CosmeticId;
};

/**
 * The cosmetic attributes of a face: the device's picks when `worn`, the defaults otherwise.
 * `animatedFoil` is the card settings' switch, which stills a cosmetic foil as it does the rarity
 * foil. Its hooks run on every face, worn or not.
 */
export function useCosmeticAttributes(face: FaceModel, worn: boolean, animatedFoil: boolean): CosmeticAttributes {
  const picks = useCosmetics();
  const wears = wornCosmetics(face, worn ? picks : null);
  const foiled = wears.foil !== DEFAULT_CARD_COSMETICS.foil;
  const moves = useFoilSlot(foiled && animatedFoil);
  return {
    ...(foiled ? { "data-cosmetic-foil": wears.foil, "data-cosmetic-motion": moves ? "animated" : "static" } : {}),
    ...(wears.finish === DEFAULT_CARD_COSMETICS.finish ? {} : { "data-finish": wears.finish }),
    ...(wears.border === DEFAULT_CARD_COSMETICS.border ? {} : { "data-frame": wears.border }),
  };
}

/** Test seam: drops the in-memory picks, the listeners and every foil slot. */
export function resetCosmeticsForTests(): void {
  current = null;
  listeners.clear();
  moving.clear();
  queued.length = 0;
  slotListeners.clear();
}
