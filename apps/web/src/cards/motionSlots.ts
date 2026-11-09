// R3340000: the motion slots. At most COSMETIC_FOIL_ANIMATED_MAX faces move their foil, finish, border or art at once
// (R1295000, R2715001): a face that wants to move claims a slot when it mounts and gives it back when it unmounts or stops
// wanting one, and the slots are no longer first come first served. They go to, in order, the face under the pointer or
// holding focus (`rank.hovered`); the Mythics, the newest first; the Legendaries; then every other face, newest first, a
// face's age being fixed when that mounted face is created (`seq`), which a re-rank or a visibility pause keeps. A release
// hands its slot to the next face in the same tick. The hand and the board share the one budget. The page hidden pauses
// every face (`usePageVisible`) without making one newly visible the newest.
//
// Presentation only (CLAUDE.md rule 7).

import { useEffect, useRef, useState, useSyncExternalStore } from "react";

import type { Rarity } from "@jackioh/shared";

import { shedLevel, shedStillsFaces, subscribeShed } from "../fx/shed.ts";
import { COSMETIC_FOIL_ANIMATED_MAX } from "./constants.ts";

/** R3340000: the rarities that rank ahead of the rest: a Mythic first, then a Legendary. */
export type SlotRarity = "Mythic" | "Legendary" | null;

/** R3340000: what a face ranks by: whether the pointer or focus is on it and the rarity its frame shows. */
export type SlotRank = { hovered: boolean; rarity: SlotRarity };

/** A face with no claim to rank ahead: every other face's rank. */
export const REST_RANK: SlotRank = { hovered: false, rarity: null };

/** R3340000: a card's rarity as a slot ranks it: Mythic and Legendary stand, every other rarity (and none) is the rest. */
export function slotRarityOf(rarity: Rarity | null): SlotRarity {
  return rarity === "Mythic" || rarity === "Legendary" ? rarity : null;
}

/** One face's claim: who, how it ranks and when it first asked. */
export type SlotClaim = { token: symbol; rank: SlotRank; seq: number };

/** The tiers a claim falls in, first to last: hovered, Mythic, Legendary, the rest. */
function tierOf(rank: SlotRank): number {
  if (rank.hovered) return 0;
  if (rank.rarity === "Mythic") return 1;
  if (rank.rarity === "Legendary") return 2;
  return 3;
}

/** R3340000: the tokens that hold a slot of `max`: the claims by tier, then newest first. */
export function allocate(claims: readonly SlotClaim[], max: number): Set<symbol> {
  const ordered = [...claims].sort((a, b) => tierOf(a.rank) - tierOf(b.rank) || b.seq - a.seq);
  return new Set(ordered.slice(0, max).map((claim) => claim.token));
}

const claims = new Map<symbol, SlotClaim>();
const holders = new Set<symbol>();
const listeners = new Set<() => void>();
let counter = 0;

/** Re-runs the allocation and tells the faces only when who holds a slot changed. */
function refresh(): void {
  const next = allocate([...claims.values()], COSMETIC_FOIL_ANIMATED_MAX);
  if (next.size === holders.size && [...next].every((token) => holders.has(token))) return;
  holders.clear();
  for (const token of next) holders.add(token);
  for (const listener of [...listeners]) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function nextSequence(): number {
  counter += 1;
  return counter;
}

function claim(token: symbol, rank: SlotRank, seq: number): void {
  claims.set(token, { token, rank, seq });
  refresh();
}

function release(token: symbol): void {
  claims.delete(token);
  refresh();
}

/** Moves a standing claim to `rank`, keeping its age; a face with no claim is left alone. */
function rerank(token: symbol, rank: SlotRank): void {
  const standing = claims.get(token);
  if (standing === undefined || (standing.rank.hovered === rank.hovered && standing.rank.rarity === rank.rarity)) return;
  claims.set(token, { ...standing, rank });
  refresh();
}

/**
 * R3340000: whether this face holds a motion slot now: it `wanted` one (claiming when it mounts or starts to want, releasing
 * when it stops or unmounts) and its `rank` puts it among the first `COSMETIC_FOIL_ANIMATED_MAX`. Its age is assigned once
 * for this mounted face, so changes of rank and releases caused by a hidden tab or an off-screen face keep it.
 */
export function useMotionSlot(wanted: boolean, rank: SlotRank): boolean {
  const [token] = useState(() => Symbol("motion-slot"));
  const [seq] = useState(nextSequence);
  // R3351000: at shed stage 3 every face past the pointer's own holds still: the hovered face is
  // the nearest to the pointer, so it alone keeps its claim. The token and its age stay, so the
  // face reclaims its old rank when the frames recover (R3340000).
  const shed = useSyncExternalStore(subscribeShed, shedLevel, () => 0);
  const active = wanted && (!shedStillsFaces(shed) || rank.hovered);
  const standing = useRef(rank);
  useEffect(() => {
    standing.current = rank;
  });
  useEffect(() => {
    if (!active) return undefined;
    claim(token, standing.current, seq);
    return () => {
      release(token);
    };
  }, [active, token, seq]);
  useEffect(() => {
    rerank(token, { hovered: rank.hovered, rarity: rank.rarity });
  }, [token, rank.hovered, rank.rarity]);
  const holds = useSyncExternalStore(
    subscribe,
    () => holders.has(token),
    () => false,
  );
  return active && holds;
}

function subscribeVisibility(listener: () => void): () => void {
  document.addEventListener("visibilitychange", listener);
  return () => {
    document.removeEventListener("visibilitychange", listener);
  };
}

/** R3340000: whether the page is showing (not a hidden tab); true where there is no document to ask. */
export function usePageVisible(): boolean {
  return useSyncExternalStore(
    subscribeVisibility,
    () => document.visibilityState !== "hidden",
    () => true,
  );
}

/** Test seam: drops every claim, every holder and every listener. */
export function resetMotionSlotsForTests(): void {
  claims.clear();
  holders.clear();
  listeners.clear();
  counter = 0;
}
