// DRAFT — needs owner and legal review
//
// `/privacy`: the privacy policy. Public, like the landing page, and linked from the site footer
// and from under the sign-up button. It is the owner's short text (R3110000), by category: what the
// code collects stays the same, and the words leave out how it works inside. The minimum age is
// `MIN_ACCOUNT_AGE_YEARS`, the owner's to choose. Change the date on the page whenever the text changes:
// with the terms' date it makes up `TERMS_VERSION`, so every account is asked to agree again (a second
// change on one day gives the version a letter, R970001; `privacy.test.tsx` fails on any change to the
// words until it moves).

import type { ReactElement } from "react";

import { MIN_ACCOUNT_AGE_YEARS } from "@jackioh/server-config";

import { DEFAULT_LOCALE, formatDay, t, useLocale } from "../i18n/i18n.ts";
import { CONTACT_EMAIL, CONTACT_URL } from "./SiteFooter.tsx";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./privacy.css";

export const privacyTestid = {
  screen: "privacy-screen",
  updated: "privacy-updated",
} as const;

/** The date the text below last changed, as the page shows it. */
export const PRIVACY_LAST_UPDATED = "2026-11-05";

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
          <h3 id="privacy-who">Who we are</h3>
          <p>
            JackiOh is made by a small team of developers. Contact: <a href={CONTACT_URL}>{CONTACT_EMAIL}</a>.
          </p>
        </section>

        <section aria-labelledby="privacy-collect">
          <h3 id="privacy-collect">What we collect</h3>
          <p>
            Your email and username, confirmation that you&rsquo;re {MIN_ACCOUNT_AGE_YEARS} or older, your decks,
            cards, matches and how you play, your friends, chats and shared replays, and technical info like your
            IP address and browser.
          </p>
        </section>

        <section aria-labelledby="privacy-why">
          <h3 id="privacy-why">Why</h3>
          <p>
            To run your account and online games, rank players, balance cards, improve our AI opponents and
            prevent abuse.
          </p>
        </section>

        <section aria-labelledby="privacy-sees">
          <h3 id="privacy-sees">Who sees it</h3>
          <p>
            Our hosting and sign-in providers process it for us. Other players see your username, rank and moves.
            Your friends, and your opponent&rsquo;s friends, can watch your online games, remove or block someone
            to stop that. Your player stats are public.
          </p>
          <p>
            We don&rsquo;t sell your data, show ads or use third-party trackers. We don&rsquo;t track you across
            sites, so a browser&rsquo;s &ldquo;Do Not Track&rdquo; setting changes nothing here.
          </p>
        </section>

        <section aria-labelledby="privacy-browser">
          <h3 id="privacy-browser">Your browser</h3>
          <p>We store your settings and sign-in in your browser.</p>
        </section>

        <section aria-labelledby="privacy-keep">
          <h3 id="privacy-keep">How long</h3>
          <p>
            We keep your data while you have an account. Card statistics that don&rsquo;t identify you may be kept
            indefinitely.
          </p>
        </section>

        <section aria-labelledby="privacy-choices">
          <h3 id="privacy-choices">Your choices</h3>
          <p>Delete your account at any time from the Account page. Email us for a copy of your data.</p>
        </section>

        <section aria-labelledby="privacy-children">
          <h3 id="privacy-children">Children</h3>
          <p>
            You must be {MIN_ACCOUNT_AGE_YEARS} or older to have an account. We delete any account we learn belongs
            to someone younger.
          </p>
        </section>

        <section aria-labelledby="privacy-changes">
          <h3 id="privacy-changes">Changes</h3>
          <p>When this policy changes, we&rsquo;ll update the date and ask you to agree again.</p>
        </section>
      </article>
    </div>
  );
}
