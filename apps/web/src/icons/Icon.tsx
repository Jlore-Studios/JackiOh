// Bespoke inline SVG icons with unified stroke weight and type-scale sizing (#5016).

import type { ReactElement } from "react";
import "./icon.css";

export type IconName = "close" | "check" | "spark" | "star" | "crown" | "skull" | "flag";

export type IconSize = "xs" | "sm" | "md" | "lg" | "xl" | "2xl" | "3xl" | "4xl";

export interface IconProps {
  name: IconName;
  size?: IconSize;
  className?: string;
}

function IconPath({ name }: { name: IconName }): ReactElement {
  switch (name) {
    case "close":
      return <path d="M3.5 3.5l9 9M12.5 3.5l-9 9" />;
    case "check":
      return <path d="M3 8.5l3.5 3.5 6.5-8" />;
    case "spark":
      return <path d="M8 1.5C8 5 5 8 1.5 8C5 8 8 11 8 14.5C8 11 11 8 14.5 8C11 8 8 5 8 1.5Z" />;
    case "star":
      return <path d="M8 1.75l1.9 3.85 4.25.62-3.08 3 .73 4.23L8 11.45l-3.8 2 .73-4.23-3.08-3 4.25-.62z" />;
    case "crown":
      return <path d="M2.5 12.5h11l-1-7-3 3.5-1.5-5-1.5 5-3-3.5z" />;
    case "skull":
      return (
        <>
          <path d="M4 7.5a4 4 0 1 1 8 0c0 1.5-.7 2.7-1.5 3.3v2.2H5.5v-2.2C4.7 10.2 4 9 4 7.5z" />
          <path d="M6 7.5v.01M10 7.5v.01M6.5 13v-1.5M9.5 13v-1.5" />
        </>
      );
    case "flag":
      return <path d="M3.5 2v12M3.5 3h8.5l-2 3.5 2 3.5h-8.5" />;
  }
}

export function Icon({ name, size, className }: IconProps): ReactElement {
  const classes = [
    "icon",
    `icon--${name}`,
    size !== undefined ? `icon--size-${size}` : "",
    className ?? "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <svg
      className={classes}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <IconPath name={name} />
    </svg>
  );
}

export default Icon;
