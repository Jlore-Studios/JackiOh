// The account's custom cards (SPEC §12.9, R915008; #918, #920): the creator's door onto
// `crates/server/src/api/custom_cards.rs`, each card the engine's `CustomCard` (`crates/engine/src/wire/custom.rs`).
// The server mints the id and judges every save with the engine's validator, whose sentences a refusal
// relays (CLAUDE.md rule 7).

import type { CustomCard, CustomFrame, CustomIssue } from "@jackioh/shared";

import { ApiRequestError, apiRequest, type SendOptions } from "./api.ts";

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

/** Version 1. Refusals: 400 with the validator's `details.issues`, 409 `conflict` with `details.limit` at `limits.cards`. */
export function createCustomCard(token: string, input: CustomCardInput): Promise<{ card: CustomCard }> {
  return apiRequest<{ card: CustomCard }>("/api/custom-cards", { method: "POST", token, body: bodyOf(input) });
}

/**
 * Replaces the card whole, naming the version it replaces: a version another save moved past is a 409
 * with the current `details.version`, so a stale tab never overwrites a newer save; 404 when not the caller's.
 */
export function updateCustomCard(token: string, id: string, version: number, input: CustomCardInput): Promise<{ card: CustomCard }> {
  return apiRequest<{ card: CustomCard }>(`/api/custom-cards/${encodeURIComponent(id)}`, {
    method: "PUT",
    token,
    body: { version, ...bodyOf(input) },
  });
}

/** `DELETE /api/custom-cards/:id`: idempotent; `deleted: false` when there was nothing to delete. */
export function deleteCustomCard(token: string, id: string, options: SendOptions = {}): Promise<{ deleted: boolean }> {
  return apiRequest<{ deleted: boolean }>(`/api/custom-cards/${encodeURIComponent(id)}`, { method: "DELETE", token, ...options });
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
