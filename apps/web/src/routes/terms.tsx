// DRAFT — needs owner and legal review

import type { ReactElement } from "react";

import { CONTACT_URL } from "./SiteFooter.tsx";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./privacy.css";

export const termsTestid = {
  screen: "terms-screen",
  updated: "terms-updated",
} as const;

export const TERMS_LAST_UPDATED = "2026-10-03";

export default function TermsRoute(): ReactElement {
  return (
    <div className="app-shell tavern privacy-screen" data-testid={termsTestid.screen}>
      <BackLink />

      <article className="panel panel--auth privacy-policy">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>Terms of service</h2>
        <p className="privacy-policy__updated" data-testid={termsTestid.updated}>
          Last updated <time dateTime={TERMS_LAST_UPDATED}>{TERMS_LAST_UPDATED}</time>
        </p>

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
        <section aria-labelledby="terms-contact">
          <h3 id="terms-contact">Contact</h3>
          <p>
            Questions about these terms: <a href={CONTACT_URL}>contact us on GitHub</a>.
          </p>
        </section>
      </article>
    </div>
  );
}
