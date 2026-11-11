// R2715001: the full-art cards' moving art. A full-art card's picture is procedural SVG (svg.ts, and
// the Stinkyscape and Training Grounds painters), so its animation is its own layers moving, each on a
// beat written for that card (`FULL_ART_MOTION`): its subject (bobbing, breathing, swaying, hopping),
// its background (drifting, swelling), what lies over it, its light (pulsing, flaring), its props (the
// motif's glyphs and the accent, floating, spinning, fluttering), its motes (twinkling) and particles
// that suit the card (`particleMarkup`: coins, sparks, bubbles, embers…), seeded by the card's id.
//
// A painter given a motion tags each layer it draws with its role (`moving`, a `<g class="art-m-…">`
// that carries no `transform` of its own, since the animation's would replace it), and puts the
// keyframes in the document (`motionStyle`): transform and opacity alone, inside a
// `prefers-reduced-motion: no-preference` gate. With no motion it draws exactly the still picture, byte
// for byte, so the still art and its golden hashes never change. The face decides when its art moves
// (CardFace, `useArtMotion`): the gate inside the image is the last line, since page CSS cannot reach
// into a background image, and the Reduce motion setting never reaches it at all.
//
// Presentation only (CLAUDE.md rule 7).

import { fmt, hashId, seededRandom } from "./hash.ts";

/** R2715001: what part of the picture a layer is, each moving on its own beat. */
export type ArtRole = "back" | "subject" | "over" | "light" | "props" | "motes";

/** The roles, in the order the style lists them. */
export const ART_ROLES: readonly ArtRole[] = ["back", "subject", "over", "light", "props", "motes"];

/** One way a layer moves (`ART_MOVES`). */
export type ArtMove =
  | "bob"
  | "breathe"
  | "sway"
  | "hop"
  | "drift"
  | "swell"
  | "pulse"
  | "flare"
  | "spin"
  | "tick"
  | "float"
  | "flutter"
  | "twinkle"
  | "rise"
  | "fall"
  | "glint";

/** What a card's particles are (`particleMarkup`). */
export type ParticleKind = "coin" | "bubble" | "ember" | "spark" | "star" | "leaf" | "crumb" | "drop" | "heart" | "note" | "confetti";

/** A layer's move and how long one loop of it takes. */
export type ArtBeat = { readonly move: ArtMove; readonly seconds: number };

/** R2715001: one card's moving art: a beat for each role it moves, and its particles. */
export type ArtMotion = { readonly [R in ArtRole]?: ArtBeat } & {
  readonly particles: { readonly kind: ParticleKind; readonly beat: ArtBeat };
  /**
   * R3340000: a live board face draws its particles as a layer of their own over the subject (`particleLayerUri`), so the
   * picture beneath leaves them out: `particleMarkup` draws none and `motionStyle` no rule for them.
   */
  readonly detached?: true;
};

/**
 * A move's keyframes (transform and opacity alone), the box its transform is measured in (the
 * picture's view box, so a whole layer turns about a point of the picture, or the layer's own box,
 * so a glyph or a particle turns about its middle) and its timing function.
 */
type MoveSpec = { readonly keyframes: string; readonly box: "view" | "fill"; readonly timing: string };

const EASE = "ease-in-out";

