// R375: `/patch-notes`, and the site footer that links it with the current version. The patches up
// to v0.1.1 are fixed, so each test pins those and reads any later ones off patches.json.

import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { closeInspect } from "../cards/index.ts";
import { INSPECT_DETAIL, INSPECT_HISTORY_TOGGLE } from "../cards/inspect/testids.ts";
import { CURRENT_VERSION, PATCHES, RECONSTRUCTED_NOTE, versionCount } from "../cards/patches.ts";
import { versionsLabel } from "../cards/inspect/History.tsx";
import { SITE_ORIGIN, paths } from "../net/navigate.ts";
import LandingRoute from "./landing.tsx";
import PatchNotesRoute, { patchNotesTestid } from "./patch-notes.tsx";
import { siteFooterTestid } from "./SiteFooter.tsx";

const { App, canonicalUrlFor, documentTitleFor } = await import("../main.tsx");

/** A lazily-imported route chunk can outrun the 1 s default when the whole suite runs at once. */
const SLOW = { timeout: 5_000 } as const;

function at(path: string): void {
  window.history.replaceState(null, "", path);
}

function patchSection(version: string): HTMLElement {
  const section = screen.getAllByTestId(patchNotesTestid.patch).find((element) => element.dataset["version"] === version);
  if (section === undefined) throw new Error(`no ${version} section`);
  return section;
}

beforeEach(() => {
  window.localStorage.clear();
  at("/");
  // Nothing on these screens needs the server; an account read that never answers is enough.
  vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>(() => {})));
});

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
  vi.unstubAllGlobals();
  document.head.querySelector('link[rel="canonical"]')?.remove();
});

