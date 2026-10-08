// The emoji emotes (issue §4, R1345): original animated stickers in the tavern's flat gold-and-ink
// style — inline SVG, no borrowed art and no `<text>`. Patch v0.2.X's five, then MN03's fourteen
// (#545). Each is drawn the same for every portrait; the bounce, hold and fade live in emotes.css
// (`data-emoji`), which Reduce Motion flattens to a fade, and so are the few parts that move inside a
// sticker (a waving hand, a beating heart, a flicker, a falling drop, confetti), which it stills.

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

const INK = "#3a2a12";
const EYE = { fill: INK } as const;
/** A stroke in the faces' ink, round-capped, as every mouth and brow is drawn. */
const LINE = { fill: "none", stroke: INK, strokeWidth: 2.5, strokeLinecap: "round" } as const;
/** A hand or an arm: a paler gold than the face, outlined in the face's brown. */
const HAND = { fill: "#f7d77e", stroke: "#5a3d1a", strokeWidth: 1.3 } as const;

/** A sticker that is not a face (a heart, a flame, a skull, a popper): the same frame, no head. */
function Sticker({ children }: { children: ReactElement }): ReactElement {
  return (
    <svg viewBox="0 0 48 48" className="emote-emoji-svg" aria-hidden="true">
      {children}
    </svg>
  );
}

