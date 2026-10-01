// R436: what a Call to Chaos rolled, shown still, where the effects layer draws nothing.
//
// The effects layer's reveal (fx/chaos.ts) is decoration and, like every effect, is not drawn under
// reduced motion or with the effects off (R200). The roll is information both players are owed, so in
// those modes CardShowcase draws this instead: a plain panel over the board with "Call to Chaos" and
// one line per effect, in the order they resolve, for as long as the reveal would have stood. It never
// moves, takes no pointer event, and is hidden from assistive tech, which hears the showcase's
// `chaos-live` region say the same words.

import type { CSSProperties, ReactElement } from "react";
import { createPortal } from "react-dom";

import { CHAOS_TEXT, showcaseTestid } from "./constants.ts";

export type ChaosBannerProps = { names: readonly string[]; seq: number };

export default function ChaosBanner({ names, seq }: ChaosBannerProps): ReactElement {
  const style: CSSProperties = { pointerEvents: "none" };
  return createPortal(
    <div className="chaos-banner" data-testid={showcaseTestid.chaos} data-seq={seq} aria-hidden="true" style={style}>
      <span className="chaos-banner-title">{CHAOS_TEXT.title}</span>
      <ol className="chaos-banner-lines">
        {names.map((name, index) => (
          <li key={`${String(index)}:${name}`} className="chaos-banner-line" data-testid={showcaseTestid.chaosLine}>
            {name}
          </li>
        ))}
      </ol>
    </div>,
    document.body,
  );
}