/** R2715001: every move, by name. */
export const ART_MOVES: Readonly<Record<ArtMove, MoveSpec>> = {
  bob: { keyframes: "0%,100%{transform:translateY(0)}50%{transform:translateY(-1.5px)}", box: "view", timing: EASE },
  breathe: { keyframes: "0%,100%{transform:scale(1)}50%{transform:scale(1.03)}", box: "view", timing: EASE },
  sway: { keyframes: "0%,100%{transform:rotate(-1.5deg)}50%{transform:rotate(1.5deg)}", box: "view", timing: EASE },
  hop: { keyframes: "0%,50%,100%{transform:translateY(0)}25%{transform:translateY(-2.5px)}", box: "view", timing: EASE },
  drift: {
    keyframes: "0%,100%{transform:scale(1.06) translateX(-1.5px)}50%{transform:scale(1.06) translateX(1.5px)}",
    box: "view",
    timing: EASE,
  },
  swell: {
    keyframes: "0%,100%{transform:scale(1.06) translateY(-1px)}50%{transform:scale(1.06) translateY(1px)}",
    box: "view",
    timing: EASE,
  },
  pulse: { keyframes: "0%,100%{opacity:1}50%{opacity:0.6}", box: "fill", timing: EASE },
  flare: { keyframes: "0%,100%{transform:scale(1);opacity:1}50%{transform:scale(1.12);opacity:0.8}", box: "fill", timing: EASE },
  spin: { keyframes: "from{transform:rotate(0deg)}to{transform:rotate(360deg)}", box: "fill", timing: "linear" },
  tick: { keyframes: "from{transform:rotate(0deg)}to{transform:rotate(360deg)}", box: "fill", timing: "steps(12)" },
  float: { keyframes: "0%,100%{transform:translateY(0)}50%{transform:translateY(-2px)}", box: "fill", timing: EASE },
  flutter: { keyframes: "0%,100%{transform:rotate(-8deg)}50%{transform:rotate(8deg)}", box: "fill", timing: EASE },
  twinkle: { keyframes: "0%,100%{transform:scale(1);opacity:1}50%{transform:scale(0.85);opacity:0.4}", box: "fill", timing: EASE },
  rise: {
    keyframes: "0%{transform:translateY(4px);opacity:0}20%{opacity:1}100%{transform:translateY(-12px);opacity:0}",
    box: "fill",
    timing: "linear",
  },
  fall: {
    keyframes: "0%{transform:translateY(-8px);opacity:0}20%{opacity:1}80%{opacity:1}100%{transform:translateY(10px);opacity:0}",
    box: "fill",
    timing: "linear",
  },
  glint: {
    keyframes: "0%,100%{transform:scale(0.4);opacity:0}50%{transform:scale(1);opacity:1}",
    box: "fill",
    timing: EASE,
  },
};

/** Where a figure's subject sways and breathes from: the foot of its body, in the picture's units. */
export const FIGURE_SUBJECT_ORIGIN = "50px 100px";
/** Where every other whole layer moves about: the picture's middle. */
export const CENTER_ORIGIN = "50px 50px";

/** How many particles a moving picture draws, and how many steps a role's staggered layers spread over. */
export const PARTICLE_COUNT = 7;
/** Salts the particles' own stream, so they move nothing the picture draws. */
const PARTICLE_SALT = 0x6a09e667;
/** The band the particles are placed in, in the picture's units: inside the crop of every art shape. */
export const PARTICLE_X_MIN = 22;
export const PARTICLE_X_MAX = 78;
export const PARTICLE_Y_MIN = 16;
export const PARTICLE_Y_MAX = 84;

const beat = (move: ArtMove, seconds: number): ArtBeat => ({ move, seconds });
const particles = (kind: ParticleKind, move: ArtMove, seconds: number): ArtMotion["particles"] => ({ kind, beat: beat(move, seconds) });

/**
 * R2715001: each full-art card's moving art, by card id (the cosmetics registry's full-art cards,
 * R2655001; a token listed under one takes its parent's, R2655002). Each moves two of its picture's
 * elements or more on beats of their own, and no two are alike; `motion.test.ts` holds every full-art
 * card of the registry to an entry. The 21 are hand-written; R3340000 adds a recipe for every other
 * Legendary and Mythic (`RARE_ART_MOTION`).
 */
