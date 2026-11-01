// Earning in practice (SPEC §14.8, R1303000): the page's door onto `crates/server/src/api/practice.rs`.
//
// A free practice game an active account starts asks the server for its seed first, and the server
// keeps the setup beside it; once the game is over the page sends the worker's log, and the server
// folds it to its result before it pays anything (R2560000). The page decides
// nothing here (CLAUDE.md rule 7): a refusal's reason is the server's, relayed.

import type { Difficulty } from "@jackioh/engine/config";
import type { LeanPick, PlayerId } from "@jackioh/shared";

import type { QuestPaid, TrackGain } from "../economy/xp.ts";
import type { LastBoardCard, PracticeClaim } from "../practice/protocol.ts";
import { apiRequest, type SendOptions } from "./api.ts";

/** The setup the server keeps beside the seed; a random deck travels as its lean, which the server deals again. */
export type PracticeSeedRequest = {
  difficulty: Difficulty;
  humanSeat: PlayerId;
  deck: { kind: "cards"; cards: string[] } | { kind: "random"; lean: LeanPick | null };
  /** `CATALOG_VERSION`. */
  catalog: string;
};

/** R1303001, R1303003: the seed, the last board the server kept for a paying game, and when the seed lapses (epoch ms). */
export type PracticeSeedIssued = { seed: string; lastBoard: LastBoardCard[]; expiresAt: number };

/**
 * What a claim earned, or why not (`economy::practice`'s `ClaimRefusal`). R1483000: `jloreCapped`: the
 * day's cap held Jlore back (`jlore` leaves it out). #1664: `tracks` and `quests` are
 * `economy::rewards::GameEconomy`'s, as the match socket's `economy`.
 */
export type PracticeClaimAnswer =
  | { paid: true; jlore: number; xp: number; jloreCapped: boolean; tracks: TrackGain[]; quests: QuestPaid[] }
  | { paid: false; reason: string };

/** 201 with the seed; a 409 while the economy is off or for a catalog the server cannot fold. */
export function requestPracticeSeed(token: string, body: PracticeSeedRequest): Promise<PracticeSeedIssued> {
  return apiRequest<PracticeSeedIssued>("/api/practice/seeds", { method: "POST", token, body });
}

/** `POST /api/practice/claims` with the worker's claim: 200 either way, a refused claim saying why. */
export function claimPracticeGame(token: string, claim: PracticeClaim): Promise<PracticeClaimAnswer> {
  return apiRequest<PracticeClaimAnswer>("/api/practice/claims", { method: "POST", token, body: claim });
}

export type PracticeSeedReleased = { released: true };

/** R2379000: the caller's unclaimed seed given back; a 404 when it is not the caller's open seed. */
export function releasePracticeSeed(token: string, seed: string, options: SendOptions = {}): Promise<PracticeSeedReleased> {
  return apiRequest<PracticeSeedReleased>(`/api/practice/seeds/${encodeURIComponent(seed)}`, { method: "DELETE", token, ...options });
}

/** The three calls the practice route earns through, injectable for its tests. */
export type PracticeEarningApi = {
  requestSeed(token: string, body: PracticeSeedRequest): Promise<PracticeSeedIssued>;
  claim(token: string, claim: PracticeClaim): Promise<PracticeClaimAnswer>;
  release(token: string, seed: string): Promise<PracticeSeedReleased>;
};

export const PRACTICE_EARNING_API: PracticeEarningApi = {
  requestSeed: requestPracticeSeed,
  claim: claimPracticeGame,
  // R2379000: sent as the game is given up, so it outlives the page.
  release: (token, seed) => releasePracticeSeed(token, seed, { keepalive: true }),
};
