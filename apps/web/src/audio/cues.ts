// The sound table (SPEC §10.11): one row per member of `GameEventType`, like `ANIMATIONS` in
// `game/animations.ts`. A row names its headline sound effect, or states why the event is silent,
// and resolves an event into the cues to play. `SOUND_CUES` is typed as a total map, so a new event
// type does not compile until it has a row here.
//
// No rule lives here (CLAUDE.md rule 7). The table reads the event and the viewer's `PlayerView`
// and nothing else, so it can say no more than the screen does (R203): a card whose `defId` is the
// sentinel never speaks, and a trap set speaks on no seat. R204 fixes which moments speak: a unit
// on its `cardPlayed` (or, when an effect put it there without a play, its `summoned`) and its
// `destroyed`, a spell on its `cardPlayed`, a trap on its `trapFired`. Each line carries its
// VOICE_PRIORITY: a death or a firing trap answers something that just happened and takes the
// channel from a play or cast line, and a summoned unit speaks only when nothing else is talking.
//
// A card the viewer can name may also colour its sounds from the public catalog (§5.1): a unit's
// summon thud carries its family's accent and a Legendary or Mythic unit enters with a sting beside
// the effects layer's light rays (R202's rarity entrance), and a spell's shimmer rings in its
// family's chimes. The family is the card art's theme (cards/art/themes.ts), so a card looks and
// sounds like the same kind of thing. R203 bounds it: only a unit on the field (always public) and a
// cast spell vary, never a card behind the sentinel and never a Trap, whose set must sound the same
// for every Trap. A token that prints a rarity (B2.5's `printedRarity`) enters with that rarity's
// sting.
//
// R506 adds the card moments of patch v0.2.0. A sound may also answer the readable play an event
// happens inside (`CueContext.playing`, which the director keeps: the innermost `cardPlayed` whose
// `cardResolved` has not come): #21 Hinder's rider landing on the victim's next refresh cracks like
// glass, and each card #27 Blood Ridden Glowy Jelly Bean turns Radiant is a blood drain and a gold
// burst, on the opponent's seat too, where that `radiantSet` carries the sentinel: the burst reads
// the public cast, never the card it hit. A card cast as it is drawn stings (`castOnDraw`), Call to
// Chaos's roll dings once for each effect it names (R436), and a mark brands its card (R437).
//
// R655: those moments are a card's hooks (`play`, `death`, `cast`), and a hook in `card-audio.json5`
// gives a line, a named effect, or both. The effect plays at the moment the line would speak, and a
// line an effect leads waits CARD_EFFECT_DELAY_MS more, so the sound reads as what the card does and
// the line as its reaction. The effect is read off the same readable entry as the line, so R203 holds
// for both. The `attack` hook is no event's: the board plays it when the viewer picks a Unit up
// (`usePickupSound`).
//
// R669 (#259): every card the viewer can read stings as it is played, sized by its rarity: Common,
// Rare and Epic Units and Spells on their `cardPlayed`, a Legendary or Mythic Spell with the
// entrance there, and a Legendary or Mythic Unit with the entrance on its `summoned` as before.
// Never a Trap (R203), never the sentinel, and never a card cast as it is drawn, whose own sting
// stands in. And every effect about a unit on the field comes from that unit's lane: `cuesFor`
// pans it left or right by where the unit stands, which the screen shows both seats alike.
//
// MN05 (docs/meditative-set.md M8): Armor and the niche moments, each keyed on what its event
// already says. R1363: a hit Armor took half or more of (`damage.absorbed * 2 >= absorbed + amount`)
// clanks dully under its impact, and one Armor took whole (`damageAbsorbed`) rings bright; both come
// from the unit's lane. R1364: a hit that does far more than the health the view shows the target
// with is overkill, a crunch on top. R1365: a Brittle crumble, an unlock, a Counter, a fuse, a Nerf
// and a Buff each have a sound of their own, and a card transformed into a Sheep bleats. R1366: a
// card taken is a steal and a card handed over a give, as `controlChanged.how` says. None of
// them varies with a card the viewer cannot read (R203), and a Nerf or a Buff sounds the same
// whatever it changed (R440).

import type {
  CardType,
  GameEvent,
  GameEventType,
  PlayerId,
  PlayerView,
  PrintedRarity,
  Rarity,
  Tag,
  UnitView,
} from "@jackioh/shared";

