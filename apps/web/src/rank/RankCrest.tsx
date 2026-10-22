// A rank drawn as a crest (#1650; SPEC §9.12, R605, R607, R608, R612): an inline SVG for Raisin, each
// Grape tier at each division and Jlorious, so a rank reads at a glance wherever it shows. Like
// `game/LockIcon.tsx` and `game/Clock.tsx` it draws its own art, so no raster asset ships.
//
// Presentation only (CLAUDE.md rule 7). It takes a `VisibleRank`, a `PeakBadge`, or a tier and a
// division (the ladder's headings and rows, which carry no pips), reads nothing else and never sees
// a rating (R612). The crest is an `img` named and titled with `rankWords` or `badgeWords`, so a
// screen reader reads what it read before the art.
//
// The tier art is one table (`TIER_ART`). A division adds the same embellishment to every Grape tier,
// one function (`divisionEmbellishment`): a stud at the crest's base per step up, and an inner
// frame line for each step after the first, so a division reads the same in every tier. Pips and
// placements are a pip bar beside the crest, and Jlorious's position is printed beside it. Its
// shimmer is the one animation: it is left out under Reduce motion, panel or OS (rankCrest.css).

import type { ReactElement } from "react";

import { useOsReducedMotion } from "../game/useOsReducedMotion.ts";
import { t } from "../i18n/i18n.ts";
import type { GrapeTier, PeakBadge, VisibleRank } from "../net/api.ts";
import { useSetting } from "../settings/store.ts";
import { badgeWords, divisionNumeral, rankWords, tierName } from "./rank.ts";

import "./rankCrest.css";

export type RankCrestSize = "small" | "large";

/** A crest's edge in CSS px: small for rows and the phone's rank line, large for the profile, Play and the ladder's own standing. Large stays 24 so the Play hub fits one screen (R1578000). */
export const CREST_SIZE_PX: Readonly<Record<RankCrestSize, number>> = { small: 16, large: 24 };

export type RankCrestProps =
  | { rank: VisibleRank; size: RankCrestSize }
  | { badge: PeakBadge; size: RankCrestSize }
  // The ladder's headings and rows, which carry no pipsPerDivision.
  | { tier: GrapeTier; division?: number; size: RankCrestSize };

/** Studs per Grape division: one more at each step from III to I, in every tier. */
const DIVISION_STUDS: Readonly<Record<number, number>> = { 3: 1, 2: 2, 1: 3 };

/** The one division rule (#1650): the studs, and one inner frame line fewer. */
export function divisionEmbellishment(division: number | null): { studs: number; frameLines: number } {
  const studs = division === null ? 0 : (DIVISION_STUDS[division] ?? 0);
  return { studs, frameLines: Math.max(0, studs - 1) };
}

/** The crest's drawing space: every coordinate below is in it, whatever size the crest is drawn at. */
const VIEW_BOX = "0 0 64 64";
const CENTRE = 32;
const SHIELD = "M32 6 L54 14 V32 C54 46 44 54 32 58 C20 54 10 46 10 32 V14 Z";

/** Berry centres, rows from the point upward: 1, 2, 3 and 4 berries. */
const BERRY_ROWS: readonly (readonly (readonly [number, number])[])[] = [
  [[32, 42]],
  [
    [27, 34],
    [37, 34],
  ],
  [
    [22, 26],
    [32, 26],
    [42, 26],
  ],
  [
    [17, 18],
    [27, 18],
    [37, 18],
    [47, 18],
  ],
];

const STUD_Y = 51;
const STUD_GAP = 6;
const STUD_R = 2;
const FRAME_LINE_STEP = 0.1;
const GLOW_R = 30;
const GLOW_OPACITY = 0.35;
const GLOSS_OFFSET = 1.5;
const GLOSS_R = 1.2;
const GLOSS_OPACITY = 0.7;
const WING_OPACITY = 0.9;
const RAY_WIDTH = 1.5;
const VINE_WIDTH = 1.5;
const FRAME_LINE_WIDTH = 1;
const HALO_COLOUR = "#f3e8ff";
const LEAF_GREEN = "#5f8f3a";
const LEAVES = [
  { cx: 25, cy: 11, rx: 6, ry: 2.5, transform: "rotate(-25 25 11)" },
  { cx: 39, cy: 11, rx: 6, ry: 2.5, transform: "rotate(25 39 11)" },
];
const VINES = ["M12 50 C6 40 6 24 12 16", "M52 50 C58 40 58 24 52 16"];
const WINGS: Readonly<Record<"none" | "small" | "full", readonly string[]>> = {
  none: [],
  small: ["M10 20 C2 16 1 30 4 36 C6 31 8 29 10 31 Z", "M54 20 C62 16 63 30 60 36 C58 31 56 29 54 31 Z"],
  full: ["M10 16 C1 10 0 34 2 44 C5 37 7 35 10 37 Z", "M54 16 C63 10 64 34 62 44 C59 37 57 35 54 37 Z"],
};
const CROWN = "M22 9 L24 1 L28 6 L32 0 L36 6 L40 1 L42 9 Z";
const RAY_COUNT = 8;
const RAY_LENGTH = 31;
const RAYS = Array.from({ length: RAY_COUNT }, (_, i) => (i * 2 * Math.PI) / RAY_COUNT).map(
  (angle) => [CENTRE + RAY_LENGTH * Math.cos(angle), CENTRE + RAY_LENGTH * Math.sin(angle)] as const,
);

