// #2645: a test's answer to whether the board is on a phone layout (`usePhoneBar`, game/phoneBar.ts).
//
// jsdom has no media queries, so `window.matchMedia` is stubbed for the one query the hook reads
// (`PHONE_BAR_QUERY`): it matches or not as the test says, and every other query goes to the original.
// The returned function puts the original back; call it in an `afterEach`.

import { PHONE_BAR_QUERY } from "../game/config.ts";

export function stubPhoneBar(matches: boolean): () => void {
  const original = window.matchMedia;
  window.matchMedia = ((query: string) => {
    if (query !== PHONE_BAR_QUERY) return original(query);
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