export const FULL_ART_CARD_MOTION: Readonly<Record<string, ArtMotion>> = {
  // Big D-fender: a shield wall that breathes behind its sparks.
  "core-001": { subject: beat("breathe", 4), back: beat("swell", 7), props: beat("float", 3), particles: particles("spark", "glint", 2.4) },
  // Gary the Gambler: coins rain while his lucky glyph spins.
  "core-004": { subject: beat("sway", 5), light: beat("pulse", 3.5), props: beat("spin", 6), particles: particles("coin", "fall", 3) },
  // Mr. Vanilla: a plain bob under twinkling stars.
  "core-008": {
    subject: beat("bob", 4.5),
    light: beat("pulse", 3),
    props: beat("float", 5),
    motes: beat("twinkle", 2.5),
    particles: particles("star", "glint", 3.2),
  },
  // Tempo Timmy: hops in time with a ticking clock.
  "core-011": { subject: beat("hop", 2), back: beat("drift", 8), props: beat("tick", 4), particles: particles("spark", "rise", 2.6) },
  // Duplicating Felinors: purring, hearts rising.
  "core-012": { subject: beat("breathe", 3.5), light: beat("flare", 4), props: beat("spin", 9), particles: particles("heart", "rise", 3.4) },
  // Jlockeed Shredder-10: shaking, embers flying.
  "core-013": { subject: beat("sway", 3), back: beat("drift", 6), particles: particles("ember", "rise", 2.2) },
  // League of Losers: a slow sway under falling confetti.
  "classicplus-019": {
    subject: beat("sway", 6),
    back: beat("drift", 9),
    light: beat("flare", 3),
    props: beat("twinkle", 2.2),
    particles: particles("confetti", "fall", 3.6),
  },
  // Bread and Butter: crumbs falling off a warm loaf.
  "core-018": { subject: beat("breathe", 5), back: beat("drift", 8), props: beat("float", 4), particles: particles("crumb", "fall", 3.2) },
  // The Mother Pancake: syrup swelling over the stack.
  "classicplus-012": { subject: beat("bob", 4), back: beat("swell", 6), over: beat("swell", 5), particles: particles("drop", "fall", 2.8) },
  // "Miss" Mrow: a cat's slow sway, hearts rising.
  "core-086": { subject: beat("sway", 3.5), back: beat("drift", 10), props: beat("twinkle", 2), particles: particles("heart", "rise", 3) },
  // API Key Fishing: a line swaying, bubbles rising.
  "meditative-021": { subject: beat("sway", 7), light: beat("pulse", 4), props: beat("float", 3), particles: particles("bubble", "rise", 3.5) },
  // Felinor Flagbearer: the flag flutters, stars glint.
  "classicplus-046": { subject: beat("breathe", 4), light: beat("pulse", 5), props: beat("flutter", 1.6), particles: particles("star", "glint", 2.8) },
  // Dice of Doom: the dice bob over a slow drift of space.
  "stinkyscape-154": { subject: beat("bob", 5), back: beat("drift", 12), particles: particles("spark", "glint", 2) },
  // KY's Papaya: leaves falling past the fruit.
  "classicplus-062": { subject: beat("breathe", 4), props: beat("float", 3.5), motes: beat("twinkle", 2.6), particles: particles("leaf", "fall", 4) },
  // In Too Deep: the deep swells, bubbles rising.
  "classic-090": { over: beat("swell", 5), light: beat("pulse", 4), props: beat("float", 3), particles: particles("bubble", "rise", 3) },
  // Jlockeed's Lobbyist: coins and a spinning glyph.
  "classicplus-048": { subject: beat("bob", 5), back: beat("drift", 9), props: beat("spin", 4), particles: particles("coin", "fall", 2.6) },
  // Tatches the Totem: embers rising round a swaying totem.
  "meditative-087": { subject: beat("sway", 6), props: beat("twinkle", 2.5), motes: beat("twinkle", 1.8), particles: particles("ember", "rise", 2.4) },
  // Buff Billy: flexing fast, sparks rising.
  "classicplus-069": { subject: beat("breathe", 2.4), props: beat("float", 2), particles: particles("spark", "rise", 2) },
  // Goetzmann Game Design: a swaying subject on a drifting yard.
  "traininggrounds-040": { subject: beat("sway", 4), back: beat("drift", 10), particles: particles("star", "glint", 3) },
  // Tommy Tempo: hops to the beat, notes rising.
  "classicplus-026": { subject: beat("hop", 2.6), light: beat("pulse", 3), props: beat("tick", 4), particles: particles("note", "rise", 3) },
  // J-lease J-Jungle EX-plorer: leaves falling over a drifting jungle.
  "classicplus-075": { subject: beat("bob", 4.2), back: beat("drift", 11), props: beat("flutter", 3), particles: particles("leaf", "fall", 3.8) },
};

