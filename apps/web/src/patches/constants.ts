// The Patch notes page's numbers (CLAUDE.md rule 9). Sizes and spacing live in patches.css.

/** How many release lines, newest first, are open when the page opens (R724000); the others show their name, span and count. */
export const RELEASES_OPEN_ON_LOAD = 1;

/** How many patches, newest first, show their cards when the page opens (R507); the rest load their snapshots only when opened. */
export const PATCHES_OPEN_ON_LOAD = 1;

/** The most characters GitHub keeps in a release's notes; a longer patch lists its cards more briefly (github.ts). */
export const GITHUB_RELEASE_BODY_LIMIT = 125_000;
