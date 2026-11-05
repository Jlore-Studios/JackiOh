// The Patch notes page's numbers (CLAUDE.md rule 9). Sizes and spacing live in patches.css.

/** How many release lines, newest first, are open when the page opens (R724000); the others show their name, span and count. */
export const RELEASES_OPEN_ON_LOAD = 1;

/** How many patches, newest first, show their cards when the page opens (R507); the rest load their snapshots only when opened. */
export const PATCHES_OPEN_ON_LOAD = 1;

/** The most characters GitHub keeps in a release's notes; a longer patch lists its cards more briefly (github.ts). */
export const GITHUB_RELEASE_BODY_LIMIT = 125_000;

/** #2681: a release's artwork is a 16:9 full-screen canvas, as a monitor shows it (releaseArt.ts). */
export const RELEASE_ART_WIDTH = 1920;
export const RELEASE_ART_HEIGHT = 1080;

/** #2681: the most cards one artwork shows; a patch that changed more is sampled evenly, first and last included. */
export const RELEASE_ART_CARD_LIMIT = 7;

/** R3012001: the most headline cards a major's poster shows, picked by rarity (releaseArt.ts); a patch's art stays at `RELEASE_ART_CARD_LIMIT`. */
export const RELEASE_ART_MAJOR_CARD_LIMIT = 11;

/** R3012001: how much smaller a major's cards are drawn than a patch's, so the poster's fan holds more of them. */
export const RELEASE_ART_MAJOR_CARD_SCALE = 0.85;

/**
 * R3012001, CLAUDE.md rule 13: the last micro patch shipped before micros stopped getting art. Published
 * releases and static pages link every earlier micro's `/release-art/<version>.svg`, so each up to and
 * including this version (in patches.json's order, never compared as a version: R105) keeps its file; a
 * micro after it has none.
 */
export const RELEASE_ART_LEGACY_MICRO_LAST = "v0.5.11n";

/** R3012000: the width at and under which the Patch notes legend collapses into a drawer; patches.css's phone block holds the same number. */
export const PATCH_LEGEND_PHONE_QUERY = "(max-width: 760px)";

/** R3012000: the band of the viewport, from its top, in which an entry counts as the one in view (an IntersectionObserver `rootMargin`). */
export const PATCH_LEGEND_BAND_MARGIN = "0px 0px -80% 0px";
