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
// R651: those moments are a card's hooks (`play`, `death`, `cast`), and a hook in `card-audio.json5`
// gives a line, a named effect, or both. The effect plays at the moment the line would speak, and a
// line an effect leads waits CARD_EFFECT_DELAY_MS more, so the sound reads as what the card does and
// the line as its reaction. The effect is read off the same readable entry as the line, so R203 holds
// for both. The `attack` hook is no event's: the board plays it when the viewer picks a Unit up
// (`usePickupSound`).

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
import {
  BLOOD_BEAN_DEF_ID,
  CARD_EFFECT_DELAY_MS,
  CHAOS_REVEAL_MAX,
  DEATH_VOICE_DELAY_MS,
  GOLD_BURST_DELAY_MS,
  HIDDEN_DEF_ID,
  HINDER_DEF_ID,
  NEXT_REFRESH_MODIFIER_ID,
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
  VoiceLineKind,
  VoicePriority,
} from "./types.ts";
import { entryFor, hookFor } from "./voiceData.ts";

/**
 * The public catalog facts a cue may colour itself with (§5.1): never looked up for "hidden".
 * `printedRarity` is a token's printed rarity (B2.5), for its summon sting only.
 */
export type CueCard = { type: CardType; tags: readonly Tag[]; rarity?: Rarity; printedRarity?: PrintedRarity };

/**
 * R506: a play in progress, as the director follows the stream: from its `cardPlayed` until its
 * `cardResolved`. `defId` is the sentinel for a card the viewer cannot read, which no card moment
 * matches. `castOnDraw`: the play is a card cast as it was drawn.
 */
export type PlayFrame = { defId: string; instanceId: string; player: PlayerId; castOnDraw: boolean };

export type CueContext = {
  /** The view the batch was planned against (pre-batch): `viewer` and seat orientation come from here. */
  view: PlayerView;
  /** The card sound table (R651): every card's hooks, with the voices and effects they name. */
  lines: CardAudioTable;
  /** Current mana this player had before this event, as the director tracks it. */
  manaBefore: (player: PlayerId) => number;
  /**
   * True when this instance's own `cardPlayed` has already sounded, so its `summoned` is that play
   * arriving and its line has been spoken. Absent: no play has sounded.
   */
  wasPlayed?: (instanceId: string) => boolean;
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
/** A crumbling card falls this long after it shatters (B3.3). */
const CRUMBLE_FALL_DELAY_MS = 70;
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
 * R651: a readable card's sounds for one hook at its moment: the hook's effect at `delayMs`, and its
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
  const cues: SoundCue[] = [sfx("summon", Object.keys(params).length === 0 ? undefined : params)];
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

/** R506: the readable play this event happens inside is `defId`'s (never true for the sentinel). */
function inPlayOf(ctx: CueContext, defId: string): boolean {
  return defId !== HIDDEN_DEF_ID && ctx.playing?.()?.defId === defId;
}

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

export const SOUND_CUES: { readonly [K in GameEventType]: CueRow<K> } = {
  // R204: a unit's play line and a spell's cast line ride its `cardPlayed`, casts included. R203:
  // the viewer's own trap set makes the set sound and says nothing; a hidden card is a plain whoosh.
  cardPlayed: {
    sfx: "play",
    cues: (event, ctx) => {
      const kind = entryFor(ctx.lines, event.defId)?.kind;
      if (kind === "trap") return [sfx("trapSet")];
      const arrive = arrival(event, ctx);
      if (kind === "unit") return [arrive, ...hookCues(ctx, event.defId, "play", VOICE_DELAY_MS, VOICE_PRIORITY.play)];
      if (kind === "spell") {
        const card = readable(ctx, event.defId);
        const timbre = card === undefined ? undefined : timbreFor(card);
        return [
          arrive,
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
  damage: {
    sfx: "impact",
    cues: (event) => (event.amount > 0 ? [sfx("impact", { amount: event.amount })] : NONE),
  },
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
  transformed: { sfx: "poof", cues: () => [sfx("poof")] },
  fused: { sfx: "poof", cues: () => [sfx("poof")] },
  positionSwitched: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  controlChanged: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
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
  // B5 E1: a Counter snuffs the card out like a called-off attack.
  countered: { sfx: "cancel", cues: () => [sfx("cancel"), sfx("poof", undefined, SPELL_SHIMMER_DELAY_MS)] },
  // B5 E2, E16: the card is whisked across the table.
  stolen: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  unlocked: { sfx: "lock", cues: () => [sfx("lock")] },
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
      return [sfx("summon", unit === null ? undefined : { amount: unit.attack + unit.health })];
    },
  },
  deanimated: { sfx: "whoosh", cues: () => [sfx("whoosh")] },
  // B3.3: a crumbling card shatters like glass, then falls: no death line, since nothing killed it.
  crumbled: { sfx: "death", cues: () => [sfx("shieldShatter"), sfx("death", undefined, CRUMBLE_FALL_DELAY_MS)] },
  degraded: { sfx: "debuff", cues: () => [sfx("debuff")] },
  upgraded: { sfx: "buff", cues: () => [sfx("buff")] },
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
};

/** The cues for one event. */
export function cuesFor(event: GameEvent, ctx: CueContext): readonly SoundCue[] {
  const row = SOUND_CUES[event.type] as { cues: (e: GameEvent, c: CueContext) => readonly SoundCue[] };
  return row.cues(event, ctx);
}
