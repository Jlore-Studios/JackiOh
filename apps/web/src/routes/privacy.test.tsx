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

import {
  CHALLENGE_TTL_MS,
  DUOS_ROOM_TTL_MS,
  MATCH_ACTION_RETENTION_DAYS,
  MIN_ACCOUNT_AGE_YEARS,
  PLAY_TELEMETRY_RETENTION_DAYS,
  TERMS_VERSION,
} from "@jackioh/server-config";

import { PUBLIC_PAGES, sitemapXml } from "../../static-pages.ts";
import { consentTestid } from "../auth/testids.ts";
import { SITE_ORIGIN, paths } from "../net/navigate.ts";
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

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

function canonical(): string | null {
  return document.head.querySelector<HTMLLinkElement>('link[rel="canonical"]')?.href ?? null;
}

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
    expect(PRIVACY_LAST_UPDATED).toBe("2026-10-17");
  });

  it("R757000 the policy names the minimum age and what is kept of the check, and holds no owner slot", () => {
    render(<PrivacyRoute />);
    const page = screen.getByTestId(privacyTestid.screen);
    expect(page.querySelector("[data-owner-slot]"), "no slot left for the owner").toBeNull();
    const section = (name: RegExp): string =>
      within(page).getByRole("heading", { name }).closest("section")?.textContent ?? "";
    const children = section(/children/i);
    expect(children).toContain(`at least ${String(MIN_ACCOUNT_AGE_YEARS)} years old`);
    expect(children).toMatch(/we keep only that you passed/);
    expect(children).toMatch(/delete it the way .Delete my account. does/);
    const collected = section(/what we collect/i);
    expect(collected).toMatch(/passed the age check, when, and the minimum age/);
    expect(collected).toMatch(/don.t keep your date of birth/);
    expect(collected).toMatch(/version of the Terms and of this policy you agreed to/);
    expect(section(/what stays on your device/i)).toMatch(/month and year of birth you gave/);
    expect(section(/what stays on your device/i)).toMatch(/no cookies/i);
    expect(section(/deleting your account/i)).toMatch(/record of your age check and of the terms you agreed to/);
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

  it("names what is collected, who handles it, and how to delete an account", () => {
    render(<PrivacyRoute />);
    const page = screen.getByTestId(privacyTestid.screen);
    for (const fact of [/email address and password/i, /IP address/, /Supabase/, /Render/, /Cloudflare/, /Vercel/, /no cookies/i, /Do Not Track/, /Delete my account/, /card statistics/]) {
      expect(page.textContent).toMatch(fact);
    }
    expect(within(page).getByRole("link", { name: CONTACT_EMAIL })).toHaveAttribute("href", CONTACT_URL);
  });

    it("names Cloudflare as the website's host and Vercel as its test copy's (#770)", () => {
      render(<PrivacyRoute />);
      const page = screen.getByTestId(privacyTestid.screen);
      const section = within(page).getByRole("heading", { name: /who handles it/i }).closest("section");
      const items = [...(section?.querySelectorAll("li") ?? [])].map((item) => item.textContent ?? "");
      expect(items).toContain("Cloudflare hosts this website at jackioh.com.");
      expect(items).toContain("Vercel hosts a test copy of it at jackioh.vercel.app.");
      expect(items).toHaveLength(4);
      expect(section?.textContent).toMatch(/We use four services to run JackiOh:/);
      expect(page.textContent).not.toMatch(/Vercel hosts this website/);
    });

  it("R1442 names the play telemetry, how long it is kept, and that deleting an account deletes it", () => {
    render(<PrivacyRoute />);
    const page = screen.getByTestId(privacyTestid.screen);
    const section = (name: RegExp): string =>
      within(page).getByRole("heading", { name }).closest("section")?.textContent ?? "";
    const collected = section(/what we collect/i);
    for (const fact of [
      /how long each of your moves took/,
      /clock was left/,
      /emotes/,
      /clock ran out/,
      /conceded/,
      /draw/,
      /rematch/,
    ]) {
      expect(collected).toMatch(fact);
    }
    expect(collected).toMatch(/kept by match and seat and names\s+no account itself/);
    expect(section(/why we collect it/i)).toMatch(/computer opponents play at a human pace/);
    expect(section(/what we don.t do/i)).toMatch(/no third-party analytics/);
    expect(section(/how long we keep it/i)).toContain(
      `how you played a finished match: ${PLAY_TELEMETRY_RETENTION_DAYS} days after the match ends`,
    );
    expect(section(/deleting your account/i)).toMatch(/the records of how you played/);
  });

  it("#936 says what Play together and co-op show, keep and delete, as the server does", () => {
    render(<PrivacyRoute />);
    const page = screen.getByTestId(privacyTestid.screen);
    const section = (name: RegExp): string =>
      within(page).getByRole("heading", { name }).closest("section")?.textContent ?? "";
    const minutes = (ms: number): number => ms / 60_000;

    // Co-op is collected: its rooms and its games, every move included.
    const collected = section(/what we collect/i);
    expect(collected).toMatch(/co-op \(Duos\)/);
    expect(collected).toMatch(/the co-op rooms you make or join/);
    expect(collected).toMatch(/every player.s deck, every move, and the result/);

    // Who sees it: a teammate sees the hand; an opponent's friends see your moves; an opponent may share.
    const handled = section(/who handles it/i);
    expect(handled).toMatch(/your teammate sees your hand, your face-down cards and the choices in front of you/);
    expect(handled).toMatch(/from your opponent.s seat, which shows your moves as your opponent sees them/);
    expect(handled).toMatch(/a replay your opponent shares shows the game from their seat, your moves included/);

    // How long: co-op moves as 1v1 moves, a challenge for its life, a share until its sharer or the
    // friendship ends it; no "until you or the other player removes them" for what only one side may.
    const kept = section(/how long we keep it/i);
    expect(kept).toContain(`co-op games included: ${String(MATCH_ACTION_RETENTION_DAYS)} days after the match ends`);
    expect(kept).toContain(`A challenge: about ${String(minutes(CHALLENGE_TTL_MS))} minutes after it was sent`);
    expect(kept).toMatch(/accepted, declined, cancelled or left unanswered/);
    expect(kept).toContain(`A co-op room: about ${String(minutes(DUOS_ROOM_TTL_MS))} minutes after it was made`);
    expect(kept).toMatch(/the player who shared it takes it back \(only they can\)/);
    expect(kept).toMatch(/removes the friendship or blocks the other, which deletes every share between you for good/);
    expect(kept).toMatch(/A block: until the player who made it lifts it/);
    expect(kept).not.toMatch(/until you or the other player removes them/);

    // Deleting: a co-op game goes for both teammates, and a live one holds the deletion back.
    const deleting = section(/deleting your account/i);
    expect(deleting).toMatch(/deleting either teammate.s account deletes the whole co-op game, its moves and its result, for both of you/);
    expect(deleting).toMatch(/during a match, a co-op game or a Conquest series that hasn.t finished/);
    expect(deleting).toMatch(/challenges, co-op rooms and replay shares/);
  });

  it("#970 names custom cards and the presence heartbeat, and deletes the custom cards with the account", () => {
    render(<PrivacyRoute />);
    const page = screen.getByTestId(privacyTestid.screen);
    const section = (name: RegExp): string =>
      within(page).getByRole("heading", { name }).closest("section")?.textContent ?? "";
    const items = (name: RegExp): string[] =>
      [...(within(page).getByRole("heading", { name }).closest("section")?.querySelectorAll("li") ?? [])].map(
        (item) => item.textContent ?? "",
      );

    const custom = items(/what we collect/i).find((item) => item.startsWith("Your custom cards")) ?? "";
    expect(custom).toMatch(/name, type, cost, rarity, stats, keywords, text and numbers/);
    expect(custom).toMatch(/its script/);
    expect(custom).toMatch(/Only you can see them\./);
    // R970002: every signed-in session beats, once its account has agreed; not only Play together's.
    const online = items(/what we collect/i).filter((item) => /told us you were online/.test(item));
    expect(online).toHaveLength(1);
    expect(online[0]).toMatch(/^While you are signed in, .* once you have agreed\s+to the Terms and this policy\./u);
    expect(items(/what we collect/i).find((item) => item.startsWith("If you use Play together"))).not.toMatch(/online/);
    expect(section(/why we collect it/i)).toMatch(/custom cards/);
    expect(section(/how long we keep it/i)).toMatch(/your custom cards, your age check/);
    expect(section(/deleting your account/i)).toMatch(/trios and cards, your custom cards, your\s+tutorial progress/);
  });

  it("R632 says the sign-in session lasts only as long as the tab, not as a stored sign-in", () => {
    render(<PrivacyRoute />);
    const page = screen.getByTestId(privacyTestid.screen);
    const device = within(page).getByRole("heading", { name: /what stays on your device/i }).closest("section");
    const lists = [...(device?.querySelectorAll("li") ?? [])].map((item) => item.textContent ?? "");
    const tabOnly = lists.find((item) => /open tab only/i.test(item)) ?? "";
    expect(tabOnly).toMatch(/sign-in session/);
    expect(lists.filter((item) => /sign-in session/.test(item))).toEqual([tabOnly]);
    expect(device?.textContent).toMatch(/sign in again in a new tab/i);
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
    expect(screen.getByTestId(accessibilityTestid.updated)).toHaveTextContent(
      `Last updated ${ACCESSIBILITY_LAST_UPDATED}`,
    );
  });

  it("R640001 the terms say ranked may pair a player with an AI opponent", () => {
    render(<TermsRoute />);
    const ranked = screen.getByRole("region", { name: "Ranked play" });
    expect(ranked).toHaveTextContent("When no suitable opponent is available, ranked may pair you with an AI opponent.");
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

  it("the terms name the minimum age, what a player submits, and link the copyright page", () => {
    render(<TermsRoute />);
    expect(screen.getByRole("region", { name: "Who can have an account" })).toHaveTextContent(
      `You must be at least ${String(MIN_ACCOUNT_AGE_YEARS)} years old to have an account.`,
    );
    expect(screen.getByRole("region", { name: "Agreeing to these terms" })).toHaveTextContent(/ticking the box/);
    const submitted = screen.getByRole("region", { name: "What you submit" });
    expect(submitted).toHaveTextContent(/We remove content that infringes/);
    expect(within(submitted).getByRole("link", { name: "Copyright" })).toHaveAttribute("href", paths.copyright);
  });

  it("#970 the terms license what a player submits, each with who sees it, and set the rules of chat", () => {
    render(<TermsRoute />);
    const submitted = screen.getByRole("region", { name: "What you submit" });
    expect(submitted).toHaveTextContent(/you let us store it, and show it to the people named below/);
    const items = within(submitted)
      .getAllByRole("listitem")
      .map((item) => item.textContent ?? "");
    expect(items).toEqual([
      "Your username, which other players see.",
      expect.stringMatching(/^Your chat messages, which the friend you send them to sees, and the reason you give when you\s+report one, which only we see\.$/u),
      "Your custom cards (their names, text and scripts) and the names of your decks, which only you see.",
      "The replays you share, which the friends you share them with see.",
    ]);
    expect(within(submitted).getByRole("link", { name: "Privacy Policy" })).toHaveAttribute("href", paths.privacy);
    const chat = screen.getByRole("region", { name: "Chat" });
    for (const rule of [/harass, threaten or bully/, /hate speech/, /spam/, /personal details/, /your own included/]) {
      expect(chat).toHaveTextContent(rule);
    }
    expect(chat).toHaveTextContent(/replace each word on our filter list with asterisks/);
    expect(chat).toHaveTextContent(/abuse in chat can get an account suspended or banned/);
  });

  it("#970 the copyright page names a custom card's name and text as something a notice can be about", () => {
    render(<CopyrightRoute />);
    expect(screen.getByRole("region", { name: /reporting content that infringes your copyright/i })).toHaveTextContent(
      /such as a username or a custom card.s name or text/,
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
      [PrivacyRoute, "Contact"],
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
