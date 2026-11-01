// DRAFT — needs owner and legal review
//
// `/ai`: how JackiOh uses AI (R2631000). Public, like `/accessibility`, and linked from the site footer.
// Every statement is something the repository does: the computer opponents (the practice AI, §9.9;
// the ranked AI opponents, §9.12 and R2614000; the co-op team, §13.7), each reading only what its
// seat may (R185); the music composed as code by an AI model (`assets/music/LICENSES.md`); the voice
// lines' text-to-speech (`apps/web/scripts/gen-voice.mjs`, `assets/voice/SOURCES.md`); the procedural
// card art (`cards/art/`); and what online games' records are used for (R1442, R1454, the Privacy
// Policy). Nothing here calls an AI service. Keep it true when any of those changes
// (docs/compliance.md §9). It is no part of the Terms or the Privacy Policy, so changing it moves
// no `TERMS_VERSION`.

import type { ReactElement } from "react";

import { DEFAULT_LOCALE, formatDay, t, useLocale } from "../i18n/i18n.ts";
import { paths } from "../net/navigate.ts";
import { CONTACT_EMAIL, CONTACT_URL } from "./SiteFooter.tsx";
import { BackLink, followInApp } from "./nav.tsx";

import "../auth/tavern.css";
import "./privacy.css";

export const aiTestid = {
  screen: "ai-screen",
  updated: "ai-updated",
} as const;

/** The date the text below last changed, as the page shows it. */
export const AI_LAST_UPDATED = "2026-10-31";

/** Where each music track came from, in the published source. */
export const MUSIC_SOURCES_URL = "https://github.com/Jlore-Studios/JackiOh/blob/main/assets/music/LICENSES.md";

export default function AiRoute(): ReactElement {
  const locale = useLocale();
  return (
    <div className="app-shell tavern privacy-screen" data-testid={aiTestid.screen}>
      <BackLink />

      <article className="panel panel--auth privacy-policy">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <h2>{t("How JackiOh uses AI")}</h2>
        {/* A translation says the English text governs; the English needs no such line. */}
        {locale === DEFAULT_LOCALE ? null : <p className="privacy-policy__governing">{t("The English text governs.")}</p>}
        <p className="privacy-policy__updated" data-testid={aiTestid.updated}>
          {t("Last updated")} <time dateTime={AI_LAST_UPDATED}>{formatDay(AI_LAST_UPDATED)}</time>
        </p>

        <section aria-labelledby="ai-opponents">
          <h3 id="ai-opponents">The computer opponents</h3>
          <p>
            JackiOh has three kinds of computer opponent. Each plays by the same rules you do and sees only what a
            player in its seat could see.
          </p>
          <ul>
            <li>In Practice, at the difficulty you pick, and in the tutorial, an opponent that runs in your browser.</li>
            <li>
              In ranked, when no one near your rating is free after a short wait, an AI opponent that our server
              plays. Nothing in the game marks it; the{" "}
              <a href={paths.terms} onClick={followInApp(paths.terms)}>
                Terms
              </a>{" "}
              say when it comes, what it looks like and how it plays.
            </li>
            <li>In co-op, the team you and a friend play against.</li>
          </ul>
          <p>
            They choose their moves by searching the moves the rules allow, with a game-playing program we wrote
            and tune on games it plays against itself. They are not chatbots, and they say nothing to you but the
            emotes a player can send.
          </p>
        </section>

        <section aria-labelledby="ai-made">
          <h3 id="ai-made">Made with AI</h3>
          <ul>
            <li>
              The music, every track and the few bars each Legendary and Mythic card plays, was composed by an AI
              model, Anthropic&rsquo;s Claude, written as code and played with free instrument sounds. No melody
              or recording was taken from another work.{" "}
              <a href={MUSIC_SOURCES_URL} target="_blank" rel="noreferrer">
                Where each track came from
              </a>
              .
            </li>
            <li>The cards&rsquo; spoken lines are read by computer text-to-speech voices, not recorded by actors.</li>
            <li>Card art is drawn by code from each card&rsquo;s name, type and tags, not by an image-generation model.</li>
            <li>JackiOh&rsquo;s code is written with the help of AI coding assistants.</li>
          </ul>
        </section>

        <section aria-labelledby="ai-data">
          <h3 id="ai-data">What AI does with your data</h3>
          <p>
            We don&rsquo;t send anything you write, or your account&rsquo;s details, to an AI service. Records of
            online games are used as the{" "}
            <a href={paths.privacy} onClick={followInApp(paths.privacy)}>
              Privacy Policy
            </a>{" "}
            says: for the card statistics, to time the ranked AI opponents&rsquo; moves and habits like a
            person&rsquo;s, and to choose the decks they bring from decks players have taken into ranked.
          </p>
        </section>

        <section aria-labelledby="ai-contact">
          <h3 id="ai-contact">Questions</h3>
          <p>
            Questions about how JackiOh uses AI: email <a href={CONTACT_URL}>{CONTACT_EMAIL}</a>.
          </p>
        </section>
      </article>
    </div>
  );
}
