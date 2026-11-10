// R3477000: the small label an aimed attack's legal target wears: the damage it takes, the damage the
// attacker takes back, a skull on whichever is destroyed and the struck-through shield mark when a Divine
// Shield breaks. Every number is the engine's forecast entry as sent (attackOutcome.ts, CLAUDE.md rule 7).
// It shows only while an attacker is picked, takes no click of its own (a press on it is the target's) and
// sits over the target out of flow, so it moves nothing (attackOutcome.css). The forecast reads the face-up
// board alone, so its title says a face-down trap can still change the outcome.

import type { AttackForecast } from "@jackioh/shared";
import type { ReactElement } from "react";

import { KEYWORD_MARK } from "../cards/keywordMarks.ts";
import { t } from "../i18n/i18n.ts";
import { testid } from "./contract.ts";
import "./attackOutcome.css";

/** What the chip's title and accessible name end on: the forecast is no promise. */
export const ATTACK_OUTCOME_NOTE = "What the face-up board predicts. A face-down trap can still change it.";

const SKULL = "☠";

/** `strikesBack` is false on a hero, which never strikes back (R3100000): its chip shows the blow alone. */
export default function AttackOutcome({
  outcome,
  target,
  strikesBack,
}: {
  outcome: AttackForecast;
  target: string;
  strikesBack: boolean;
}): ReactElement {
  const words = [t("Deals {n}", { n: outcome.dealt })];
  if (outcome.kills) words.push(t("Destroys it"));
  if (outcome.pops) words.push(t("Breaks its shield"));
  if (strikesBack) {
    words.push(t("Takes {n} back", { n: outcome.taken }));
    if (!outcome.survives) words.push(t("Your Unit is destroyed"));
  }
  const note = t(ATTACK_OUTCOME_NOTE);
  return (
    <span
      className="attack-outcome"
      role="img"
      data-testid={testid.attackOutcome(target)}
      data-dealt={outcome.dealt}
      data-taken={strikesBack ? outcome.taken : undefined}
      data-kills={outcome.kills ? "true" : undefined}
      data-falls={strikesBack && !outcome.survives ? "true" : undefined}
      data-pops={outcome.pops ? "true" : undefined}
      title={note}
      aria-label={[...words, note].join(". ")}
    >
      <span className="attack-outcome-dealt" aria-hidden="true">
        −{outcome.dealt}
        {outcome.kills && <span data-mark="kills">{SKULL}</span>}
      </span>
      {outcome.pops && (
        <span className="attack-outcome-pops" data-mark="pops" aria-hidden="true">
          {KEYWORD_MARK["Divine Shield"]}
        </span>
      )}
      {strikesBack && (
        <span className="attack-outcome-taken" aria-hidden="true">
          ↩−{outcome.taken}
          {!outcome.survives && <span data-mark="falls">{SKULL}</span>}
        </span>
      )}
    </span>
  );
}
