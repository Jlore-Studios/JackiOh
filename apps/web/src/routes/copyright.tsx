// DRAFT — needs owner and legal review
//
// `/copyright`: the copyright and takedown page (issue #757). Public, like the terms, and linked
// from the site footer and from `/terms`. What players put where others can see it is their
// username and the names they give (terms.tsx, "What you submit"); everything else on the site is
// JackiOh's own. The designated agent is the owner's to register with the U.S. Copyright Office and
// to fill in below (docs/compliance.md). Change the date on the page whenever the text changes.

import type { ReactElement } from "react";

import { DEFAULT_LOCALE, formatDay, t, useLocale } from "../i18n/i18n.ts";
import { CONTACT_EMAIL, CONTACT_URL } from "./SiteFooter.tsx";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";
import "./privacy.css";

export const copyrightTestid = {
  screen: "copyright-screen",
  updated: "copyright-updated",
} as const;

/** The date the text below last changed, as the page shows it. */
export const COPYRIGHT_LAST_UPDATED = "2026-10-15";

export default function CopyrightRoute(): ReactElement {
  const locale = useLocale();
  return (
    <div className="app-shell tavern privacy-screen" data-testid={copyrightTestid.screen}>
      <BackLink />

      <article className="panel panel--auth privacy-policy">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("Copyright")}</h2>
        {/* A translation says the English text governs; the English needs no such line. */}
        {locale === DEFAULT_LOCALE ? null : <p className="privacy-policy__governing">{t("The English text governs.")}</p>}
        <p className="privacy-policy__updated" data-testid={copyrightTestid.updated}>
          {t("Last updated")} <time dateTime={COPYRIGHT_LAST_UPDATED}>{formatDay(COPYRIGHT_LAST_UPDATED)}</time>
        </p>

        <section aria-labelledby="copyright-report">
          <h3 id="copyright-report">Reporting content that infringes your copyright</h3>
          <p>
            If you believe something on JackiOh infringes a copyright you own or act for, such as a username or
            a custom card&rsquo;s name or text, send a written notice to our designated agent below. Under the Digital Millennium Copyright Act
            (17 U.S.C. &sect; 512(c)(3)), the notice must include:
          </p>
          <ul>
            <li>Your physical or electronic signature, as the copyright owner or someone authorised to act for them.</li>
            <li>The copyrighted work you say is infringed.</li>
            <li>The material you say infringes it, with enough detail for us to find it on JackiOh.</li>
            <li>Your name, postal address, telephone number and email address.</li>
            <li>
              A statement that you believe in good faith that the use is not authorised by the copyright owner,
              its agent or the law.
            </li>
            <li>
              A statement that the information in the notice is accurate and, under penalty of perjury, that you
              are the copyright owner or authorised to act for them.
            </li>
          </ul>
          <p>
            When we receive a notice that has these, we remove or disable the material and tell the player
            who posted it.
          </p>
        </section>

        <section aria-labelledby="copyright-agent">
          <h3 id="copyright-agent">Our designated agent</h3>
          <p>
            Email notices to <a href={CONTACT_URL}>{CONTACT_EMAIL}</a>.
          </p>
          {/* OWNER: the agent registered at copyright.gov/dmca-directory, exactly as registered, with
              the email above. The registration expires every three years (docs/compliance.md). */}
          <p data-owner-slot="dmca-agent">
            [Owner to add: the designated agent&rsquo;s name, postal address and telephone number, as
            registered with the U.S. Copyright Office.]
          </p>
        </section>

        <section aria-labelledby="copyright-counter">
          <h3 id="copyright-counter">If your content was removed</h3>
          <p>
            If you believe material of yours was removed by mistake or misidentification, you can send our
            agent a counter-notice (17 U.S.C. &sect; 512(g)(3)) with your signature, the material and where it
            was before it was removed, a statement under penalty of perjury that you believe in good faith it
            was removed by mistake or misidentification, your name, address and telephone number, and your
            consent to the jurisdiction of the federal court for your address (or, outside the United States,
            any judicial district where JackiOh may be found) and to accept service from the person who sent the
            notice. Unless they tell us they have gone to court, we may restore the material 10 to 14 business
            days after we receive the counter-notice.
          </p>
        </section>

        <section aria-labelledby="copyright-repeat">
          <h3 id="copyright-repeat">Repeat infringers</h3>
          <p>We close the accounts of players who repeatedly infringe others&rsquo; copyrights.</p>
        </section>

        <section aria-labelledby="copyright-contact">
          <h3 id="copyright-contact">Contact</h3>
          <p>
            A notice goes to the designated agent above. Other questions: email <a href={CONTACT_URL}>{CONTACT_EMAIL}</a>.
          </p>
        </section>
      </article>
    </div>
  );
}
