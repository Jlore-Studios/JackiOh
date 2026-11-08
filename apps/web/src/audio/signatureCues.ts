// R3400001, R3405001: the signature sounds of Meditative+'s, Training Grounds' and Meditative's Legendaries
// and Mythics, beside the sound table's own (cues.ts `cuesFor` appends them after the event's row, as it does
// the Battleship's): a Spell's as its play is read (`cardPlayed`), a Unit's as it lands face up in a unit zone
// (`summoned`), the moment its signature's own cast or entrance starts. R3405000: a Field Spell that reaches
// the field without a play (The Cane, put there by Pareto Optimality) sounds as it lands face up in the
// backrow (`landed`: a `summoned` into the backrow). A row reads only the `defId` the event itself carries,
// so a hidden play (R97's sentinel), a card set face down (its `formerId`, R227) and every other card sound
// nothing more (R203).

import type { GameEvent } from "@jackioh/shared";

import { HIDDEN_DEF_ID } from "./constants.ts";
import type { SfxId, SoundCue } from "./types.ts";

/** The moment a signature sounds: a play read, a Unit landing in a unit zone, or (R3405000) a card landing face up in the backrow. */
export type SignatureMoment = "cardPlayed" | "summoned" | "landed";

/** Each signature card's own sound, and the moment it sounds on: a Spell's play, a Unit's landing or a Field Spell's landing without a play. */
export const SIGNATURE_SFX: Readonly<Record<string, { sfx: SfxId; on: SignatureMoment }>> = {
  // Bob's Plan: a brush slapped on paper, a war-drum hit, a satisfied grunt.
  "meditativeplus-001": { sfx: "inkSummit", on: "cardPlayed" },
  // Bob, Wisconsin's Strongest Soldier: a snowplow revving, a grunt of effort, a deep thud.
  "meditativeplus-024": { sfx: "plowHeave", on: "summoned" },
  // Scratch Pad: a cursor tick, keys clattering into silence, paper tearing.
  "meditativeplus-026": { sfx: "cursorTear", on: "cardPlayed" },
  // Seance: a murmured chant, candles guttering, a ghostly exhale.
  "traininggrounds-037": { sfx: "seanceChant", on: "cardPlayed" },
  // Tutorial Master: a pointer rapped on slate, a weary sigh, a chalk squeak.
  "traininggrounds-038": { sfx: "slateRap", on: "summoned" },
  // Call to Chaos (Training-Grounds Edition): dice clattering, a held breath, a gong.
  "traininggrounds-039": { sfx: "diceGong", on: "cardPlayed" },
  // The Training-Grounds Golem: timber creaking, ropes snapping taut, a hollow thump.
  "traininggrounds-039-1": { sfx: "timberLash", on: "summoned" },
  // Goetzmann Game Design: a pencil scratching, a napkin rustle, a rising arpeggio.
  "traininggrounds-040": { sfx: "pencilArpeggio", on: "cardPlayed" },
  // R3405001: Meditative's.
  // Introspection: a deep inhale, a singing bowl's hum, a sharp exhale.
  "meditative-007": { sfx: "bowlBreath", on: "cardPlayed" },
  // Reach the Summit: wind over a peak, then three rising flute steps.
  "meditative-008": { sfx: "peakFlute", on: "cardPlayed" },
  // Double Header: a stadium horn, then a ticket tearing in two.
  "meditative-011": { sfx: "ticketHorn", on: "cardPlayed" },
  // True Craft a Card: soft keystrokes building in rhythm, then a forge's quench hiss.
  "meditative-017": { sfx: "forgeQuench", on: "cardPlayed" },
  // Aestheticize the Game: a long brush stroke on paper, a guzheng glissando.
  "meditative-020": { sfx: "brushGuzheng", on: "cardPlayed" },
  // Alternate Fate: a dry well's hollow echo, then a gush of water.
  "meditative-026": { sfx: "wellGush", on: "cardPlayed" },
  // Red Jade: a pick strike, a low mountain groan, a crystal ring.
  "meditative-039-4": { sfx: "minePick", on: "cardPlayed" },
  // Jade Beauty: stone cracking, a zither glissando, a sigh.
  "meditative-039-5": { sfx: "jadeZither", on: "summoned" },
  // Feng Shui: a wind chime, a soft gong, one guqin string.
  "meditative-040": { sfx: "windChime", on: "cardPlayed" },
  // CN Flea Market: a gong, crowd chatter, one abacus clack.
  "meditative-042": { sfx: "marketGong", on: "cardPlayed" },
  // Knowledge Breaker: chalk snapping, a long hush, a cracked porcelain chime.
  "meditative-045": { sfx: "chalkHush", on: "summoned" },
  // Knowledge Breaker Prime: porcelain shattering, a deep roar, a rolling boom.
  "meditative-045-1": { sfx: "porcelainRoar", on: "summoned" },
  // 饕餮: a deep bronze bell, a long inhale, a gulp.
  "meditative-047": { sfx: "bronzeGulp", on: "summoned" },
  // YileGPT: a brush stroke, a soft modem chirp, a guqin note.
  "meditative-049": { sfx: "modemGuqin", on: "summoned" },
  // YileGPT Prime: a thunderous brush stroke, a server hum, a choir of chirps.
  "meditative-049-1": { sfx: "serverChoir", on: "summoned" },
  // Yile's Virus: a dial-up screech chopped into stutters.
  "meditative-049-2": { sfx: "dialupStutter", on: "summoned" },
  // AI Girlfriend: a hologram chime, a soft bow, palms snapping a block.
  "meditative-049-3": { sfx: "holoBow", on: "summoned" },
  // Devin Bot: a synth hum, then a quick run of pops.
  "meditative-051": { sfx: "synthPops", on: "summoned" },
  // Skull of J'Nari: a rattling jaw, a ghostly cough, an interrupted mutter.
  "meditative-063": { sfx: "skullMutter", on: "cardPlayed" },
  // Keymaster Keenus: a key ring's jangle, then locks clicking open in turn.
  "meditative-065": { sfx: "keyJangle", on: "summoned" },
  // Pareto Optimality: a cane whistling through the air, a sharp crack, a tut.
  "meditative-071": { sfx: "caneCrack", on: "cardPlayed" },
  // The Cane: thunder rolling, a lacquer creak, a sharp rap on wood; as it lands, since Pareto puts it there without a play.
  "meditative-071-1": { sfx: "caneThunder", on: "landed" },
  // Ever Growing Tree: creaking wood, a deep root groan, rustling leaves.
  "meditative-075": { sfx: "rootCreak", on: "cardPlayed" },
  // Touched by KY: a chalk squeak, a gasp, a rush of pages.
  "meditative-079": { sfx: "chalkGasp", on: "summoned" },
  // Aluneth: whispered pages, a frosty chime, three wingbeats.
  "meditative-080": { sfx: "frostPages", on: "cardPlayed" },
  // Mayor Medinamogger: a coin's ping, a distant crowd cheering.
  "meditative-086": { sfx: "coinCheer", on: "summoned" },
  // Tatches the Totem: five different drums, one after another.
  "meditative-087": { sfx: "totemDrums", on: "summoned" },
  // Call to Chaos (Meditative Edition): a temple bell cut short by a crash, a rattling die.
  "meditative-095": { sfx: "templeDie", on: "cardPlayed" },
  // CN Golem: stone blocks locking, a lacquer creak, a ground-shaking stomp.
  "meditative-095-1": { sfx: "golemStomp", on: "summoned" },
  // Jlockeed's Evil Blueprints: paper unrolling, a klaxon, gauntlets slamming down on steel.
  "meditative-097": { sfx: "klaxonSlam", on: "cardPlayed" },
  // Prison: an iron clang, rattling chains, a cell door slamming.
  "meditative-097-8": { sfx: "cellSlam", on: "summoned" },
  // Jlockeed's Headquarters: hydraulic doors grinding, a radar ping, truck engines revving.
  "meditative-097-9": { sfx: "hangarRev", on: "summoned" },
};

const NONE: readonly SoundCue[] = [];

/** The moment `event` is: a play, a summon into a unit zone, or (R3405000) a summon into the backrow. */
function momentOf(event: Extract<GameEvent, { type: "cardPlayed" | "summoned" }>): SignatureMoment {
  if (event.type === "cardPlayed") return "cardPlayed";
  return event.row === "units" ? "summoned" : "landed";
}

/** R3400001, R3405000: the signature sound `event` starts, or none: another event or moment, another card, or a card the viewer cannot read. */
export function signatureCues(event: GameEvent): readonly SoundCue[] {
  if (event.type !== "cardPlayed" && event.type !== "summoned") return NONE;
  if (event.defId === HIDDEN_DEF_ID || event.formerId !== undefined) return NONE;
  const own = Object.prototype.hasOwnProperty.call(SIGNATURE_SFX, event.defId) ? SIGNATURE_SFX[event.defId] : undefined;
  if (own === undefined || own.on !== momentOf(event)) return NONE;
  return [{ kind: "sfx", id: own.sfx, delayMs: 0 }];
}