/** R3340000: the kinds of card a recipe is built for, by the card's type: a Unit, a Spell, a Field Spell, a Trap or Field Trap. */
type Archetype = "unit" | "spell" | "field" | "trap";

/**
 * R3340000: each archetype's roles: the moves a recipe picks among (the one at its running count, wrapping) and the
 * seconds of the role's loop before the recipe's stretch (`RECIPE_STRETCH_SECONDS`).
 */
const ARCHETYPES: Readonly<Record<Archetype, readonly (readonly [ArtRole, readonly ArtMove[], number])[]>> = {
  unit: [
    ["subject", ["bob", "breathe", "sway", "hop"], 4],
    ["back", ["drift"], 9],
    ["props", ["float"], 3.5],
  ],
  spell: [
    ["subject", ["breathe"], 4.5],
    ["light", ["flare"], 3.5],
    ["props", ["spin"], 7],
  ],
  field: [
    ["back", ["swell", "drift"], 8],
    ["over", ["swell"], 6],
    ["light", ["pulse"], 4.5],
  ],
  trap: [
    ["subject", ["breathe"], 4],
    ["props", ["spin", "tick"], 5],
    ["light", ["flare"], 3.2],
  ],
};

/** R3340000: how much slower each recipe in the list runs than the one before it, so no two recipes are alike. */
const RECIPE_STRETCH_SECONDS = 0.05;
/** R3340000: how a recipe's particles move, by kind: leaves and drops fall, the rest rise, and every loop takes this long before the stretch. */
const RARE_PARTICLE_SECONDS = 2.4;
const RARE_FALLING: ReadonlySet<ParticleKind> = new Set(["leaf", "drop"]);

/** Seconds, rounded to hundredths so a stretch never leaves a float's tail in a beat. */
const hundredths = (seconds: number): number => Math.round(seconds * 100) / 100;

/**
 * R3340000: the recipe for the `index`th card of the list: its archetype's roles, each on the move that index picks and a
 * loop `index * RECIPE_STRETCH_SECONDS` longer, with particles of `kind`.
 */
function recipe(archetype: Archetype, kind: ParticleKind, index: number): ArtMotion {
  const stretch = index * RECIPE_STRETCH_SECONDS;
  const roles: { -readonly [R in ArtRole]?: ArtBeat } = {};
  for (const [role, moves, seconds] of ARCHETYPES[archetype]) {
    roles[role] = beat(moves[index % moves.length] ?? "float", hundredths(seconds + stretch));
  }
  const move: ArtMove = RARE_FALLING.has(kind) ? "fall" : "rise";
  return { ...roles, particles: particles(kind, move, hundredths(RARE_PARTICLE_SECONDS + stretch)) };
}

/**
 * R3340000: the Legendaries' and Mythics' recipes that were not hand-written, by card id: each card's archetype (its
 * type) and its particles (its tag, else its set), in card-id order. None draws a gold particle (R2725004: a base face's
 * art is tinted no gold). `motion.test.ts` holds every Legendary and Mythic of the catalog to an entry.
 */
