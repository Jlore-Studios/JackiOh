// R3223000 (#3223): a menu screen has one visible `h1`, and it is the PageTitle (routes/PageTitle.tsx). The one
// assertion every route test shares, so the claim is the same word for word on every screen.

import { expect } from "vitest";

import { pageTitleTestid } from "../routes/PageTitle.tsx";

/** What makes an element unseen: the tutorial's screen-reader-only class, `hidden`, and `aria-hidden`. */
const UNSEEN = ".tutorial-sr-only, [hidden], [aria-hidden='true']";

/** `root` holds exactly one `h1`: the PageTitle, named `name`, drawn (not visually hidden). */
export function expectOnePageTitle(name: string, root: ParentNode = document.body): HTMLElement {
  const headings = Array.from(root.querySelectorAll("h1"));
  expect(
    headings.map((heading) => heading.textContent),
    "the page's h1s",
  ).toHaveLength(1);
  const [title] = headings;
  if (title === undefined) throw new Error("unreachable: one h1 was counted");
  expect(title.getAttribute("data-testid"), "the one h1 is the PageTitle").toBe(pageTitleTestid);
  expect(title).toHaveClass("page-title");
  expect(title.textContent).toBe(name);
  expect(title.closest(UNSEEN), "the title is drawn, not hidden").toBeNull();
  // Below the title no heading skips a level: a section is an h2, and an h3 sits inside an h2.
  let level = 1;
  for (const heading of Array.from(root.querySelectorAll("h1, h2, h3, h4, h5, h6"))) {
    const next = Number(heading.tagName.slice(1));
    expect(next, `${heading.tagName} "${heading.textContent}" skips a heading level after h${level}`).toBeLessThanOrEqual(level + 1);
    level = next;
  }
  return title;
}
