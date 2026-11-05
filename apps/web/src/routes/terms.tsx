// DRAFT — needs owner and legal review
//
// `/terms`: the owner's short text (R3110000). `TERMS_LAST_UPDATED` and the privacy policy's date make up
// `TERMS_VERSION` (crates/server/src/config.rs), the version each account agrees to (R757001), so a change
// to the text changes the date, and every account is asked to agree again; a second change on one day gives
// the version a letter (R970001). `privacy.test.tsx` fails on any change to the words until it moves.
// "AI opponents" is R2614000's disclosure of the ladder bots (SPEC §9.12), cut down by R3110000.

import type { ReactElement } from "react";

import { MIN_ACCOUNT_AGE_YEARS } from "@jackioh/server-config";

import { DEFAULT_LOCALE, formatDay, t, useLocale } from "../i18n/i18n.ts";
import { paths } from "../net/navigate.ts";
import { CONTACT_EMAIL, CONTACT_URL } from "./SiteFooter.tsx";
import { BackLink, followInApp } from "./nav.tsx";

import "../auth/tavern.css";
import "./privacy.css";

export const termsTestid = {
  screen: "terms-screen",
  updated: "terms-updated",
} as const;

export const TERMS_LAST_UPDATED = "2026-11-05";

export default function TermsRoute(): ReactElement {
  const locale = useLocale();
  return (
    <div className="app-shell tavern privacy-screen" data-testid={termsTestid.screen}>
      <BackLink />

      <article className="panel panel--auth privacy-policy">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Terms of service")}</h2>
        {/* A translation says the English text governs; the English needs no such line. */}
        {locale === DEFAULT_LOCALE ? null : <p className="privacy-policy__governing">{t("The English text governs.")}</p>}
        <p className="privacy-policy__updated" data-testid={termsTestid.updated}>
          {t("Last updated")} <time dateTime={TERMS_LAST_UPDATED}>{formatDay(TERMS_LAST_UPDATED)}</time>
        </p>

        <p>
          By creating an account, you agree to these Terms and the{" "}
          <a href={paths.privacy} onClick={followInApp(paths.privacy)}>
            Privacy Policy
          </a>
          . You must be {MIN_ACCOUNT_AGE_YEARS} or older.
        </p>
        <section aria-labelledby="terms-fair">
          <h3 id="terms-fair">Play fair</h3>
          <p>
            Don&rsquo;t cheat, exploit bugs, use bots, or try to break the service or get into anything that
            isn&rsquo;t yours. Don&rsquo;t harass, threaten, spam or share anyone&rsquo;s personal info. We may
            moderate content and suspend or ban accounts that break these rules.
          </p>
        </section>
        <section aria-labelledby="terms-content">
          <h3 id="terms-content">Your content</h3>
          <p>
            You keep the rights to what you create or post. You let us store it and show it to the players
            it&rsquo;s meant for. Don&rsquo;t post what you don&rsquo;t have the rights to. We remove infringing
            content and ban repeat infringers (see{" "}
            <a href={paths.copyright} onClick={followInApp(paths.copyright)}>
              Copyright
            </a>
            ).
          </p>
        </section>
        <section aria-labelledby="terms-ai">
          <h3 id="terms-ai">AI opponents</h3>
          <p>
            When no one near your rank is available, ranked may match you against an AI opponent. Nothing in the
            game marks which opponents are AI.
          </p>
        </section>
        <section aria-labelledby="terms-guarantees">
          <h3 id="terms-guarantees">No guarantees</h3>
          <p>
            JackiOh is free and provided &ldquo;as is.&rdquo; We may change or end it at any time. As far as the
            law allows, we aren&rsquo;t liable for damages from using it.
          </p>
        </section>
        <section aria-labelledby="terms-changes">
          <h3 id="terms-changes">Changes</h3>
          <p>When these Terms change, we&rsquo;ll ask you to agree again.</p>
        </section>
        <section aria-labelledby="terms-contact">
          <h3 id="terms-contact">Contact</h3>
          <p>
            <a href={CONTACT_URL}>{CONTACT_EMAIL}</a>
          </p>
        </section>
      </article>
    </div>
  );
}