const RARE_CARDS: readonly (readonly [string, Archetype, ParticleKind])[] = [
  ["classic-004", "field", "ember"], // Palantir
  ["classic-007", "field", "ember"], // InfiniScepter
  ["classic-009", "trap", "ember"], // Income Tax
  ["classic-028", "field", "ember"], // Second Wind
  ["classic-033", "unit", "ember"], // Joro
  ["classic-044", "spell", "ember"], // Back from the GY
  ["classic-045", "unit", "ember"], // Nature Titan
  ["classic-056", "unit", "ember"], // Spell Tyrant
  ["classic-080", "unit", "ember"], // BOOM! Big Max
  ["classic-085", "unit", "ember"], // King Wagtoggle
  ["classicplus-013", "unit", "note"], // Mommy Barker
  ["classicplus-027", "spell", "ember"], // Zephrys Zealotism
  ["classicplus-029", "spell", "ember"], // Portal to the Past
  ["classicplus-035", "spell", "ember"], // Rollback
  ["classicplus-037", "unit", "ember"], // Wardrum
  ["classicplus-042", "spell", "leaf"], // KY's Test
  ["classicplus-043", "spell", "ember"], // AI Slop
  ["classicplus-047", "spell", "ember"], // Jogg's Box
  ["classicplus-073", "spell", "ember"], // Call to Chaos (Classic+ Edition)
  ["classicplus-074", "trap", "ember"], // Twice Forward One Step Backwards
  ["classicplus-078", "field", "ember"], // Claude's Datacenter
  ["core-052", "unit", "note"], // Silly Silas
  ["core-083", "spell", "ember"], // Transmogulate
  ["core-085", "trap", "ember"], // Unlicensed Experimentation
  ["core-087", "spell", "ember"], // Pocket Chaos
  ["core-092", "unit", "note"], // Felinor Fiender
  ["core-093", "field", "ember"], // Combo-Index
  ["core-095", "spell", "ember"], // Call to Chaos (Core Edition)
  ["core-096", "trap", "ember"], // My Pawn
  ["core-097", "spell", "ember"], // Zephrys
  ["core-098", "field", "ember"], // Heroic Power
  ["core-099", "spell", "ember"], // Craft a Card
  ["core-100", "unit", "ember"], // Ceaseless Void
  ["meditative-007", "field", "bubble"], // Introspection
  ["meditative-008", "spell", "ember"], // Reach the Summit
  ["meditative-011", "field", "bubble"], // Double Header
  ["meditative-017", "spell", "bubble"], // True Craft a Card
  ["meditative-020", "spell", "ember"], // Aestheticize the Game
  ["meditative-026", "field", "bubble"], // Alternate Fate
  ["meditative-040", "field", "ember"], // Feng Shui
  ["meditative-042", "spell", "ember"], // CN Flea Market
  ["meditative-045", "unit", "ember"], // Knowledge Breaker
  ["meditative-047", "unit", "ember"], // 饕餮
  ["meditative-049", "unit", "ember"], // YileGPT
  ["meditative-051", "unit", "bubble"], // Devin Bot
  ["meditative-063", "field", "bubble"], // Skull of J'Nari
  ["meditative-065", "unit", "bubble"], // Keymaster Keenus
  ["meditative-071", "field", "bubble"], // Pareto Optimality
  ["meditative-075", "field", "bubble"], // Ever Growing Tree
  ["meditative-079", "unit", "leaf"], // Touched by KY
  ["meditative-080", "field", "bubble"], // Aluneth
  ["meditative-086", "unit", "bubble"], // Mayor Medinamogger
  ["meditative-095", "spell", "ember"], // Call to Chaos (Meditative Edition)
  ["meditative-097", "spell", "bubble"], // Jlockeed's Evil Blueprints
  ["meditativeplus-001", "spell", "ember"], // Bob's Plan
  ["meditativeplus-024", "unit", "bubble"], // Bob, Wisconsin's Strongest Soldier
  ["meditativeplus-026", "spell", "bubble"], // Scratch Pad
  ["stinkyscape-004", "spell", "bubble"], // Olivian Expanse
  ["stinkyscape-006", "unit", "note"], // Tylannt
  ["stinkyscape-007", "field", "bubble"], // The Greatest Heist
  ["stinkyscape-016", "field", "bubble"], // Rod of Ages
  ["stinkyscape-018", "spell", "bubble"], // Imitated Instrumentality
  ["stinkyscape-026", "unit", "note"], // ZolarKeth
  ["stinkyscape-034", "spell", "bubble"], // Drone Core VIII
  ["stinkyscape-035", "spell", "bubble"], // Drone Core IX
  ["stinkyscape-036", "spell", "bubble"], // Drone Core X
  ["stinkyscape-037", "unit", "note"], // Kwzka
  ["stinkyscape-039", "field", "bubble"], // The Narcor Project
  ["stinkyscape-044", "spell", "ember"], // Great Paperclip Theory
  ["stinkyscape-058", "unit", "bubble"], // Beam v5
  ["stinkyscape-059", "unit", "bubble"], // Repeater v5
  ["stinkyscape-060", "unit", "bubble"], // Cannon v5
  ["stinkyscape-061", "unit", "bubble"], // Flak v5
  ["stinkyscape-066", "unit", "bubble"], // Miner v5
  ["stinkyscape-070", "unit", "bubble"], // Z'Zoth
  ["stinkyscape-071", "unit", "bubble"], // A'Zoth
  ["stinkyscape-073", "unit", "bubble"], // Paragon Mk2
  ["stinkyscape-074", "unit", "bubble"], // Paragon-X
  ["stinkyscape-080", "unit", "bubble"], // Galaxy
  ["stinkyscape-085", "unit", "bubble"], // Prototype-X1
  ["stinkyscape-086", "unit", "bubble"], // Prototype-X2
  ["stinkyscape-087", "unit", "heart"], // Siva 1
  ["stinkyscape-088", "unit", "note"], // Siva 2
  ["stinkyscape-109", "unit", "note"], // xxFreshJack
  ["stinkyscape-110", "unit", "note"], // iKwzki
  ["stinkyscape-111", "unit", "note"], // Shockwave447
  ["stinkyscape-112", "field", "bubble"], // Job Application
  ["stinkyscape-115", "unit", "note"], // Scornboi
  ["stinkyscape-116", "unit", "ember"], // Megabase
  ["stinkyscape-137", "unit", "note"], // Starfleet Commander
  ["stinkyscape-147", "unit", "note"], // IN33DH3
  ["stinkyscape-148", "field", "ember"], // Alteran Reach
  ["stinkyscape-149", "spell", "bubble"], // Wildar
  ["stinkyscape-150", "spell", "ember"], // Call to Chaos (Stinky Edition)
  ["stinkyscape-151", "spell", "ember"], // Call to Chaos (Scape Edition)
  ["stinkyscape-152", "field", "bubble"], // Sanctum
  ["stinkyscape-153", "spell", "bubble"], // Stimmycheck
  ["stinkyscape-176", "spell", "bubble"], // Preemptive Strike
  ["stinkyscape-t-dormant-starship", "unit", "bubble"], // Dormant Starship
  ["traininggrounds-037", "spell", "leaf"], // Seance
  ["traininggrounds-038", "unit", "note"], // Tutorial Master
  ["traininggrounds-039", "spell", "ember"], // Call to Chaos (Training-Grounds Edition)
];

