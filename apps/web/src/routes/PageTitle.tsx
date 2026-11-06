// The one title of a menu screen (#3223, R3223000): its only `h1`, in the top bar's title slot
// (BackLink's `title`, routes/nav.tsx, R3218000), or directly in the panel of a screen that has no bar.
// A screen passes its name to BackLink and never draws an `h1` of its own; the brand is the `Wordmark`.

import type { ReactElement } from "react";

import "./page-title.css";

/** The page's one `h1` carries this id, so a landmark may label itself by it (the almanac, the patch notes). */
export const PAGE_TITLE_ID = "page-title";

export const pageTitleTestid = "page-title";

export function PageTitle({ children }: { children: string }): ReactElement {
  return (
    <h1 id={PAGE_TITLE_ID} className="page-title" data-testid={pageTitleTestid}>
      {children}
    </h1>
  );
}
