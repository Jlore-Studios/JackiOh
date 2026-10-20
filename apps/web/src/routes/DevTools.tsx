// The Dev tools panel (SPEC §14.10, R1300000): a dev account moves its own economy by hand, so each
// part of the economy can be tried on staging and production.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). `/account` mounts this only when `/api/auth/me` says
// `isDev`, and that is all the client decides: every button sends its request as typed (an amount
// goes as `Number(text)`, whatever it is), each dev route checks `DEV_PROFILE_IDS` itself, and the
// line under the buttons shows the server's answer, its refusal's message included, word for word.

import { useId, useState, type ReactElement } from "react";

import { SHIPPED_SETS, type SetName } from "@jackioh/shared";
import { t } from "../i18n/i18n.ts";
import { devClear, devFill, devGiveJlore, devGiveXp, type DevClearPart } from "../net/api.ts";

export const devToolsTestid = {
  panel: "dev-tools",
  set: "dev-tools-set",
  xp: "dev-tools-xp",
  giveXp: "dev-tools-give-xp",
  jlore: "dev-tools-jlore",
  giveJlore: "dev-tools-give-jlore",
  fill: "dev-tools-fill",
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

/** Where the last request stands: none sent, one out, answered, or refused with the server's words. */
type Status = { kind: "idle" } | { kind: "working" } | { kind: "done" } | { kind: "failed"; message: string };

export default function DevTools({ token }: { token: string }): ReactElement {
  const ids = useId();
  const [set, setSet] = useState<SetName>(SHIPPED_SETS[0]);
  const [xp, setXp] = useState("");
  const [jlore, setJlore] = useState("");
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const working = status.kind === "working";

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
      <p>{t("These change this account’s own cards, cosmetics, Jlore and tracks.")}</p>

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