describe("R375 the patch notes page", () => {
  it("R375 lists every patch newest first, and names the current version", () => {
    render(<PatchNotesRoute />);
    const versions = screen.getAllByTestId(patchNotesTestid.patch).map((section) => section.dataset["version"]);
    expect(versions).toEqual([...PATCHES].reverse().map((patch) => patch.version));
    expect(versions.slice(-5)).toEqual(["v0.1.1", "v0.1.0-r3", "v0.1.0-r2", "v0.1.0-r1", "v0.1.0"]);
    expect(screen.getByTestId(patchNotesTestid.current)).toHaveTextContent(`The current version is ${CURRENT_VERSION}.`);
    const newest = patchSection("v0.1.1");
    expect(newest).toHaveTextContent("v0.1.1 · Patch v0.1.1");
    expect(newest).toHaveTextContent("2026-09-27");
    expect(newest).toHaveTextContent("The Ghoul Token is added");
  });

  it("R375 badges every version named only later as reconstructed, and says what that means", () => {
    render(<PatchNotesRoute />);
    const badged = screen.getAllByTestId(patchNotesTestid.patch).filter(
      (section) => within(section).queryByTestId(patchNotesTestid.badge) !== null,
    );
    expect(badged.map((section) => section.dataset["version"])).toEqual(["v0.1.0-r3", "v0.1.0-r2", "v0.1.0-r1", "v0.1.0"]);
    expect(within(patchSection("v0.1.1")).queryByTestId(patchNotesTestid.badge)).toBeNull();
    expect(within(patchSection("v0.1.0")).getByTestId(patchNotesTestid.badge)).toHaveTextContent("Reconstructed");
    expect(within(patchSection("v0.1.0")).getByTestId(patchNotesTestid.badge)).toHaveAttribute("title", RECONSTRUCTED_NOTE);
    expect(screen.getByTestId(patchNotesTestid.screen)).toHaveTextContent("reconstructed from the repository's history");
  });

  it("R375 links each patch's issues and pull requests, and its commits, on GitHub", () => {
    render(<PatchNotesRoute />);
    const hrefs = (version: string, testid: string): (string | null)[] =>
      within(patchSection(version))
        .queryAllByTestId(testid)
        .map((link) => link.getAttribute("href"));
    expect(hrefs("v0.1.1", patchNotesTestid.source)).toEqual([
      "https://github.com/jgoetzmann/JackiOh/issues/27",
      "https://github.com/jgoetzmann/JackiOh/pull/28",
    ]);
    expect(within(patchSection("v0.1.1")).getAllByTestId(patchNotesTestid.source).map((link) => link.textContent)).toEqual([
      "Issue #27",
      "PR #28",
    ]);
    expect(hrefs("v0.1.0-r1", patchNotesTestid.source)).toEqual([
      "https://github.com/jgoetzmann/JackiOh/issues/1",
      "https://github.com/jgoetzmann/JackiOh/pull/2",
    ]);
    expect(hrefs("v0.1.0-r2", patchNotesTestid.source)).toEqual([
      "https://github.com/jgoetzmann/JackiOh/pull/11",
      "https://github.com/jgoetzmann/JackiOh/pull/14",
    ]);
    expect(hrefs("v0.1.0", patchNotesTestid.source)).toEqual([]);
    expect(hrefs("v0.1.0", patchNotesTestid.commit)).toEqual([
      "https://github.com/jgoetzmann/JackiOh/commit/46266903212386e2d73f7b1eee8c4b8b5d73fa9b",
    ]);
    expect(within(patchSection("v0.1.0")).getByTestId(patchNotesTestid.commit)).toHaveTextContent(/^4626690$/);
  });

  it("R375 lists the cards each patch created and changed, and opens a card's detail with its History", async () => {
    render(<PatchNotesRoute />);
    const cardsOf = (version: string): (string | undefined)[] =>
      within(patchSection(version))
        .getAllByTestId(patchNotesTestid.card)
        .map((button) => button.dataset["card"]);
    expect(cardsOf("v0.1.0-r2")).toEqual(["core-t-coin", "core-095"]);
    expect(within(patchSection("v0.1.0-r2")).getByText("New cards (1)")).toBeInTheDocument();
    expect(within(patchSection("v0.1.0-r2")).getByText("Changed cards (1)")).toBeInTheDocument();
    expect(cardsOf("v0.1.1")).toHaveLength(106);
    expect(cardsOf("v0.1.1")[0]).toBe("core-t-ghoul");
    expect(cardsOf("v0.1.0")).toHaveLength(109);

    const coin = within(patchSection("v0.1.0-r2")).getAllByTestId(patchNotesTestid.card)[0];
    if (coin === undefined) throw new Error("The Coin's button");
    expect(coin).toHaveTextContent("The Coin");
    await userEvent.click(coin);
    const detail = screen.getByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("data-card", "core-t-coin");
    expect(within(detail).getByTestId(INSPECT_HISTORY_TOGGLE)).toHaveTextContent(
      `History (${versionsLabel(versionCount("core-t-coin"))})`,
    );
  });

  it("R375 is served at /patch-notes, with no account needed, and named in the tab", async () => {
    at(paths.patchNotes);
    render(<App />);
    expect(await screen.findByTestId(patchNotesTestid.screen, undefined, SLOW)).toBeInTheDocument();
    await waitFor(() => {
      expect(document.title).toBe("Patch notes · JackiOh");
    });
    expect(documentTitleFor(paths.patchNotes)).toBe("Patch notes · JackiOh");
    expect(canonicalUrlFor(paths.patchNotes)).toBe(`${SITE_ORIGIN}/patch-notes`);
  });
});

describe("R375 the site footer", () => {
  it("R375 links the patch notes and shows the current version, linking to them too", () => {
    render(<LandingRoute />);
    const footer = screen.getByTestId(siteFooterTestid.root);
    expect(within(footer).getByTestId(siteFooterTestid.patchNotes)).toHaveAttribute("href", paths.patchNotes);
    const version = within(footer).getByTestId(siteFooterTestid.version);
    expect(version).toHaveTextContent(`JackiOh ${CURRENT_VERSION}`);
    expect(version).toHaveAttribute("href", paths.patchNotes);
  });

  it("R375 opens the patch notes in place", async () => {
    at(paths.landing);
    render(<App />);
    await userEvent.click(screen.getByTestId(siteFooterTestid.version));
    expect(window.location.pathname).toBe(paths.patchNotes);
    expect(await screen.findByTestId(patchNotesTestid.screen, undefined, SLOW)).toBeInTheDocument();
  });
});
