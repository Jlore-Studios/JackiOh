// `/account`'s two-step sign-in (R665): turn an authenticator app on or off for this account.
// Enrolling makes an UNVERIFIED factor that counts for nothing until a code from the app is typed
// here. That raises the session to `aal2`, which the server then requires (R665), so the raised
// session is written over the stored one, never adopted: `adoptSession` would revoke the session in use.

import { useEffect, useState, type FormEvent, type ReactElement } from "react";

import {
  AUTH_MESSAGES,
  AUTH_NOTICES,
  AuthError,
  authConfig,
  enrollTotp,
  listTotpFactors,
  removeFactor,
  verifySecondFactor,
  type TotpEnrolment,
} from "../net/auth.ts";
import { announceAccountChange } from "../net/gate.ts";
import { readSession, writeSession, type Session } from "../net/session.ts";
import { t } from "../i18n/i18n.ts";
import { requiredProblem } from "./validation.ts";

import "./two-step.css";

export const twoStepTestid = {
  section: "account-two-step",
  state: "account-two-step-state",
  start: "account-two-step-start",
  qr: "account-two-step-qr",
  secret: "account-two-step-secret",
  code: "account-two-step-code",
  confirm: "account-two-step-confirm",
  cancel: "account-two-step-cancel",
  remove: "account-two-step-remove",
  removeConfirm: "account-two-step-remove-confirm",
  notice: "account-two-step-notice",
  error: "account-two-step-error",
} as const;

type Phase =
  | { kind: "loading" }
  | { kind: "off" }
  | { kind: "enrolling"; enrolment: TotpEnrolment }
  | { kind: "on"; factorId: string }
  | { kind: "removing"; factorId: string }
  /** The factors could not be read: nothing is offered rather than the wrong thing. */
  | { kind: "unknown" };

/** The session behind `token`: the stored one when it is that token's, else the token alone. */
function sessionFor(token: string): Session {
  const stored = readSession();
  return stored !== null && stored.accessToken === token ? stored : { accessToken: token };
}

function messageOf(cause: unknown): string {
  return cause instanceof AuthError ? cause.message : AUTH_MESSAGES.service;
}

/** Nothing at all on a build with no auth provider (the end-to-end fixture builds): nothing to set up. */
export default function TwoStepSettings({ token }: { token: string }): ReactElement | null {
  const [configured] = useState(() => authConfig() !== null);
  return configured ? <TwoStepPanel token={token} /> : null;
}

