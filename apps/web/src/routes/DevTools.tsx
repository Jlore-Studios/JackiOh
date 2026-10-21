// The Dev tools panel (SPEC §14.10, R1300000): a dev account moves its own economy by hand, so each
// part of the economy can be tried on staging and production. The quest tools (§14.9, #1304) give a
// quest at once and complete one picked from the held quests, which `GET /api/quests` lists again
// whenever a request ends. The bar tools (§14.5, R1310003) fill the card or the cosmetic token bar,
// so it pays its token at once: a card token, or the cosmetic token cycle's next step.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). `/account` mounts this only when `/api/auth/me` says
// `isDev`, and that is all the client decides: every button sends its request as typed (an amount
// goes as `Number(text)`, whatever it is), each dev route checks `DEV_PROFILE_IDS` itself, and the
// line under the buttons shows the server's answer, its refusal's message included, word for word.

import { useEffect, useId, useState, type ReactElement } from "react";

import { SHIPPED_SETS, type SetName } from "@jackioh/shared";
import { t } from "../i18n/i18n.ts";
import {
  devClear,
  devCompleteQuest,
  devFill,
  devFillBar,
  devGiveJlore,
  devGiveQuest,
  devGiveXp,
  getQuests,
  type DevBar,
  type DevClearPart,
  type QuestView,
} from "../net/api.ts";

export const devToolsTestid = {
  panel: "dev-tools",
  set: "dev-tools-set",
  xp: "dev-tools-xp",
  giveXp: "dev-tools-give-xp",
  jlore: "dev-tools-jlore",
  giveJlore: "dev-tools-give-jlore",
  fill: "dev-tools-fill",
  fillCardBar: "dev-tools-fill-card-bar",
  fillCosmeticBar: "dev-tools-fill-cosmetic-bar",
  giveQuest: "dev-tools-give-quest",
  quest: "dev-tools-quest",
  completeQuest: "dev-tools-complete-quest",
  status: "dev-tools-status",
} as const;

/** The clears, each its own button (`dev-tools-clear-<part>`), in the order §14.10 lists them. */
export const DEV_CLEAR_PARTS: readonly DevClearPart[] = ["cards", "cosmetics", "jlore", "counters", "tracks", "everything"];

/** A clear's button, in a player's words. */
function clearLabel(part: DevClearPart): string {
  switch (part) {
    case "cards":
      return t("Clear cards");
    case "cosmetics":
      return t("Clear cosmetics");
    case "jlore":
      return t("Clear Jlore");
    case "counters":
      return t("Clear bars and pity");
    case "tracks":
      return t("Clear track progress");
    case "everything":
      return t("Clear everything");
  }
}

/** R1310003: the token bars, each filled by its own button, in the order §14.5 lists them. */
const DEV_BARS: readonly DevBar[] = ["card", "cosmetic"];

/** R1310003: a bar's button: its testid and its words. */
function barButton(bar: DevBar): { testid: string; label: string } {
  switch (bar) {
    case "card":
      return { testid: devToolsTestid.fillCardBar, label: t("Fill the card token bar") };
    case "cosmetic":
      return { testid: devToolsTestid.fillCosmeticBar, label: t("Fill the cosmetic token bar") };
  }
}

/** Where the last request stands: none sent, one out, answered, or refused with the server's words. */
type Status = { kind: "idle" } | { kind: "working" } | { kind: "done" } | { kind: "failed"; message: string };

