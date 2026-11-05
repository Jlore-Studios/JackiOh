// ME-SECRET's words (Meditative MB05, R860–R865): how the client names a secret choice (the log's line about a
// prediction is jackioh-shared's, #3108). Presentation only (CLAUDE.md rule 7): the engine's `viewFor` decides who
// reads a choice, and this file only turns the choice it carries into words.

import type { SecretChoice } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";

export const SECRET_WORDS: Record<SecretChoice, string> = {
  greed: "Greed",
  attack: "Attack",
  defend: "Defend",
};

/**
 * A secret badge's text (R860): "Secret: Greed" when the view carries the choice — the owner's
 * seat, or anyone once revealed — and "Secret" otherwise.
 */
export function secretBadgeText(choice?: SecretChoice): string {
  return choice === undefined ? t("Secret") : t("Secret: {choice}", { choice: t(SECRET_WORDS[choice]) });
}