import { themeFor } from "../cards/art/themes.ts";
import { armorTookHalf, damageTier, UNIT_SLAM } from "../game/damageFeel.ts";
import { slamStatsOf, slamTier } from "../game/unitSlam.ts";
import {
  BLEAT_DELAY_MS,
  BLOOD_BEAN_DEF_ID,
  CARD_EFFECT_DELAY_MS,
  CHAOS_REVEAL_MAX,
  DEATH_VOICE_DELAY_MS,
  GOLD_BURST_DELAY_MS,
  HIDDEN_DEF_ID,
  HINDER_DEF_ID,
  LANE_PAN_MAX,
  NEXT_REFRESH_MODIFIER_ID,
  OVERKILL_DELAY_MS,
  OVERKILL_MIN_EXCESS,
  SHEEP_DEF_IDS,
  STING_DELAY_MS,
  VOICE_DELAY_MS,
  VOICE_PRIORITY,
} from "./constants.ts";
import type {
  CardAudioTable,
  CardHook,
  SfxId,
  SfxParams,
  SfxTimbre,
  SoundCue,
  StingTier,
  VoiceLineKind,
  VoicePriority,
} from "./types.ts";
import { entryFor, hookFor } from "./voiceData.ts";

/**
 * The public catalog facts a cue may colour itself with (§5.1): never looked up for "hidden".
 * `printedRarity` is a token's printed rarity (B2.5), for its summon sting only.
 */
export type CueCard = {
  type: CardType;
  tags: readonly Tag[];
  rarity?: Rarity;
  printedRarity?: PrintedRarity;
  /** The base face's rules text: a landing Unit's printed Tribute (#185, unitSlam.ts). */
  text?: string;
};

/**
 * R506: a play in progress, as the director follows the stream: from its `cardPlayed` until its
 * `cardResolved`. `defId` is the sentinel for a card the viewer cannot read, which no card moment
 * matches. `castOnDraw`: the play is a card cast as it was drawn.
 */
export type PlayFrame = { defId: string; instanceId: string; player: PlayerId; castOnDraw: boolean };

export type CueContext = {
  /** The view the batch was planned against (pre-batch): `viewer` and seat orientation come from here. */
  view: PlayerView;
  /** The card sound table (R655): every card's hooks, with the voices and effects they name. */
  lines: CardAudioTable;
  /** Current mana this player had before this event, as the director tracks it. */
  manaBefore: (player: PlayerId) => number;
  /**
   * True when this instance's own `cardPlayed` has already sounded, so its `summoned` is that play
   * arriving and its line has been spoken. Absent: no play has sounded.
   */
  wasPlayed?: (instanceId: string) => boolean;
  /** R669: the newest view the director has, for where a unit that has just arrived stands. Absent: none. */
  newestView?: () => PlayerView | null;
  /** The unit as the newest view shows it (its size and Radiance), or null. Absent: unknown. */
  unitNow?: (instanceId: string) => UnitView | null;
  /** The public catalog, by a defId the viewer can read. Absent (or undefined): the plain sounds. */
  card?: (defId: string) => CueCard | undefined;
  /**
   * R506: the innermost play this event happens inside, or null. Absent: none is known, and every
   * event makes its row's plain sound.
   */
  playing?: () => PlayFrame | null;
  /**
   * R506: true when this `cardPlayed` is the card a readable `drawn` has just drawn, cast as it was
   * drawn. Absent: no play is a cast on draw.
   */
  castOnDraw?: (instanceId: string) => boolean;
};

export type CueRow<K extends GameEventType> = {
  /** The row's headline SFX, or null for an explicit silence. */
  sfx: SfxId | null;
  /** Required and non-empty exactly when sfx is null. */
  silentBecause?: string;
  cues: (event: Extract<GameEvent, { type: K }>, ctx: CueContext) => readonly SoundCue[];
};

/**
 * Patch v0.2.0's tags and their families (R506). The card art gives them themes of the same ids;
 * until `themeFor` names one (cards/art/themes.ts is the art's file), `timbreFor` reads the tag
 * itself, after every tag the art already knows and before Token and the type, as a tag theme
 * comes.
 */
const NEW_TAG_TIMBRES: readonly (readonly [Tag, SfxTimbre])[] = [
  ["Book", "book"],
  ["Pancake", "pancake"],
  ["AI", "ai"],
];

/**
 * A card's sound family: the card art's theme (tags first, then Token, then the type), where it is
 * one the recipes know. A plain Unit, Spell or Trap theme has no family of its own.
 */
