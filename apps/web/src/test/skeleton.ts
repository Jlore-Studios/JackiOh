// #2115: the check every converted panel's test shares. Test support only; no component imports it.

import { expect } from "vitest";

import { skeletonTestid, type SkeletonVariant } from "../skeleton/Skeleton.tsx";

/** `status` is a panel's loading words: a status line inside a skeleton of `variant`, whose blocks a screen reader skips. */
export function expectSkeleton(status: HTMLElement, variant: SkeletonVariant): void {
  expect(status).toHaveAttribute("role", "status");
  const skeleton = status.closest(`[data-testid="${skeletonTestid.root}"]`);
  expect(skeleton, "the words sit in a skeleton").not.toBeNull();
  expect(skeleton).toHaveAttribute("data-variant", variant);
  const blocks = skeleton?.querySelector(`[data-testid="${skeletonTestid.blocks}"]`);
  expect(blocks).toHaveAttribute("aria-hidden", "true");
  expect(blocks?.querySelectorAll(".skeleton__block").length).toBeGreaterThan(0);
}