const RARE_ART_MOTION: Readonly<Record<string, ArtMotion>> = Object.fromEntries(
  RARE_CARDS.map(([id, archetype, kind], index) => [id, recipe(archetype, kind, index)] as const),
);

/**
 * R2715001, R3340000: every moving art by card id: the 21 full-art cards' own and a recipe for every other Legendary and
 * Mythic, so none borrows one (`artMotionFor`).
 */
export const FULL_ART_MOTION: Readonly<Record<string, ArtMotion>> = { ...FULL_ART_CARD_MOTION, ...RARE_ART_MOTION };

/**
 * R2725004: the particle kinds drawn in a gold or a cream near it (`particleShape`), which a card's borrowed
 * motion never takes: the Animated augment tints no base face's art Radiant's gold.
 */
export const GOLD_PARTICLES: ReadonlySet<ParticleKind> = new Set(["coin", "spark", "star", "crumb", "confetti"]);

/**
 * R2725000: the motions a card with none of its own may borrow, in `FULL_ART_CARD_MOTION`'s order: those with no gold
 * particle. R3340000: only the 21's, so a card that borrows keeps the motion it borrowed before the rare recipes.
 */
const BORROWED_MOTIONS: readonly ArtMotion[] = Object.values(FULL_ART_CARD_MOTION).filter((motion) => !GOLD_PARTICLES.has(motion.particles.kind));