export function timbreFor(card: CueCard): SfxTimbre | undefined {
  // Widened on purpose: the art's theme ids may grow the new families' ids before or after this.
  const theme: string = themeFor(card.tags, card.type);
  switch (theme) {
    case "human":
    case "felinor":
    case "ky":
    case "cn":
    case "fruit":
    case "chaos":
    case "quickdraw":
    case "book":
    case "pancake":
    case "ai":
      return theme;
    default:
      break;
  }
  for (const [tag, timbre] of NEW_TAG_TIMBRES) if (card.tags.includes(tag)) return timbre;
  if (theme === "token") return "token";
  if (theme === "field-spell") return "field";
  return undefined;
}

/** The catalog facts for a card the viewer can name, else undefined (R203: never for "hidden"). */
function readable(ctx: CueContext, defId: string): CueCard | undefined {
  return defId === HIDDEN_DEF_ID ? undefined : ctx.card?.(defId);
}

/** A spell's shimmer lands just after the card whoosh, under its cast line (the cast beat). */
const SPELL_SHIMMER_DELAY_MS = 60;
/** A Radiant unit's golden glint lands on top of its summon thud. */
const RADIANT_GLINT_DELAY_MS = 90;

const NONE: readonly SoundCue[] = [];

function sfx(id: SfxId, params?: SfxParams, delayMs = 0): SoundCue {
  return params === undefined ? { kind: "sfx", id, delayMs } : { kind: "sfx", id, params, delayMs };
}

function voice(defId: string, line: VoiceLineKind, delayMs: number, priority: VoicePriority): SoundCue {
  return { kind: "voice", defId, line, delayMs, priority };
}

/**
 * R655: a readable card's sounds for one hook at its moment: the hook's effect at `delayMs`, and its
 * line then, or CARD_EFFECT_DELAY_MS later when the effect leads it. None for a card behind the
 * sentinel or a hook the table does not give.
 */
export function hookCues(
  ctx: CueContext,
  defId: string,
  hook: CardHook,
  delayMs: number,
  priority: VoicePriority,
): readonly SoundCue[] {
  const assignment = hookFor(ctx.lines, defId, hook);
  if (assignment === null) return NONE;
  const cues: SoundCue[] = [];
  if (assignment.effect !== undefined) cues.push({ kind: "effect", defId, hook, delayMs, priority });
  if (assignment.voice !== undefined) {
    cues.push(voice(defId, hook, assignment.effect === undefined ? delayMs : delayMs + CARD_EFFECT_DELAY_MS, priority));
  }
  return cues;
}

/**
 * A unit arriving: a thud sized by the unit (attack plus health, so a 1/1 Sheep taps the table and a
 * 7/7 shakes it) with its family's accent, a Legendary or Mythic sting as the light rays rise, a
 * golden glint when it is Radiant, and, when no play of it has sounded (a token, a Recruit, a
 * Reborn, a copy), its play line at the lowest priority. Only a unit colours its thud: the backrow
 * holds face-down Traps, whose arrival must sound alike (R203).
 */
function summonCues(event: Extract<GameEvent, { type: "summoned" }>, ctx: CueContext): readonly SoundCue[] {
  const unit = ctx.unitNow?.(event.instanceId) ?? null;
  const card = event.row === "units" ? readable(ctx, event.defId) : undefined;
  const timbre = card === undefined ? undefined : timbreFor(card);
  const params: SfxParams = {};
  if (unit !== null) params.amount = unit.attack + unit.health;
  if (timbre !== undefined) params.timbre = timbre;
  const cues: SoundCue[] = [...slamSound(unit, card, params)];
  // B2.5: a token's printed rarity is the one it enters with; its `rarity` stays "Token".
  const rarity = card?.printedRarity ?? card?.rarity;
  if (rarity === "Legendary") cues.push(sfx("entrance"));
  else if (rarity === "Mythic") cues.push(sfx("entrance", { mythic: true }));
  if (unit?.radiant === true) cues.push(sfx("radiant", undefined, RADIANT_GLINT_DELAY_MS));
  const played = ctx.wasPlayed?.(event.instanceId) ?? false;
  if (!played && entryFor(ctx.lines, event.defId)?.kind === "unit") {
    cues.push(...hookCues(ctx, event.defId, "play", VOICE_DELAY_MS, VOICE_PRIORITY.summon));
  }
  return cues;
}

