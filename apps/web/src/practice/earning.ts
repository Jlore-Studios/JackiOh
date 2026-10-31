// Earning in practice, as the page sees it (SPEC §14.8, R1303000): which games ask the server for a
// seed, and what the result dialog says about a claim.
//
// Page-safe: it never imports `core.ts`, and it decides no rule (CLAUDE.md rule 7). Whether a game
// pays is the server's to say once it has folded the log to its result (R2560000); this only names
// the setups the server is asked about and words its answer.

import { CATALOG_VERSION } from "@jackioh/cards";

import type { QuestPaid, TrackGain } from "../economy/xp.ts";
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

/**
 * What the result dialog says about a game on a server seed: checking, paid, refused (and why), or no
 * answer. #1664: a paid game's `tracks` and `quests` are the answer's, played by the dialog's XP
 * reveal; a paid one without either shows no bar.
 */
export type PracticeEarning =
  | { kind: "pending" }
  | { kind: "paid"; jlore: number; xp: number; jloreCapped: boolean; tracks?: readonly TrackGain[]; quests?: readonly QuestPaid[] }
  | { kind: "refused"; reason: string }
  | { kind: "unreachable" }
  | { kind: "unearned"; /** R2379000: the server's reason for issuing no seed, or the page's unreachable sentence; null when neither. */ reason: string | null };

/**
 * Why the server paid nothing, in words: `economy::practice`'s `ClaimRefusal` by its wire name,
 * and a name this build does not know as it came.
 */
function refusalText(reason: string): string {
  switch (reason) {
    case "economy_off":
      return t("rewards are switched off for now");
    case "unknown_seed":
      return t("we have no record of it starting");
    case "already_claimed":
      return t("it was claimed already");
    case "expired":
      return t("it came in too late to count");
    case "does_not_fold":
      return t("we couldn't check how it was played");
    case "not_finished":
      return t("it did not finish");
    case "voided":
      return t("a Glitch voided it");
    case "result_mismatch":
      return t("its result didn't check out");
    default:
      return reason;
  }
}

export function earningText(earning: PracticeEarning): string {
  switch (earning.kind) {
    case "pending":
      return t("Checking this game on the server…");
    case "paid": {
      const paid = t("+{jlore} Jlore, +{xp} XP", { jlore: earning.jlore, xp: earning.xp });
      // R1483000: what was paid stays in the line; the cap's holdback follows it.
      return earning.jloreCapped ? `${paid} · ${t("Daily Jlore reached")}` : paid;
    }
    case "refused":
      return t("This game earned nothing ({reason}).", { reason: refusalText(earning.reason) });
    case "unreachable":
      return t("The server could not check this game.");
    case "unearned":
      return earning.reason === null
        ? t("This game won't earn anything.")
        : t("This game won't earn anything: {reason}", { reason: t(earning.reason) });
  }
}