/**
 * R2725000: the moving art `defId` draws at an animated level (R3015002: every animated and signed level of every
 * variant, one motion per card): its own (`FULL_ART_MOTION`), else one of `BORROWED_MOTIONS` picked by its id, its
 * particles placed by its id too (`particleMarkup`).
 */
export function artMotionFor(defId: string): ArtMotion | null {
  return FULL_ART_MOTION[defId] ?? BORROWED_MOTIONS[hashId(defId) % BORROWED_MOTIONS.length] ?? null;
}

/** A beat's loop as the style writes it: `art-<move> <seconds>s <timing> infinite`. */
function animation(step: ArtBeat): string {
  return `art-${step.move} ${fmt(step.seconds)}s ${ART_MOVES[step.move].timing} infinite`;
}

/** One role's (or the particles') rule: its box, its origin and its loop. R3350000: a diorama's slice writes its loop's with it. */
export function roleRule(selector: string, step: ArtBeat, viewOrigin: string): string {
  const spec = ART_MOVES[step.move];
  const box = spec.box === "view" ? "view-box" : "fill-box";
  const origin = spec.box === "view" ? viewOrigin : "center";
  return `${selector}{transform-box:${box};transform-origin:${origin};animation:${animation(step)}}`;
}

/**
 * R2715001: the `<style>` a moving picture carries after its `<defs>`: a rule for each role `motion`
 * moves and for its particles, then the keyframes of each move used, all inside the reduced-motion
 * gate. The subject's whole-layer moves turn about `subjectOrigin`; every other whole layer about the
 * picture's middle. Empty with no motion.
 */
export function motionStyle(motion: ArtMotion | null, subjectOrigin: string): string {
  if (motion === null) return "";
  const rules: string[] = [];
  const used = new Set<ArtMove>();
  for (const role of ART_ROLES) {
    const step = motion[role];
    if (step === undefined) continue;
    rules.push(roleRule(`.art-m-${role}`, step, role === "subject" ? subjectOrigin : CENTER_ORIGIN));
    used.add(step.move);
  }
  if (motion.detached !== true) {
    rules.push(roleRule(".art-m-particle", motion.particles.beat, CENTER_ORIGIN));
    used.add(motion.particles.beat.move);
  }
  const frames = [...used].map((move) => `@keyframes art-${move}{${ART_MOVES[move].keyframes}}`).join("");
  return `<style>@media (prefers-reduced-motion: no-preference){${rules.join("")}${frames}}</style>`;
}

/**
 * R3340000: the `<style>` of a particle layer drawn on its own (`particleLayerUri`): the rule for the particles and
 * the keyframes of their move, inside the reduced-motion gate.
 */
export function particleStyle(motion: ArtMotion): string {
  const move = motion.particles.beat.move;
  return `<style>@media (prefers-reduced-motion: no-preference){${roleRule(".art-m-particle", motion.particles.beat, CENTER_ORIGIN)}@keyframes art-${move}{${ART_MOVES[move].keyframes}}}</style>`;
}

/** A staggered layer's start: `index` steps of its loop already run, so a row of glyphs never moves as one. */
function stagger(index: number, seconds: number): string {
  return index > 0 ? ` style="animation-delay:-${fmt((index * seconds) / PARTICLE_COUNT)}s"` : "";
}

/**
 * R2715001: `markup` tagged as `role`'s layer when `motion` moves that role (the `index`th of a row of
 * them, which starts its loop that far along), and `markup` unchanged otherwise, so a picture with no
 * motion is byte for byte the still one. The wrapper carries no `transform`: the animation's would
 * replace it.
 */
export function moving(role: ArtRole, markup: string, motion: ArtMotion | null, index = 0): string {
  const step = motion?.[role];
  if (step === undefined || markup === "") return markup;
  return `<g class="art-m-${role}"${stagger(index, step.seconds)}>${markup}</g>`;
}

