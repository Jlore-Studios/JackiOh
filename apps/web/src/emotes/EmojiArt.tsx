// The five emoji emotes (issue §4): original animated stickers in the tavern's flat gold-and-ink
// style — inline SVG, no borrowed art. Each is drawn the same for every portrait; the bounce,
// hold and fade live in emotes.css (`data-emoji`), which Reduce Motion flattens to a fade.

import type { ReactElement } from "react";

import type { EmojiEmoteId } from "@jackioh/shared";

/** A shared face: the sticker's head, eyes and mouth as one drawing per id. */
function Face({ children }: { children: ReactElement }): ReactElement {
  return (
    <svg viewBox="0 0 48 48" className="emote-emoji-svg" aria-hidden="true">
      <circle cx="24" cy="24" r="20" fill="#f0c04a" stroke="#5a3d1a" strokeWidth="2.5" />
      {children}
    </svg>
  );
}

const EYE = { fill: "#3a2a12" } as const;

/** The five stickers. `wahWah` carries its small trombone sliding in beside the glum face. */
export function EmojiArt({ emoji }: { emoji: EmojiEmoteId }): ReactElement {
  switch (emoji) {
    case "sob":
      return (
        <Face>
          <>
            <circle cx="17" cy="19" r="3" {...EYE} />
            <circle cx="31" cy="19" r="3" {...EYE} />
            {/* the wobble mouth */}
            <path d="M16 32 q4 -4 8 0 t8 0" fill="none" stroke="#3a2a12" strokeWidth="2.5" strokeLinecap="round" />
            {/* the tears */}
            <path className="emote-tear" d="M14 24 q-4 7 0 10 q4 -3 0 -10" fill="#7cc4f0" />
            <path className="emote-tear emote-tear-r" d="M34 24 q4 7 0 10 q-4 -3 0 -10" fill="#7cc4f0" />
          </>
        </Face>
      );
    case "yawn":
      return (
        <Face>
          <>
            {/* closed, drooping eyes */}
            <path d="M12 19 q4 4 9 0" fill="none" stroke="#3a2a12" strokeWidth="2.5" strokeLinecap="round" />
            <path d="M27 19 q4 4 9 0" fill="none" stroke="#3a2a12" strokeWidth="2.5" strokeLinecap="round" />
            {/* the yawn itself */}
            <ellipse cx="24" cy="31" rx="7" ry="9" fill="#3a2a12" />
            <ellipse cx="24" cy="34" rx="4" ry="5" fill="#b0453f" />
          </>
        </Face>
      );
    case "laugh":
      return (
        <Face>
          <>
            {/* squeezed-shut laughing eyes */}
            <path d="M11 18 l7 -2 M13 21 l6 -4" stroke="#3a2a12" strokeWidth="2.2" strokeLinecap="round" />
            <path d="M37 18 l-7 -2 M35 21 l-6 -4" stroke="#3a2a12" strokeWidth="2.2" strokeLinecap="round" />
            {/* the open grin with teeth */}
            <path d="M12 26 q12 16 24 0 z" fill="#3a2a12" />
            <path d="M14.5 26.5 h19 q-2 4 -9.5 4 t-9.5 -4" fill="#f7ead0" />
          </>
        </Face>
      );
    case "angry":
      return (
        <Face>
          <>
            <circle cx="24" cy="24" r="20" fill="#d4574a" stroke="#5a1a12" strokeWidth="2.5" />
            {/* furrowed brows */}
            <path d="M12 15 l8 4 M36 15 l-8 4" stroke="#3a100a" strokeWidth="2.6" strokeLinecap="round" />
            <circle cx="17" cy="22" r="2.6" fill="#3a100a" />
            <circle cx="31" cy="22" r="2.6" fill="#3a100a" />
            {/* the frown */}
            <path d="M16 34 q8 -6 16 0" fill="none" stroke="#3a100a" strokeWidth="2.6" strokeLinecap="round" />
          </>
        </Face>
      );
    case "wahWah":
      return (
        <Face>
          <>
            {/* a glum face, and the little trombone that plays the sting */}
            <circle cx="17" cy="19" r="3" {...EYE} />
            <circle cx="31" cy="19" r="3" {...EYE} />
            <path d="M17 34 q7 -5 14 0" fill="none" stroke="#3a2a12" strokeWidth="2.5" strokeLinecap="round" />
            <g className="emote-trombone">
              <rect x="8" y="38" width="24" height="3" rx="1.5" fill="#c99435" stroke="#5a3d1a" strokeWidth="1" />
              <rect x="8" y="41" width="14" height="3" rx="1.5" fill="#dcb254" stroke="#5a3d1a" strokeWidth="1" />
              <path d="M32 34 l8 7 h-8 z" fill="#dcb254" stroke="#5a3d1a" strokeWidth="1" />
            </g>
          </>
        </Face>
      );
  }
}

export const EMOJI_LABEL: Record<EmojiEmoteId, string> = {
  sob: "Sob",
  yawn: "Yawn",
  laugh: "Laugh",
  angry: "Angry",
  wahWah: "Wah Wah",
};
