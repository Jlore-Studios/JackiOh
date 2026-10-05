// The Glitch token's face (SPEC §7, R662): a completely blank card, and where its art would be a
// glitching blob that breaks out of the card's frame. Nothing on it is text: its name and words are
// drawn corrupted wherever else they show (glitch.ts), and here they are not drawn at all.
//
// It keeps the `.cf` root and the attributes the board and the overlays read off a face, and, like
// every face, only spans, so it sits anywhere a CardFace does (B12). With no words on it, its name, as
// corrupted as everywhere else, is its label, and the state rail (CardStates) still shows what the view
// says of the card in play, a Brittle count or an enchantment. The blob is a child of the root
// outside `.cf-scale`, whose clip it escapes; the root carries no `data-foil`, so the foil's clip
// (cards.css) does not hold it either, and a board card holding it lets it out too (board.css,
// `[data-glitch]`). It never moves under Reduce motion (glitch.css).

import type { ReactElement } from "react";

import type { CardFaceProps } from "./CardFace.tsx";
import { CardStates } from "./CardStates.tsx";

import "./glitch.css";

/** The blob's layers: its body, two colour channels torn off it, and a slice that jumps across it. */
const LAYERS = ["body", "red", "cyan", "slice"] as const;

export function GlitchFace({ face, layout = "full", className }: CardFaceProps): ReactElement {
  return (
    <span
      className={["cf", `cf--${layout}`, "cf-glitch", className].filter(Boolean).join(" ")}
      data-layout={layout}
      data-card-type={face.type}
      data-glitch="true"
      data-radiant-face={face.radiant ? "true" : undefined}
      data-in-play={face.inPlay ? "true" : undefined}
      role="img"
      aria-label={face.name}
    >
      <span className="cf-scale">
        <CardStates face={face} />
      </span>
      <span className="cf-glitch-blob" aria-hidden="true">
        {LAYERS.map((layer) => (
          <span key={layer} className="cf-glitch-layer" data-layer={layer} />
        ))}
      </span>
    </span>
  );
}
