// The consent screen (R757000, R757001): what an account that owes the age check or agreement to the
// terms in force sees on the gated screens (main.tsx `Gated`), before the code screen and the
// username prompt.
//
// The gate decides nothing here either (CLAUDE.md rule 7): what is owed is the server's
// `ageCheckOwed` and `termsOwed` on `/api/auth/me`, and `POST /api/account/consent` counts the age,
// deletes an account under the minimum and records the agreement. This screen asks:
//
// - A PENDING ACCOUNT THAT HAS NOT ANSWERED THE AGE SCREEN. The sign-up form on this device held
//   its answer for this address (`auth/consent.ts`): it is posted without asking again. Otherwise
//   (another device, an OAuth sign-up) the screen asks the month and year and for the box.
// - AN ACCOUNT WHOSE TERMS CHANGED. The box alone, saying the Terms or the Privacy Policy changed.
//
// An answer under the minimum signs the device out and locks sign-up on it, as the sign-up form's
// own refusal does, and says only that an account cannot be made. A device that refused a sign-up
// before is not asked: it sees the refusal and Sign out.
//
// THE GATE'S FRAME is `ShellPanel`'s markup, written out as `UsernamePrompt.tsx` does, with its way
// out, Sign out.

import { useEffect, useRef, useState, type FormEvent, type ReactElement } from "react";

import { TERMS_VERSION } from "@jackioh/server-config";
import { t } from "../i18n/i18n.ts";
import { ApiRequestError, postConsent, type MeResponse } from "../net/api.ts";
import { announceAccountChange, callWithRenewal } from "../net/gate.ts";
import { loginPath, navigate } from "../net/navigate.ts";
import { clearSession, forgetPendingAddresses } from "../net/session.ts";
import { signOut, signOutLabel, useSigningOut } from "../routes/account.tsx";
import { forgetSignUpConsent, heldSignUpConsent, refuseSignUpHere, signUpRefusedHere } from "./consent.ts";
import {
  BirthMonthYear,
  CONSENT_MESSAGES,
  NO_BIRTH_ANSWER,
  SignUpRefused,
  TermsAgree,
  birthOf,
  type BirthAnswer,
} from "./ConsentFields.tsx";
import { consentTestid, shellTestid } from "./testids.ts";

import "./tavern.css";
import "./consent.css";

/** A post that failed for any reason the server did not name: nothing was recorded, and it can be sent again. */
export const CONSENT_FAILED = "Couldn’t save that just now. Try again in a moment.";
/** 409 `update_required`: the server holds newer terms than this page shows. */
export const CONSENT_TERMS_CHANGED = "The terms changed. Reload the page to read them.";

export type ConsentPromptProps = {
  token: string;
  me: MeResponse;
  /** The server recorded the answer: the gate opens its screen. */
  onAnswered: () => void;
};

type Outcome = "asking" | "posting" | "refused";

