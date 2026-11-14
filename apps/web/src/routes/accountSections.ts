// `/account`'s sections (#3733, R3733000): a profile header over a settings list, each section at its own
// `?section=` URL so Back and links land on it.

/** The sections in the list's order. */
export const ACCOUNT_SECTIONS = ["overview", "history", "security", "notifications", "privacy", "delete"] as const;

export type AccountSection = (typeof ACCOUNT_SECTIONS)[number];

/** A pending account has no record, history or notifications to show (§9.4): the rest of the list. */
export const PENDING_ACCOUNT_SECTIONS: readonly AccountSection[] = ["overview", "security", "privacy", "delete"];

/** How many of the newest online games the Match history section lists; `/replays` has the rest. */
export const ACCOUNT_HISTORY_ROWS = 10;

/** The section a URL's search string names, or null when it names none or an unknown one. */
export function accountSectionOf(search: string): AccountSection | null {
  const wanted = new URLSearchParams(search).get("section");
  return ACCOUNT_SECTIONS.find((id) => id === wanted) ?? null;
}

/** R3733000: a section's URL. Here, not in `paths`: a path function there takes one argument for every path (MenuShell.test.tsx). */
export function accountSectionPath(section: AccountSection): string {
  return `/account?section=${section}`;
}
