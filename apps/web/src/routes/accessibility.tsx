// DRAFT — needs owner and legal review

import type { ReactElement } from "react";

import { formatDay, t } from "../i18n/i18n.ts";
import { CONTACT_EMAIL, CONTACT_URL } from "./SiteFooter.tsx";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./privacy.css";

export const accessibilityTestid = {
  screen: "accessibility-screen",
  updated: "accessibility-updated",
} as const;

export const ACCESSIBILITY_LAST_UPDATED = "2026-10-15";

export default function AccessibilityRoute(): ReactElement {
  // Split around the link, so the sentence is one entry and English reads as it always did.
  const [helpBefore = "", helpAfter = ""] = t("If something is hard to use, please tell us: {link}.").split("{link}");
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
              "We want JackiOh to be usable by as many players as we can make it. It has had an automated check against the Web Content Accessibility Guidelines (WCAG) 2.2 at levels A and AA, and no audit by a person. We do not claim that it conforms to WCAG.",
            )}
          </p>
        </section>
        <section aria-labelledby="a11y-checked">
          <h3 id="a11y-checked">{t("What we checked")}</h3>
          <p>
            {t(
              "On the date above we checked every screen with axe-core 4.14 in Chrome, against its WCAG 2.0, 2.1 and 2.2 rules at levels A and AA, and fixed every serious or critical problem it found, except those listed below. The same check runs every day, and fails when a new one appears.",
            )}
          </p>
          <ul>
            <li>{t("The home page, signing in, creating an account, resetting a password, the invite code screen and the screen that asks for your date of birth and agreement to the terms.")}</li>
            <li>{t("Play, Decks, Account and the leaderboard.")}</li>
            <li>{t("The Almanac, Patch notes and Stats.")}</li>
            <li>{t("Practice, its setup and a game, and a game on one device.")}</li>
            <li>{t("The Privacy Policy, the Terms, this page and the Copyright page.")}</li>
          </ul>
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
        <section aria-labelledby="a11y-gaps">
          <h3 id="a11y-gaps">{t("Known gaps")}</h3>
          <ul>
            <li>{t("A card shown as a button, on the home page, in the Almanac’s card pool and in a game’s prompts, is named by a short summary that names the card, not by all the text on its face.")}</li>
            <li>{t("In the deck builder, a card in the pool is named by the same short summary, and a card in the open deck by what pressing it does, “Remove” and the card’s name, not by the cost shown beside it.")}</li>
            <li>{t("Dragging a card to play it needs a mouse or a touch screen. Choosing the card and then where it goes does the same, by click or by Enter on each.")}</li>
            <li>{t("With a mouse, the card names in the game log are closer together than 24 pixels; on a touch screen each line is 44 pixels tall.")}</li>
            <li>{t("Colour contrast is checked only where the tool can measure it, so text over card art and gradients has not been checked.")}</li>
            <li>{t("No one has tested JackiOh with a screen reader or other assistive technology yet.")}</li>
          </ul>
        </section>
        <section aria-labelledby="a11y-help">
          <h3 id="a11y-help">{t("Report an issue")}</h3>
          <p>
            {helpBefore}
            <a href={CONTACT_URL}>{CONTACT_EMAIL}</a>
            {helpAfter}
          </p>
        </section>
      </article>
    </div>
  );
}