/**
 * #185: a Unit the viewer can read lands with its tier's weight: the summon thud from a soft tap to a
 * deep boom (pitch within SLAM_PITCH_SPREAD), and under a Huge or MASSIVE one an impact (the crack,
 * the boom). A Unit behind the sentinel, or one already gone from the newest view, keeps the plain
 * thud sized by its stats (R203).
 */
function slamSound(unit: UnitView | null, card: CueCard | undefined, base: SfxParams): readonly SoundCue[] {
  if (unit === null || card === undefined) return [sfx("summon", Object.keys(base).length === 0 ? undefined : base)];
  const tier = slamTier(slamStatsOf(unit, card.text));
  const variation = Math.random();
  const cues: SoundCue[] = [sfx("summon", { ...base, slamTier: tier, variation })];
  const impact = UNIT_SLAM[tier].impact;
  if (impact !== "none") cues.push(sfx("impact", { impactTier: impact, variation }));
  return cues;
}

/** R506: the readable play this event happens inside is `defId`'s (never true for the sentinel). */
function inPlayOf(ctx: CueContext, defId: string): boolean {
  return defId !== HIDDEN_DEF_ID && ctx.playing?.()?.defId === defId;
}

/**
 * R669: the sting of a card the viewer can read as it is played, by its rarity, or none: a Trap's
 * play is its set (R203), a token prints its rarity or has none, and a card cast as it is drawn has
 * its own sting already. A Legendary or Mythic Unit's sting is its entrance on `summoned`.
 */
function playSting(event: Extract<GameEvent, { type: "cardPlayed" }>, ctx: CueContext): readonly SoundCue[] {
  const card = readable(ctx, event.defId);
  if (card === undefined || card.type === "Trap" || card.type === "Field Trap") return NONE;
  if (ctx.castOnDraw?.(event.instanceId) === true) return NONE;
  const rarity = card.printedRarity ?? card.rarity;
  const tier = STING_TIERS[rarity ?? "Token"];
  if (tier !== undefined) return [sfx("sting", tier === "common" ? undefined : { tier }, STING_DELAY_MS)];
  if (card.type === "Unit") return NONE;
  if (rarity === "Legendary") return [sfx("entrance", undefined, STING_DELAY_MS)];
  if (rarity === "Mythic") return [sfx("entrance", { mythic: true }, STING_DELAY_MS)];
  return NONE;
}

/** R669: the rarities a play sting sizes itself by; Legendary and Mythic enter instead, a Token has none. */
const STING_TIERS: Readonly<Partial<Record<Rarity | PrintedRarity, StingTier>>> = {
  Common: "common",
  Rare: "rare",
  Epic: "epic",
};

/**
 * R506: a card cast as it is drawn arrives with the cast-on-draw sting in place of the play whoosh.
 * Only a card the viewer can read (the director never marks the sentinel), and never a Trap, whose
 * set sounds like every Trap's (R203).
 */
function arrival(event: Extract<GameEvent, { type: "cardPlayed" }>, ctx: CueContext): SoundCue {
  if (event.defId === HIDDEN_DEF_ID || ctx.castOnDraw?.(event.instanceId) !== true) return sfx("play");
  const type = readable(ctx, event.defId)?.type;
  return type === "Trap" || type === "Field Trap" ? sfx("play") : sfx("castOnDraw");
}

/** R436: the slot machine's spin and a ding for each effect the roll names (both seats read them). */
export function chaosRollCues(event: Extract<GameEvent, { type: "chaosRolled" }>): readonly SoundCue[] {
  return [sfx("chaosRoll", { amount: Math.min(CHAOS_REVEAL_MAX, event.effects.length) })];
}

/**
 * R437: a mark brands its card as it lands and lets go softly as it lifts. The same whatever the
 * mark, its colour or the card, which may be the sentinel: the sound says no more than the event.
 */
export function markCues(event: Extract<GameEvent, { type: "marked" }>): readonly SoundCue[] {
  return [event.added ? sfx("brand") : sfx("brand", { release: true })];
}

function silent(because: string): { sfx: null; silentBecause: string; cues: () => readonly SoundCue[] } {
  return { sfx: null, silentBecause: because, cues: () => NONE };
}