/** Each particle kind's colors. */
const COIN_FILL = "#f4c542";
const COIN_EDGE = "#a87a12";
const BUBBLE_EDGE = "#cfefff";
const EMBER_FILL = "#ff8a3d";
const SPARK_FILL = "#fff3b0";
const STAR_FILL = "#ffe27a";
const LEAF_FILL = "#6fbf5a";
const CRUMB_FILL = "#d9a35b";
const DROP_FILL = "#7cc8ff";
const HEART_FILL = "#ff6f91";
const NOTE_FILL = "#f2f2ff";
const CONFETTI_FILLS = ["#ff6f91", "#ffd166", "#06d6a0", "#4cc9f0", "#c77dff"] as const;

/** A particle of `kind`, drawn about the origin; the `index`th of its picture (which picks a confetti's color). */
function particleShape(kind: ParticleKind, index: number): string {
  switch (kind) {
    case "coin":
      return `<circle r="2" fill="${COIN_FILL}" stroke="${COIN_EDGE}" stroke-width="0.5"/>`;
    case "bubble":
      return `<circle r="1.6" fill="none" stroke="${BUBBLE_EDGE}" stroke-width="0.4" opacity="0.8"/>`;
    case "ember":
      return `<circle r="0.9" fill="${EMBER_FILL}"/>`;
    case "spark":
      return `<path d="M0 -2.2L0.5 -0.5L2.2 0L0.5 0.5L0 2.2L-0.5 0.5L-2.2 0L-0.5 -0.5Z" fill="${SPARK_FILL}"/>`;
    case "star":
      return `<path d="M0 -2L0.6 -0.6L2 -0.6L0.9 0.3L1.3 1.8L0 0.9L-1.3 1.8L-0.9 0.3L-2 -0.6L-0.6 -0.6Z" fill="${STAR_FILL}"/>`;
    case "leaf":
      return `<ellipse rx="1.8" ry="0.9" fill="${LEAF_FILL}"/>`;
    case "crumb":
      return `<rect x="-0.7" y="-0.55" width="1.4" height="1.1" fill="${CRUMB_FILL}"/>`;
    case "drop":
      return `<path d="M0 -2L1 0A1 1 0 1 1 -1 0Z" fill="${DROP_FILL}"/>`;
    case "heart":
      return `<path d="M0 1.6L-1.6 0A0.9 0.9 0 0 1 0 -1A0.9 0.9 0 0 1 1.6 0Z" fill="${HEART_FILL}"/>`;
    case "note":
      return `<path d="M0.4 -2.2V0.9A0.9 0.7 0 1 1 -0.1 0.2V-2.2Z" fill="${NOTE_FILL}"/>`;
    case "confetti":
      return `<rect x="-0.8" y="-0.4" width="1.6" height="0.8" fill="${CONFETTI_FILLS[index % CONFETTI_FILLS.length] ?? COIN_FILL}"/>`;
  }
}

/**
 * R2715001: a moving picture's particles, drawn over everything but the vignette: `PARTICLE_COUNT` of
 * `motion`'s kind, placed by a stream of `defId`'s own and staggered along their loop. Empty with no
 * motion. R3340000: and with a detached one, whose particles are a layer of their own.
 */
export function particleMarkup(motion: ArtMotion | null, defId: string): string {
  if (motion === null || motion.detached === true) return "";
  const rng = seededRandom((hashId(defId) ^ PARTICLE_SALT) >>> 0);
  const { kind, beat: step } = motion.particles;
  const out: string[] = [];
  for (let i = 0; i < PARTICLE_COUNT; i += 1) {
    const x = PARTICLE_X_MIN + rng() * (PARTICLE_X_MAX - PARTICLE_X_MIN);
    const y = PARTICLE_Y_MIN + rng() * (PARTICLE_Y_MAX - PARTICLE_Y_MIN);
    out.push(`<g class="art-m-particle"${stagger(i, step.seconds)}><g transform="translate(${fmt(x)} ${fmt(y)})">${particleShape(kind, i)}</g></g>`);
  }
  return out.join("");
}