function TwoStepPanel({ token }: { token: string }): ReactElement {
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    listTotpFactors(token)
      .then((factors) => {
        if (cancelled) return;
        const verified = factors.find((factor) => factor.verified);
        // Read once: a token handed down later (a renewal, this screen's own raise) is the same account.
        setPhase((now) => (now.kind !== "loading" ? now : verified === undefined ? { kind: "off" } : { kind: "on", factorId: verified.id }));
      })
      .catch(() => {
        if (!cancelled) setPhase((now) => (now.kind === "loading" ? { kind: "unknown" } : now));
      });
    return () => {
      cancelled = true;
    };
  }, [token]);

  const start = (): void => {
    setBusy(true);
    setError(null);
    setNotice(null);
    enrollTotp(sessionFor(token))
      .then((enrolment) => {
        setCode("");
        setPhase({ kind: "enrolling", enrolment });
      })
      .catch((cause: unknown) => {
        setError(messageOf(cause));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  const confirm = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    if (phase.kind !== "enrolling" || busy) return;
    const problem = requiredProblem(code, "code");
    if (problem !== null) {
      setError(problem);
      return;
    }
    const { factorId } = phase.enrolment;
    setBusy(true);
    setError(null);
    verifySecondFactor(sessionFor(token), factorId, code)
      .then((raised) => {
        // The same session, raised to aal2: written over the stored one (see the header).
        writeSession(raised);
        announceAccountChange();
        setPhase({ kind: "on", factorId });
        setNotice(AUTH_NOTICES.mfaEnrolled);
      })
      .catch((cause: unknown) => {
        setError(messageOf(cause));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  const cancelEnrolment = (): void => {
    if (phase.kind !== "enrolling") return;
    // Best effort: an unverified factor counts for nothing, and the next enrolment clears it anyway.
    void removeFactor(sessionFor(token), phase.enrolment.factorId).catch(() => undefined);
    setPhase({ kind: "off" });
    setError(null);
  };

  const remove = (): void => {
    if (phase.kind !== "removing" || busy) return;
    setBusy(true);
    setError(null);
    removeFactor(sessionFor(token), phase.factorId)
      .then(() => {
        setPhase({ kind: "off" });
        setNotice(AUTH_NOTICES.mfaRemoved);
      })
      .catch((cause: unknown) => {
        setError(messageOf(cause));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <div className="account-two-step" data-testid={twoStepTestid.section}>
      <h3 className="account-heading">{t("Two-step sign-in")}</h3>
      {phase.kind === "loading" ? <p className="auth-hint">{t("Checking…")}</p> : null}
      {phase.kind === "unknown" ? (
        <p className="auth-hint" data-testid={twoStepTestid.state} data-state="unknown">
          {t("Two-step sign-in can’t be checked just now. Reload the page to try again.")}
        </p>
      ) : null}
      {phase.kind === "off" ? (
        <>
          <p data-testid={twoStepTestid.state} data-state="off">
            {t("Off. Turn it on to ask for a code from an authenticator app each time you sign in, so a password alone can’t get into this account.")}
          </p>
          <div className="account-actions">
            <button type="button" data-testid={twoStepTestid.start} disabled={busy} onClick={start}>
              {busy ? t("Starting…") : t("Turn on two-step sign-in")}
            </button>
          </div>
        </>
      ) : null}
      {phase.kind === "enrolling" ? (
        <form className="form-card account-two-step__form" onSubmit={confirm} noValidate>
          <p>
            {t("Scan this code with an authenticator app (such as Google Authenticator, 1Password or Authy), or type the key into it by hand. Then type the 6-digit code it shows.")}
          </p>
          {phase.enrolment.qrCode !== null ? (
            <img
              className="account-two-step__qr"
              data-testid={twoStepTestid.qr}
              src={phase.enrolment.qrCode}
              alt={t("QR code for your authenticator app")}
            />
          ) : null}
          <p>
            {t("Key:")}{" "}
            <span className="account-two-step__secret" data-testid={twoStepTestid.secret}>
              {phase.enrolment.secret}
            </span>
          </p>
          <label htmlFor="account-two-step-code">{t("Code from the app")}</label>
          <input
            id="account-two-step-code"
            data-testid={twoStepTestid.code}
            type="text"
            inputMode="numeric"
            autoComplete="one-time-code"
            autoCorrect="off"
            spellCheck={false}
            autoFocus
            value={code}
            disabled={busy}
            onChange={(event) => {
              setCode(event.target.value);
              setError(null);
            }}
          />
          <div className="account-actions">
            <button type="submit" data-testid={twoStepTestid.confirm} disabled={busy} aria-busy={busy}>
              {busy ? t("Checking…") : t("Turn it on")}
            </button>
            <button
              type="button"
              className="link-button"
              data-testid={twoStepTestid.cancel}
              disabled={busy}
              onClick={cancelEnrolment}
            >
              {t("Cancel")}
            </button>
          </div>
        </form>
      ) : null}
      {phase.kind === "on" ? (
        <>
          <p data-testid={twoStepTestid.state} data-state="on">
            {t("On. Signing in asks for a code from your authenticator app.")}
          </p>
          <div className="account-actions">
            <button
              type="button"
              className="link-button"
              data-testid={twoStepTestid.remove}
              onClick={() => {
                setNotice(null);
                setError(null);
                setPhase({ kind: "removing", factorId: phase.factorId });
              }}
            >
              {t("Turn off two-step sign-in")}
            </button>
          </div>
        </>
      ) : null}
      {phase.kind === "removing" ? (
        <>
          <p data-testid={twoStepTestid.state} data-state="removing">
            {t("Turn it off? Signing in will ask only for your email and password again.")}
          </p>
          <div className="account-actions">
            <button
              type="button"
              data-testid={twoStepTestid.removeConfirm}
              disabled={busy}
              aria-busy={busy}
              onClick={remove}
            >
              {busy ? t("Turning off…") : t("Turn it off")}
            </button>
            <button
              type="button"
              className="link-button"
              disabled={busy}
              onClick={() => {
                setPhase({ kind: "on", factorId: phase.factorId });
              }}
            >
              {t("Keep it on")}
            </button>
          </div>
        </>
      ) : null}
      {notice !== null ? (
        <p className="notice" data-testid={twoStepTestid.notice} role="status">
          {t(notice)}
        </p>
      ) : null}
      {error !== null ? (
        <p className="notice" data-testid={twoStepTestid.error} role="alert">
          {t(error)}
        </p>
      ) : null}
    </div>
  );
}