/** B5 E7: the difference from the health the view showed, as a heal or a drain; no change is a notice. */
function healthSetCues(event: Extract<GameEvent, { type: "healthSet" }>, ctx: CueContext): readonly SoundCue[] {
  const side = event.player === ctx.view.viewer ? ctx.view.you : ctx.view.opponent;
  const change = event.health - side.hero.health;
  if (change > 0) return [sfx("heal", { amount: change })];
  if (change < 0) return [sfx("drain", { amount: -change })];
  return [sfx("notify")];
}

/** Each impact gets an independent ±5% pitch sample, including repeated hits on the same target. */
function impactVariation(_event: Extract<GameEvent, { type: "damage" }>): number {
  return Math.random();
}

/**
 * R1364: the health a hit's target had before it, as the view the entry was planned against shows it
 * (a hero's or a unit's on the field: public on both seats), or null when the view shows neither.
 */
export function healthBefore(view: PlayerView, targetId: string): number | null {
  for (const side of [view.you, view.opponent]) {
    if (targetId === `hero-${side.player}`) return side.hero.health;
    const unit = side.units.find((u) => u !== null && u.instanceId === targetId);
    if (unit !== undefined && unit !== null) return unit.health;
  }
  return null;
}

/**
 * R1364: overkill, a hit that does far more than the health left: what it does beyond that health is
 * at least OVERKILL_MIN_EXCESS and at least the health itself (a 2-health unit taking 5, a 5 taking 10).
 */
export function isOverkill(amount: number, health: number | null): boolean {
  if (health === null || health <= 0) return false;
  const beyond = amount - health;
  return beyond >= Math.max(OVERKILL_MIN_EXCESS, health);
}

/** The hit's impact, with R1363's clank when Armor took half or more and R1364's crunch when it overkills. */
function damageCues(event: Extract<GameEvent, { type: "damage" }>, ctx: CueContext): readonly SoundCue[] {
  if (event.amount <= 0) return NONE;
  const cues: SoundCue[] = [
    sfx("impact", { amount: event.amount, impactTier: damageTier(event.amount), variation: impactVariation(event) }),
  ];
  if (armorTookHalf(event.absorbed, event.amount)) cues.push(sfx("armorClank"));
  if (isOverkill(event.amount, healthBefore(ctx.view, event.targetId))) cues.push(sfx("overkill", undefined, OVERKILL_DELAY_MS));
  return cues;
}

/**
 * R1366: the verb the event names (`how`): a card taken is a steal and one handed over a give
 * (R1423); a card a board move carried across (a swap, a rotation, a rollback) is the plain whoosh.
 * The field says which way the move went, never which card moved (R203).
 */
function controlCues(event: Extract<GameEvent, { type: "controlChanged" }>): readonly SoundCue[] {
  if (event.how === "steal") return [sfx("steal")];
  if (event.how === "give") return [sfx("give")];
  return [sfx("whoosh")];
}

/** R1365: a transform puffs; into a Sheep the viewer can read, the Sheep bleats as it comes out (R203). */
function transformCues(event: Extract<GameEvent, { type: "transformed" }>): readonly SoundCue[] {
  if (event.toDefId === HIDDEN_DEF_ID || !SHEEP_DEF_IDS.includes(event.toDefId)) return [sfx("poof")];
  return [sfx("poof"), sfx("bleat", undefined, BLEAT_DELAY_MS)];
}

