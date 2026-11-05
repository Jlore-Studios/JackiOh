// `/privacy`, `/terms`, `/accessibility` and `/copyright`, the footer that links them, the terms box
// on "Create account" (R757001), and the tab title and canonical link `App` keeps for every path.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { MIN_ACCOUNT_AGE_YEARS, TERMS_VERSION } from "@jackioh/server-config";

import { PUBLIC_PAGES, sitemapXml } from "../../static-pages.ts";
import { consentTestid } from "../auth/testids.ts";
import { SITE_ORIGIN, paths } from "../net/navigate.ts";
import { at } from "../test/stubServer.ts";
import AccessibilityRoute, { ACCESSIBILITY_LAST_UPDATED, accessibilityTestid } from "./accessibility.tsx";
import CopyrightRoute, { COPYRIGHT_LAST_UPDATED, copyrightTestid } from "./copyright.tsx";
import LandingRoute from "./landing.tsx";
import LoginRoute, { signUpPrivacyTestid } from "./login.tsx";
import PrivacyRoute, { PRIVACY_LAST_UPDATED, privacyTestid } from "./privacy.tsx";
import { CONTACT_EMAIL, CONTACT_URL, siteFooterTestid } from "./SiteFooter.tsx";
import TermsRoute, { TERMS_LAST_UPDATED, termsTestid } from "./terms.tsx";

const { App, canonicalUrlFor, documentTitleFor } = await import("../main.tsx");

const HERE = dirname(fileURLToPath(import.meta.url));

/** A lazily-imported route chunk can outrun the 1 s default when the whole suite runs at once. */
const SLOW = { timeout: 5_000 } as const;

/**
 * R970001: each `TERMS_VERSION` and the SHA-256 of the words it covers, both pages' English text as
 * `agreedText` reads it. A row is added for each new version and never edited: a change to either
 * page's words is a new version, so the test below fails until the version moves and its row is here.
 */
const AGREED_TEXT: Readonly<Record<string, string>> = {
  "2026-10-15b/2026-10-15b": "33b0dd18c38c52456ee05a0cfaa51d881cc168411edac71c1672a9254d997e13",
  "2026-10-15b/2026-10-17": "37fbbebe49f56312ba164323c624142fa3e88a5294677e1c654c6cfc86de2774",
  "2026-10-15b/2026-10-21": "df87894475e985f8f9915b94e155f6245a841e53969765cd96dbd6d2f90f2be4",
  "2026-10-31/2026-10-31": "5ac8a025c4405ca53208415c1aa5af89e7c6dceae143d442c3e36d5d5b3fc0a3",
  "2026-10-31/2026-11-01": "b21cb121e8ed21750bf3a141392ed8f0238f8cb657b05833d802e55094387dd2",
  "2026-11-05/2026-11-05": "85285ca4c01ff4e3e39548d8795ffe6709cc7b58ea2538b557fca1b4dd7cfabb",
};

/** Both pages' English text, the Terms then the Privacy Policy, as rendered. */
function agreedText(): string {
  const text = (Page: () => ReactElement): string => {
    const { container, unmount } = render(<Page />);
    const words = container.textContent ?? "";
    unmount();
    return words;
  };
  return `${text(TermsRoute)}\n${text(PrivacyRoute)}`;
}

function canonical(): string | null {
  return document.head.querySelector<HTMLLinkElement>('link[rel="canonical"]')?.href ?? null;
}

/** A rendered page's section by its heading, which the page names as its region. */
const region = (name: string): HTMLElement => screen.getByRole("region", { name });

/** The headings of the rendered page's sections, in order. */
const headings = (): string[] => [...document.querySelectorAll("h3")].map((heading) => heading.textContent ?? "");

beforeEach(() => {
  window.localStorage.clear();
  at("/");
  // Nothing on these screens needs the server; an account read that never answers is enough.
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  document.head.querySelector('link[rel="canonical"]')?.remove();
});

