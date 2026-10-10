// The keyword treatments on a board minion (R438; the map and its caps are keywordVisuals.ts, the
// look and the loops keywords.css). MinionFace mounts this right after the portrait.
//
// Each treatment is a `span.kw-fx[data-keyword-fx="<kind>"]` holding one or two parts, each part an
// inline, aria-hidden SVG with no <text> or <title> (a face is span, strong, img and aria-hidden SVG,
// docs/polish/6-cards.md B12). A part that loops carries `kw-anim`; keywords.css runs its keyframes
// only while the treatment is `data-kw-motion="on"`, and never under reduced motion. The numbers a
// treatment prints (Brittle's count, Spell Damage's bonus, Lucky's X) come from its `data-n` through
// CSS, so the unit's text content stays what it was: the chips still name every keyword.
//
// Divine Shield's bubble and Armor's plate are MinionFace's own elements (`.shield-icon`,
// `.stat-armor`), which carry the same attributes; this draws every other treatment.

import type { ReactElement, ReactNode } from "react";

import { keywordFxAttributes, type KeywordFxPlan } from "./keywordVisuals.ts";
import type { KeywordKind } from "@jackioh/shared";

/** The treatments MinionFace draws with elements it already has. */
export const DRAWN_BY_MINION: readonly KeywordKind[] = ["Divine Shield", "Armor"];

type PartProps = { className: string; anim?: boolean; fit?: "fill" | "contain"; children: ReactNode };

/** One layer of a treatment: a span sized by keywords.css with its SVG drawn in a 100x100 box. */
function Part({ className, anim = false, fit = "fill", children }: PartProps): ReactElement {
  return (
    <span className={anim ? `kw-part kw-anim ${className}` : `kw-part ${className}`}>
      <svg
        viewBox="0 0 100 100"
        preserveAspectRatio={fit === "fill" ? "none" : "xMidYMid meet"}
        aria-hidden="true"
        focusable="false"
      >
        {children}
      </svg>
    </span>
  );
}

/** A dark underlay stroke and a light stroke over it, so a line reads on any art. */
function Inked({ d, ink, width, className }: { d: string; ink: string; width: number; className?: string }): ReactElement {
  return (
    <g className={className}>
      <path d={d} fill="none" stroke="#0b0d12" strokeWidth={width + 2.2} strokeLinecap="round" strokeLinejoin="round" />
      <path d={d} fill="none" stroke={ink} strokeWidth={width} strokeLinecap="round" strokeLinejoin="round" />
    </g>
  );
}

/** A four-pointed sparkle centred on (x, y). */
function sparkle(x: number, y: number, r: number): string {
  const w = r * 0.28;
  return `M${x} ${y - r} L${x + w} ${y - w} L${x + r} ${y} L${x + w} ${y + w} L${x} ${y + r} L${x - w} ${y + w} L${x - r} ${y} L${x - w} ${y - w} Z`;
}

/** Chain links along the x axis at y = 50, alternating flat and edge-on; rotated into place by the caller. */
function chainLinks(): string {
  const links: string[] = [];
  for (let i = 0; i < 9; i += 1) {
    const x = -8 + i * 15;
    links.push(
      i % 2 === 0
        ? `M${x} 45 H${x + 12} A5 5 0 0 1 ${x + 12} 55 H${x} A5 5 0 0 1 ${x} 45 Z`
        : `M${x - 1} 49 H${x + 13} A1 1 0 0 1 ${x + 13} 51 H${x - 1} A1 1 0 0 1 ${x - 1} 49 Z`,
    );
  }
  return links.join(" ");
}

/** A cog of `teeth` teeth round (50, 50), outer radius 46. */
function cog(teeth: number): string {
  const points: string[] = [];
  const outer = 46;
  const inner = 36;
  for (let i = 0; i < teeth * 4; i += 1) {
    const angle = (i / (teeth * 4)) * Math.PI * 2;
    const r = i % 4 === 1 || i % 4 === 2 ? outer : inner;
    points.push(`${(50 + r * Math.cos(angle)).toFixed(2)} ${(50 + r * Math.sin(angle)).toFixed(2)}`);
  }
  return `M${points.join(" L")} Z`;
}

