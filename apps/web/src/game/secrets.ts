// ME-SECRET's words (Meditative MB05, R860–R865): how the client names a secret choice and a
// prediction's outcome. Presentation only (CLAUDE.md rule 7): the engine's `viewFor` decides who
// reads a choice, and this file only turns the choice it carries into words.

import type { PredictOutcome, SecretChoice } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";

export const SECRET_WORDS: Record<SecretChoice, string> = {
  greed: "Greed",
  attack: "Attack",
  defend: "Defend",
};

export const PREDICTED_WORDS: Record<PredictOutcome, string> = {
  won: "the reward is canceled",
  same: "the same, nothing happens",
  lost: "the penalty lands",
};

/**
 * A secret badge's text (R860): "Secret: Greed" when the view carries the choice — the owner's
 * seat, or anyone once revealed — and "Secret" otherwise.
 */
export function secretBadgeText(choice?: SecretChoice): string {
  return choice === undefined ? t("Secret") : t("Secret: {choice}", { choice: t(SECRET_WORDS[choice]) });
}
