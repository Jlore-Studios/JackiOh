// R2787000: the standing looks a card's `aura` may wear (docs/polish/9-card-animations.md, Auras), and
// the scopes it may cover. Each look is a tint, a slow drift of motes and an edge glow, drawn in the
// card's palette (FLAVOR_LOOKS); these numbers are the look's motion and weight (CLAUDE.md rule 9).
// R3400000: a Unit's aura may stand on its own `tile`, its unit zone, the nearest scope; and three looks
// join for the Units whose signatures wear one there: Bob's snow `flurry`, Tutorial Master's `chalk`
// dust and the Training-Grounds Golem's `timber` dust. R3405000: five more for Meditative's signatures:
// Introspection's slow `ripple`, the `orbit` of a ring circling its card (keys, brushes, a coin), a bronze
// cauldron's or a jade mist's `steam`, Devin Bot's rising `incense` of hologram lines and Yile's Virus's `glitch`.
// R3389000: a signature's own looks beside the shared seventeen, each worn by one signature alone (the
// loader fails one on any other entry, tables.ts) and each within AURA_SIGNATURE_MAX_INTENSITY.
// R3399001: eighteen more join them, one of each Classic+ signature permanent's own (#3399), each moving as its
// card's concept does (syrup creeping down, steam rising, a banner in a breeze). R3402000: Core's three join
// them: Silly Silas's `mirrorball`, Combo-Index's `stadium` and Heroic Power's `pedestal`.
// R3407000: fifteen more join them, each one Stinkyscape card's own "on its tile" (part two).
// R3395001: three join them for Stinkyscape's signature Field Spells: The Greatest Heist's sweeping green
// `laserGrid`, Rod of Ages' rising `silverRise` and The Narcor Project's slow `labHaze`.

/** The seventeen looks, as a card's table entry or an Anomaly's backdrop (`fx/anomaly.ts`, R3337003) names them. */
export const AURA_LOOK_NAMES = [
  "ember",
  "frost",
  "bloom",
  "void",
  "storm",
  "holy",
  "toxic",
  "arcane",
  "drift",
  "flurry",
  "chalk",
  "timber",
  "ripple",
  "orbit",
  "steam",
  "incense",
  "glitch",
] as const;

/** What an aura covers, nearest first: a Unit's own tile, its card's own zone, its lane, its side or the whole board. The index is its stacking rank. */
export const AURA_SCOPES = ["tile", "zone", "lane", "side", "board"] as const;

/** The card types that stand in a zone while they hold, so the only ones whose aura may cover their own zone or lane. */
export const AURA_FIELD_TYPES = ["Field Spell", "Field Trap"] as const;

/**
 * R3389000: the looks of Classic's signatures (#3389), each its card's own: Palantir's watching iris,
 * InfiniScepter's loop, Second Wind's breeze over the graveyard, Joro's web, Nature Titan's moss, Spell
 * Tyrant's crown-sigil, BOOM! Big Max's crater smoke, King Wagtoggle's cloak and In Too Deep's whirlpool.
 * R3395001: Stinkyscape's three signature Field Spells', The Greatest Heist's laser grid, Rod of Ages' silver
 * bands and The Narcor Project's lab haze.
 * R3402000: then Core's (#3402): Silly Silas's mirrorball, Combo-Index's stadium and Heroic Power's pedestal.
 */