const CHAINS = chainLinks();
const COG_8 = cog(8);
const COG_6 = cog(6);

/** The drawing of every treatment this component owns, by kind. */
const DRAW: Readonly<Record<Exclude<KeywordKind, "Divine Shield" | "Armor">, () => ReactNode>> = {
  // Frame: MinionFace's steel shield, moved here from `.cf-portrait::before`; CSS draws it.
  Taunt: () => <span className="kw-part kw-taunt-shield" />,

  "Immune to Spells": () => (
    <>
      <Part className="kw-ward">
        <polygon points="50,2 97,26 97,74 50,98 3,74 3,26" fill="none" stroke="#0b0d12" strokeWidth="6" />
        <polygon points="50,2 97,26 97,74 50,98 3,74 3,26" fill="none" stroke="#c9a6ff" strokeWidth="3" />
        <polygon points="50,9 90,30 90,70 50,91 10,70 10,30" fill="none" stroke="#c9a6ff" strokeWidth="1.2" strokeOpacity="0.7" />
      </Part>
      <Part className="kw-ward-sparks" anim>
        {[
          [50, 2],
          [97, 26],
          [97, 74],
          [50, 98],
          [3, 74],
          [3, 26],
        ].map(([x = 0, y = 0]) => (
          <path key={`${x}-${y}`} d={sparkle(x, y, 7)} fill="#f3e9ff" stroke="#3b1f6e" strokeWidth="1" />
        ))}
      </Part>
    </>
  ),

  Reborn: () => (
    <>
      <Part className="kw-echo" anim>
        <ellipse cx="50" cy="50" rx="46" ry="46" fill="none" stroke="#0b0d12" strokeWidth="5" strokeOpacity="0.5" />
        <ellipse cx="50" cy="50" rx="46" ry="46" fill="none" stroke="#b6f4ff" strokeWidth="3" strokeDasharray="10 6" />
      </Part>
      <Part className="kw-halo">
        <ellipse cx="50" cy="50" rx="44" ry="26" fill="none" stroke="#0b0d12" strokeWidth="14" strokeOpacity="0.45" />
        <ellipse cx="50" cy="50" rx="44" ry="26" fill="none" stroke="#dffaff" strokeWidth="9" />
      </Part>
    </>
  ),

  Indestructible: () => (
    <>
      <Part className="kw-plating">
        <ellipse cx="50" cy="50" rx="46" ry="46" fill="none" stroke="#0b0d12" strokeWidth="11" pathLength="100" strokeDasharray="10 2.5" />
        <ellipse cx="50" cy="50" rx="46" ry="46" fill="none" stroke="#8c94a3" strokeWidth="7.5" pathLength="100" strokeDasharray="10 2.5" />
        <ellipse cx="50" cy="50" rx="46" ry="46" fill="none" stroke="#c8ced9" strokeWidth="1.6" pathLength="100" strokeDasharray="1.2 11.3" strokeDashoffset="-4.4" />
      </Part>
      <Part className="kw-runes" anim>
        <ellipse cx="50" cy="50" rx="46" ry="46" fill="none" stroke="#ffb347" strokeWidth="6" pathLength="100" strokeDasharray="1.2 11.3" strokeDashoffset="1.2" />
      </Part>
    </>
  ),

  Stack: () => (
    <>
      <span className="kw-part kw-anim kw-edge kw-edge--far" />
      <span className="kw-part kw-anim kw-edge kw-edge--near" />
    </>
  ),

  // Veil: over the art, clipped to the oval.
  "Can't attack": () => (
    <Part className="kw-chains" anim>
      <g transform="rotate(38 50 50)">
        <path d={CHAINS} fill="none" stroke="#0b0d12" strokeWidth="5" />
        <path d={CHAINS} fill="none" stroke="#d5d9e2" strokeWidth="2.6" />
      </g>
      <g transform="rotate(-38 50 50)">
        <path d={CHAINS} fill="none" stroke="#0b0d12" strokeWidth="5" />
        <path d={CHAINS} fill="none" stroke="#d5d9e2" strokeWidth="2.6" />
      </g>
      <circle cx="50" cy="50" r="7" fill="#3a3f4b" stroke="#0b0d12" strokeWidth="2" />
      <circle cx="50" cy="50" r="7" fill="none" stroke="#d5d9e2" strokeWidth="2" />
    </Part>
  ),

  Poisonous: () => (
    <>
      <Part className="kw-venom">
        <path
          d="M0 0 H100 V9 C94 14 90 8 84 12 C79 16 75 11 70 13 C64 16 60 10 54 13 C48 17 44 10 38 13 C32 16 28 11 22 14 C16 17 12 10 6 13 C3 15 1 12 0 11 Z"
          fill="#48d85a"
          stroke="#0e4a16"
          strokeWidth="2"
        />
      </Part>
      <Part className="kw-venom-drops" anim>
        <path d="M32 11 C35 18 39 23 39 28 A7 7 0 0 1 25 28 C25 23 29 18 32 11 Z" fill="#62ef72" stroke="#0e4a16" strokeWidth="2" />
        <path d="M68 12 C70 17 73 20 73 24 A5 5 0 0 1 63 24 C63 20 66 17 68 12 Z" fill="#62ef72" stroke="#0e4a16" strokeWidth="2" />
      </Part>
    </>
  ),

  Charge: () => (
    <>
      <Part className="kw-streaks" anim>
        <Inked d="M18 30 H74 M8 44 H62 M24 58 H80 M12 72 H58" ink="#ff7a45" width={3.4} />
      </Part>
      <Part className="kw-chevrons">
        <Inked d="M70 38 L82 50 L70 62 M82 38 L94 50 L82 62" ink="#ffd1b8" width={4.2} />
      </Part>
    </>
  ),

  Rush: () => (
    <Part className="kw-dashes" anim>
      <Inked d="M74 38 H94 M78 50 H98 M74 62 H94" ink="#ffbe55" width={4} />
    </Part>
  ),

  Brittle: () => (
    <Part className="kw-cracks" anim>
      <Inked className="kw-crack kw-crack-1" d="M74 14 L64 28 L68 36 L58 50 L62 58 L52 74" ink="#eaf5ff" width={1.3} />
      <Inked className="kw-crack kw-crack-2" d="M64 28 L50 32 L42 44 L28 46 M58 50 L44 58" ink="#eaf5ff" width={1.6} />
      <Inked className="kw-crack kw-crack-3" d="M62 58 L74 64 L78 78 L90 82 M42 44 L36 30 L20 26 M52 74 L40 86" ink="#eaf5ff" width={1.9} />
    </Part>
  ),

  Lifesteal: () => (
    <Part className="kw-veins" anim>
      <Inked
        d="M2 46 C14 44 18 52 28 50 C34 49 36 44 42 46 M28 50 C30 58 26 64 30 70 M98 42 C86 44 82 36 72 40 C66 42 64 48 58 46 M72 40 C70 32 74 26 70 20 M22 90 C28 80 36 82 40 74 M80 90 C74 82 66 84 62 76"
        ink="#ff2f52"
        width={2.2}
      />
    </Part>
  ),

  "Spell Damage": () => (
    <Part className="kw-rune-ring" anim fit="contain">
      <circle cx="50" cy="50" r="44" fill="none" stroke="#0b0d12" strokeWidth="6" />
      <circle cx="50" cy="50" r="44" fill="none" stroke="#9cc2ff" strokeWidth="3" />
      <circle cx="50" cy="50" r="33" fill="none" stroke="#9cc2ff" strokeWidth="1.4" strokeOpacity="0.8" />
      {[0, 60, 120, 180, 240, 300].map((angle) => (
        <path
          key={angle}
          d="M50 7 L54 13 L50 19 L46 13 Z"
          transform={`rotate(${angle} 50 50)`}
          fill="#dbe8ff"
          stroke="#0b0d12"
          strokeWidth="1.2"
        />
      ))}
    </Part>
  ),

  "Animated on your turn": () => (
    <>
      <Part className="kw-dial" fit="contain">
        <circle cx="50" cy="50" r="42" fill="rgb(44 30 8 / 0.55)" stroke="#0b0d12" strokeWidth="7" />
        <circle cx="50" cy="50" r="42" fill="none" stroke="#e2b04a" strokeWidth="4" />
        {Array.from({ length: 12 }, (_, i) => (
          <path key={i} d="M50 13 V21" transform={`rotate(${i * 30} 50 50)`} stroke="#f7dc98" strokeWidth={i % 3 === 0 ? 4 : 2} />
        ))}
      </Part>
      <Part className="kw-hand" anim fit="contain">
        <path d="M50 50 V20" stroke="#0b0d12" strokeWidth="7" strokeLinecap="round" />
        <path d="M50 50 V20" stroke="#fff3cf" strokeWidth="3.5" strokeLinecap="round" />
        <circle cx="50" cy="50" r="5" fill="#e2b04a" stroke="#0b0d12" strokeWidth="1.5" />
      </Part>
    </>
  ),

  Animated: () => (
    <>
      <Part className="kw-cog kw-cog--big" anim fit="contain">
        <path d={COG_8} fill="#d6a13f" stroke="#3d2805" strokeWidth="3" strokeLinejoin="round" />
        <circle cx="50" cy="50" r="15" fill="#3d2805" />
        <circle cx="50" cy="50" r="24" fill="none" stroke="#f6d38a" strokeWidth="2" />
      </Part>
      <Part className="kw-cog kw-cog--small" anim fit="contain">
        <path d={COG_6} fill="#c28b2c" stroke="#3d2805" strokeWidth="3.5" strokeLinejoin="round" />
        <circle cx="50" cy="50" r="13" fill="#3d2805" />
      </Part>
    </>
  ),

  // Glyphs: a shaped emblem on a dark disc.
  "First Strike": () => (
    <>
      <Part className="kw-glyph-art" fit="contain">
        <path d="M50 3 L65 22 L65 60 L35 60 L35 22 Z" fill="#f1f5fb" stroke="#0b0d12" strokeWidth="4" strokeLinejoin="round" />
        <path d="M50 12 V56" stroke="#8d99af" strokeWidth="4" />
        <path d="M20 60 H80 V72 H20 Z" fill="#f0bd4f" stroke="#0b0d12" strokeWidth="4" strokeLinejoin="round" />
        <path d="M43 72 H57 V96 H43 Z" fill="#8a5220" stroke="#0b0d12" strokeWidth="4" />
      </Part>
      <Part className="kw-glint" anim fit="contain">
        <path d={sparkle(62, 16, 13)} fill="#ffffff" stroke="#0b0d12" strokeWidth="1.5" />
      </Part>
    </>
  ),

  Trample: () => (
    <>
      <Part className="kw-shock" anim fit="contain">
        <circle cx="50" cy="50" r="44" fill="none" stroke="#ffd08a" strokeWidth="5" />
      </Part>
      <Part className="kw-glyph-art" fit="contain">
        <path
          d="M30 34 C30 16 70 16 70 34 L70 62 C70 71 61 73 57 66 L55 46 C55 39 45 39 45 46 L43 66 C39 73 30 71 30 62 Z"
          fill="#c98a45"
          stroke="#0b0d12"
          strokeWidth="3.5"
          strokeLinejoin="round"
        />
        <path d="M36 30 C40 24 60 24 64 30" fill="none" stroke="#f3c690" strokeWidth="2.5" strokeLinecap="round" />
      </Part>
    </>
  ),

  Cleave: () => (
    <Part className="kw-glyph-art kw-crescent" anim fit="contain">
      <path d="M10 82 A48 48 0 0 1 90 18 A62 62 0 0 0 10 82 Z" fill="#dccfff" stroke="#0b0d12" strokeWidth="4" strokeLinejoin="round" />
      <path d="M22 66 A40 40 0 0 1 72 22" fill="none" stroke="#ffffff" strokeWidth="3" strokeLinecap="round" />
    </Part>
  ),

  Pierce: () => (
    <Part className="kw-glyph-art kw-spear" anim fit="contain">
      <path d="M50 4 L64 34 L50 46 L36 34 Z" fill="#9fe9ee" stroke="#0b0d12" strokeWidth="3.5" strokeLinejoin="round" />
      <path d="M50 10 V40" stroke="#e6fdff" strokeWidth="2.5" />
      <path d="M43 46 H57 V52 H43 Z" fill="#e2b04a" stroke="#0b0d12" strokeWidth="2.5" />
      <path d="M46 52 H54 V96 H46 Z" fill="#8a5a2b" stroke="#0b0d12" strokeWidth="2.5" />
    </Part>
  ),

  Windfury: () => (
    <Part className="kw-glyph-art kw-gust" anim fit="contain">
      <path d="M8 36 H62 C84 36 84 12 66 14" fill="none" stroke="#0b0d12" strokeWidth="9" strokeLinecap="round" />
      <path d="M8 36 H62 C84 36 84 12 66 14" fill="none" stroke="#bfe8ff" strokeWidth="4.5" strokeLinecap="round" />
      <path d="M8 66 H70 C94 66 94 90 74 88" fill="none" stroke="#0b0d12" strokeWidth="9" strokeLinecap="round" />
      <path d="M8 66 H70 C94 66 94 90 74 88" fill="none" stroke="#bfe8ff" strokeWidth="4.5" strokeLinecap="round" />
    </Part>
  ),

  Temporary: () => (
    <>
      <Part className="kw-glyph-art" fit="contain">
        <path d="M24 8 H76 L76 20 L56 50 L76 80 L76 92 H24 L24 80 L44 50 L24 20 Z" fill="#f3e3b4" stroke="#0b0d12" strokeWidth="4" strokeLinejoin="round" />
        <path d="M34 20 H66 L50 44 Z" fill="#d9a441" />
      </Part>
      <Part className="kw-sand" anim fit="contain">
        <path d="M50 56 V84" stroke="#d9a441" strokeWidth="4" strokeLinecap="round" />
        <path d="M34 84 H66 L50 66 Z" fill="#d9a441" />
      </Part>
    </>
  ),

  Deft: () => (
    <Part className="kw-glyph-art kw-dart" anim fit="contain">
      <path d="M22 74 L66 22 L78 22 L78 34 L34 78 Z" fill="#e8b4e0" stroke="#0b0d12" strokeWidth="3.5" strokeLinejoin="round" />
      <path d="M30 62 L58 34" stroke="#fdf3fb" strokeWidth="3" strokeLinecap="round" />
    </Part>
  ),

  Untributable: () => (
    <Part className="kw-glyph-art kw-anchor" anim fit="contain">
      <path d="M50 14 L50 78 M28 34 L72 34" stroke="#0b0d12" strokeWidth="8" strokeLinecap="round" />
      <path d="M50 14 L50 78 M28 34 L72 34" stroke="#c9b88a" strokeWidth="4" strokeLinecap="round" />
      <path d="M26 56 C26 76 38 86 50 86 C62 86 74 76 74 56" fill="none" stroke="#c9b88a" strokeWidth="4" strokeLinecap="round" />
    </Part>
  ),

  "Immune to tribal tag based hate": () => (
    <Part className="kw-glyph-art kw-tribal" anim fit="contain">
      <circle cx="50" cy="50" r="30" fill="none" stroke="#0b0d12" strokeWidth="6" />
      <circle cx="50" cy="50" r="30" fill="none" stroke="#7fb6dd" strokeWidth="3" />
      <circle cx="50" cy="50" r="18" fill="none" stroke="#7fb6dd" strokeWidth="2" strokeOpacity="0.7" />
    </Part>
  ),

  Lucky: () => (
    <>
      <Part className="kw-glyph-art" fit="contain">
        <path d="M50 58 C52 72 58 82 70 90" fill="none" stroke="#0b0d12" strokeWidth="7" strokeLinecap="round" />
        <path d="M50 58 C52 72 58 82 70 90" fill="none" stroke="#2f9e55" strokeWidth="3.5" strokeLinecap="round" />
        {[
          [36, 34],
          [64, 34],
          [36, 60],
          [64, 60],
        ].map(([x = 0, y = 0]) => (
          <circle key={`${x}-${y}`} cx={x} cy={y} r="15" fill="#48d17a" stroke="#0b0d12" strokeWidth="3" />
        ))}
        <circle cx="50" cy="47" r="5" fill="#2f9e55" />
      </Part>
      <Part className="kw-sparkle" anim fit="contain">
        <path d={sparkle(84, 14, 12)} fill="#fffbd0" stroke="#0b0d12" strokeWidth="1.5" />
      </Part>
    </>
  ),

  Immutable: () => (
    <>
      <Part className="kw-glyph-art" fit="contain">
        <path d="M34 48 V34 A16 16 0 0 1 66 34 V48" fill="none" stroke="#0b0d12" strokeWidth="12" />
        <path d="M34 48 V34 A16 16 0 0 1 66 34 V48" fill="none" stroke="#c9d3e6" strokeWidth="6" />
        <rect x="24" y="46" width="52" height="44" rx="7" fill="#aeb9cf" stroke="#0b0d12" strokeWidth="3.5" />
        <path d="M50 60 A5 5 0 0 1 53 69 L55 80 H45 L47 69 A5 5 0 0 1 50 60 Z" fill="#20252f" />
      </Part>
      <Part className="kw-seal" anim fit="contain">
        <path d="M30 86 L44 50 L52 50 L38 86 Z" fill="#ffffff" />
      </Part>
    </>
  ),
};

