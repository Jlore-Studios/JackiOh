// `/patch-notes` (R388, R724000): a public route, served by public/_redirects (net/deploy-routes.test.ts reads `paths`).

import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PUBLIC_PAGES, sitemapXml } from "../../static-pages.ts";
import { SITE_ORIGIN, paths } from "../net/navigate.ts";
import { PatchSourceProvider } from "../patches/context.tsx";
import { fixtureSource } from "../patches/fixtures.ts";
import { RELEASES, nestMicros, releaseGroups, tiers } from "../patches/releases.ts";
import { realPatchSource, type Patch } from "../patches/source.ts";
import { patchTestid } from "../patches/testids.ts";
import { at } from "../test/stubServer.ts";
import LandingRoute from "./landing.tsx";
import LoginRoute from "./login.tsx";
import PatchNotesRoute from "./patch-notes.tsx";
import { siteFooterTestid } from "./SiteFooter.tsx";

const { App, canonicalUrlFor, documentTitleFor } = await import("../main.tsx");

/** A lazily-imported route chunk and the real history's first read can outrun the 1 s default. */
const SLOW = { timeout: 10_000 } as const;

/** R3012000: the version of the newest entry of the newest line: the last major or patch, which holds the micro patches after it. */
function newestEntry(patches: readonly Patch[]): string | undefined {
  const tierList = tiers(patches);
  const tierOf = new Map(patches.map((patch, at) => [patch.version, tierList[at] ?? "patch"] as const));
  return nestMicros(releaseGroups(patches, RELEASES)[0]?.patches ?? [], tierOf)[0]?.patch.version;
}

beforeEach(() => {
  window.localStorage.clear();
  at("/");
  // The page needs no server: an account read that never answers changes nothing.
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  document.head.querySelector('link[rel="canonical"]')?.remove();
});

describe("R388 the /patch-notes route", () => {
  it("R388 is served at /patch-notes with no account, and reads the real history", async () => {
    at(paths.patchNotes);
    render(<App />);
    expect(await screen.findByTestId(patchTestid.screen, undefined, SLOW)).toBeInTheDocument();
    const entries = await screen.findAllByTestId(patchTestid.patch, undefined, SLOW);
    // The newest entry opens the page: patches.json's last major or patch, whatever a promotion named it (R646),
    // holding the micro patches shipped after it (R3012000).
    const shipped = await realPatchSource.patches();
    expect(entries[0]?.dataset.version).toBe(newestEntry(shipped));
    expect(document.title).toBe("Patch notes · JackiOh");
    // Nothing asked the server who is signed in: the page is not behind the gate.
    expect(vi.mocked(fetch).mock.calls.some(([url]) => String(url).includes("/api/auth/me"))).toBe(false);
  });

  it("R724000 opens on the newest release line, the others shut", async () => {
    at(paths.patchNotes);
    render(<App />);
    const sets = await screen.findAllByTestId(patchTestid.set, undefined, SLOW);
    const shipped = await realPatchSource.patches();
    const toggles = sets.map((set) => within(set).getByTestId(patchTestid.setToggle).getAttribute("aria-expanded"));
    expect(toggles).toEqual(sets.map((_set, at) => (at === 0 ? "true" : "false")));
    expect(sets.length).toBeGreaterThan(1);
    expect(within(sets[0] ?? document.body).getAllByTestId(patchTestid.patch)[0]?.dataset.version).toBe(newestEntry(shipped));
    for (const set of sets.slice(1)) expect(within(set).queryAllByTestId(patchTestid.patch)).toEqual([]);
  });

  it("R388 has a tab title, a canonical link and a heading", () => {
    expect(paths.patchNotes).toBe("/patch-notes");
    expect(documentTitleFor(paths.patchNotes)).toBe("Patch notes · JackiOh");
    expect(canonicalUrlFor(paths.patchNotes)).toBe(`${SITE_ORIGIN}/patch-notes`);
    render(
      <PatchSourceProvider source={fixtureSource()}>
        <PatchNotesRoute />
      </PatchSourceProvider>,
    );
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Patch notes");
    expect(screen.getByTestId("nav-back")).toBeInTheDocument();
  });

  it("R388 the site footer links it on the landing page and the sign-in screen, and opens it in place", async () => {
    render(<LandingRoute />);
    const link = within(screen.getByTestId(siteFooterTestid.root)).getByTestId(siteFooterTestid.patchNotes);
    expect(link).toHaveAttribute("href", paths.patchNotes);
    expect(link).toHaveTextContent("Patch notes");
    cleanup();

    render(<LoginRoute />);
    expect(within(screen.getByTestId(siteFooterTestid.root)).getByTestId(siteFooterTestid.patchNotes)).toBeInTheDocument();
    cleanup();

    at(paths.landing);
    render(<App />);
    await userEvent.click(screen.getByTestId(siteFooterTestid.patchNotes));
    expect(window.location.pathname).toBe(paths.patchNotes);
    expect(await screen.findByTestId(patchTestid.screen, undefined, SLOW)).toBeInTheDocument();
  });

  it("R388 is in the sitemap", () => {
    const sitemap = sitemapXml(PUBLIC_PAGES);
    expect(sitemap).toContain(`<loc>${SITE_ORIGIN}/patch-notes</loc>`);
  });
});
