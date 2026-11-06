// The data of `jackioh-shared`'s face model that specs and component specs read, straight from the crate's
// files (`crates/shared/data/`), so no table of what a face prints is kept here. The browser never reads them
// at run time: the face model is called through the WebAssembly module (`model.ts`).

import powerWordsJson from "../../../../crates/shared/data/power-words.json";
import shortFormsJson from "../../../../crates/shared/data/short-forms.json";

/** §8 #98's thirteen powers, by the view's power name: X, name on each face and words, base and Radiant. */
export const POWER_WORDS: Readonly<Record<string, { x: number; title: string; radiantTitle: string; base: string; radiant: string }>> = powerWordsJson;

/**
 * R1017000: every catalog face whose printed text runs past TEXT_TIER_MAX.xl (260 characters), as "defId
 * base|radiant". textLimit.test.ts pins this list on every pull request; the B15 and MN09 component specs
 * read it for their clamp diagnostics.
 */
export const LONG_FACES: readonly string[] = shortFormsJson.long;