export const SOUND_CUES: { readonly [K in GameEventType]: CueRow<K> } = {
  // R204: a unit's play line and a spell's cast line ride its `cardPlayed`, casts included. R203:
  // the viewer's own trap set makes the set sound and says nothing; a hidden card is a plain whoosh.
  cardPlayed: {
    sfx: "play",
    cues: (event, ctx) => {
      const kind = entryFor(ctx.lines, event.defId)?.kind;
      if (kind === "trap") return [sfx("trapSet")];
      const arrive = arrival(event, ctx);
      if (kind === "unit") {
        return [arrive, ...playSting(event, ctx), ...hookCues(ctx, event.defId, "play", VOICE_DELAY_MS, VOICE_PRIORITY.play)];
      }
      if (kind === "spell") {
        const card = readable(ctx, event.defId);
        const timbre = card === undefined ? undefined : timbreFor(card);
        return [
          arrive,
          ...playSting(event, ctx),
          sfx("spell", timbre === undefined ? undefined : { timbre }, SPELL_SHIMMER_DELAY_MS),
          ...hookCues(ctx, event.defId, "cast", VOICE_DELAY_MS, VOICE_PRIORITY.play),
        ];
      }
      return [arrive];
    },
  },
  cardResolved: silent("the effects a card resolves into carry their own events"),
  // R204: a unit an effect puts onto the field without playing it speaks its play line here.
  summoned: { sfx: "summon", cues: summonCues },
  // R1363, R1364: the hit's impact, clanking when Armor took half or more and crunching on overkill.
  damage: { sfx: "impact", cues: damageCues },
  healthLost: {
    sfx: "drain",
    cues: (event) => (event.amount > 0 ? [sfx("drain", { amount: event.amount })] : NONE),
  },
  healed: {
    sfx: "heal",
    cues: (event) => (event.amount > 0 ? [sfx("heal", { amount: event.amount })] : NONE),
  },
  divineShieldLost: { sfx: "shieldShatter", cues: () => [sfx("shieldShatter")] },
  // R204: a death line on `destroyed` (R89 carries the defId), tokens included; never on a bounce,
  // an exile or a transform, whose rows below have no voice. It takes the channel from a play line.
  destroyed: {
    sfx: "death",
    cues: (event, ctx) =>
      entryFor(ctx.lines, event.defId)?.kind === "unit"
        ? [sfx("death"), ...hookCues(ctx, event.defId, "death", DEATH_VOICE_DELAY_MS, VOICE_PRIORITY.react)]
        : [sfx("death")],
  },
  enteredGraveyard: silent("the destroy, discard or resolve that sent it there already sounded"),
  exiled: { sfx: "poof", cues: () => [sfx("poof")] },
  bounced: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  // R319: the three overflows (§2.4) each have their own sound, and none varies with the card, so a
  // card behind the sentinel sounds the same (R203); none is a moment that speaks (R204). A burn is
  // the full hand's; the hit after a `fatigue` sounds its own impact.
  burned: { sfx: "burn", cues: () => [sfx("burn")] },
  fatigue: { sfx: "fatigue", cues: () => [sfx("fatigue")] },
  libraryOverflow: { sfx: "refuse", cues: () => [sfx("refuse")] },
  discarded: { sfx: "draw", cues: () => [sfx("draw")] },
  drawn: { sfx: "draw", cues: () => [sfx("draw")] },
  addedToHand: { sfx: "draw", cues: () => [sfx("draw")] },
  shuffledIn: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  buffed: {
    sfx: "buff",
    cues: (event) => (event.attack + event.health >= 0 ? [sfx("buff")] : [sfx("debuff")]),
  },
  keywordGranted: { sfx: "buff", cues: () => [sfx("buff")] },
  counterChanged: { sfx: "uiClick", cues: () => [sfx("uiClick")] },
  costChanged: silent("the gem ticks visually, and a cost recomputed on every read (#100) would chatter"),
  // R506: #21 Hinder's rider landing on the victim's next refresh (added, or cancelling a rider
  // already there) cracks the crystal it takes; every other change notifies as it appears.
  modifierChanged: {
    sfx: "notify",
    cues: (event, ctx) => {
      if (event.modifierId === NEXT_REFRESH_MODIFIER_ID && inPlayOf(ctx, HINDER_DEF_ID)) return [sfx("manaCrack")];
      return event.added ? [sfx("notify")] : NONE;
    },
  },
  // R506: a card #27 turns Radiant is paid for in blood, then bursts gold; on the other seat the
  // event carries the sentinel and the same sound plays, read off the public cast alone (R203).
  radiantSet: {
    sfx: "radiant",
    cues: (_event, ctx) =>
      inPlayOf(ctx, BLOOD_BEAN_DEF_ID)
        ? [sfx("bloodDrain"), sfx("goldBurst", undefined, GOLD_BURST_DELAY_MS)]
        : [sfx("radiant")],
  },
  // R1365: a Sheep bleats as it comes out of the puff.
  transformed: { sfx: "poof", cues: transformCues },
  fused: { sfx: "fuse", cues: () => [sfx("fuse")] },
  positionSwitched: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  // R1366: a steal or a give says so; a board move carries a card across with a whoosh.
  controlChanged: { sfx: "whoosh", cues: controlCues },
  rotated: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  swapped: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  locked: { sfx: "lock", cues: () => [sfx("lock")] },
  // R203 + R154: only the controller's seat reads the trap, so only it hears the cast line. R17's
  // play-reactive traps fire on the very play they answer, so the line cuts in on that play's line.
  trapFired: {
    sfx: "trapSting",
    cues: (event, ctx) => [
      sfx("trapSting"),
      ...hookCues(ctx, event.defId, "cast", VOICE_DELAY_MS, VOICE_PRIORITY.react),
    ],
  },
  attackDeclared: { sfx: "attack", cues: () => [sfx("attack")] },
  attackCancelled: { sfx: "cancel", cues: () => [sfx("cancel")] },
  manaChanged: {
    sfx: "mana",
    cues: (event, ctx) => {
      const before = ctx.manaBefore(event.player);
      if (event.current <= before) return NONE;
      return [sfx("mana", { mine: event.player === ctx.view.viewer, amount: event.current - before })];
    },
  },
  turnStarted: {
    sfx: "turnStart",
    cues: (event, ctx) => [sfx("turnStart", { mine: event.player === ctx.view.viewer })],
  },
  turnEnded: silent("the end-turn click has its UI tick and the next turnStarted announces the change"),
  turnAutoEnded: { sfx: "notify", cues: () => [sfx("notify")] },
  promptOpened: {
    sfx: "notify",
    cues: (event, ctx) => (event.player === ctx.view.viewer ? [sfx("notify")] : NONE),
  },
  promptAnswered: silent("the answering click already ticked"),
  // §2.5, R36: the offer is a question for the other seat, so only that seat hears it, and it rings
  // as a question (the urgent notify, a doorbell) rather than a routine notice. The offerer clicked.
  drawOffered: {
    sfx: "notify",
    cues: (event, ctx) => (event.player !== ctx.view.viewer ? [sfx("notify", { urgent: true })] : NONE),
  },
  // The reply is the offerer's news: a decline falls away like a called-off attack, and an accepted
  // offer sounds nothing of its own because the `gameOver` right behind it sounds the draw. The
  // seat that answered has already heard its own click.
  drawAnswered: {
    sfx: "cancel",
    cues: (event, ctx) => (event.player === ctx.view.viewer || event.accept ? NONE : [sfx("cancel")]),
  },
  emoted: silent("the emote layer sounds it (R644)"),
  gameOver: {
    sfx: "victory",
    cues: (event, ctx) => {
      if (event.winner === "draw") return [sfx("notify")];
      return event.winner === ctx.view.viewer ? [sfx("victory")] : [sfx("defeat")];
    },
  },

  // ---- Patch v0.2.0 (docs/classic-sets.md B3, B5) ----
  // B5 E1: the announce is the play's own moment; its `cardPlayed` sounds and speaks once it lands.
  cardAnnounced: silent("the play it announces sounds on its cardPlayed; a countered one sounds on countered"),
  // B5 E1, R1365: a Counter slams the play shut and it fizzles out.
  countered: { sfx: "counterspell", cues: () => [sfx("counterspell")] },
  // B5 E2, E16, R1366: the card is snatched across the table.
  stolen: { sfx: "steal", cues: () => [sfx("steal")] },
  // R1365: the latch springs open, where `locked` clanked shut.
  unlocked: { sfx: "unlock", cues: () => [sfx("unlock")] },
  // B3.2: an ability is a small cast; the effects it resolves into carry their own sounds.
  activated: {
    sfx: "spell",
    cues: (event, ctx) => {
      const card = readable(ctx, event.defId);
      const timbre = card === undefined ? undefined : timbreFor(card);
      return [sfx("spell", timbre === undefined ? undefined : { timbre })];
    },
  },
  // B3.1: the card lands in its unit zone with a summon's thud, sized by the Unit it now is. It keeps no
  // family accent: an Animated Trap must arrive like every Trap (R203).
  animated: {
    sfx: "summon",
    cues: (event, ctx) => {
      const unit = ctx.unitNow?.(event.instanceId) ?? null;
      const card = readable(ctx, event.defId);
      return slamSound(unit, card, unit === null ? {} : { amount: unit.attack + unit.health });
    },
  },
  deanimated: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  // B3.3, R1365: a crumbling card cracks dry and falls apart: no death line, since nothing killed it.
  crumbled: { sfx: "crumble", cues: () => [sfx("crumble")] },
  // R1365, R440: a Nerf goes out of tune and a Buff tunes up, whatever the one change was.
  degraded: { sfx: "degrade", cues: () => [sfx("degrade")] },
  upgraded: { sfx: "upgrade", cues: () => [sfx("upgrade")] },
  numberChanged: { sfx: "uiClick", cues: () => [sfx("uiClick")] },
  redirected: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  // B5 E7: a hero's health set outright sounds the way it went, a heal or a drain of the difference.
  healthSet: { sfx: "drain", cues: healthSetCues },
  questProgressed: { sfx: "uiClick", cues: () => [sfx("uiClick")] },
  questCompleted: { sfx: "radiant", cues: () => [sfx("radiant"), sfx("notify")] },
  rolledBack: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  // R436: the roll is announced to both seats: a slot machine's spin, a ding for each effect named.
  chaosRolled: { sfx: "chaosRoll", cues: chaosRollCues },
  flickered: { sfx: "poof", cues: () => [sfx("poof")] },
  // B5 E3: the draw is called off, like an attack; R319 keeps the refusal sound the full library's own.
  drawLimited: { sfx: "cancel", cues: () => [sfx("cancel")] },
  turnCutShort: { sfx: "notify", cues: () => [sfx("notify", { urgent: true })] },
  // R437: a mark brands its card as it lands (#50's pending steal) and lets go softly as it lifts.
  marked: { sfx: "brand", cues: markCues },
  // R676: a Glitch tears the match: the rollback's rush with a shattering glass over it.
  glitched: { sfx: "whoosh", cues: () => [sfx("whoosh"), sfx("shieldShatter")] },
  translated: silent("a translation changes only the language a card is shown in (R1301)"),

  // ---- Patch v0.3.X (docs/meditative-set.md M8, MN05) ----
  // R1363: the Armor took the whole hit: a bright ring from the unit's lane (a hero's is centred).
  damageAbsorbed: { sfx: "armorRing", cues: () => [sfx("armorRing")] },
};