export default function ConsentPrompt({ token, me, onAnswered }: ConsentPromptProps): ReactElement {
  const ageOwed = me.ageCheckOwed === true;
  const leaving = useSigningOut();
  // Read once: whether this device refused a sign-up before, and the sign-up form's held answer.
  const [lockedHere] = useState(() => ageOwed && signUpRefusedHere());
  const [held] = useState(() => (ageOwed ? heldSignUpConsent(me.email) : null));
  const autoPost = held !== null && held.termsVersion === TERMS_VERSION && !lockedHere;
  const [outcome, setOutcome] = useState<Outcome>(lockedHere ? "refused" : autoPost ? "posting" : "asking");
  const [birth, setBirth] = useState<BirthAnswer>(() =>
    held === null ? NO_BIRTH_ANSWER : { month: String(held.birthMonth), year: String(held.birthYear) },
  );
  const [birthError, setBirthError] = useState<string | null>(null);
  const [agreed, setAgreed] = useState(false);
  const [agreeError, setAgreeError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** StrictMode runs the effect twice: the held answer is posted once. */
  const autoPosted = useRef(false);

  const send = (body: { birthYear?: number; birthMonth?: number }): void => {
    setOutcome("posting");
    setError(null);
    callWithRenewal(token, (bearer) => postConsent(bearer, { termsVersion: TERMS_VERSION, ...body }))
      .then((result) => {
        forgetSignUpConsent();
        if (result.kind === "signedOut") {
          navigate(result.expired ? loginPath({ reason: "expired" }) : loginPath(), { replace: true });
          return;
        }
        // `switched`: every screen has been told to read the account again, which re-asks if owed.
        if (result.kind === "ok") {
          onAnswered();
          announceAccountChange();
        }
      })
      .catch((cause: unknown) => {
        if (cause instanceof ApiRequestError && cause.code === "under_age") {
          // R757000: the server has deleted the account. The device signs out and remembers.
          refuseSignUpHere();
          clearSession();
          forgetPendingAddresses();
          setOutcome("refused");
          return;
        }
        forgetSignUpConsent();
        setOutcome("asking");
        if (cause instanceof ApiRequestError && cause.code === "update_required") setError(CONSENT_TERMS_CHANGED);
        else if (cause instanceof ApiRequestError && cause.status === 400) setError(cause.message);
        else setError(CONSENT_FAILED);
      });
  };

  useEffect(() => {
    if (!autoPost || held === null || autoPosted.current) return;
    autoPosted.current = true;
    send({ birthYear: held.birthYear, birthMonth: held.birthMonth });
    // Once, on mount: the held answer is the sign-up form's, and nothing here changes it.
  }, []);

  const onSubmit = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    if (outcome !== "asking") return;
    const born = birthOf(birth);
    const nextBirthError = ageOwed && born === null ? CONSENT_MESSAGES.birthMissing : null;
    const nextAgreeError = agreed ? null : CONSENT_MESSAGES.agreeMissing;
    setBirthError(nextBirthError);
    setAgreeError(nextAgreeError);
    if (nextBirthError !== null) {
      document.getElementById(birth.month === "" ? "consent-birth-month" : "consent-birth-year")?.focus();
      return;
    }
    if (nextAgreeError !== null) {
      document.getElementById("consent-agree")?.focus();
      return;
    }
    send(ageOwed && born !== null ? born : {});
  };

  const signOutButton =
    outcome === "refused" && !lockedHere ? null : (
      <button
        type="button"
        className="link-button"
        data-testid={shellTestid.signOut}
        disabled={leaving}
        aria-busy={leaving}
        onClick={() => {
          signOut();
        }}
      >
        {signOutLabel(leaving)}
      </button>
    );

  return (
    <div className="app-shell tavern shell-panel" data-testid={consentTestid.prompt} data-outcome={outcome}>
      <section className="panel panel--auth">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Before you go on")}</h2>
        {outcome === "refused" ? (
          <SignUpRefused />
        ) : (
          <form className="form-card" onSubmit={onSubmit} noValidate aria-busy={outcome === "posting"}>
            {!ageOwed ? (
              <p className="auth-hint" data-testid={consentTestid.changed}>
                {t("The Terms or the Privacy Policy changed since you last agreed to them.")}
              </p>
            ) : null}
            {ageOwed ? (
              <BirthMonthYear
                idPrefix="consent"
                value={birth}
                disabled={outcome === "posting"}
                error={birthError}
                onChange={(next) => {
                  setBirth(next);
                  setBirthError(null);
                }}
              />
            ) : null}
            <TermsAgree
              idPrefix="consent"
              checked={agreed}
              disabled={outcome === "posting"}
              error={agreeError}
              onChange={(next) => {
                setAgreed(next);
                setAgreeError(null);
              }}
            />
            {error !== null ? (
              <p className="notice" data-testid={consentTestid.error} role="alert">
                {t(error)}
              </p>
            ) : null}
            <button type="submit" data-testid={consentTestid.submit} disabled={outcome === "posting"}>
              {outcome === "posting" ? t("Saving…") : t("Continue")}
            </button>
          </form>
        )}
        {signOutButton === null ? null : <div className="row">{signOutButton}</div>}
      </section>
    </div>
  );
}