type TierArt = {
  frame: string;
  edge: string;
  edgeWidth: number;
  dash: string | undefined;
  grape: string;
  berry: number;
  /** How many of `BERRY_ROWS`, from the point up, the cluster fills. */
  rows: number;
  opacity: number;
  leaves: boolean;
  vine: boolean;
  gloss: boolean;
  halo: boolean;
  wings: keyof typeof WINGS;
  glow: string | null;
  crown: boolean;
  rays: boolean;
  shimmer: boolean;
};

/** The tier art's one place: each tier a step up from the one below (#1650). */
const TIER_ART: Readonly<Record<VisibleRank["tier"], TierArt>> = {
  raisin: {
    frame: "none",
    edge: "#6b5a4e",
    edgeWidth: 1.5,
    dash: "3 3",
    grape: "#4a3728",
    berry: 3.5,
    rows: 2,
    opacity: 0.6,
    leaves: false,
    vine: false,
    gloss: false,
    halo: false,
    wings: "none",
    glow: null,
    crown: false,
    rays: false,
    shimmer: false,
  },
  rotten: {
    frame: "#3b3530",
    edge: "#6e6259",
    edgeWidth: 1.5,
    dash: "14 3",
    grape: "#5e4b5a",
    berry: 4.5,
    rows: 2,
    opacity: 1,
    leaves: false,
    vine: false,
    gloss: false,
    halo: false,
    wings: "none",
    glow: null,
    crown: false,
    rays: false,
    shimmer: false,
  },
  normal: {
    frame: "#2f2a4a",
    edge: "#9aa0b5",
    edgeWidth: 2,
    dash: undefined,
    grape: "#7b4fa0",
    berry: 4.5,
    rows: 3,
    opacity: 1,
    leaves: false,
    vine: false,
    gloss: false,
    halo: false,
    wings: "none",
    glow: null,
    crown: false,
    rays: false,
    shimmer: false,
  },
  large: {
    frame: "#2c2448",
    edge: "#c0c6d8",
    edgeWidth: 2.5,
    dash: undefined,
    grape: "#8a4fc0",
    berry: 4.5,
    rows: 4,
    opacity: 1,
    leaves: true,
    vine: false,
    gloss: false,
    halo: false,
    wings: "none",
    glow: null,
    crown: false,
    rays: false,
    shimmer: false,
  },
  golden: {
    frame: "#3a2a10",
    edge: "#e8b931",
    edgeWidth: 3,
    dash: undefined,
    grape: "#9b4fd0",
    berry: 4.5,
    rows: 4,
    opacity: 1,
    leaves: true,
    vine: true,
    gloss: true,
    halo: false,
    wings: "none",
    glow: null,
    crown: false,
    rays: false,
    shimmer: false,
  },
  mythic: {
    frame: "#2a1a45",
    edge: "#c9a6ff",
    edgeWidth: 3,
    dash: undefined,
    grape: "#b06cff",
    berry: 4.5,
    rows: 4,
    opacity: 1,
    leaves: true,
    vine: true,
    gloss: true,
    halo: true,
    wings: "small",
    glow: "#b06cff",
    crown: false,
    rays: false,
    shimmer: false,
  },
  jlorious: {
    frame: "#3a2208",
    edge: "#ffd76a",
    edgeWidth: 3.5,
    dash: undefined,
    grape: "#d9a6ff",
    berry: 4.5,
    rows: 4,
    opacity: 1,
    leaves: true,
    vine: true,
    gloss: true,
    halo: false,
    wings: "full",
    glow: "#ffd76a",
    crown: true,
    rays: true,
    shimmer: true,
  },
};

type CrestFace = {
  tier: VisibleRank["tier"];
  division: number | null;
  position: number | null;
  progress: { filled: number; total: number } | null;
  words: string;
};

