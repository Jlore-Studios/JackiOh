// Earning in practice, as the page sees it (SPEC §14.8, R1303000): which games ask the server for a
// seed, and what the result dialog says about a claim.
//
// Page-safe: it never imports `core.ts`, and it decides no rule (CLAUDE.md rule 7). Whether a game
// pays is the server's to say once it has folded the log and played the AI's side again
// (R1303002); this only names the setups the server is asked about and words its answer.

import { CATALOG_VERSION } from "@jackioh/cards";

import { t } from "../i18n/i18n.ts";
import type { PracticeSeedRequest } from "../net/practice.ts";
import { presetById } from "./decks.ts";
import { randomDeckLean, type PracticeStartConfig } from "./protocol.ts";

/**
 * R1303000: the seed request for a free game on the setup's own deck, or null for a game that never
 * earns: a lesson, a sandbox game, one with custom cards, the creator's Try it, and a game whose AI
 * deck is dealt from the e2e fixture pool (`aiBanned`), which the server never deals.
 */
export function seedRequestOf(config: PracticeStartConfig): PracticeSeedRequest | null {
  if (config.lesson !== undefined || config.sandbox === true || config.aiBanned !== undefined) return null;
  if (config.customCards !== undefined && config.customCards.length > 0) return null;
  const base = { difficulty: config.difficulty, humanSeat: config.humanSeat, catalog: CATALOG_VERSION };
  const deck = config.deck;
  switch (deck.kind) {
    case "random":
      return { ...base, deck: { kind: "random", lean: randomDeckLean(deck) } };
    case "preset": {
      const preset = presetById(deck.id);
      return preset === undefined ? null : { ...base, deck: { kind: "cards", cards: [...preset.cards] } };
    }
    case "saved":
      return { ...base, deck: { kind: "cards", cards: [...deck.cards] } };
    case "try":
      return null;
  }
}

/** What the result dialog says about a game on a server seed: checking, paid, refused (and why), or no answer. */
export type PracticeEarning =
  | { kind: "pending" }
  | { kind: "paid"; jlore: number; xp: number }
  | { kind: "refused"; reason: string }
  | { kind: "unreachable" };

/**
 * Why the server paid nothing, in words: `economy::practice`'s `ClaimRefusal` by its wire name,
 * and a name this build does not know as it came.
 */
function refusalText(reason: string): string {
  switch (reason) {
    case "economy_off":
      return t("earning is off");
    case "unknown_seed":
      return t("the server has no seed for it");
    case "already_claimed":
      return t("it was claimed already");
    case "expired":
      return t("its seed expired");
    case "does_not_fold":
      return t("the server could not replay it");
    case "ai_mismatch":
      return t("the AI chose another move on the server");
    case "not_finished":
      return t("it did not finish");
    case "voided":
      return t("a Glitch voided it");
    case "result_mismatch":
      return t("its result was not the server's");
    default:
      return reason;
  }
}

export function earningText(earning: PracticeEarning): string {
  switch (earning.kind) {
    case "pending":
      return t("Checking this game on the server…");
    case "paid":
      return t("+{jlore} Jlore, +{xp} XP", { jlore: earning.jlore, xp: earning.xp });
    case "refused":
      return t("This game earned nothing ({reason}).", { reason: refusalText(earning.reason) });
    case "unreachable":
      return t("The server could not check this game.");
  }
}
