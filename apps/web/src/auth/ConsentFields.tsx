// The age screen's and the terms box's fields (R757000, R757001), shared by the sign-up form
// (`routes/login.tsx`) and the consent screen (`ConsentPrompt.tsx`). They ask and decide nothing:
// the server counts the age and records the agreement (CLAUDE.md rule 7).
//
// NEUTRAL. The age screen asks for a month and a year, and nothing is chosen until the player
// chooses: no year is pre-selected, and nothing on it names an age or says who may not join. A
// refusal says only that an account cannot be made, and points to what needs none (Practice).
//
// THE BOX starts unticked and is the player's own act. Its links open the Terms and the Privacy
// Policy in a new tab, so reading them never loses what was typed.

import type { ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { paths } from "../net/navigate.ts";
import { followInApp } from "../routes/nav.tsx";
import { birthMonths, birthYears, utcToday } from "./consent.ts";
import { consentTestid } from "./testids.ts";

import "./consent.css";

/** A select's value: `""` until the player chooses. */
export type BirthAnswer = { month: string; year: string };

export const NO_BIRTH_ANSWER: BirthAnswer = { month: "", year: "" };

/** The chosen month and year as numbers, or null while either is not chosen. */
export function birthOf(answer: BirthAnswer): { birthYear: number; birthMonth: number } | null {
  if (answer.month === "" || answer.year === "") return null;
  return { birthYear: Number(answer.year), birthMonth: Number(answer.month) };
}

/** Problems the fields show, the client's own sentences. */
export const CONSENT_MESSAGES = {
  birthMissing: "Choose the month and year you were born.",
  agreeMissing: "Tick the box to agree to the Terms and the Privacy Policy.",
} as const;

/** Any day in a month gives its name; the 1st, at noon UTC, is in that month everywhere it is read as UTC. */
const MONTH_NAME_DAY = 1;
const MONTH_NAME_HOUR_UTC = 12;
const MONTH_NAME_YEAR = 2000;

function monthName(month: number, locale: string): string {
  return new Intl.DateTimeFormat(locale, { month: "long", timeZone: "UTC" }).format(
    Date.UTC(MONTH_NAME_YEAR, month - 1, MONTH_NAME_DAY, MONTH_NAME_HOUR_UTC),
  );
}

export type BirthMonthYearProps = {
  /** Prefixes the fields' ids, so two forms on one page never share one. */
  idPrefix: string;
  value: BirthAnswer;
  onChange: (next: BirthAnswer) => void;
  /** The problem to show under the fields, or null. */
  error: string | null;
  disabled?: boolean;
};

/** R757000: the month and year of birth, nothing chosen until the player chooses. */
export function BirthMonthYear({ idPrefix, value, onChange, error, disabled = false }: BirthMonthYearProps): ReactElement {
  const locale = useLocale();
  const errorId = `${idPrefix}-birth-error`;
  const monthId = `${idPrefix}-birth-month`;
  const yearId = `${idPrefix}-birth-year`;
  const invalid = error !== null;
  return (
    <fieldset className="consent-birth" aria-describedby={invalid ? errorId : undefined}>
      <legend>{t("Date of birth")}</legend>
      <div className="consent-birth__fields">
        <div className="consent-birth__field">
          <label htmlFor={monthId}>{t("Month")}</label>
          <select
            id={monthId}
            data-testid={consentTestid.birthMonth}
            value={value.month}
            disabled={disabled}
            aria-invalid={invalid && value.month === ""}
            onChange={(event) => {
              onChange({ ...value, month: event.target.value });
            }}
          >
            <option value="">{t("Month")}</option>
            {birthMonths().map((month) => (
              <option key={month} value={String(month)}>
                {monthName(month, locale)}
              </option>
            ))}
          </select>
        </div>
        <div className="consent-birth__field">
          <label htmlFor={yearId}>{t("Year")}</label>
          <select
            id={yearId}
            data-testid={consentTestid.birthYear}
            value={value.year}
            disabled={disabled}
            aria-invalid={invalid && value.year === ""}
            onChange={(event) => {
              onChange({ ...value, year: event.target.value });
            }}
          >
            <option value="">{t("Year")}</option>
            {birthYears(utcToday().year).map((year) => (
              <option key={year} value={String(year)}>
                {String(year)}
              </option>
            ))}
          </select>
        </div>
      </div>
      {invalid ? (
        <p id={errorId} className="auth-field-error consent-error" data-testid={consentTestid.birthError}>
          {t(error)}
        </p>
      ) : null}
    </fieldset>
  );
}

export type TermsAgreeProps = {
  idPrefix: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  error: string | null;
  disabled?: boolean;
  /** The wrapper's test id (the sign-up form keeps its own). */
  testId?: string;
};

/** R757001: the unticked box that agrees to the Terms and the Privacy Policy. */
export function TermsAgree({
  idPrefix,
  checked,
  onChange,
  error,
  disabled = false,
  testId = consentTestid.agreeLine,
}: TermsAgreeProps): ReactElement {
  const boxId = `${idPrefix}-agree`;
  const errorId = `${idPrefix}-agree-error`;
  // Split around the links, so English keeps the text nodes it always had.
  const [before = "", between = "", after = ""] = t("I agree to the {terms} and the {privacy}.").split(
    /\{terms\}|\{privacy\}/,
  );
  return (
    <div className="consent-agree" data-testid={testId}>
      <div className="consent-agree__row">
        <input
          id={boxId}
          type="checkbox"
          data-testid={consentTestid.agree}
          checked={checked}
          disabled={disabled}
          aria-invalid={error !== null}
          aria-describedby={error !== null ? errorId : undefined}
          onChange={(event) => {
            onChange(event.target.checked);
          }}
        />
        <label htmlFor={boxId}>
          {before}
          <a className="consent-link" href={paths.terms} target="_blank" rel="noopener noreferrer">
            {t("Terms")}
          </a>
          {between}
          <a className="consent-link" href={paths.privacy} target="_blank" rel="noopener noreferrer">
            {t("Privacy Policy")}
          </a>
          {after}
        </label>
      </div>
      {error !== null ? (
        <p id={errorId} className="auth-field-error consent-error" data-testid={consentTestid.agreeError}>
          {t(error)}
        </p>
      ) : null}
    </div>
  );
}

/** R757000: the neutral refusal, in place of the form, and the way to play without an account. */
export function SignUpRefused(): ReactElement {
  return (
    <div className="consent-refused" data-testid={consentTestid.refused}>
      <p className="notice" role="status">
        {t("We can't create an account for you.")}
      </p>
      <p className="auth-hint">{t("You can still play against the AI in Practice, with no account.")}</p>
      <div className="auth-actions">
        <a className="link-button" href={paths.practice} onClick={followInApp(paths.practice)}>
          {t("Go to Practice")}
        </a>
      </div>
    </div>
  );
}
