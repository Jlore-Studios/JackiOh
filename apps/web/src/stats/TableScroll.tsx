// The horizontal scroll box every /stats table sits in. A phone is narrower than a table, so the box
// scrolls, and a box that scrolls must take the keyboard (WCAG 2.1.1, axe's
// scrollable-region-focusable): it is a focusable region named by the heading or tab it belongs to,
// whether or not its table holds a control of its own (the Created cards' holds none, #2158).

import type { ReactElement, ReactNode } from "react";

export function TableScroll({ labeledBy, children }: { labeledBy: string; children: ReactNode }): ReactElement {
  return (
    <div className="stats-table-wrapper" role="region" aria-labelledby={labeledBy} tabIndex={0}>
      {children}
    </div>
  );
}
