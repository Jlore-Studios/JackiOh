// DRAFT — needs owner and legal review
//
// `/privacy`: the privacy policy. Public, like the landing page, and linked from the site footer
// and from under the sign-up button.
//
// DRAFTED FROM THE CODE, NOT FROM A TEMPLATE. Every statement below is something this repository
// does: what the sign-up form sends (net/auth.ts), what the server stores (crates/server/src/db/
// migrations/), what this browser keeps (net/session.ts, auth/pkce.ts, settings/store.ts,
// tutorial/progress.ts, auth/redirect.ts), where it runs (render.yaml, vercel.json), how long it
// is kept (the retention constants in crates/server/src/config.rs, purged by migration 0013), what
// deleting an account removes (migration 0012), the summary each finished
// match leaves for the card statistics (migration 0014, SPEC §9.11) and the record of how each
// seat played it (migration 0029, R1442), and the age check and the terms each account agreed to
// (migration 0034, R757000, R757001). The minimum age is `MIN_ACCOUNT_AGE_YEARS`, the owner's to
// choose. Change the date on the page whenever the text changes: with the terms' date it makes up
// `TERMS_VERSION`, so every account is asked to agree again.

import type { ReactElement } from "react";

import {
  AUTH_PENDING_ADDRESS_TTL_SECONDS,
  CODE_ATTEMPT_RETENTION_DAYS,
  MATCH_ACTION_RETENTION_DAYS,
  MIN_ACCOUNT_AGE_YEARS,
  PLAY_TELEMETRY_RETENTION_DAYS,
} from "@jackioh/server-config";

import { DEFAULT_LOCALE, formatDay, t, useLocale } from "../i18n/i18n.ts";
import { CONTACT_URL } from "./SiteFooter.tsx";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./privacy.css";

/** Unit conversion, not configuration. */
const SECONDS_PER_HOUR = 3600;

export const privacyTestid = {
  screen: "privacy-screen",
  updated: "privacy-updated",
} as const;

/** The date the text below last changed, as the page shows it. */
export const PRIVACY_LAST_UPDATED = "2026-10-13";