describe("the privacy policy", () => {
  it("opens as a draft for the owner: its first source line says so", () => {
    const source = readFileSync(join(HERE, "privacy.tsx"), "utf8");
    expect(source.split("\n")[0]).toBe("// DRAFT — needs owner and legal review");
  });

  it("shows the date it was last updated", () => {
    render(<PrivacyRoute />);
    expect(screen.getByTestId(privacyTestid.updated)).toHaveTextContent(`Last updated ${PRIVACY_LAST_UPDATED}`);
    expect(PRIVACY_LAST_UPDATED).toBe("2026-11-05");
  });

  it("R757000 R3110000 both pages name the minimum age, and the policy keeps only confirmation of it", () => {
    const age = String(MIN_ACCOUNT_AGE_YEARS);
    render(<TermsRoute />);
    expect(screen.getByTestId(termsTestid.screen)).toHaveTextContent(`You must be ${age} or older.`);
    cleanup();
    render(<PrivacyRoute />);
    const page = screen.getByTestId(privacyTestid.screen);
    expect(page.querySelector("[data-owner-slot]"), "no slot left for the owner").toBeNull();
    expect(region("What we collect")).toHaveTextContent(`confirmation that you’re ${age} or older`);
    expect(region("Children")).toHaveTextContent(`You must be ${age} or older to have an account.`);
    expect(page.textContent).not.toMatch(/date of birth|month and year/);
  });

  it("R757001 R970001 TERMS_VERSION is the terms' date, then the privacy policy's, each with a letter for a second change that day", () => {
    // A change to either page's text changes its date, so every account is asked to agree again.
    const [terms, privacy] = TERMS_VERSION.split("/");
    expect(terms).toMatch(new RegExp(`^${TERMS_LAST_UPDATED}[b-z]?$`, "u"));
    expect(privacy).toMatch(new RegExp(`^${PRIVACY_LAST_UPDATED}[b-z]?$`, "u"));
  });

  it("R970001 a change to either page's words is a new TERMS_VERSION, even on a day the page already changed", () => {
    const digest = createHash("sha256").update(agreedText()).digest("hex");
    expect(
      AGREED_TEXT[TERMS_VERSION],
      "the Terms or the Privacy Policy changed: move TERMS_VERSION (docs/compliance.md §4) and add its row to AGREED_TEXT",
    ).toBe(digest);
  });

  it("R3110000 the policy names what it collects, why, who sees it, the browser, how long and the choices", () => {
    render(<PrivacyRoute />);
    const page = screen.getByTestId(privacyTestid.screen);
    expect(headings()).toEqual([
      "Who we are",
      "What we collect",
      "Why",
      "Who sees it",
      "Your browser",
      "How long",
      "Your choices",
      "Children",
      "Changes",
    ]);
    expect(region("What we collect")).toHaveTextContent("technical info like your IP address and browser");
    expect(region("Your browser")).toHaveTextContent("We store your settings and sign-in in your browser.");
    expect(region("How long")).toHaveTextContent("We keep your data while you have an account.");
    expect(region("How long")).toHaveTextContent("Card statistics that don’t identify you may be kept indefinitely.");
    expect(region("Your choices")).toHaveTextContent("Delete your account at any time from the Account page.");
    expect(region("Your choices")).toHaveTextContent("Email us for a copy of your data.");
    expect(region("Changes")).toHaveTextContent("we’ll update the date and ask you to agree again");
    // The owner left the providers, the retention periods and how it works inside out of the words.
    expect(page.textContent).not.toMatch(/Supabase|Cloudflare|Render|Vercel|Do Not Track.*days/);
    expect(within(region("Who we are")).getByRole("link", { name: CONTACT_EMAIL })).toHaveAttribute("href", CONTACT_URL);
  });

  it("R3110000 the policy does not sell, show ads or track, and says what Do Not Track does", () => {
    render(<PrivacyRoute />);
    const sees = region("Who sees it");
    expect(sees).toHaveTextContent("We don’t sell your data, show ads or use third-party trackers.");
    expect(sees).toHaveTextContent(
      "We don’t track you across sites, so a browser’s “Do Not Track” setting changes nothing here.",
    );
  });

  it("R1442 the policy says it collects how you play and uses it to improve the AI opponents", () => {
    render(<PrivacyRoute />);
    expect(region("What we collect")).toHaveTextContent(/how you play/);
    expect(region("Why")).toHaveTextContent(/improve our AI opponents/);
  });

  it("R2615000 the policy says friends, and the opponent’s friends, can watch, and removing or blocking stops it", () => {
    render(<PrivacyRoute />);
    expect(region("Who sees it")).toHaveTextContent(
      "Your friends, and your opponent’s friends, can watch your online games, remove or block someone to stop that.",
    );
  });

  it("R2615001 the policy says the player stats are public", () => {
    render(<PrivacyRoute />);
    expect(region("Who sees it")).toHaveTextContent("Your player stats are public.");
  });

  it("is served at /privacy, with no account needed", async () => {
    at(paths.privacy);
    render(<App />);
    expect(await screen.findByTestId(privacyTestid.screen, undefined, SLOW)).toBeInTheDocument();
  });
});

