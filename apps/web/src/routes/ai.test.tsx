// `/ai`: how JackiOh uses AI (R2631000): what the page says, and that it is public, linked from the
// footer, titled and in the sitemap like the other statements.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PUBLIC_PAGES, sitemapXml } from "../../static-pages.ts";
import { SITE_ORIGIN, paths } from "../net/navigate.ts";
import { at } from "../test/stubServer.ts";
import AiRoute, { MUSIC_SOURCES_URL, aiTestid } from "./ai.tsx";
import LandingRoute from "./landing.tsx";
import { siteFooterTestid } from "./SiteFooter.tsx";

const { App, documentTitleFor } = await import("../main.tsx");

const HERE = dirname(fileURLToPath(import.meta.url));

/** A lazily-imported route chunk can outrun the 1 s default when the whole suite runs at once. */
const SLOW = { timeout: 5_000 } as const;

beforeEach(() => {
  window.localStorage.clear();
  at("/");
  // Nothing on these screens needs the server; an account read that never answers is enough.
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("R2631000 how JackiOh uses AI", () => {
  it("R2631000 names the computer opponents, what was made with AI, and what AI does with your data", () => {
    render(<AiRoute />);

    const opponents = screen.getByRole("region", { name: "The computer opponents" });
    expect(opponents).toHaveTextContent("Each plays by the same rules you do and sees only what a player in its seat could see.");
    expect(opponents).toHaveTextContent("In Practice, at the difficulty you pick, and in the tutorial");
    expect(opponents).toHaveTextContent("In ranked, when no one near your rating is free after a short wait, an AI opponent that our server plays.");
    expect(opponents).toHaveTextContent("Nothing in the game marks it");
    expect(within(opponents).getByRole("link", { name: "Terms" })).toHaveAttribute("href", paths.terms);
    expect(opponents).toHaveTextContent("In co-op, the team you and a friend play against.");
    expect(opponents).toHaveTextContent("They are not chatbots");

    const made = screen.getByRole("region", { name: "Made with AI" });
    expect(made).toHaveTextContent("was composed by an AI model, Anthropic’s Claude, written as code");
    expect(within(made).getByRole("link", { name: "Where each track came from" })).toHaveAttribute("href", MUSIC_SOURCES_URL);
    expect(made).toHaveTextContent("read by computer text-to-speech voices, not recorded by actors");
    expect(made).toHaveTextContent("not by an image-generation model");
    expect(made).toHaveTextContent("written with the help of AI coding assistants");

    const data = screen.getByRole("region", { name: "What AI does with your data" });
    expect(data).toHaveTextContent("We don’t send anything you write, or your account’s details, to an AI service.");
    expect(within(data).getByRole("link", { name: "Privacy Policy" })).toHaveAttribute("href", paths.privacy);
  });

  it("R2631000 is public, linked from the site footer, titled and in the sitemap", async () => {
    at(paths.ai);
    render(<App />);
    expect(await screen.findByTestId(aiTestid.screen, undefined, SLOW)).toBeInTheDocument();
    cleanup();

    render(<LandingRoute />);
    const footer = screen.getByTestId(siteFooterTestid.root);
    expect(within(footer).getByTestId(siteFooterTestid.ai)).toHaveAttribute("href", paths.ai);

    expect(documentTitleFor(paths.ai)).toBe("How JackiOh uses AI · JackiOh");
    expect(sitemapXml(PUBLIC_PAGES)).toContain(`<loc>${SITE_ORIGIN}/ai</loc>`);
  });

  it("R2631000 opens as a draft for the owner, as the other statements do", () => {
    const source = readFileSync(join(HERE, "ai.tsx"), "utf8");
    expect(source.split("\n")[0]).toBe("// DRAFT — needs owner and legal review");
  });
});