/**
 * R669: the unit an event is about, by instance id, for its pan: the one that arrives, is hit, dies,
 * attacks or is changed. An event about a hero, a hand or a player has none and stays centred.
 */
function unitOf(event: GameEvent): string | null {
  switch (event.type) {
    case "summoned":
      return event.row === "units" ? event.instanceId : null;
    case "damage":
    case "damageAbsorbed":
    case "healed":
      return event.targetId;
    case "fused":
      return event.resultInstanceId;
    case "attackDeclared":
      return event.attackerId;
    case "divineShieldLost":
    case "destroyed":
    case "buffed":
    case "keywordGranted":
    case "radiantSet":
    case "transformed":
    case "animated":
    case "crumbled":
    case "degraded":
    case "upgraded":
    case "controlChanged":
      return event.instanceId;
    default:
      return null;
  }
}

/**
 * R669: the pan of a unit in lane `index` (0-based) of `lanes`: the middle lane centred, the outer
 * ones LANE_PAN_MAX either way. Both rows run left to right in the same lane order on both seats'
 * screens, so the opponent's lane 1 sits above the viewer's lane 1.
 */
export function lanePan(index: number, lanes: number): number {
  if (lanes <= 1) return 0;
  const half = (lanes - 1) / 2;
  return ((index - half) / half) * LANE_PAN_MAX;
}