export const AURA_SIGNATURE_LOOK_NAMES = [
  "seeingIris",
  "sceptreLoop",
  "graveBreeze",
  "silkWeb",
  "mossCreep",
  "crownSigil",
  "craterSmoke",
  "cloakShimmer",
  "abyssWhirl",
  // R3399001: Classic+'s (#3399), one per signature permanent, in catalog order: The Mother Pancake's syrup,
  // Anti-Waffle Shell's dome, Frostspatula's mist, Mommy Barker's oven steam, the five Losers' (Top's lane wall,
  // Jungle's snoring leaves, Mid's coin, Support's staff, Bot's bow), Wardrum's heartbeat, KY's Gift's apple and
  // ribbons, the two Flagbearers' banners, the Lobbyist's laser dot, Classic Golem's stone faces, Twice Forward's
  // stair cracks, J-lease's leaves and Claude's Datacenter's server lights.
  "syrup",
  "waffleDome",
  "frostMist",
  "ovenSteam",
  "laneWall",
  "snoreLeaves",
  "coinGlint",
  "staffBreath",
  "bowNotch",
  "warHeartbeat",
  "appleRibbon",
  "bannerBreeze",
  "greatBannerShade",
  "laserDot",
  "stoneFaces",
  "stairCracks",
  "jungleLeaves",
  "serverLights",
  // R3402000: Core's three.
  "mirrorball",
  "stadium",
  "pedestal",
  // R3407000: Stinkyscape part two's: the MAGI's scan line, heartbeat and glint, xxFreshJack's pixel crawl,
  // iKwzki's halo, Shockwave447's arcs, Job Application's step lights, Scornboi's searchlight, Megabase's
  // rings, IN33DH3's wisp, Prime's crown, the Wardrum Blob's drumbeat, Alteran Reach's star path, the
  // Stinkyscape Golem's vapour and Sanctum's pages.
  "scanline",
  "heartbeat",
  "glint",
  "pixelcrawl",
  "duohalo",
  "arcflick",
  "steplights",
  "searchlight",
  "stationrings",
  "aiwisp",
  "crownpulse",
  "drumbeat",
  "starpath",
  "stinkvapour",
  "pages",
  // R3395001: Stinkyscape 001 to 090's three Field Spells'.
  "laserGrid",
  "silverRise",
  "labHaze",
] as const;

/** A shared look, any entry's to wear. */
export type AuraSharedLook = (typeof AURA_LOOK_NAMES)[number];
/** R3389000: a signature's own look. */
export type AuraSignatureLook = (typeof AURA_SIGNATURE_LOOK_NAMES)[number];
export type AuraLook = AuraSharedLook | AuraSignatureLook;
export type AuraScope = (typeof AURA_SCOPES)[number];

export function isAuraLook(name: string): name is AuraLook {
  return (AURA_LOOK_NAMES as readonly string[]).includes(name) || isSignatureAuraLook(name);
}

/** R3389000: whether `name` is a signature's own look. */
export function isSignatureAuraLook(name: string): name is AuraSignatureLook {
  return (AURA_SIGNATURE_LOOK_NAMES as readonly string[]).includes(name);
}

export function isAuraScope(name: string): name is AuraScope {
  return (AURA_SCOPES as readonly string[]).includes(name);
}

export function isAuraFieldType(type: string): boolean {
  return (AURA_FIELD_TYPES as readonly string[]).includes(type);
}

/**
 * One look. `vx` and `vy` are a mote's velocity range in px/s (a negative `vy` rises), `size` its
 * diameter and `glow` the edge glow's spread in px, `life` a mote's lifetime in ms; `alpha` (a mote's
 * peak), `tint` (the body's fill) and `edge` (the glow) are opacities; `blend` is how motes composite.
 */
export type AuraLookSpec = {
  vx: readonly [number, number];
  vy: readonly [number, number];
  size: readonly [number, number];
  life: readonly [number, number];
  alpha: number;
  tint: number;
  glow: number;
  edge: number;
  blend: "lighter" | "source-over";
};

const L = "lighter";
const S = "source-over";

