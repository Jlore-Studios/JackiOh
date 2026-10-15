// The account's custom cards (SPEC §12.9, R915008; #918, #920): the creator's door onto
// `crates/server/src/api/custom_cards.rs`. Each card travels as the engine's `CustomCard`
// (`crates/engine/src/wire/custom.rs`), the shape a practice game's setup takes too. The server
// mints a card's id at its first save and judges every save with the engine's own validator, so a
// refusal's issues are the validator's sentences, relayed (`api.ts`' header, CLAUDE.md rule 7).

import type { CustomCard, CustomFrame, CustomIssue } from "@jackioh/shared";

import { ApiRequestError, apiRequest } from "./api.ts";

/** `GET /api/custom-cards`: the caller's cards, oldest first, and the cap they live under. */
export type CustomCardsResponse = { cards: CustomCard[]; limits: { cards: number } };

export function getCustomCards(token: string): Promise<CustomCardsResponse> {
  return apiRequest<CustomCardsResponse>("/api/custom-cards", { token });
}

/** What a save sends: the card but its id and version, which are the server's. */
export type CustomCardInput = { frame: CustomFrame; source: string; builder?: unknown };

function bodyOf(input: CustomCardInput): Record<string, unknown> {
  return {
    frame: input.frame,
    source: input.source,
    ...(input.builder === undefined || input.builder === null ? {} : { builder: input.builder }),
  };
}

/**
 * `POST /api/custom-cards`: a new card at version 1, its id minted by the server. Refusals: 400
 * `bad_request` with the validator's issues as `details.issues`, 409 `conflict` with
 * `details.limit` when the account already keeps `limits.cards`.
 */
export function createCustomCard(token: string, input: CustomCardInput): Promise<{ card: CustomCard }> {
  return apiRequest<{ card: CustomCard }>("/api/custom-cards", { method: "POST", token, body: bodyOf(input) });
}

/**
 * `PUT /api/custom-cards/:id`: replaces the card whole, naming the version it replaces. A version
 * another save has moved past is a 409 with the current one as `details.version`, so a stale tab
 * never overwrites a newer save; 404 when the id is not the caller's.
 */
export function updateCustomCard(token: string, id: string, version: number, input: CustomCardInput): Promise<{ card: CustomCard }> {
  return apiRequest<{ card: CustomCard }>(`/api/custom-cards/${encodeURIComponent(id)}`, {
    method: "PUT",
    token,
    body: { version, ...bodyOf(input) },
  });
}

/** `DELETE /api/custom-cards/:id`: idempotent; `deleted: false` when there was nothing to delete. */
export function deleteCustomCard(token: string, id: string): Promise<{ deleted: boolean }> {
  return apiRequest<{ deleted: boolean }>(`/api/custom-cards/${encodeURIComponent(id)}`, { method: "DELETE", token });
}

/** The validator's issues a 400 carries, or `[]` for any other failure. */
export function customIssuesOf(error: unknown): CustomIssue[] {
  if (!(error instanceof ApiRequestError) || error.code !== "bad_request") return [];
  const issues = (error.details as { issues?: unknown } | undefined)?.issues;
  if (!Array.isArray(issues)) return [];
  return issues.filter(
    (issue): issue is CustomIssue =>
      typeof issue === "object" &&
      issue !== null &&
      typeof (issue as { field?: unknown }).field === "string" &&
      typeof (issue as { message?: unknown }).message === "string",
  );
}

/** The version a stale save lost to (a 409 with `details.version`), or `null`. */
export function staleVersionOf(error: unknown): number | null {
  if (!(error instanceof ApiRequestError) || error.status !== 409) return null;
  const version = (error.details as { version?: unknown } | undefined)?.version;
  return typeof version === "number" ? version : null;
}