function isDrawnHere(kind: KeywordKind): kind is Exclude<KeywordKind, "Divine Shield" | "Armor"> {
  return !DRAWN_BY_MINION.includes(kind);
}

function Treatment({ entry }: { entry: KeywordFxPlan }): ReactElement | null {
  const kind = entry.kind;
  if (!isDrawnHere(kind)) return null;
  return (
    <span className="kw-fx" {...keywordFxAttributes(entry)}>
      {DRAW[kind]()}
    </span>
  );
}

/**
 * The frame, veil and glyph treatments of one unit, in priority order: frames and veils as the
 * portrait's neighbours, glyphs in their row. `plan` is `keywordFxPlan(unit)`.
 */
export function KeywordFx({ plan }: { plan: readonly KeywordFxPlan[] }): ReactElement | null {
  const drawn = plan.filter((entry) => isDrawnHere(entry.kind));
  if (drawn.length === 0) return null;
  const glyphs = drawn.filter((entry) => entry.visual.layer === "glyph");
  return (
    <>
      {drawn
        .filter((entry) => entry.visual.layer !== "glyph")
        .map((entry) => (
          <Treatment key={entry.kind} entry={entry} />
        ))}
      {glyphs.length > 0 && (
        <span className="kw-glyphs">
          {glyphs.map((entry) => (
            <Treatment key={entry.kind} entry={entry} />
          ))}
        </span>
      )}
    </>
  );
}
