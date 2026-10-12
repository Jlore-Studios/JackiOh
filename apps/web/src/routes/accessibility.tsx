// DRAFT — needs owner and legal review

import type { ReactElement } from "react";

import { formatDay, t } from "../i18n/i18n.ts";
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
        <h2>{t("Accessibility")}</h2>
        <p className="privacy-policy__updated" data-testid={accessibilityTestid.updated}>
          {t("Last updated")} <time dateTime={ACCESSIBILITY_LAST_UPDATED}>{formatDay(ACCESSIBILITY_LAST_UPDATED)}</time>
        </p>

        <section aria-labelledby="a11y-goal">
          <h3 id="a11y-goal">{t("Our goal")}</h3>
          <p>
            {t(
              "We want JackiOh to be usable by as many players as we can make it. It has not had a formal accessibility audit, and some parts are not yet fully accessible.",
            )}
          </p>
        </section>
        <section aria-labelledby="a11y-features">
          <h3 id="a11y-features">{t("Current support")}</h3>
          <ul>
            <li>{t("Menus, the deck builder, and the board’s cards, zones and prompts can be used from the keyboard.")}</li>
            <li>{t("Focused controls are marked, and each screen has headings.")}</li>
            <li>{t("Card art is decorative and hidden from screen readers; each card’s name and text are on the card as text.")}</li>
            <li>{t("Reduce motion, effects and sound can be changed in Settings.")}</li>
          </ul>
        </section>
        <section aria-labelledby="a11y-help">
          <h3 id="a11y-help">{t("Report an issue")}</h3>
          <p>
            {t("If something is hard to use, please tell us:")} <a href={CONTACT_URL}>{t("contact us on GitHub")}</a>.
          </p>
        </section>
      </article>
    </div>
  );
}
