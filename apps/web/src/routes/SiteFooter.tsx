// The site footer: the privacy policy, the terms, the accessibility and copyright pages, the patch
// notes (R388), the Card Almanac (R630), the replays (R768000), and a way to reach the people who run
// JackiOh.
//
// On the landing page and the sign-in screen, where a visitor decides whether to hand over an
// email address. Kept off the board. The contact is the owner's email address, the one the Terms,
// the Privacy Policy, the Copyright and Accessibility pages and `security.txt` name too.

import type { ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import { paths } from "../net/navigate.ts";
import { followInApp } from "./nav.tsx";

import "./site-footer.css";

/** Where a player reaches the people who run JackiOh, for questions, legal notices and data requests. */
export const CONTACT_EMAIL = "jmgoetzmann6@gmail.com";
/** `CONTACT_EMAIL` as a link. */
export const CONTACT_URL = `mailto:${CONTACT_EMAIL}`;

export const siteFooterTestid = {
  root: "site-footer",
  privacy: "site-footer-privacy",
  terms: "site-footer-terms",
  accessibility: "site-footer-accessibility",
  copyright: "site-footer-copyright",
  patchNotes: "site-footer-patch-notes",
  almanac: "site-footer-almanac",
  stats: "site-footer-stats",
  replays: "site-footer-replays",
  contact: "site-footer-contact",
} as const;

export function SiteFooter(): ReactElement {
  return (
    <footer className="site-footer" data-testid={siteFooterTestid.root}>
      <nav className="site-footer__links" aria-label={t("About JackiOh")}>
        <a href={paths.privacy} data-testid={siteFooterTestid.privacy} onClick={followInApp(paths.privacy)}>
          {t("Privacy")}
        </a>
        <a href={paths.terms} data-testid={siteFooterTestid.terms} onClick={followInApp(paths.terms)}>
          {t("Terms")}
        </a>
        <a
          href={paths.accessibility}
          data-testid={siteFooterTestid.accessibility}
          onClick={followInApp(paths.accessibility)}
        >
          {t("Accessibility")}
        </a>
        <a href={paths.copyright} data-testid={siteFooterTestid.copyright} onClick={followInApp(paths.copyright)}>
          {t("Copyright")}
        </a>
        <a href={paths.patchNotes} data-testid={siteFooterTestid.patchNotes} onClick={followInApp(paths.patchNotes)}>
          {t("Patch notes")}
        </a>
        <a href={paths.almanac} data-testid={siteFooterTestid.almanac} onClick={followInApp(paths.almanac)}>
          {t("Card almanac")}
        </a>
        <a href={paths.stats} data-testid={siteFooterTestid.stats} onClick={followInApp(paths.stats)}>
          {t("Stats")}
        </a>
        <a href={paths.replays} data-testid={siteFooterTestid.replays} onClick={followInApp(paths.replays)}>
          {t("Replays")}
        </a>
        <a href={CONTACT_URL} data-testid={siteFooterTestid.contact}>
          {t("Contact us")}
        </a>
      </nav>
    </footer>
  );
}
