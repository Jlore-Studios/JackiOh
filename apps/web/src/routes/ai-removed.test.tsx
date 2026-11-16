// `/ai` is removed (R4037000, amending R2631000): no page, footer link, title, sitemap or
// static-page entry. The path draws `NotFound` like any unknown path.

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { PAGE_TEXT, PUBLIC_PAGES, sitemapXml } from "../../static-pages.ts";
import { shellTestid } from "../auth/testids.ts";
import { at } from "../test/stubServer.ts";
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

describe("R4037000 /ai is removed", () => {
  it("R4037000 removes /ai, amending R2631000: no footer link, sitemap or static page, and the path draws NotFound like any unknown path", () => {
    expect(PUBLIC_PAGES.map((page) => page.path)).not.toContain("/ai");
    expect(PAGE_TEXT["/ai"]).toBeUndefined();
    expect(sitemapXml(PUBLIC_PAGES)).not.toContain("/ai");

    render(<SiteFooter />);
    const footer = screen.getByTestId(siteFooterTestid.root);
    expect(footer.querySelector('[data-testid="site-footer-ai"]')).toBeNull();
    expect(footer.querySelector('[href="/ai"]')).toBeNull();
    expect(within(footer).queryByText("How JackiOh uses AI")).toBeNull();

    // The unknown-path title, identical to any other path nothing serves.
    expect(documentTitleFor("/ai")).toBe(documentTitleFor("/nope"));
    expect(documentTitleFor("/ai")).not.toContain("How JackiOh uses AI");

    at("/ai");
    render(<App />);
    expect(screen.getByTestId(shellTestid.notFound)).toBeInTheDocument();
  });
});
