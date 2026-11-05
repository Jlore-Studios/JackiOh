// Holdings and Dev tools: everything the web's Dev tools panel offers (apps/web/src/routes/DevTools.tsx,
// SPEC §14.10, R1300000), with the account's Jlore and held quests above it. Each button sends what was
// typed (an amount as `Number(text)`); each dev route checks `DEV_PROFILE_IDS` itself and the status line
// shows its answer.

import { useId, useState, type ReactElement } from "react";

import { answered, useRead, type SectionProps } from "./section.ts";
import { testids } from "./testids.ts";
import { BARS, CLEAR_PARTS, type Bar, type ClearPart } from "./types.ts";
import { SHIPPED_SETS, type SetName } from "./wire.ts";

const CLEAR_LABELS: Readonly<Record<ClearPart, string>> = {
  cards: "Clear cards",
  cosmetics: "Clear cosmetics",
  jlore: "Clear Jlore",
  counters: "Clear bars and pity",
  tracks: "Clear track progress",
  everything: "Clear everything",
};

const BAR_LABELS: Readonly<Record<Bar, string>> = {
  card: "Fill the card token bar",
  cosmetic: "Fill the cosmetic token bar",
};

export function Holdings({ api, revision, working, run }: SectionProps): ReactElement {
  const ids = useId();
  const wallet = useRead(() => api.wallet(), [api, revision]);
  const quests = useRead(() => api.quests(), [api, revision]);
  const [set, setSet] = useState<SetName>(SHIPPED_SETS[0]);
  const [xp, setXp] = useState("");
  const [jlore, setJlore] = useState("");
  const [questId, setQuestId] = useState("");
  const held = quests.value?.quests ?? [];
  const chosen = held.some((quest) => quest.questId === questId) ? questId : (held[0]?.questId ?? "");

  return (
    <section data-testid={testids.holdings} aria-labelledby={`${ids}-heading`}>
      <h2 id={`${ids}-heading`}>Holdings and Dev tools</h2>
      <p>These change this account&apos;s own cards, cosmetics, Jlore, tracks and quests.</p>

      <dl className="facts">
        <dt>Jlore</dt>
        <dd data-testid={testids.walletJlore}>{wallet.value === null ? "…" : String(wallet.value.jlore)}</dd>
      </dl>
      {wallet.error === null ? null : (
        <p className="error" data-testid={testids.loadError}>
          {wallet.error}
        </p>
      )}
      <h3>Held quests</h3>
      {quests.error === null ? null : (
        <p className="error" data-testid={testids.loadError}>
          {quests.error}
        </p>
      )}
      {held.length === 0 ? (
        <p>{quests.value === null ? "…" : "None held."}</p>
      ) : (
        <ul>
          {held.map((quest) => (
            <li key={quest.questId} data-testid={testids.questRow} data-quest-id={quest.questId}>
              {`${quest.title}: ${String(quest.progress)} of ${String(quest.target)}, pays ${String(quest.jlore)} Jlore and ${String(quest.xp)} XP`}
            </li>
          ))}
        </ul>
      )}

      <h3>Dev tools</h3>
      <div className="grid">
        <label htmlFor={`${ids}-set`}>Set track</label>
        <select
          id={`${ids}-set`}
          data-testid={testids.set}
          value={set}
          disabled={working}
          onChange={(event) => {
            setSet(event.target.value as SetName);
          }}
        >
          {SHIPPED_SETS.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
        <label htmlFor={`${ids}-xp`}>XP</label>
        <div className="row">
          <input
            id={`${ids}-xp`}
            data-testid={testids.xpInput}
            type="number"
            inputMode="numeric"
            value={xp}
            disabled={working}
            onChange={(event) => {
              setXp(event.target.value);
            }}
          />
          <button
            type="button"
            data-testid={testids.xpGive}
            disabled={working}
            onClick={() => {
              run(async () => answered(await api.giveXp(set, Number(xp))));
            }}
          >
            Give XP
          </button>
        </div>
        <label htmlFor={`${ids}-jlore`}>Jlore</label>
        <div className="row">
          <input
            id={`${ids}-jlore`}
            data-testid={testids.jloreInput}
            type="number"
            inputMode="numeric"
            value={jlore}
            disabled={working}
            onChange={(event) => {
              setJlore(event.target.value);
            }}
          />
          <button
            type="button"
            data-testid={testids.jloreGive}
            disabled={working}
            onClick={() => {
              run(async () => answered(await api.giveJlore(Number(jlore))));
            }}
          >
            Give Jlore
          </button>
        </div>
      </div>

      <div className="row">
        <button
          type="button"
          data-testid={testids.fill}
          disabled={working}
          onClick={() => {
            run(async () => answered(await api.fill()));
          }}
        >
          Add all cosmetics and cards
        </button>
        <button
          type="button"
          data-testid={testids.unlockCosmetics}
          disabled={working}
          onClick={() => {
            run(async () => answered(await api.unlockCosmetics()));
          }}
        >
          Unlock all cosmetics
        </button>
        {BARS.map((bar) => (
          <button
            key={bar}
            type="button"
            data-testid={testids.bar(bar)}
            disabled={working}
            onClick={() => {
              run(async () => answered(await api.fillBar(bar)));
            }}
          >
            {BAR_LABELS[bar]}
          </button>
        ))}
      </div>

      <div className="row">
        <button
          type="button"
          data-testid={testids.questGive}
          disabled={working}
          onClick={() => {
            run(async () => answered(await api.giveQuest()));
          }}
        >
          Give a quest
        </button>
      </div>
      <div className="grid">
        <label htmlFor={`${ids}-quest`}>Held quest</label>
        <div className="row">
          <select
            id={`${ids}-quest`}
            data-testid={testids.questSelect}
            value={chosen}
            disabled={working || held.length === 0}
            onChange={(event) => {
              setQuestId(event.target.value);
            }}
          >
            {held.map((quest) => (
              <option key={quest.questId} value={quest.questId}>
                {quest.title}
              </option>
            ))}
          </select>
          <button
            type="button"
            data-testid={testids.questComplete}
            disabled={working || chosen === ""}
            onClick={() => {
              run(async () => answered(await api.completeQuest(chosen)));
            }}
          >
            Complete quest
          </button>
        </div>
      </div>

      <div className="row">
        {CLEAR_PARTS.map((part) => (
          <button
            key={part}
            type="button"
            className="danger"
            data-testid={testids.clear(part)}
            disabled={working}
            onClick={() => {
              run(async () => answered(await api.clear(part)));
            }}
          >
            {CLEAR_LABELS[part]}
          </button>
        ))}
      </div>
    </section>
  );
}
