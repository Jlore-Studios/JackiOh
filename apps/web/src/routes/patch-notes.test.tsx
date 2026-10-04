// `/patch-notes` (R388): a public route, lazy like the others, with its tab title and canonical link,
// linked from the site footer, served by vercel.json (net/deploy-routes.test.ts reads `paths`) and
// listed in the sitemap.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SITE_ORIGIN, paths } from "../net/navigate.ts";
import { PatchSourceProvider } from "../patches/context.tsx";
import { fixtureSource } from "../patches/fixtures.ts";
import { realPatchSource } from "../patches/source.ts";
import { patchTestid } from "../patches/testids.ts";
import LandingRoute from "./landing.tsx";
import LoginRoute from "./login.tsx";
import PatchNotesRoute from "./patch-notes.tsx";
import { siteFooterTestid } from "./SiteFooter.tsx";

const { App, canonicalUrlFor, documentTitleFor } = await import("../main.tsx");

const HERE = dirname(fileURLToPath(import.meta.url));

/** A lazily-imported route chunk and the real history's first read can outrun the 1 s default. */
const SLOW = { timeout: 10_000 } as const;

function at(path: string): void {
  window.history.replaceState(null, "", path);
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
<<<<<<< HEAD
    expect(entries[0]?.dataset.version).toBe("v0.2.2");
=======
    // The newest patch — patches.json's last entry — opens the page.
    const shipped = await realPatchSource.patches();
    expect(entries[0]?.dataset.version).toBe(shipped.at(-1)?.version);
>>>>>>> origin/main
    expect(document.title).toBe("Patch notes · JackiOh");
    // Nothing asked the server who is signed in: the page is not behind the gate.
    expect(vi.mocked(fetch).mock.calls.some(([url]) => String(url).includes("/api/auth/me"))).toBe(false);
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
    const sitemap = readFileSync(join(HERE, "../../public/sitemap.xml"), "utf8");
    expect(sitemap).toContain(`<loc>${SITE_ORIGIN}/patch-notes</loc>`);
  });
});
