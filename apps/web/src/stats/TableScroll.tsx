// Every /stats table's horizontal scroll box. A box that scrolls must take the keyboard (WCAG 2.1.1, axe's
// scrollable-region-focusable): a focusable region named by its heading or tab, even when its table holds
// no control of its own (the Created cards', #2158).

import type { ReactElement, ReactNode } from "react";

export function TableScroll({ labeledBy, children }: { labeledBy: string; children: ReactNode }): ReactElement {
  return (
    <div className="stats-table-wrapper" role="region" aria-labelledby={labeledBy} tabIndex={0}>
      {children}
    </div>
  );
}