export default function PrivacyRoute(): ReactElement {
  const locale = useLocale();
  return (
    <div className="app-shell tavern privacy-screen" data-testid={privacyTestid.screen}>
      <BackLink />

      <article className="panel panel--auth privacy-policy">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Privacy policy")}</h2>
        {/* A translation says the English text governs; the English needs no such line. */}
        {locale === DEFAULT_LOCALE ? null : <p className="privacy-policy__governing">{t("The English text governs.")}</p>}
        <p className="privacy-policy__updated" data-testid={privacyTestid.updated}>
          {t("Last updated")} <time dateTime={PRIVACY_LAST_UPDATED}>{formatDay(PRIVACY_LAST_UPDATED)}</time>
        </p>

        <section aria-labelledby="privacy-who">
          <h3 id="privacy-who">Who runs JackiOh</h3>
          <p>
            JackiOh is run by its developer, who publishes its code at{" "}
            <a href="https://github.com/jgoetzmann/JackiOh">github.com/jgoetzmann/JackiOh</a>. You can reach us
            through the contact link at the end of this page.
          </p>
        </section>

        <section aria-labelledby="privacy-collect">
          <h3 id="privacy-collect">What we collect</h3>
          <p>You can play against the AI without an account. If you create one, we collect:</p>
          <ul>
            <li>
              Your email address and password. Your browser sends them straight to Supabase, the sign-in
              service we use, and our own database doesn&rsquo;t store your password.
            </li>
            <li>
              Your account&rsquo;s status (waiting for an invite code, active or banned), your rating, and when
              the account was created and activated.
            </li>
            <li>
              That your account passed the age check, when, and the minimum age it was checked against. We
              don&rsquo;t keep your date of birth: the month and year you give are used only to work out your
              age.
            </li>
            <li>The version of the Terms and of this policy you agreed to, and when.</li>
            <li>
              A record of each invite code you try: your account, whether the code worked, the time, and a
              scrambled form of your IP address (hashed with a secret key). We use these records to limit how
              often codes can be guessed.
            </li>
            <li>
              Your decks and their names, your deck trios, your cards, the matches you queue for and play
              (every move included), their results, your Conquest series and deck picks, and the tutorial
              lessons you finish while signed in.
            </li>
            <li>
              A summary of each online match you finish, kept for card statistics: its mode, the cards each
              player&rsquo;s deck held, the ones each player started with, drew and played, who went first and
              who won. The summary itself names no account.
            </li>
            <li>
              A record of how you play each online match: how long each of your moves took and how much of
              your clock was left, the emotes you send and when, how often your clock ran out, and whether you
              conceded, offered or accepted a draw, or offered a rematch. It is kept by match and seat and names
              no account itself; the match it belongs to links it to you until it is deleted.
            </li>
            <li>
              Our hosts receive your IP address, your browser type and the pages you ask for with each
              request, and may keep them in their logs. Supabase also records your IP address and browser
              when you sign in.
            </li>
          </ul>
          <p>
            A practice game against the AI runs in your browser. Its moves are not sent to us.
          </p>
        </section>

        <section aria-labelledby="privacy-why">
          <h3 id="privacy-why">Why we collect it</h3>
          <ul>
            <li>To create your account, sign you in, and email you the links you ask for: to confirm your address or reset your password.</li>
            <li>To run online matches, keep score, and set your rating.</li>
            <li>To see how often each card wins, so the cards can be balanced.</li>
            <li>To make the computer opponents play at a human pace.</li>
            <li>To save your decks and tutorial progress so they follow your account.</li>
            <li>To stop invite-code guessing and other abuse.</li>
            <li>To keep the site running and fix problems.</li>
          </ul>
        </section>

        <section aria-labelledby="privacy-who-gets">
          <h3 id="privacy-who-gets">Who handles it</h3>
          <p>We use three services to run JackiOh:</p>
          <ul>
            <li>
              Supabase runs sign-in and our database, in the United States (Ohio). It also sends the sign-up
              and password-reset emails.
            </li>
            <li>Render runs our game server, in the United States (Ohio).</li>
            <li>Vercel hosts this website.</li>
          </ul>
          <p>In an online match, your opponent sees the moves you make.</p>
        </section>

        <section aria-labelledby="privacy-not">
          <h3 id="privacy-not">What we don&rsquo;t do</h3>
          <ul>
            <li>We don&rsquo;t sell your data or share it for advertising.</li>
            <li>We don&rsquo;t show ads, and we use no third-party analytics, tracking pixels or session recording.</li>
            <li>
              We don&rsquo;t track you across other sites, and no third party tracks you across sites through
              JackiOh. So a browser&rsquo;s &ldquo;Do Not Track&rdquo; signal doesn&rsquo;t change anything
              here.
            </li>
          </ul>
        </section>

        <section aria-labelledby="privacy-device">
          <h3 id="privacy-device">What stays on your device</h3>
          <p>JackiOh sets no cookies. It keeps these in your browser&rsquo;s own storage, and only to work:</p>
          <ul>
            <li>
              The email address of a sign-up or password reset you are waiting on, and a one-time key that
              lets the emailed link sign in on this browser.
            </li>
            <li>
              While a new account waits for its email to be confirmed: the month and year of birth you gave and
              the version of the terms you agreed to, until your first sign-in on this browser sends them to us,
              for {AUTH_PENDING_ADDRESS_TTL_SECONDS / SECONDS_PER_HOUR} hours at most.
            </li>
            <li>If this browser was refused an account: a note saying so, with no age or date in it.</li>
            <li>Your settings, such as sound and reduced motion.</li>
            <li>Your tutorial progress.</li>
            <li>
              For the open tab only, and gone when you close it: your sign-in session and a password-reset
              link you opened.
            </li>
          </ul>
          <p>
            Your sign-in session stays in the tab you signed in on. You sign in again in a new tab, or after
            you close the browser. Signing out removes your session from this browser.
          </p>
        </section>

        <section aria-labelledby="privacy-keep">
          <h3 id="privacy-keep">How long we keep it</h3>
          <ul>
            <li>Your account and game data, your age check and the terms you agreed to: until you delete your account.</li>
            <li>Invite-code attempt records: {CODE_ATTEMPT_RETENTION_DAYS} days.</li>
            <li>
              The move-by-move record of a finished match: {MATCH_ACTION_RETENTION_DAYS} days after the match
              ends. Its result, and the rating change it made, are kept.
            </li>
            <li>
              A finished match&rsquo;s summary for card statistics: for as long as the statistics are kept. It
              names no account, and once your account is deleted nothing links it to you.
            </li>
            <li>
              The record of how you played a finished match: {PLAY_TELEMETRY_RETENTION_DAYS} days after the
              match ends.
            </li>
            <li>Our hosts&rsquo; request logs: as long as each host&rsquo;s own policy says.</li>
          </ul>
        </section>

        <section aria-labelledby="privacy-choices">
          <h3 id="privacy-choices">Deleting your account or getting a copy</h3>
          <p>
            You can delete your account yourself: sign in, open your Account page, and choose &ldquo;Delete my
            account&rdquo;. That deletes your sign-in, your decks, trios and cards, your tutorial progress, the
            records of how you played, and the record of your age check and of the terms you agreed to.
            Finished matches and their results stay in the other player&rsquo;s history, with your account
            removed from them. You can&rsquo;t delete your account during a match or a Conquest series that
            hasn&rsquo;t finished.
          </p>
          <p>
            To ask for a copy of your data, contact us. Please don&rsquo;t put your email address or other
            personal details in a public GitHub issue.
          </p>
        </section>

        <section aria-labelledby="privacy-children">
          <h3 id="privacy-children">Children</h3>
          <p>
            You must be at least {MIN_ACCOUNT_AGE_YEARS} years old to have an account. Creating one asks for the
            month and year you were born, and we keep only that you passed. Playing against the AI in Practice
            needs no account and asks nothing.
          </p>
          <p>
            If we learn that an account belongs to someone under {MIN_ACCOUNT_AGE_YEARS}, for example from a
            parent or guardian, we delete it the way &ldquo;Delete my account&rdquo; does. To tell us, contact
            us.
          </p>
        </section>

        <section aria-labelledby="privacy-changes">
          <h3 id="privacy-changes">Changes to this policy</h3>
          <p>When this policy changes, we&rsquo;ll post the new version here and change the date at the top.</p>
        </section>

        <section aria-labelledby="privacy-contact">
          <h3 id="privacy-contact">Contact</h3>
          <p>
            Questions about your data, or a request about it: <a href={CONTACT_URL}>contact us on GitHub</a>.
          </p>
        </section>
      </article>
    </div>
  );
}
