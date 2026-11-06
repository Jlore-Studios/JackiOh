// The brand on a menu screen (#3223, R3223000): the logo word, not a heading. The screen's `h1` is its
// PageTitle (routes/PageTitle.tsx), so the brand is a `span` in the stylesheets' `.brand`.

import type { ReactElement } from "react";

export function Wordmark(): ReactElement {
  return (
    <div className="brand">
      <span className="brand__word">JackiOh</span>
    </div>
  );
}
