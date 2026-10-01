// The site footer: the privacy policy, the patch notes with the current version (R375), and a way
// to reach the people who run JackiOh.
//
// On the landing page and the sign-in screen, where a visitor decides whether to hand over an
// email address. Kept off the board. The contact is the repository's public issue tracker until
// the owner names a private address.

import type { ReactElement } from "react";

import { CURRENT_VERSION } from "../cards/patches.ts";
import { paths } from "../net/navigate.ts";
import { followInApp } from "./nav.tsx";

import "./site-footer.css";

/** Where a player reaches the people who run JackiOh. */
export const CONTACT_URL = "https://github.com/jgoetzmann/JackiOh/issues";

export const siteFooterTestid = {
  root: "site-footer",
  privacy: "site-footer-privacy",
  terms: "site-footer-terms",
  accessibility: "site-footer-accessibility",
  patchNotes: "site-footer-patch-notes",
  version: "site-footer-version",
  contact: "site-footer-contact",
} as const;

export function SiteFooter(): ReactElement {
  return (
    <footer className="site-footer" data-testid={siteFooterTestid.root}>
      <nav className="site-footer__links" aria-label="About JackiOh">
        <a href={paths.privacy} data-testid={siteFooterTestid.privacy} onClick={followInApp(paths.privacy)}>
          Privacy
        </a>
        <a href={paths.terms} data-testid={siteFooterTestid.terms} onClick={followInApp(paths.terms)}>
          Terms
        </a>
        <a
          href={paths.accessibility}
          data-testid={siteFooterTestid.accessibility}
          onClick={followInApp(paths.accessibility)}
        >
          Accessibility
        </a>
        <a href={paths.patchNotes} data-testid={siteFooterTestid.patchNotes} onClick={followInApp(paths.patchNotes)}>
          Patch notes
        </a>
        <a href={CONTACT_URL} data-testid={siteFooterTestid.contact} rel="noopener noreferrer">
          Contact us on GitHub
        </a>
      </nav>
      <p className="site-footer__version">
        <a href={paths.patchNotes} data-testid={siteFooterTestid.version} onClick={followInApp(paths.patchNotes)}>
          JackiOh {CURRENT_VERSION}
        </a>
      </p>
    </footer>
  );
}
