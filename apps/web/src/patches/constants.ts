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

/** #2681: the seconds one slow drift of a major's animated artwork takes, there and back. */
export const RELEASE_ART_MAJOR_CYCLE_SECONDS = 18;
