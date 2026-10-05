// The Account tab's email change (R663): a signed-in player asks the auth provider to move the
// account to another address, straight from the client (`net/auth.ts` `requestEmailChange`, SPEC
// §9.2's arrow to the auth provider). The provider mails a confirmation link, and the account's
// address changes only once it is opened; the link comes back to `/login`, which says so
// (`routes/login.tsx`, `AUTH_NOTICES.emailChanged`).
//
// Nothing here decides anything about the account: the provider checks the address, and its
// refusals arrive as our own sentences (`AuthError`). A token the provider no longer takes is
// renewed once and the request sent again (R194), as the gate does for the API.

import { useState, type FormEvent, type ReactElement } from "react";

import { emailProblem, normalizeEmail } from "../auth/validation.ts";
import { AUTH_MESSAGES, AUTH_NOTICES, AuthError, authConfig, requestEmailChange } from "../net/auth.ts";
import { renewAfterUnauthorized } from "../net/gate.ts";
import { readSession } from "../net/session.ts";

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

/**
 * The address the session's access token was issued for (its `email` claim), for the line that
 * says who is signed in and to spare a request for the same address. Display only: the token is
 * not verified here, and nothing is decided on it. Null when the token has no readable claim.
 */
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

  // Only a signed-in player on a build with an auth provider has an address to change.
  const session = readSession();
  if (session === null || authConfig() === null) return null;
  const currentEmail = tokenEmail(session.accessToken);

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
        Change email
      </label>
      {currentEmail != null ? (
        <p className="settings-account__status">
          Signed in as <strong>{currentEmail}</strong>.
        </p>
      ) : null}
      <input
        id={changeEmailTestid.input}
        className="settings-email__input"
        data-testid={changeEmailTestid.input}
        type="email"
        autoComplete="email"
        inputMode="email"
        placeholder="New email address"
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
        {phase.kind === "sending" ? "Sending…" : "Send confirmation link"}
      </button>
      {phase.kind === "sent" ? (
        <p className="settings-account__status" data-testid={changeEmailTestid.sent} role="status">
          {AUTH_NOTICES.emailChangeSent}
        </p>
      ) : null}
      {phase.kind === "failed" ? (
        <p className="settings-account__status settings-email__error" id={changeEmailTestid.error} data-testid={changeEmailTestid.error} role="alert">
          {phase.message}
        </p>
      ) : null}
    </form>
  );
}
