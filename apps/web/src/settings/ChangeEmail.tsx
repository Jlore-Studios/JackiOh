// Email changes are confirmed by the auth provider (R663; SPEC §9.2); the address changes only
// when its link returns to `/login` (`AUTH_NOTICES.emailChanged`). Provider failures surface as
// `AuthError`; a rejected expired token is renewed once (R194).

import { useState, type FormEvent, type ReactElement } from "react";

import { emailProblem, normalizeEmail } from "../auth/validation.ts";
import { AUTH_MESSAGES, AUTH_NOTICES, AuthError, authConfig, requestEmailChange } from "../net/auth.ts";
import { renewAfterUnauthorized } from "../net/gate.ts";
import { readSession } from "../net/session.ts";
import { t } from "../i18n/i18n.ts";
import "./changeEmail.css";

type Phase =
  | { kind: "idle" }
  | { kind: "sending" }
  | { kind: "sent"; address: string }
  | { kind: "failed"; message: string };

/** The ids the e2e and unit specs read. */
export const changeEmailTestid = {
  root: "settings-email",
  input: "settings-email-input",
  submit: "settings-email-submit",
  sent: "settings-email-sent",
  error: "settings-email-error",
} as const;

/** Reads the unverified `email` claim for display only; null if unreadable. */
export function tokenEmail(token: string): string | null {
  const payload = token.split(".")[1];
  if (payload === undefined) return null;
  try {
    const json = atob(payload.replace(/-/gu, "+").replace(/_/gu, "/"));
    const claims: unknown = JSON.parse(json);
    if (typeof claims !== "object" || claims === null) return null;
    const email = (claims as { email?: unknown }).email;
    return typeof email === "string" && email.length > 0 ? email : null;
  } catch {
    return null;
  }
}

function failureMessage(cause: unknown): string {
  return cause instanceof AuthError ? cause.message : AUTH_MESSAGES.service;
}

/** The request, renewed once if the provider refused the token as expired (R194). */
async function sendChange(email: string): Promise<void> {
  const session = readSession();
  if (session === null) throw new AuthError("sessionEnded");
  try {
    await requestEmailChange(session.accessToken, email);
  } catch (cause) {
    if (!(cause instanceof AuthError) || cause.failure !== "sessionEnded") throw cause;
    const renewal = await renewAfterUnauthorized(session.accessToken);
    if (renewal.kind !== "renewed") throw cause;
    await requestEmailChange(renewal.token, email);
  }
}

export default function ChangeEmail(): ReactElement | null {
  const [value, setValue] = useState("");
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });

  const session = readSession();
  if (session === null || authConfig() === null) return null;
  const currentEmail = tokenEmail(session.accessToken);
  // "Signed in as a@b.c.": split around the address, so English keeps the text nodes it had.
  const [signedInBefore = "", signedInAfter = ""] = t("Signed in as {email}.").split("{email}");

  async function onSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    if (phase.kind === "sending") return;
    const problem = emailProblem(value);
    if (problem !== null) {
      setPhase({ kind: "failed", message: problem });
      return;
    }
    const email = normalizeEmail(value);
    if (currentEmail != null && currentEmail.toLowerCase() === email.toLowerCase()) {
      setPhase({ kind: "failed", message: AUTH_MESSAGES.sameEmail });
      return;
    }
    setPhase({ kind: "sending" });
    try {
      await sendChange(email);
      setPhase({ kind: "sent", address: email });
      setValue("");
    } catch (cause) {
      setPhase({ kind: "failed", message: failureMessage(cause) });
    }
  }

  return (
    <form
      className="settings-account settings-email"
      data-testid={changeEmailTestid.root}
      data-phase={phase.kind}
      noValidate
      onSubmit={(event) => {
        void onSubmit(event);
      }}
    >
      <label className="settings-email__label" htmlFor={changeEmailTestid.input}>
        {t("Change email")}
      </label>
      {currentEmail != null ? (
        <p className="settings-account__status">
          {signedInBefore}
          <strong>{currentEmail}</strong>
          {signedInAfter}
        </p>
      ) : null}
      <input
        id={changeEmailTestid.input}
        className="settings-email__input"
        data-testid={changeEmailTestid.input}
        type="email"
        autoComplete="email"
        inputMode="email"
        placeholder={t("New email address")}
        value={value}
        aria-invalid={phase.kind === "failed" ? true : undefined}
        aria-describedby={phase.kind === "failed" ? changeEmailTestid.error : undefined}
        onChange={(event) => {
          setValue(event.target.value);
          if (phase.kind === "failed") setPhase({ kind: "idle" });
        }}
      />
      <button
        type="submit"
        className="settings-account__action"
        data-testid={changeEmailTestid.submit}
        disabled={phase.kind === "sending"}
      >
        {phase.kind === "sending" ? t("Sending…") : t("Send confirmation link")}
      </button>
      {phase.kind === "sent" ? (
        <p className="settings-account__status" data-testid={changeEmailTestid.sent} role="status">
          {t(AUTH_NOTICES.emailChangeSent)}
        </p>
      ) : null}
      {phase.kind === "failed" ? (
        <p className="settings-account__status settings-email__error" id={changeEmailTestid.error} data-testid={changeEmailTestid.error} role="alert">
          {t(phase.message)}
        </p>
      ) : null}
    </form>
  );
}