/** R669: where the unit stands in the view, as a pan, or null when it is on no units row. */
function panOf(view: PlayerView, instanceId: string): number | null {
  for (const side of [view.you, view.opponent]) {
    const index = side.units.findIndex((u) => u !== null && u.instanceId === instanceId);
    if (index >= 0) return lanePan(index, side.units.length);
  }
  return null;
}

/** R669: every effect about a unit on the field, panned to its lane; a centred one is left as it was. */
function panned(event: GameEvent, ctx: CueContext, cues: readonly SoundCue[]): readonly SoundCue[] {
  const id = unitOf(event);
  if (id === null || id === HIDDEN_DEF_ID) return cues;
  // The view the event was planned against holds a unit that leaves (a death); an arrival is only
  // in the newest one, which `unitNow` reads.
  let pan = panOf(ctx.view, id);
  if (pan === null) {
    const newest = ctx.newestView?.() ?? null;
    if (newest !== null) pan = panOf(newest, id);
  }
  if (pan === null || pan === 0) return cues;
  const at = pan;
  return cues.map((cue) => (cue.kind === "sfx" ? { ...cue, params: { ...cue.params, pan: at } } : cue));
}

/** The cues for one event, panned to the lane of the unit it is about (R669). */
export function cuesFor(event: GameEvent, ctx: CueContext): readonly SoundCue[] {
  const row = SOUND_CUES[event.type] as { cues: (e: GameEvent, c: CueContext) => readonly SoundCue[] };
  return panned(event, ctx, row.cues(event, ctx));
}