export const AURA_LOOKS: Readonly<Record<AuraSharedLook, AuraLookSpec>> = {
  ember: { vx: [-6, 6], vy: [-28, -12], size: [2, 4], life: [1800, 3200], alpha: 0.8, tint: 0.1, glow: 14, edge: 0.55, blend: L },
  frost: { vx: [-5, 5], vy: [6, 16], size: [2, 3.5], life: [2400, 4000], alpha: 0.7, tint: 0.12, glow: 10, edge: 0.6, blend: L },
  bloom: { vx: [-8, 8], vy: [-10, -2], size: [3, 5], life: [2600, 4200], alpha: 0.6, tint: 0.1, glow: 12, edge: 0.45, blend: S },
  void: { vx: [-10, 10], vy: [-10, 10], size: [3, 6], life: [2000, 3600], alpha: 0.55, tint: 0.16, glow: 18, edge: 0.5, blend: S },
  storm: { vx: [24, 48], vy: [-4, 4], size: [1.5, 3], life: [900, 1600], alpha: 0.75, tint: 0.08, glow: 12, edge: 0.65, blend: L },
  holy: { vx: [-4, 4], vy: [-18, -6], size: [2, 4], life: [2200, 3800], alpha: 0.75, tint: 0.1, glow: 16, edge: 0.6, blend: L },
  toxic: { vx: [-4, 4], vy: [-14, -4], size: [3, 6], life: [2000, 3400], alpha: 0.6, tint: 0.13, glow: 12, edge: 0.5, blend: S },
  arcane: { vx: [-12, 12], vy: [-12, 12], size: [2, 3.5], life: [1600, 3000], alpha: 0.8, tint: 0.09, glow: 14, edge: 0.6, blend: L },
  // R3337003: an asteroid field's slow sideways drift.
  drift: { vx: [4, 10], vy: [-3, 3], size: [3, 7], life: [3600, 6000], alpha: 0.5, tint: 0.06, glow: 10, edge: 0.35, blend: S },
  flurry: { vx: [-8, 8], vy: [4, 12], size: [1.5, 3], life: [2600, 4200], alpha: 0.55, tint: 0.06, glow: 8, edge: 0.35, blend: L },
  chalk: { vx: [-6, 6], vy: [-8, -2], size: [1.5, 3], life: [2400, 4000], alpha: 0.45, tint: 0.05, glow: 6, edge: 0.3, blend: S },
  timber: { vx: [-3, 3], vy: [6, 14], size: [1.5, 3], life: [2000, 3400], alpha: 0.4, tint: 0.07, glow: 8, edge: 0.4, blend: S },
  // R3405000: Meditative's signatures'.
  ripple: { vx: [-3, 3], vy: [-3, 3], size: [4, 7], life: [3200, 5200], alpha: 0.4, tint: 0.06, glow: 12, edge: 0.4, blend: L },
  orbit: { vx: [10, 18], vy: [-2, 2], size: [1.5, 3], life: [2800, 4400], alpha: 0.6, tint: 0.05, glow: 8, edge: 0.35, blend: L },
  steam: { vx: [-4, 4], vy: [-12, -4], size: [4, 8], life: [2600, 4200], alpha: 0.35, tint: 0.07, glow: 10, edge: 0.35, blend: S },
  incense: { vx: [-1, 1], vy: [-16, -8], size: [1, 2], life: [1800, 3000], alpha: 0.5, tint: 0.05, glow: 6, edge: 0.4, blend: L },
  glitch: { vx: [-24, 24], vy: [-6, 6], size: [1.5, 3], life: [500, 900], alpha: 0.6, tint: 0.06, glow: 6, edge: 0.45, blend: L },
};

/**
 * R3389000: each signature look's motion and weight, every opacity within AURA_SIGNATURE_MAX_INTENSITY
 * (signatureAuras.test.ts), its motion after its card's concept.
 */
