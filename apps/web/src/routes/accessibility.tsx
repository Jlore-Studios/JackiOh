// DRAFT — needs owner and legal review

import type { ReactElement } from "react";

import { CONTACT_URL } from "./SiteFooter.tsx";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./privacy.css";

export const accessibilityTestid = {
  screen: "accessibility-screen",
  updated: "accessibility-updated",
} as const;

export const ACCESSIBILITY_LAST_UPDATED = "2026-10-03";

export default function AccessibilityRoute(): ReactElement {
  return (
    <div className="app-shell tavern privacy-screen" data-testid={accessibilityTestid.screen}>
      <BackLink />

      <article className="panel panel--auth privacy-policy">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>Accessibility</h2>
        <p className="privacy-policy__updated" data-testid={accessibilityTestid.updated}>
          Last updated <time dateTime={ACCESSIBILITY_LAST_UPDATED}>{ACCESSIBILITY_LAST_UPDATED}</time>
        </p>

        <section aria-labelledby="a11y-goal">
          <h3 id="a11y-goal">Our goal</h3>
          <p>
            We want JackiOh to be usable by as many players as we can make it. It has not had a formal
            accessibility audit, and some parts are not yet fully accessible.
          </p>
        </section>
        <section aria-labelledby="a11y-features">
          <h3 id="a11y-features">Current support</h3>
          <ul>
            <li>Menus, the deck builder, and the board&rsquo;s cards, zones and prompts can be used from the keyboard.</li>
            <li>Focused controls are marked, and each screen has headings.</li>
            <li>Card art is decorative and hidden from screen readers; each card&rsquo;s name and text are on the card as text.</li>
            <li>Reduce motion, effects and sound can be changed in Settings.</li>
          </ul>
        </section>
        <section aria-labelledby="a11y-help">
          <h3 id="a11y-help">Report an issue</h3>
          <p>
            If something is hard to use, please tell us: <a href={CONTACT_URL}>contact us on GitHub</a>.
          </p>
        </section>
      </article>
    </div>
  );
}
