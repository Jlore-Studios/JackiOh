// Earning in practice (SPEC §14.8, R1303000): the page's door onto `crates/server/src/api/practice.rs`.
//
// A free practice game an active account starts asks the server for its seed first, and the server
// keeps the setup beside it; once the game is over the page sends the worker's log, and the server
// folds it and plays the AI's side again before it pays anything (R1303002). The page decides
// nothing here (CLAUDE.md rule 7): a refusal's reason is the server's, relayed.

import type { Difficulty } from "@jackioh/engine/config";
import type { LeanPick, PlayerId } from "@jackioh/shared";

import type { LastBoardCard, PracticeClaim } from "../practice/protocol.ts";
import { apiRequest } from "./api.ts";

/**
 * `POST /api/practice/seeds`' body: the setup the server keeps beside the seed. A preset or a saved
 * deck travels as its cards; a random deck as the set it leans on, which the server deals again.
 */
export type PracticeSeedRequest = {
  difficulty: Difficulty;
  humanSeat: PlayerId;
  deck: { kind: "cards"; cards: string[] } | { kind: "random"; lean: LeanPick | null };
  /** The catalog version this build plays on (`CATALOG_VERSION`). */
  catalog: string;
};

/** R1303001, R1303003: the seed, the last board the server kept for a paying game, and when the seed lapses (epoch ms). */
export type PracticeSeedIssued = { seed: string; lastBoard: LastBoardCard[]; expiresAt: number };

/** What a claim earned, or why it earned nothing (`economy::practice`'s `ClaimRefusal`, snake case). */
export type PracticeClaimAnswer = { paid: true; jlore: number; xp: number } | { paid: false; reason: string };

/** 201 with the seed; a 409 while the economy is off or for a catalog the server cannot fold. */
export function requestPracticeSeed(token: string, body: PracticeSeedRequest): Promise<PracticeSeedIssued> {
  return apiRequest<PracticeSeedIssued>("/api/practice/seeds", { method: "POST", token, body });
}

/** `POST /api/practice/claims` with the worker's claim: 200 either way, a refused claim saying why. */
export function claimPracticeGame(token: string, claim: PracticeClaim): Promise<PracticeClaimAnswer> {
  return apiRequest<PracticeClaimAnswer>("/api/practice/claims", { method: "POST", token, body: claim });
}

/** The two calls the practice route earns through, injectable for its tests. */
export type PracticeEarningApi = {
  requestSeed(token: string, body: PracticeSeedRequest): Promise<PracticeSeedIssued>;
  claim(token: string, claim: PracticeClaim): Promise<PracticeClaimAnswer>;
};

export const PRACTICE_EARNING_API: PracticeEarningApi = { requestSeed: requestPracticeSeed, claim: claimPracticeGame };
