// Glitch's art (issue #170): a glitchy blob that breaks out of the card's frame. Decoration only,
// aria-hidden and wordless, drawn by glitch.css from three offset layers and a band of scanlines.
// It animates with CSS alone, and stands still under "Reduce motion" (index.css's attribute and the
// media query, as the foil does).

import type { ReactElement } from "react";

import "./glitch.css";

export function GlitchBlob(): ReactElement {
  return (
    <span className="cf-glitch" aria-hidden="true">
      <span className="cf-glitch-blob cf-glitch-blob--red" />
      <span className="cf-glitch-blob cf-glitch-blob--cyan" />
      <span className="cf-glitch-blob cf-glitch-blob--core" />
      <span className="cf-glitch-scan" />
    </span>
  );
}
