// One level's public facts before Fight. The Assault day supplies every rule-bearing value; this
// component only names and lays them out (CLAUDE.md rule 7).

import { useMemo, useState, type ReactElement } from "react";

import type { AssaultContent, AssaultLevel } from "@jackioh/run";

import { readerName } from "../../cards/chinese.ts";
import { t, useLocale } from "../../i18n/i18n.ts";
import { paths } from "../../net/navigate.ts";
import { BackLink } from "../../routes/nav.tsx";
import { Wordmark } from "../../routes/Wordmark.tsx";
import { registeredCatalog } from "../../wasm/index.ts";
import { ASSAULT_COMMANDER_CARDS_SHOWN, ASSAULT_RARITIES } from "../config.ts";
import { upgradeName } from "../fight/UpgradeTray.tsx";
import { assaultTestid } from "../testids.ts";

type BriefingProps = {
  content: AssaultContent;
  level: AssaultLevel;
  onFight(): void;
  onAbandon(): void;
};

export function Briefing({ content, level, onFight, onAbandon }: BriefingProps): ReactElement {
  useLocale();
  const [asking, setAsking] = useState(false);
  const doctrine = content.doctrines.find((candidate) => candidate.id === level.doctrine);
  const commander = content.commanders.find((candidate) => candidate.id === level.commander);
  const cardNames = useMemo(() => {
    const defs = registeredCatalog();
    return commander?.cards.slice(0, ASSAULT_COMMANDER_CARDS_SHOWN).map((id) => {
      const def = defs[id];
      return def === undefined ? id : readerName(def);
    }) ?? [];
  }, [commander]);
  const newUpgrades = new Set(
    level.escalations.flatMap((escalation) => (escalation.kind === "upgrade" ? [escalation.upgrade] : [])),
  );
  return (
    <div className="app-shell tavern assault-screen" data-testid={assaultTestid.briefing}>
      <BackLink to={paths.play} title={t("Infinite Assault")} />
      <section className="panel panel--auth assault-panel">
        <Wordmark />
        <p className="assault-kicker">{t("Level {level}", { level: String(level.level) })}</p>
        <h2>{t(level.enemy)}</h2>
        {doctrine === undefined ? null : (
          <p>{t("{doctrine}: {line}", { doctrine: t(doctrine.name), line: t(doctrine.line) })}</p>
        )}
        {commander === undefined ? null : (
          <section className="assault-block">
            <h3>{t(commander.name)}</h3>
            <p>{t("Commander rank {rank}", { rank: String(commander.rank) })}</p>
            <ul>
              {cardNames.map((name) => <li key={name}>{name}</li>)}
            </ul>
          </section>
        )}
        <section className="assault-block">
          <h3>{t("Assault strength")}</h3>
          <ul className="assault-upgrades">
            {level.upgrades.map((upgrade) => (
              <li
                key={upgrade.kind}
                data-new={newUpgrades.has(upgrade.kind) ? "true" : undefined}
              >
                {t("{upgrade} × {stacks}", { upgrade: upgradeName(upgrade.kind), stacks: String(upgrade.stacks) })}
                {newUpgrades.has(upgrade.kind) ? ` ${t("New this level")}` : null}
              </li>
            ))}
          </ul>
          <p>{t("Deck: {count} cards", { count: String(level.deck.length) })}</p>
          <p>
            {ASSAULT_RARITIES.map((rarity, index) =>
              t("{rarity}: {count}", { rarity: t(rarity), count: String(level.rarities[index] ?? 0) }),
            ).join(t(" · "))}
          </p>
        </section>
        <div className="assault-actions">
          <button type="button" className="button-primary" data-testid={assaultTestid.fightStart} onClick={onFight}>
            {t("Fight")}
          </button>
          {asking ? (
            <div role="alertdialog" aria-label={t("Abandon this attempt?")}>
              <button type="button" onClick={() => { setAsking(false); }}>{t("Cancel")}</button>
              <button type="button" className="button-danger" onClick={onAbandon}>{t("Abandon attempt")}</button>
            </div>
          ) : (
            <button type="button" className="button-secondary" onClick={() => { setAsking(true); }}>
              {t("Abandon attempt")}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}
