// The Patch notes page's numbers (CLAUDE.md rule 9). Sizes and spacing live in patches.css.

/** How many release lines, newest first, are open when the page opens (R724000); the others show their name, span and count. */
export const RELEASES_OPEN_ON_LOAD = 1;

/**
 * How many patches, newest first, show their cards when the page opens (R507). The rest show their
 * version, date, title, note and counts, and load their snapshots only when opened.
 */
export const PATCHES_OPEN_ON_LOAD = 1;
