import type { ReactElement } from "react";

import "./arrow-icon.css";

export function ArrowIcon({ direction }: { direction: "back" | "forward" }): ReactElement {
  return (
    <svg
      className={`arrow-icon arrow-icon--${direction}`}
      viewBox="0 0 16 16"
      aria-hidden="true"
      focusable="false"
    >
      <path
        fill="none"
        stroke="currentColor"
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        d={direction === "back" ? "M13 8H3m4-4L3 8l4 4" : "M3 8h10M9 4l4 4-4 4"}
      />
    </svg>
  );
}
