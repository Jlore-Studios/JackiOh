// #2108: the check every converted empty panel's test shares. Test support only; no component imports it.

import { expect } from "vitest";

import { emptyStateTestid } from "../empty/EmptyState.tsx";

/** `text` is a panel's empty words, inside an empty state whose tankard a screen reader skips. Returns the empty state. */
export function expectEmptyState(text: HTMLElement): HTMLElement {
  const root = text.closest<HTMLElement>(`[data-testid="${emptyStateTestid.root}"]`);
  if (root === null) throw new Error("the words sit in no empty state");
  expect(root.querySelector(`[data-testid="${emptyStateTestid.mark}"]`)).toHaveAttribute("aria-hidden", "true");
  return root;
}