export const AURA_SIGNATURE_LOOKS: Readonly<Record<AuraSignatureLook, AuraLookSpec>> = {
  // Palantir: a slow cyan iris drifting to watch the other side, blinking now and then.
  seeingIris: { vx: [-4, 4], vy: [-4, 4], size: [2, 3], life: [3000, 4600], alpha: 0.6, tint: 0.08, glow: 12, edge: 0.5, blend: L },
  // InfiniScepter: the held Spell turning in its loop, a figure-eight trail circling it.
  sceptreLoop: { vx: [-6, 6], vy: [-6, 6], size: [2, 3.5], life: [2200, 3600], alpha: 0.7, tint: 0.1, glow: 14, edge: 0.55, blend: L },
  // Second Wind: a soft breeze ruffling the graveyard, leaf-motes lifting sideways off it.
  graveBreeze: { vx: [10, 22], vy: [-6, -1], size: [2.5, 4.5], life: [1600, 2800], alpha: 0.55, tint: 0.06, glow: 10, edge: 0.4, blend: S },
  // Joro: a faint web in the zone's corners, a thread swaying above him.
  silkWeb: { vx: [-2, 2], vy: [2, 8], size: [1, 2], life: [2800, 4400], alpha: 0.5, tint: 0.05, glow: 8, edge: 0.35, blend: L },
  // Nature Titan: moss creeping slowly over its frame, a petal drifting now and then.
  mossCreep: { vx: [-3, 3], vy: [-8, -2], size: [3, 5.5], life: [3200, 5000], alpha: 0.55, tint: 0.12, glow: 10, edge: 0.45, blend: S },
  // Spell Tyrant: a small violet crown-sigil turning slowly above him.
  crownSigil: { vx: [-6, 6], vy: [-9, -3], size: [2, 3], life: [2000, 3200], alpha: 0.75, tint: 0.09, glow: 16, edge: 0.6, blend: L },
  // BOOM! Big Max: smoke curling up off the crater's rim, embers ticking round his fists.
  craterSmoke: { vx: [-5, 5], vy: [-22, -8], size: [3, 6], life: [2000, 3400], alpha: 0.5, tint: 0.1, glow: 14, edge: 0.5, blend: S },
  // King Wagtoggle: a slow crimson shimmer running along his cloak's hem.
  cloakShimmer: { vx: [8, 16], vy: [-2, 2], size: [1.5, 3], life: [1400, 2400], alpha: 0.65, tint: 0.07, glow: 12, edge: 0.5, blend: L },
  // In Too Deep: a slow whirlpool of black-blue water, a faint light at its bottom.
  abyssWhirl: { vx: [-14, 14], vy: [-14, 14], size: [3, 6], life: [2400, 4000], alpha: 0.5, tint: 0.16, glow: 18, edge: 0.55, blend: S },
  // R3399001: Classic+'s, in catalog order.
  // The Mother Pancake: syrup creeping slowly down her sides, a thin curl of steam among it.
  syrup: { vx: [-3, 3], vy: [4, 10], size: [3, 5], life: [3000, 4600], alpha: 0.6, tint: 0.12, glow: 10, edge: 0.45, blend: S },
  // Anti-Waffle Shell: a slow pale blue waffle-grid shimmer over its side, its cells lighting in turn.
  waffleDome: { vx: [-1, 1], vy: [-1, 1], size: [3, 4.5], life: [2200, 3400], alpha: 0.6, tint: 0.1, glow: 16, edge: 0.65, blend: L },
  // Frostspatula: a thin cold mist curling off its blade, frost creeping out and back across its griddle.
  frostMist: { vx: [-7, 7], vy: [2, 8], size: [3, 6], life: [2800, 4400], alpha: 0.5, tint: 0.14, glow: 14, edge: 0.55, blend: S },
  // Mommy Barker: a flour mote off her apron and a wisp of oven steam rising behind her.
  ovenSteam: { vx: [-5, 5], vy: [-16, -8], size: [2.5, 4.5], life: [2400, 3800], alpha: 0.55, tint: 0.08, glow: 12, edge: 0.4, blend: S },
  // Top Loser: a faint grey lane wall around his zone, his cape stirring in a lonely wind.
  laneWall: { vx: [-2, 2], vy: [-24, -14], size: [1.5, 2.5], life: [1400, 2400], alpha: 0.45, tint: 0.16, glow: 20, edge: 0.75, blend: S },
  // Jungle Loser: slow rustling leaves around him and a snore bubble that swells and pops.
  snoreLeaves: { vx: [-4, 4], vy: [-8, -2], size: [4, 7], life: [3200, 5000], alpha: 0.5, tint: 0.08, glow: 10, edge: 0.4, blend: S },
  // Mid Loser: his coin rolling over his knuckles, half his aura bright and half dull.
  coinGlint: { vx: [-14, 14], vy: [-6, 0], size: [1.5, 3], life: [1200, 2200], alpha: 0.8, tint: 0.08, glow: 12, edge: 0.55, blend: L },
  // Support Loser: his Divine Shield bubble breathing slowly, a faint white glow on his staff.
  staffBreath: { vx: [-3, 3], vy: [-10, -4], size: [3, 5], life: [3400, 5200], alpha: 0.6, tint: 0.11, glow: 18, edge: 0.6, blend: L },
  // Bot Loser: notches glowing on his bow and a cocky flex that loops slowly.
  bowNotch: { vx: [-6, 6], vy: [-14, -6], size: [1.5, 3], life: [1600, 2600], alpha: 0.75, tint: 0.1, glow: 12, edge: 0.6, blend: L },
  // Wardrum: a slow red-orange heartbeat ring from his chest, motes thrown outward on each beat.
  warHeartbeat: { vx: [-18, 18], vy: [-18, 18], size: [2, 3.5], life: [900, 1500], alpha: 0.7, tint: 0.14, glow: 18, edge: 0.7, blend: L },
  // KY's Gift: an apple glinting on its desk and ribbons fluttering on the boxes.
  appleRibbon: { vx: [-8, 8], vy: [-6, 4], size: [2, 3.5], life: [2000, 3200], alpha: 0.7, tint: 0.09, glow: 10, edge: 0.5, blend: L },
  // Felinor Flagbearer: his banner rippling in a light breeze, a soft red glow beside him.
  bannerBreeze: { vx: [10, 22], vy: [-4, 2], size: [2, 3.5], life: [1800, 3000], alpha: 0.65, tint: 0.1, glow: 14, edge: 0.55, blend: L },
  // Felinor Flagbearer Prime: the great banner swaying over him, its shadow drifting across his side.
  greatBannerShade: { vx: [6, 14], vy: [-2, 2], size: [4, 7], life: [3200, 5000], alpha: 0.45, tint: 0.15, glow: 22, edge: 0.5, blend: S },
  // Jlockeed's Lobbyist: a slow red laser dot wandering over his briefcase, in steel blue.
  laserDot: { vx: [-9, 9], vy: [-6, 6], size: [1.5, 2.5], life: [2600, 4200], alpha: 0.8, tint: 0.07, glow: 8, edge: 0.5, blend: L },
  // Classic Golem: carved faces sliding across his chest, dust trickling from his joints.
  stoneFaces: { vx: [-2, 2], vy: [10, 20], size: [1.5, 3], life: [1800, 3000], alpha: 0.55, tint: 0.12, glow: 8, edge: 0.4, blend: S },
  // Twice Forward One Step Backwards: its stair-stones shuffling two up and one back, cracks pulsing in the void.
  stairCracks: { vx: [-4, 4], vy: [-12, 6], size: [2.5, 5], life: [2000, 3400], alpha: 0.6, tint: 0.16, glow: 18, edge: 0.6, blend: S },
  // J-lease J-Jungle EX-plorer: a few leaves drifting down and a vine swaying behind her.
  jungleLeaves: { vx: [-10, 10], vy: [8, 18], size: [3, 5], life: [2600, 4000], alpha: 0.6, tint: 0.09, glow: 10, edge: 0.45, blend: S },
  // Claude's Datacenter: blinking status lights and a gentle heat shimmer above its racks.
  serverLights: { vx: [-1, 1], vy: [-12, -6], size: [1.5, 2.5], life: [700, 1300], alpha: 0.8, tint: 0.1, glow: 12, edge: 0.6, blend: L },
  // Silly Silas: quick small mirrorball glints sweeping across his tile.
  mirrorball: { vx: [18, 40], vy: [-6, 6], size: [1.5, 3], life: [700, 1400], alpha: 0.8, tint: 0.08, glow: 12, edge: 0.55, blend: L },
  // Combo-Index: large slow motes rising like its stadium's lit rings.
  stadium: { vx: [-3, 3], vy: [-12, -4], size: [4, 7], life: [2800, 4400], alpha: 0.5, tint: 0.1, glow: 16, edge: 0.55, blend: L },
  // Heroic Power: a few slow glints over its crystal pedestal.
  pedestal: { vx: [-2, 2], vy: [-6, -2], size: [2.5, 4.5], life: [3000, 4800], alpha: 0.7, tint: 0.09, glow: 18, edge: 0.6, blend: L },
  // R3407000: MELCHIOR's scan line sweeping its zone, counting.
  scanline: { vx: [30, 50], vy: [-1, 1], size: [1.5, 2.5], life: [800, 1400], alpha: 0.7, tint: 0.08, glow: 10, edge: 0.55, blend: L },
  // BALTHASAR's gentle heartbeat pulsing along its zone's edge.
  heartbeat: { vx: [-3, 3], vy: [-3, 3], size: [3, 5], life: [900, 1300], alpha: 0.65, tint: 0.1, glow: 14, edge: 0.6, blend: L },
  // CASPER's rose glint drifting across a mirror sheen.
  glint: { vx: [14, 24], vy: [-6, -2], size: [2, 3], life: [1400, 2400], alpha: 0.75, tint: 0.07, glow: 12, edge: 0.5, blend: L },
  // xxFreshJack's pixel shimmer crawling up his edge, like an unstable patch.
  pixelcrawl: { vx: [-2, 2], vy: [-30, -16], size: [2, 2.5], life: [700, 1300], alpha: 0.7, tint: 0.06, glow: 8, edge: 0.5, blend: L },
  // iKwzki's slow halo of motes.
  duohalo: { vx: [-6, 6], vy: [-6, 6], size: [2.5, 4], life: [3000, 4600], alpha: 0.65, tint: 0.08, glow: 16, edge: 0.55, blend: L },
  // Shockwave447's thin arcs flicking between his keywords.
  arcflick: { vx: [-40, 40], vy: [-30, 30], size: [1, 2], life: [250, 600], alpha: 0.8, tint: 0.06, glow: 12, edge: 0.65, blend: L },
  // Job Application's step lights over its door, blinking softly where they stand.
  steplights: { vx: [0, 0], vy: [0, 0], size: [3, 4], life: [1200, 1800], alpha: 0.7, tint: 0.08, glow: 10, edge: 0.5, blend: L },
  // Scornboi's slow crimson searchlight sweeping out from his zone.
  searchlight: { vx: [18, 30], vy: [-2, 2], size: [4, 7], life: [2000, 3000], alpha: 0.5, tint: 0.12, glow: 18, edge: 0.6, blend: S },
  // Megabase's slow-turning rings, its shuttles circling and its windows twinkling.
  stationrings: { vx: [-8, 8], vy: [-3, 3], size: [1.5, 3], life: [2400, 4000], alpha: 0.7, tint: 0.1, glow: 14, edge: 0.55, blend: L },
  // IN33DH3's AI wisp orbiting his head, blinking as it thinks.
  aiwisp: { vx: [-14, 14], vy: [-14, 4], size: [2.5, 4], life: [1000, 1800], alpha: 0.75, tint: 0.07, glow: 12, edge: 0.5, blend: L },
  // IN33DH3 Prime's slow pulse circling his neural crown.
  crownpulse: { vx: [-5, 5], vy: [-12, -4], size: [2, 3.5], life: [1600, 2600], alpha: 0.7, tint: 0.08, glow: 16, edge: 0.6, blend: L },
  // Wardrum Blob's soft wobble in time to a slow drumbeat.
  drumbeat: { vx: [-4, 4], vy: [-8, 8], size: [4, 7], life: [800, 1000], alpha: 0.6, tint: 0.12, glow: 12, edge: 0.5, blend: S },
  // Alteran Reach's faint star path across its zone.
  starpath: { vx: [6, 12], vy: [-2, 2], size: [1.5, 3], life: [2600, 4200], alpha: 0.75, tint: 0.1, glow: 12, edge: 0.55, blend: L },
  // Stinkyscape Golem's lazy green stink vapour curling off its stones.
  stinkvapour: { vx: [-6, 6], vy: [-16, -6], size: [5, 9], life: [2400, 3800], alpha: 0.5, tint: 0.12, glow: 12, edge: 0.45, blend: S },
  // Sanctum's silver motes drifting up like turning pages.
  pages: { vx: [-6, 6], vy: [-20, -8], size: [2, 3.5], life: [2200, 3600], alpha: 0.7, tint: 0.08, glow: 14, edge: 0.55, blend: L },
  // R3395001: The Greatest Heist: cash-green laser glints sweeping sideways across its vault.
  laserGrid: { vx: [-16, 16], vy: [-2, 2], size: [1, 2], life: [1800, 3000], alpha: 0.6, tint: 0.06, glow: 12, edge: 0.5, blend: L },
  // Rod of Ages: pale silver motes rising off its bands.
  silverRise: { vx: [-2, 2], vy: [-16, -8], size: [1.5, 3], life: [2600, 4200], alpha: 0.6, tint: 0.08, glow: 12, edge: 0.5, blend: L },
  // The Narcor Project: a slow green haze seeping from its cracks.
  labHaze: { vx: [-5, 5], vy: [-8, -2], size: [4, 7], life: [3000, 5000], alpha: 0.45, tint: 0.12, glow: 12, edge: 0.45, blend: S },
};

/** R3389000: any look's spec, shared or a signature's own. */
export function auraLookSpec(look: AuraLook): AuraLookSpec {
  return isSignatureAuraLook(look) ? AURA_SIGNATURE_LOOKS[look] : AURA_LOOKS[look];
}
