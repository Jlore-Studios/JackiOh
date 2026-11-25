// `/accessibility` is removed (R4507000): no page, footer link, title, sitemap or
// static-page entry. The path draws `NotFound` like any unknown path.

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PAGE_TEXT, PUBLIC_PAGES, sitemapXml } from "../../static-pages.ts";
import { shellTestid } from "../auth/testids.ts";
import { at } from "../test/stubServer.ts";
import { commandPages } from "./commandEntries.ts";
import { SiteFooter, siteFooterTestid } from "./SiteFooter.tsx";

const { App, documentTitleFor } = await import("../main.tsx");

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

describe("R4507000 /accessibility is removed", () => {
  it("R4507000 removes /accessibility: no footer link, sitemap, static page or command entry, and the path draws NotFound like any unknown path", () => {
    expect(PUBLIC_PAGES.map((page) => page.path)).not.toContain("/accessibility");
    expect(PAGE_TEXT["/accessibility"]).toBeUndefined();
    expect(sitemapXml(PUBLIC_PAGES)).not.toContain("/accessibility");

    render(<SiteFooter />);
    const footer = screen.getByTestId(siteFooterTestid.root);
    expect(footer.querySelector('[data-testid="site-footer-accessibility"]')).toBeNull();
    expect(footer.querySelector('[href="/accessibility"]')).toBeNull();
    expect(within(footer).queryByText("Accessibility")).toBeNull();

    expect(commandPages().map((entry) => entry.path)).not.toContain("/accessibility");
    expect(commandPages().map((entry) => entry.id)).not.toContain("page-accessibility");

    // The unknown-path title, identical to any other path nothing serves.
    expect(documentTitleFor("/accessibility")).toBe(documentTitleFor("/nope"));
    expect(documentTitleFor("/accessibility")).not.toContain("Accessibility");

    at("/accessibility");
    render(<App />);
    expect(screen.getByTestId(shellTestid.notFound)).toBeInTheDocument();
  });
});