describe("the legal pages", () => {
  it("serves terms and accessibility with their updated dates", () => {
    render(
      <>
        <TermsRoute />
        <AccessibilityRoute />
      </>,
    );
    expect(screen.getByTestId(termsTestid.updated)).toHaveTextContent(`Last updated ${TERMS_LAST_UPDATED}`);
    expect(TERMS_LAST_UPDATED).toBe("2026-11-05");
    expect(screen.getByTestId(accessibilityTestid.updated)).toHaveTextContent(
      `Last updated ${ACCESSIBILITY_LAST_UPDATED}`,
    );
  });

  it("R640001 R2614000 R3110000 the terms say ranked may match an AI opponent, and that nothing marks it", () => {
    render(<TermsRoute />);
    const ai = region("AI opponents");
    expect(ai).toHaveTextContent(
      "When no one near your rank is available, ranked may match you against an AI opponent.",
    );
    expect(ai).toHaveTextContent("Nothing in the game marks which opponents are AI.");
    // The owner dropped what it looks like, how it plays and the friend challenge (R3110000).
    expect(ai.textContent).not.toMatch(/portrait|cosmetic|leaderboard|pace|friend|Jlorious/);
    expect(screen.queryByRole("region", { name: "Ranked play" })).toBeNull();
  });

  it("open as drafts for the owner: their first source lines say so", () => {
    for (const file of ["terms.tsx", "accessibility.tsx", "copyright.tsx"]) {
      const source = readFileSync(join(HERE, file), "utf8");
      expect(source.split("\n")[0], file).toBe("// DRAFT — needs owner and legal review");
    }
  });

  it("are in the sitemap", () => {
    const sitemap = sitemapXml(PUBLIC_PAGES);
    expect(sitemap).toContain(`<loc>${SITE_ORIGIN}/terms</loc>`);
    expect(sitemap).toContain(`<loc>${SITE_ORIGIN}/accessibility</loc>`);
    expect(sitemap).toContain(`<loc>${SITE_ORIGIN}/copyright</loc>`);
  });

  it("R3110000 the terms license what a player submits and link Privacy Policy and Copyright", () => {
    render(<TermsRoute />);
    const content = region("Your content");
    expect(content).toHaveTextContent("You let us store it and show it to the players it’s meant for.");
    expect(within(content).getByRole("link", { name: "Copyright" })).toHaveAttribute("href", paths.copyright);
    expect(screen.getByRole("link", { name: "Privacy Policy" })).toHaveAttribute("href", paths.privacy);
  });

  it("R3110000 the terms' sections are the owner's, in order", () => {
    render(<TermsRoute />);
    expect(headings()).toEqual(["Play fair", "Your content", "AI opponents", "No guarantees", "Changes", "Contact"]);
    expect(region("Play fair")).toHaveTextContent("use bots");
    expect(region("No guarantees")).toHaveTextContent("provided “as is.”");
    expect(region("Changes")).toHaveTextContent("we’ll ask you to agree again");
  });

  it("#970 the copyright page names a username as something a notice can be about", () => {
    render(<CopyrightRoute />);
    expect(screen.getByRole("region", { name: /reporting content that infringes your copyright/i })).toHaveTextContent(
      /such as a username,/,
    );
  });

  it("the copyright page says how to send a notice, leaves the agent to the owner, and shows its date", () => {
    render(<CopyrightRoute />);
    const page = screen.getByTestId(copyrightTestid.screen);
    expect(screen.getByTestId(copyrightTestid.updated)).toHaveTextContent(`Last updated ${COPYRIGHT_LAST_UPDATED}`);
    const report = screen.getByRole("region", { name: /reporting content that infringes your copyright/i });
    // 17 U.S.C. § 512(c)(3)'s six elements.
    expect(within(report).getAllByRole("listitem")).toHaveLength(6);
    expect(page.querySelector('[data-owner-slot="dmca-agent"]')).not.toBeNull();
    expect(screen.getByRole("region", { name: /if your content was removed/i })).toHaveTextContent(/counter-notice/);
    expect(screen.getByRole("region", { name: /repeat infringers/i })).toHaveTextContent(/close the accounts/);
  });

  it("name the contact email, and no GitHub issue tracker, on every page", () => {
    for (const [Page, name] of [
      [TermsRoute, "Contact"],
      [PrivacyRoute, "Who we are"],
      [CopyrightRoute, "Contact"],
      [AccessibilityRoute, "Report an issue"],
    ] as const) {
      render(<Page />);
      const section = screen.getByRole("region", { name });
      expect(within(section).getByRole("link", { name: CONTACT_EMAIL })).toHaveAttribute("href", CONTACT_URL);
      expect(document.body.innerHTML).not.toMatch(/github\.com\/[^"]*\/issues/);
      cleanup();
    }
    expect(CONTACT_URL).toBe(`mailto:${CONTACT_EMAIL}`);
    const security = readFileSync(join(HERE, "../../public/.well-known/security.txt"), "utf8");
    expect(security).toContain(`Contact: ${CONTACT_URL}`);
  });

  it("serves /terms, /accessibility and /copyright with no account needed", async () => {
    at(paths.terms);
    render(<App />);
    expect(await screen.findByTestId(termsTestid.screen, undefined, SLOW)).toBeInTheDocument();
    cleanup();
    at(paths.accessibility);
    render(<App />);
    expect(await screen.findByTestId(accessibilityTestid.screen, undefined, SLOW)).toBeInTheDocument();
    cleanup();
    at(paths.copyright);
    render(<App />);
    expect(await screen.findByTestId(copyrightTestid.screen, undefined, SLOW)).toBeInTheDocument();
  });
});

describe("the site footer", () => {
  it("is on the landing page, with the privacy policy and a contact", () => {
    render(<LandingRoute />);
    const footer = screen.getByTestId(siteFooterTestid.root);
    expect(within(footer).getByTestId(siteFooterTestid.privacy)).toHaveAttribute("href", paths.privacy);
    expect(within(footer).getByTestId(siteFooterTestid.terms)).toHaveAttribute("href", paths.terms);
    expect(within(footer).getByTestId(siteFooterTestid.accessibility)).toHaveAttribute(
      "href",
      paths.accessibility,
    );
    expect(within(footer).getByTestId(siteFooterTestid.copyright)).toHaveAttribute("href", paths.copyright);
    expect(within(footer).getByTestId(siteFooterTestid.contact)).toHaveAttribute("href", CONTACT_URL);
  });

  it("is on the sign-in screen, in both of its modes", async () => {
    render(<LoginRoute />);
    expect(screen.getByTestId(siteFooterTestid.root)).toBeInTheDocument();
    await userEvent.click(screen.getByTestId("login-mode"));
    expect(screen.getByTestId(siteFooterTestid.root)).toBeInTheDocument();
  });

  it("opens the privacy policy in place", async () => {
    at(paths.landing);
    render(<App />);
    await userEvent.click(screen.getByTestId(siteFooterTestid.privacy));
    expect(window.location.pathname).toBe(paths.privacy);
    expect(await screen.findByTestId(privacyTestid.screen, undefined, SLOW)).toBeInTheDocument();
  });
});

describe("under Create account", () => {
  it("R757001 an unticked box agrees to the Terms and the Privacy Policy, and only on the sign-up form", async () => {
    render(<LoginRoute />);
    expect(screen.queryByTestId(signUpPrivacyTestid)).toBeNull();
    await userEvent.click(screen.getByTestId("login-mode"));
    const line = screen.getByTestId(signUpPrivacyTestid);
    const box = within(line).getByRole("checkbox", { name: "I agree to the Terms and the Privacy Policy." });
    expect(box).toBe(screen.getByTestId(consentTestid.agree));
    expect(box).not.toBeChecked();
    // Each link opens its page in a new tab, so reading it loses nothing typed.
    for (const [name, href] of [
      ["Terms", paths.terms],
      ["Privacy Policy", paths.privacy],
    ] as const) {
      const link = within(line).getByRole("link", { name });
      expect(link).toHaveAttribute("href", href);
      expect(link).toHaveAttribute("target", "_blank");
      expect(link).toHaveAttribute("rel", "noopener noreferrer");
    }
  });
});

describe("the tab title and the canonical link", () => {
  it("names each screen in the tab", () => {
    expect(documentTitleFor(paths.landing)).toBe("JackiOh");
    expect(documentTitleFor(paths.login)).toBe("Sign in · JackiOh");
    expect(documentTitleFor(paths.practice)).toBe("Practice · JackiOh");
    expect(documentTitleFor(paths.decks)).toBe("Decks · JackiOh");
    expect(documentTitleFor(paths.play)).toBe("Play online · JackiOh");
    expect(documentTitleFor(paths.account)).toBe("Account · JackiOh");
    expect(documentTitleFor(paths.invite)).toBe("Invite code · JackiOh");
    expect(documentTitleFor(paths.privacy)).toBe("Privacy · JackiOh");
    expect(documentTitleFor(paths.terms)).toBe("Terms · JackiOh");
    expect(documentTitleFor(paths.accessibility)).toBe("Accessibility · JackiOh");
    expect(documentTitleFor(paths.copyright)).toBe("Copyright · JackiOh");
    expect(documentTitleFor(paths.match("m-1"))).toBe("Match · JackiOh");
    expect(documentTitleFor("/nope")).toBe("Page not found · JackiOh");
  });

  it("points the canonical link at the site's own address for the path, and at nothing for a 404", () => {
    expect(canonicalUrlFor(paths.landing)).toBe(`${SITE_ORIGIN}/`);
    expect(canonicalUrlFor(paths.practice)).toBe(`${SITE_ORIGIN}/practice`);
    expect(canonicalUrlFor(paths.terms)).toBe(`${SITE_ORIGIN}/terms`);
    expect(canonicalUrlFor("/nope")).toBeNull();
  });

  it("follows the path as the player moves", async () => {
    at(paths.privacy);
    render(<App />);
    await waitFor(() => {
      expect(document.title).toBe("Privacy · JackiOh");
    });
    expect(canonical()).toBe(`${SITE_ORIGIN}/privacy`);
    expect(document.head.querySelectorAll('link[rel="canonical"]')).toHaveLength(1);

    cleanup();
    at("/nope");
    render(<App />);
    await waitFor(() => {
      expect(document.title).toBe("Page not found · JackiOh");
    });
    expect(canonical()).toBeNull();
  });
});
