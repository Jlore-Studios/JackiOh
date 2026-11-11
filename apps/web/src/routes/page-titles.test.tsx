// R3223000 (#3223): PageTitle is the one `h1` of a menu, and the brand beside it is the wordmark, not a heading. The
// legal pages need no account, so their own routes are rendered here as they are; every other route's one title is
// asserted in that route's test file.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { at } from "../test/stubServer.ts";
import { expectOnePageTitle } from "../test/pageTitle.ts";
import AccessibilityRoute from "./accessibility.tsx";
import AiRoute from "./ai.tsx";
import CopyrightRoute from "./copyright.tsx";
import { PageTitle, PAGE_TITLE_ID, pageTitleTestid } from "./PageTitle.tsx";
import PrivacyRoute from "./privacy.tsx";
import TermsRoute from "./terms.tsx";
import { Wordmark } from "./Wordmark.tsx";

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

describe("R3223000 PageTitle", () => {
  it("R3223000 PageTitle is an h1 with the page's one title id, class and test id", () => {
    render(<PageTitle>Screen</PageTitle>);
    const title = screen.getByRole("heading", { level: 1, name: "Screen" });
    expect(title).toHaveAttribute("id", PAGE_TITLE_ID);
    expect(title).toHaveClass("page-title");
    expect(title).toHaveAttribute("data-testid", pageTitleTestid);
    expectOnePageTitle("Screen");
  });

  it("R3223000 the wordmark is the brand, drawn but not a heading", () => {
    render(<Wordmark />);
    expect(screen.getByText("JackiOh")).toHaveClass("brand__word");
    expect(screen.queryByRole("heading")).toBeNull();
  });

  it.each([
    ["Terms of service", TermsRoute],
    ["Privacy policy", PrivacyRoute],
    ["Copyright", CopyrightRoute],
    ["Accessibility", AccessibilityRoute],
    ["How JackiOh uses AI", AiRoute],
  ])("R3223000 %s has one visible h1, the PageTitle, with the brand as the wordmark and its sections as h2", (name, Route) => {
    render(<Route />);
    expectOnePageTitle(name);
    expect(screen.queryByRole("heading", { name: "JackiOh" })).toBeNull();
    expect(document.querySelector(".brand .brand__word")).toHaveTextContent("JackiOh");
    const sections = screen.getAllByRole("heading", { level: 2 });
    expect(sections.length).toBeGreaterThan(1);
    for (const section of sections) expect(section).toHaveClass("section-heading");
    expect(screen.queryAllByRole("heading", { level: 3 })).toHaveLength(0);
  });
});
