// SPEC §14.14, R3013001: a card's proficiency in the Collection's card view. It shows the level the account's plays
// of the card reached, of PROFICIENCY_LEVELS, its progress to the next on the curve (PROFICIENCY_LEVEL_PLAYS), and
// the stamps: each level's, the ones the card has not reached locked with the level each needs, and No stamp. A
// click on an unlocked one picks it (`writeOwnStamp`), which `cards/proficiencySync.ts` sends to the account; a
// locked one writes nothing. Every number is the account's answer or serverConfig.ts's (CLAUDE.md rule 9), and
// the server checks every pick (CLAUDE.md rule 7).

import type { ReactElement } from "react";

import { PROFICIENCY_LEVELS } from "@jackioh/server-config";
import type { CardDef } from "@jackioh/shared";

import { ProficiencyStamp } from "../cards/ProficiencyStamp.tsx";
import { STAMP_LEVELS, playsForLevel, useOwnProficiency, writeOwnStamp } from "../cards/stamps.ts";
import FillBar from "../economy/FillBar.tsx";
import { formatNumber, t } from "../i18n/i18n.ts";
import { collectionTestid as T } from "./collection.ts";

export default function ProficiencyPanel({ def }: { def: CardDef }): ReactElement {
  const held = useOwnProficiency(def.id);
  const level = held?.level ?? 0;
  const plays = held?.plays ?? 0;
  const worn = held?.stamp ?? null;
  // The plays the level reached took, and those the next one takes; none past the top.
  const reached = playsForLevel(level) ?? 0;
  const next = playsForLevel(level + 1);
  return (
    <section className="collection-proficiency" data-testid={T.proficiency} aria-label={t("Proficiency")}>
      <h4 className="collection-proficiency__title">{t("Proficiency")}</h4>
      <p className="collection-proficiency__level" data-testid={T.proficiencyLevel}>
        {t("Level {level} of {levels}", { level: formatNumber(level), levels: formatNumber(PROFICIENCY_LEVELS) })}
      </p>
      {next === null ? (
        <p className="collection-proficiency__plays" data-testid={T.proficiencyPlays}>
          {t("Top level reached.")}
        </p>
      ) : (
        <FillBar
          label={t("Progress to the next level")}
          size={next - reached}
          value={Math.max(0, plays - reached)}
          caption={t("{plays} of {needed} plays to the next level", { plays: formatNumber(plays), needed: formatNumber(next) })}
          still
          testid={T.proficiencyPlays}
        />
      )}
      <div className="collection-proficiency__head">
        <h5 className="collection-proficiency__stamps-title">{t("Stamps")}</h5>
        <button
          type="button"
          className="collection-proficiency__none"
          data-testid={T.stampNone}
          aria-pressed={worn === null}
          onClick={() => {
            writeOwnStamp(def.id, null);
          }}
        >
          {t("No stamp")}
        </button>
      </div>
      <ul className="collection-proficiency__stamps">
        {STAMP_LEVELS.map((stamp) => {
          const locked = stamp > level;
          return (
            <li key={stamp}>
              <button
                type="button"
                className="collection-proficiency__stamp"
                data-testid={T.stamp(stamp)}
                data-locked={locked ? "true" : undefined}
                aria-pressed={worn === stamp}
                aria-disabled={locked ? true : undefined}
                onClick={() => {
                  if (!locked) writeOwnStamp(def.id, stamp);
                }}
              >
                <ProficiencyStamp level={stamp} />
                {locked ? <span className="collection-proficiency__lock">{t("Unlocks at level {level}", { level: formatNumber(stamp) })}</span> : null}
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