/** The stickers. `wahWah` carries its small trombone sliding in beside the glum face. */
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
    case "wave":
      return (
        <Face>
          <>
            {/* a sunny smile, and the hand that waves hello at the face's lower right */}
            <path d="M12 19 q4 -4 8 0" {...LINE} />
            <path d="M24 19 q4 -4 8 0" {...LINE} />
            <path d="M13 27 q8 8 16 0" {...LINE} />
            <g className="emote-wave-hand">
              <rect x="33" y="24" width="3" height="10" rx="1.5" {...HAND} />
              <rect x="36.2" y="22.5" width="3" height="11" rx="1.5" {...HAND} />
              <rect x="39.4" y="23.5" width="3" height="10" rx="1.5" {...HAND} />
              <rect x="42.4" y="26" width="2.8" height="8" rx="1.4" {...HAND} />
              <rect x="28.5" y="31" width="7" height="3" rx="1.5" transform="rotate(-35 32 32.5)" {...HAND} />
              <rect x="33" y="30" width="12" height="11" rx="4.5" {...HAND} />
            </g>
            <path d="M42 15 q4 2 4 6" fill="none" stroke={INK} strokeWidth="1.5" strokeLinecap="round" />
            <path d="M44 11 q3 2 3.5 5" fill="none" stroke={INK} strokeWidth="1.2" strokeLinecap="round" />
          </>
        </Face>
      );
    case "clap":
      return (
        <Face>
          <>
            {/* happy closed eyes, an open grin, and two hands meeting under it */}
            <path d="M12 18 q4 -5 8 0" {...LINE} />
            <path d="M28 18 q4 -5 8 0" {...LINE} />
            <path d="M15 23 q9 9 18 0 z" fill={INK} />
            <g className="emote-clap-hands">
              <ellipse cx="19.5" cy="38" rx="5.5" ry="8.5" transform="rotate(-24 19.5 38)" {...HAND} />
              <ellipse cx="28.5" cy="38" rx="5.5" ry="8.5" transform="rotate(24 28.5 38)" {...HAND} />
              <path d="M18 33 l3 6 M21.5 31.5 l2 5 M30 33 l-3 6 M26.5 31.5 l-2 5" stroke="#5a3d1a" strokeWidth="0.9" strokeLinecap="round" />
            </g>
            <path d="M12 31 l-3 -3 M36 31 l3 -3 M11 37 h-3.5 M37 37 h3.5" stroke={INK} strokeWidth="1.6" strokeLinecap="round" />
          </>
        </Face>
      );
    case "thumbsUp":
      return (
        <Sticker>
          <>
            {/* a fist, its thumb up, and the cuff of a blue sleeve */}
            <rect x="34" y="24" width="8" height="17" rx="2" fill="#4f7fbf" stroke="#1e3552" strokeWidth="1.5" />
            <rect x="17" y="5" width="9" height="22" rx="4.5" {...HAND} strokeWidth={2} />
            <rect x="11" y="21" width="24" height="21" rx="6" {...HAND} strokeWidth={2} />
            <path d="M12 28 h10 M12 33.5 h10 M13 38.5 h9" stroke="#5a3d1a" strokeWidth="1.6" strokeLinecap="round" />
          </>
        </Sticker>
      );
    case "facepalm":
      return (
        <Face>
          <>
            {/* one shut eye under the hand, a flat wavy mouth, and the palm across the brow */}
            <path d="M28 22 h7" {...LINE} />
            <path d="M15 33 q3 -2 6 0 t6 0 t6 0" {...LINE} />
            <g transform="rotate(-24 22 15)">
              <rect x="9" y="7" width="3.2" height="9" rx="1.6" {...HAND} />
              <rect x="12.6" y="5.5" width="3.2" height="10" rx="1.6" {...HAND} />
              <rect x="16.2" y="6" width="3.2" height="10" rx="1.6" {...HAND} />
              <rect x="19.8" y="7.5" width="3" height="8.5" rx="1.5" {...HAND} />
              <rect x="8" y="12" width="17" height="11" rx="4.5" {...HAND} />
            </g>
            <rect x="0" y="22" width="13" height="7" rx="3" transform="rotate(32 6.5 25.5)" {...HAND} />
          </>
        </Face>
      );
    case "shrug":
      return (
        <Face>
          <>
            {/* raised brows, a flat mouth, and both palms turned up at the sides */}
            <path d="M12 14 q4 -4 8 0" {...LINE} />
            <path d="M28 14 q4 -4 8 0" {...LINE} />
            <circle cx="17" cy="20" r="2.6" {...EYE} />
            <circle cx="31" cy="20" r="2.6" {...EYE} />
            <path d="M18 31 q6 -2 12 1" {...LINE} />
            <path d="M1 31 q5 -5 11 -1 l-1.5 3.5 q-4 -2 -8.5 -0.5 z" {...HAND} />
            <path d="M47 31 q-5 -5 -11 -1 l1.5 3.5 q4 -2 8.5 -0.5 z" {...HAND} />
          </>
        </Face>
      );
    case "thinking":
      return (
        <Face>
          <>
            {/* one brow up, eyes turned up and aside, a slanted mouth, a finger on the chin */}
            <path d="M12 15 h8" {...LINE} />
            <path d="M28 14 q4 -5 8 -1" {...LINE} />
            <circle cx="18" cy="19" r="2.6" {...EYE} />
            <circle cx="33" cy="18.5" r="2.6" {...EYE} />
            <path d="M22 30 l9 -2" {...LINE} />
            <rect x="21" y="33" width="3.4" height="9" rx="1.7" transform="rotate(-15 22.7 37.5)" {...HAND} />
            <rect x="12" y="36" width="12" height="9" rx="4" {...HAND} />
          </>
        </Face>
      );
    case "heart":
      return (
        <Sticker>
          <>
            <g className="emote-heartbeat">
              <path
                d="M24 42 C10 32 4 25 4 16.5 C4 9.5 9 5 15 5 C19 5 22 7.5 24 11 C26 7.5 29 5 33 5 C39 5 44 9.5 44 16.5 C44 25 38 32 24 42 Z"
                fill="#d4574a"
                stroke="#5a1a12"
                strokeWidth="2.5"
                strokeLinejoin="round"
              />
              {/* the shine on its upper left */}
              <path d="M10 15 q1 -5 6 -5.5" fill="none" stroke="#f6b3a8" strokeWidth="2.6" strokeLinecap="round" />
            </g>
          </>
        </Sticker>
      );
    case "fire":
      return (
        <Sticker>
          <>
            <g className="emote-flame">
              <path
                d="M24 3 C27 11 37 15 37 28 C37 38 31 45 24 45 C17 45 11 38 11 29 C11 22 15 18 18 13 C19 18 21 21 23 22 C22 15 22 9 24 3 Z"
                fill="#e8823a"
                stroke="#5a1a12"
                strokeWidth="2.5"
                strokeLinejoin="round"
              />
              <path
                d="M24 20 C26.5 26 31 28.5 31 35 C31 40 28 43 24 43 C20 43 17 40 17 36 C17 32 20 30 21 26.5 C22 29.5 23 30.5 24.5 31 C23.5 27 23 23.5 24 20 Z"
                fill="#f0c04a"
              />
              <ellipse cx="24" cy="38.5" rx="3.5" ry="3.8" fill="#f7ead0" />
            </g>
          </>
        </Sticker>
      );
    case "skull":
      return (
        <Sticker>
          <>
            <path
              d="M24 5 C13 5 7 12 7 21 C7 27 10 31 14 33 L14 39 Q14 42 17 42 L31 42 Q34 42 34 39 L34 33 C38 31 41 27 41 21 C41 12 35 5 24 5 Z"
              fill="#efe6d2"
              stroke={INK}
              strokeWidth="2.5"
              strokeLinejoin="round"
            />
            <ellipse cx="17" cy="22" rx="4.6" ry="5" {...EYE} />
            <ellipse cx="31" cy="22" rx="4.6" ry="5" {...EYE} />
            <path d="M24 27.5 l-2.6 4.5 h5.2 z" {...EYE} />
            <path d="M19.5 36 v6 M24 36 v6 M28.5 36 v6" stroke={INK} strokeWidth="1.6" strokeLinecap="round" />
          </>
        </Sticker>
      );
    case "sweat":
      return (
        <Face>
          <>
            {/* worried brows, a nervous toothy grin, and a drop running down the brow */}
            <path d="M12 16 l7 -3" {...LINE} />
            <path d="M29 13 l7 3" {...LINE} />
            <circle cx="17" cy="21" r="2.6" {...EYE} />
            <circle cx="31" cy="21" r="2.6" {...EYE} />
            <path d="M13 29 h22 q-2 7 -11 7 t-11 -7 z" fill="#f7ead0" stroke={INK} strokeWidth="2" strokeLinejoin="round" />
            <path d="M18 29.5 v5 M24 29.5 v6 M30 29.5 v5" stroke={INK} strokeWidth="1.2" />
            <path className="emote-sweat-drop" d="M38 7 q-5 8 0 11 q5 -3 0 -11" fill="#7cc4f0" stroke="#3a6a8a" strokeWidth="1" />
          </>
        </Face>
      );
    case "cool":
      return (
        <Face>
          <>
            {/* dark shades with their glint, and a sideways smirk */}
            <path
              d="M7 16 h34 v3 h-2 q-1 7 -7.5 7 q-6 0 -6.5 -7 h-2 q-0.5 7 -6.5 7 q-6.5 0 -7.5 -7 h-2 z"
              fill="#1d1408"
              stroke="#1d1408"
              strokeWidth="1"
              strokeLinejoin="round"
            />
            <path d="M12 19.5 l3 3 M28 19.5 l3 3" stroke="#8c7550" strokeWidth="1.6" strokeLinecap="round" />
            <path d="M17 33 q8 4 14 -3" {...LINE} />
          </>
        </Face>
      );
    case "gasp":
      return (
        <Face>
          <>
            {/* brows flown up, eyes wide, and the round O of a gasp */}
            <path d="M11 12 q5 -4 9 -1" {...LINE} />
            <path d="M28 11 q4 -3 9 1" {...LINE} />
            <circle cx="17" cy="20" r="4.6" fill="#f7ead0" stroke={INK} strokeWidth="1.6" />
            <circle cx="31" cy="20" r="4.6" fill="#f7ead0" stroke={INK} strokeWidth="1.6" />
            <circle cx="17" cy="20.5" r="2" {...EYE} />
            <circle cx="31" cy="20.5" r="2" {...EYE} />
            <ellipse cx="24" cy="33.5" rx="4.6" ry="6" {...EYE} />
          </>
        </Face>
      );
    case "salute":
      return (
        <Face>
          <>
            {/* a level look, a firm small smile, and a flat hand snapped to the brow */}
            <path d="M12 17 h8 M28 17 h8" {...LINE} />
            <circle cx="16" cy="22" r="2.6" {...EYE} />
            <circle cx="32" cy="22" r="2.6" {...EYE} />
            <path d="M18 32 q6 4 12 0" {...LINE} />
            <rect x="35" y="17" width="15" height="7" rx="3" transform="rotate(62 42.5 20.5)" {...HAND} />
            <g transform="rotate(-18 34 10.5)">
              <rect x="24" y="7" width="21" height="7.5" rx="3.6" {...HAND} />
              <path d="M27 9.5 h14 M27 12 h14" stroke="#5a3d1a" strokeWidth="0.9" strokeLinecap="round" />
            </g>
          </>
        </Face>
      );
    case "party":
      return (
        <Sticker>
          <>
            {/* a popper's striped cone, its gold rim, and the confetti bursting out of it */}
            <path d="M5 44 L15 19 L30 34 Z" fill="#d4574a" stroke={INK} strokeWidth="2" strokeLinejoin="round" />
            <path d="M9 35 l9 8.5 M12 27.5 l11.5 11" stroke="#f0c04a" strokeWidth="2.6" strokeLinecap="round" />
            <ellipse cx="22.5" cy="26.5" rx="10.5" ry="3.6" transform="rotate(45 22.5 26.5)" fill="#f0c04a" stroke={INK} strokeWidth="1.6" />
            <g className="emote-confetti">
              <rect x="30" y="8" width="4" height="4" rx="1" transform="rotate(20 32 10)" fill="#7cc4f0" />
              <rect x="39" y="15" width="4" height="4" rx="1" transform="rotate(-25 41 17)" fill="#d4574a" />
              <circle cx="36" cy="4.5" r="2" fill="#f0c04a" />
              <circle cx="43" cy="27" r="2" fill="#7cc4f0" />
              <circle cx="25" cy="9" r="1.8" fill="#9fd36b" />
              <path d="M28 18 q4 -6 8 -4 t6 -6" fill="none" stroke="#9fd36b" strokeWidth="2" strokeLinecap="round" />
              <path d="M32 26 q6 -2 8 2 t6 0" fill="none" stroke="#e8823a" strokeWidth="2" strokeLinecap="round" />
            </g>
          </>
        </Sticker>
      );
  }
}

/** Every sticker's accessible name: the menu item's `aria-label` and `title` (R1345). */
export const EMOJI_LABEL: Record<EmojiEmoteId, string> = {
  sob: "Sob",
  yawn: "Yawn",
  laugh: "Laugh",
  angry: "Angry",
  wahWah: "Wah Wah",
  wave: "Wave",
  clap: "Clap",
  thumbsUp: "Thumbs Up",
  facepalm: "Facepalm",
  shrug: "Shrug",
  thinking: "Thinking",
  heart: "Heart",
  fire: "Fire",
  skull: "Skull",
  sweat: "Sweat",
  cool: "Cool",
  gasp: "Gasp",
  salute: "Salute",
  party: "Party",
};