/** What a crest says: its tier, division or position, the pip bar's fill, and the words it is named with. */
function crestFace(props: RankCrestProps): CrestFace {
  if ("rank" in props) {
    const { rank } = props;
    const words = rankWords(rank);
    if (rank.tier === "raisin") {
      return { tier: rank.tier, division: null, position: null, progress: { filled: rank.placementsPlayed, total: rank.placementGames }, words };
    }
    if (rank.tier === "jlorious") return { tier: rank.tier, division: null, position: rank.position, progress: null, words };
    return { tier: rank.tier, division: rank.division, position: null, progress: { filled: rank.pips, total: rank.pipsPerDivision }, words };
  }
  if ("badge" in props) {
    const { badge } = props;
    const words = badgeWords(badge);
    if (badge.tier === "jlorious") return { tier: badge.tier, division: null, position: badge.position, progress: null, words };
    return { tier: badge.tier, division: badge.division, position: null, progress: null, words };
  }
  const division = props.division ?? null;
  const words = division === null ? tierName(props.tier) : `${tierName(props.tier)} ${divisionNumeral(division)}`;
  return { tier: props.tier, division, position: null, progress: null, words };
}

export default function RankCrest(props: RankCrestProps): ReactElement {
  const panelReduces = useSetting("reduceMotion");
  const osReduces = useOsReducedMotion();
  const reduced = panelReduces || osReduces;
  const { tier, division, position, progress, words } = crestFace(props);
  // match.tsx only checks that a tier is a string, so an unknown tier from a newer server draws as
  // Raisin instead of crashing.
  const art = TIER_ART[tier] ?? TIER_ART.raisin;
  const { studs, frameLines } = divisionEmbellishment(division);
  const berries = BERRY_ROWS.slice(0, art.rows).flat();
  const edge = CREST_SIZE_PX[props.size];

  return (
    <span
      className="rank-crest"
      role="img"
      aria-label={words}
      title={words}
      data-size={props.size}
      data-tier={tier}
      data-division={division ?? undefined}
      data-motion={reduced ? "reduced" : "full"}
    >
      <svg className="rank-crest__art" viewBox={VIEW_BOX} width={edge} height={edge} aria-hidden="true" focusable="false">
        <g opacity={art.opacity}>
          {art.glow !== null ? <circle cx={CENTRE} cy={CENTRE} r={GLOW_R} fill={art.glow} opacity={GLOW_OPACITY} /> : null}
          {art.rays
            ? RAYS.map(([x, y], i) => <line key={i} x1={CENTRE} y1={CENTRE} x2={x} y2={y} stroke={art.edge} strokeWidth={RAY_WIDTH} />)
            : null}
          {WINGS[art.wings].map((d, i) => (
            <path key={i} d={d} fill={art.edge} opacity={WING_OPACITY} />
          ))}
          <path d={SHIELD} fill={art.frame} stroke={art.edge} strokeWidth={art.edgeWidth} strokeDasharray={art.dash} />
          {Array.from({ length: frameLines }, (_, i) => (
            <path
              key={i}
              className="rank-crest__frame-line"
              d={SHIELD}
              fill="none"
              stroke={art.edge}
              strokeWidth={FRAME_LINE_WIDTH}
              transform={`translate(${String(CENTRE)} ${String(CENTRE)}) scale(${String(1 - FRAME_LINE_STEP * (i + 1))}) translate(-${String(CENTRE)} -${String(CENTRE)})`}
            />
          ))}
          {art.vine
            ? VINES.map((d, i) => <path key={i} d={d} fill="none" stroke={art.edge} strokeWidth={VINE_WIDTH} />)
            : null}
          {art.leaves ? LEAVES.map((leaf, i) => <ellipse key={i} {...leaf} fill={LEAF_GREEN} />) : null}
          {berries.map(([x, y], i) => (
            <g key={i}>
              <circle cx={x} cy={y} r={art.berry} fill={art.grape} />
              {art.gloss ? (
                <circle cx={x - GLOSS_OFFSET} cy={y - GLOSS_OFFSET} r={GLOSS_R} fill="#ffffff" opacity={GLOSS_OPACITY} />
              ) : null}
            </g>
          ))}
          {Array.from({ length: studs }, (_, i) => (
            <circle
              key={i}
              className="rank-crest__stud"
              cx={CENTRE + (i - (studs - 1) / 2) * STUD_GAP}
              cy={STUD_Y}
              r={STUD_R}
              fill={art.edge}
            />
          ))}
          {art.halo ? <ellipse cx={CENTRE} cy={3.5} rx={11} ry={2.5} fill="none" stroke={HALO_COLOUR} strokeWidth={1.5} /> : null}
          {art.crown ? <path d={CROWN} fill={art.edge} /> : null}
          {art.shimmer && !reduced ? <path className="rank-crest__shimmer" d={SHIELD} fill="#ffffff" /> : null}
        </g>
      </svg>
      {position !== null ? (
        <span className="rank-crest__position" aria-hidden="true">
          {t("#{position}", { position: String(position) })}
        </span>
      ) : null}
      {progress !== null ? (
        <span className="rank-crest__pips" aria-hidden="true">
          {Array.from({ length: progress.total }, (_, i) => (
            <span key={i} className="rank-crest__pip" data-filled={i < progress.filled ? "true" : "false"} />
          ))}
        </span>
      ) : null}
    </span>
  );
}
