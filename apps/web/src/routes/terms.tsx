// DRAFT — needs owner and legal review
//
// `/terms`. `TERMS_LAST_UPDATED` and the privacy policy's date make up `TERMS_VERSION`
// (crates/server/src/config.rs), the version each account agrees to (R757001), so a change to the
// text changes the date, and the version with it, and every account is asked to agree again. A
// second change on a day the page already changed keeps the date and gives the version a letter
// (R970001); `privacy.test.tsx` fails on any change to the words until the version moves.

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

export const TERMS_LAST_UPDATED = "2026-10-15";

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

        <section aria-labelledby="terms-agree">
          <h3 id="terms-agree">Agreeing to these terms</h3>
          <p>
            You agree to these terms and to the{" "}
            <a href={paths.privacy} onClick={followInApp(paths.privacy)}>
              Privacy Policy
            </a>{" "}
            when you create an account, by ticking the box that says so. When either of them changes, we ask
            you to agree again before you go on, and keep a record of the version you agreed to and when.
          </p>
        </section>
        <section aria-labelledby="terms-age">
          <h3 id="terms-age">Who can have an account</h3>
          <p>
            You must be at least {MIN_ACCOUNT_AGE_YEARS} years old to have an account. Anyone can play against
            the AI in Practice without one.
          </p>
        </section>
        <section aria-labelledby="terms-use">
          <h3 id="terms-use">Using JackiOh</h3>
          <p>
            You may use JackiOh for personal play. Don&rsquo;t try to break the service, automate abuse,
            or bypass game rules or access controls.
          </p>
        </section>
        <section aria-labelledby="terms-account">
          <h3 id="terms-account">Accounts</h3>
          <p>
            You are responsible for activity on your account. Keep your sign-in details private. The
            owner may suspend or ban accounts for abuse, cheating, or violations of these terms.
          </p>
        </section>
        <section aria-labelledby="terms-content">
          <h3 id="terms-content">Content and availability</h3>
          <p>
            JackiOh is provided as-is and may change, pause, or stop at any time. Patch notes and game
            updates can change cards, mechanics, and progression.
          </p>
        </section>
        <section aria-labelledby="terms-submit">
          <h3 id="terms-submit">What you submit</h3>
          <p>
            You keep whatever rights you have in what you submit. By submitting it, you let us store it, and
            show it to the people named below, for as long as we keep it (the{" "}
            <a href={paths.privacy} onClick={followInApp(paths.privacy)}>
              Privacy Policy
            </a>{" "}
            says how long):
          </p>
          <ul>
            <li>Your username, which other players see.</li>
            <li>
              Your chat messages, which the friend you send them to sees, and the reason you give when you
              report one, which only we see.
            </li>
            <li>Your custom cards (their names, text and scripts) and the names of your decks, which only you see.</li>
            <li>The replays you share, which the friends you share them with see.</li>
          </ul>
          <p>
            Don&rsquo;t submit anything that infringes someone else&rsquo;s rights. We remove content that
            infringes, and close the accounts of players who repeatedly infringe; see{" "}
            <a href={paths.copyright} onClick={followInApp(paths.copyright)}>
              Copyright
            </a>{" "}
            for how to report it.
          </p>
        </section>
        <section aria-labelledby="terms-chat">
          <h3 id="terms-chat">Chat</h3>
          <p>
            Be respectful in chat. Don&rsquo;t harass, threaten or bully anyone, use hate speech, send spam,
            or share anyone&rsquo;s personal details, such as an address, a phone number or an email address,
            your own included.
          </p>
          <p>
            Before a message is stored, we replace each word on our filter list with asterisks. You can report
            any message a friend sends you. We review reports, and abuse in chat can get an account suspended or
            banned.
          </p>
        </section>
        <section aria-labelledby="terms-ranked">
          <h3 id="terms-ranked">Ranked play</h3>
          <p>When no suitable opponent is available, ranked may pair you with an AI opponent.</p>
        </section>
        <section aria-labelledby="terms-contact">
          <h3 id="terms-contact">Contact</h3>
          <p>
            Questions about these terms, or a legal notice: email <a href={CONTACT_URL}>{CONTACT_EMAIL}</a>.
          </p>
        </section>
      </article>
    </div>
  );
}
