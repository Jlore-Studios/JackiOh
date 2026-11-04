// #2645: a test's answer to whether the board is on a phone layout (`usePhoneBar`, game/phoneBar.ts).
// jsdom has no media queries, so `window.matchMedia` is stubbed for the one query the hook reads; the
// returned function puts the original back, so call it in an `afterEach`.

import { PHONE_BAR_QUERY, PHONE_PORTRAIT_QUERY } from "../game/config.ts";
import { PATCH_LEGEND_PHONE_QUERY } from "../patches/constants.ts";

function stubQueries(answers: readonly string[], matches: boolean): () => void {
  const original = window.matchMedia;
  window.matchMedia = ((query: string) => {
    if (!answers.includes(query)) return original(query);
    return {
      media: query,
      matches,
      onchange: null,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      addListener: () => undefined,
      removeListener: () => undefined,
      dispatchEvent: () => false,
    } as unknown as MediaQueryList;
  }) as typeof window.matchMedia;
  return () => {
    window.matchMedia = original;
  };
}

export function stubPhoneBar(matches: boolean): () => void {
  return stubQueries([PHONE_BAR_QUERY], matches);
}

/**
 * #2932: a test's answer to whether the board is on the phone portrait layout (`usePhonePortrait`). A
 * real portrait phone matches `PHONE_BAR_QUERY` too (its first clause is the same width), so both
 * are answered.
 */
export function stubPhonePortrait(matches: boolean): () => void {
  return stubQueries([PHONE_PORTRAIT_QUERY, PHONE_BAR_QUERY], matches);
}

/** R3012000: a test's answer to whether the Patch notes legend is collapsed into a drawer (`usePatchLegendCollapsed`). */
export function stubPatchLegendPhone(matches: boolean): () => void {
  return stubQueries([PATCH_LEGEND_PHONE_QUERY], matches);
}