export default function DevTools({ token }: { token: string }): ReactElement {
  const ids = useId();
  const [set, setSet] = useState<SetName>(SHIPPED_SETS[0]);
  const [xp, setXp] = useState("");
  const [jlore, setJlore] = useState("");
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const working = status.kind === "working";
  const [held, setHeld] = useState<QuestView[]>([]);
  const [questId, setQuestId] = useState("");
  const chosen = held.some((quest) => quest.questId === questId) ? questId : (held[0]?.questId ?? "");

  // The held quests, listed again each time a request ends, so the pick follows a give or a complete.
  useEffect(() => {
    if (working) return;
    let cancelled = false;
    getQuests(token)
      .then((answer) => {
        if (!cancelled) setHeld(answer.quests);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [token, working]);

  const run = (request: () => Promise<unknown>): void => {
    if (working) return;
    setStatus({ kind: "working" });
    Promise.resolve()
      .then(request)
      .then(
        () => {
          setStatus({ kind: "done" });
        },
        (cause: unknown) => {
          setStatus({ kind: "failed", message: cause instanceof Error ? cause.message : String(cause) });
        },
      );
  };

  return (
    <div className="form-card dev-tools" data-testid={devToolsTestid.panel}>
      <p>{t("These change this account’s own cards, cosmetics, Jlore, tracks and quests.")}</p>

      <label htmlFor={`${ids}-set`}>{t("Set track")}</label>
      <select
        id={`${ids}-set`}
        data-testid={devToolsTestid.set}
        value={set}
        disabled={working}
        onChange={(event) => {
          setSet(event.target.value as SetName);
        }}
      >
        {SHIPPED_SETS.map((name) => (
          <option key={name} value={name}>
            {t(name)}
          </option>
        ))}
      </select>
      <label htmlFor={`${ids}-xp`}>{t("XP")}</label>
      <input
        id={`${ids}-xp`}
        data-testid={devToolsTestid.xp}
        type="number"
        inputMode="numeric"
        value={xp}
        disabled={working}
        onChange={(event) => {
          setXp(event.target.value);
        }}
      />
      <div className="account-actions">
        <button
          type="button"
          data-testid={devToolsTestid.giveXp}
          disabled={working}
          onClick={() => {
            run(() => devGiveXp(token, set, Number(xp)));
          }}
        >
          {t("Give XP")}
        </button>
      </div>

      <label htmlFor={`${ids}-jlore`}>{t("Jlore")}</label>
      <input
        id={`${ids}-jlore`}
        data-testid={devToolsTestid.jlore}
        type="number"
        inputMode="numeric"
        value={jlore}
        disabled={working}
        onChange={(event) => {
          setJlore(event.target.value);
        }}
      />
      <div className="account-actions">
        <button
          type="button"
          data-testid={devToolsTestid.giveJlore}
          disabled={working}
          onClick={() => {
            run(() => devGiveJlore(token, Number(jlore)));
          }}
        >
          {t("Give Jlore")}
        </button>
      </div>

      <div className="account-actions">
        <button
          type="button"
          data-testid={devToolsTestid.fill}
          disabled={working}
          onClick={() => {
            run(() => devFill(token));
          }}
        >
          {t("Fill collection")}
        </button>
      </div>

      <div className="account-actions">
        {DEV_BARS.map((bar) => {
          const { testid, label } = barButton(bar);
          return (
            <button
              key={bar}
              type="button"
              data-testid={testid}
              disabled={working}
              onClick={() => {
                run(() => devFillBar(token, bar));
              }}
            >
              {label}
            </button>
          );
        })}
      </div>

      <div className="account-actions">
        <button
          type="button"
          data-testid={devToolsTestid.giveQuest}
          disabled={working}
          onClick={() => {
            run(() => devGiveQuest(token));
          }}
        >
          {t("Give a quest")}
        </button>
      </div>
      <label htmlFor={`${ids}-quest`}>{t("Held quest")}</label>
      <select
        id={`${ids}-quest`}
        data-testid={devToolsTestid.quest}
        value={chosen}
        disabled={working || held.length === 0}
        onChange={(event) => {
          setQuestId(event.target.value);
        }}
      >
        {held.map((quest) => (
          <option key={quest.questId} value={quest.questId}>
            {t(quest.title)}
          </option>
        ))}
      </select>
      <div className="account-actions">
        <button
          type="button"
          data-testid={devToolsTestid.completeQuest}
          disabled={working || chosen === ""}
          onClick={() => {
            run(() => devCompleteQuest(token, chosen));
          }}
        >
          {t("Complete quest")}
        </button>
      </div>

      <div className="account-actions">
        {DEV_CLEAR_PARTS.map((part) => (
          <button
            key={part}
            type="button"
            data-testid={`dev-tools-clear-${part}`}
            disabled={working}
            onClick={() => {
              run(() => devClear(token, part));
            }}
          >
            {clearLabel(part)}
          </button>
        ))}
      </div>

      <p data-testid={devToolsTestid.status} role="status">
        {status.kind === "working" ? t("Working…") : status.kind === "done" ? t("Done.") : status.kind === "failed" ? t(status.message) : null}
      </p>
    </div>
  );
}
