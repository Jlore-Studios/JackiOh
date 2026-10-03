// `/privacy`, the footer that links it, the line under "Create account", and the tab title and
// canonical link `App` keeps for every path.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SITE_ORIGIN, paths } from "../net/navigate.ts";
import AccessibilityRoute, { ACCESSIBILITY_LAST_UPDATED, accessibilityTestid } from "./accessibility.tsx";
import LandingRoute from "./landing.tsx";
import LoginRoute, { signUpPrivacyTestid } from "./login.tsx";
import PrivacyRoute, { PRIVACY_LAST_UPDATED, privacyTestid } from "./privacy.tsx";
import { CONTACT_URL, siteFooterTestid } from "./SiteFooter.tsx";
import TermsRoute, { TERMS_LAST_UPDATED, termsTestid } from "./terms.tsx";

const { App, canonicalUrlFor, documentTitleFor } = await import("../main.tsx");

const HERE = dirname(fileURLToPath(import.meta.url));

/** A lazily-imported route chunk can outrun the 1 s default when the whole suite runs at once. */
const SLOW = { timeout: 5_000 } as const;

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
    expect(PRIVACY_LAST_UPDATED).toBe("2026-10-01");
  });

  it("names what is collected, who handles it, and how to delete an account", () => {
    render(<PrivacyRoute />);
    const page = screen.getByTestId(privacyTestid.screen);
    for (const fact of [/email address and password/i, /IP address/, /Supabase/, /Render/, /Vercel/, /no cookies/i, /Do Not Track/, /Delete my account/, /card statistics/]) {
      expect(page.textContent).toMatch(fact);
    }
    expect(within(page).getByRole("link", { name: /contact us on GitHub/i })).toHaveAttribute("href", CONTACT_URL);
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

  it("serves /terms and /accessibility with no account needed", async () => {
    at(paths.terms);
    render(<App />);
    expect(await screen.findByTestId(termsTestid.screen, undefined, SLOW)).toBeInTheDocument();
    cleanup();
    at(paths.accessibility);
    render(<App />);
    expect(await screen.findByTestId(accessibilityTestid.screen, undefined, SLOW)).toBeInTheDocument();
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
  it("links the privacy policy, and only on the sign-up form", async () => {
    render(<LoginRoute />);
    expect(screen.queryByTestId(signUpPrivacyTestid)).toBeNull();
    await userEvent.click(screen.getByTestId("login-mode"));
    const line = screen.getByTestId(signUpPrivacyTestid);
    expect(line).toHaveTextContent("By creating an account you agree to the Terms and Privacy Policy.");
    expect(within(line).getByRole("link", { name: "Terms" })).toHaveAttribute("href", paths.terms);
    expect(within(line).getByRole("link", { name: "Privacy Policy" })).toHaveAttribute("href", paths.privacy);
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
