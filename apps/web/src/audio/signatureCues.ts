// R3400001, R3405001: the signature sounds of Meditative+'s, Training Grounds' and Meditative's Legendaries
// and Mythics, beside the sound table's own (cues.ts `cuesFor` appends them after the event's row, as it does
// the Battleship's): a Spell's as its play is read (`cardPlayed`), a Unit's as it lands face up in a unit zone
// (`summoned`), the moment its signature's own cast or entrance starts. R3405000: a Field Spell that reaches
// the field without a play (The Cane, put there by Pareto Optimality) sounds as it lands face up in the
// backrow (a `summoned` into the backrow, R3407001's row). A row reads only the `defId` the event itself carries,
// so a hidden play (R97's sentinel), a card set face down (its `formerId`, R227) and every other card sound
// nothing more (R203).
// R3402001: Core's ten Legendaries and Mythics sound theirs the same way, on their first beat: a Trap's on
// its `trapFired` (its play is a face-down set, and a face-up Field Trap firing again reveals nothing,
// R2657001), Core's two Field Spells' on their `cardPlayed`.
// R3407001: Stinkyscape's 23 signature cards joined, one sound each, so that every card with a signature of
// its own also sounds its own; a Field Spell lands face up in the backrow with a `summoned` of its own, the
// moment its signature starts, so it sounds there (never on its `cardPlayed`, which would sound it twice),
// under the same R97 and R227 guards. R3395000: Stinkyscape 001 to 090's 24 Legendaries' and Mythics'
// signatures join them on the same terms, its three Field Spells' as they land in the backrow.

import type { GameEvent, Row } from "@jackioh/shared";

import { HIDDEN_DEF_ID } from "./constants.ts";
import type { SfxId, SoundCue } from "./types.ts";

/** A signature card's own sound and its moment: a Spell's play, a Unit's or Field Spell's landing in its row, or a Trap's firing. */
export type SignatureSound =
  | { readonly sfx: SfxId; readonly on: "cardPlayed" }
  | { readonly sfx: SfxId; readonly on: "summoned"; readonly row: Row }
  | { readonly sfx: SfxId; readonly on: "trapFired" };

/** Each signature card's own sound, and the event it sounds on. */
export const SIGNATURE_SFX: Readonly<Record<string, SignatureSound>> = {
  // Bob's Plan: a brush slapped on paper, a war-drum hit, a satisfied grunt.
  "meditativeplus-001": { sfx: "inkSummit", on: "cardPlayed" },
  // Bob, Wisconsin's Strongest Soldier: a snowplow revving, a grunt of effort, a deep thud.
  "meditativeplus-024": { sfx: "plowHeave", on: "summoned", row: "units" },
  // Scratch Pad: a cursor tick, keys clattering into silence, paper tearing.
  "meditativeplus-026": { sfx: "cursorTear", on: "cardPlayed" },
  // Seance: a murmured chant, candles guttering, a ghostly exhale.
  "traininggrounds-037": { sfx: "seanceChant", on: "cardPlayed" },
  // Tutorial Master: a pointer rapped on slate, a weary sigh, a chalk squeak.
  "traininggrounds-038": { sfx: "slateRap", on: "summoned", row: "units" },
  // Call to Chaos (Training-Grounds Edition): dice clattering, a held breath, a gong.
  "traininggrounds-039": { sfx: "diceGong", on: "cardPlayed" },
  // The Training-Grounds Golem: timber creaking, ropes snapping taut, a hollow thump.
  "traininggrounds-039-1": { sfx: "timberLash", on: "summoned", row: "units" },
  // Goetzmann Game Design: a pencil scratching, a napkin rustle, a rising arpeggio.
  "traininggrounds-040": { sfx: "pencilArpeggio", on: "cardPlayed" },
  // R3402001: Core's ten.
  "core-052": { sfx: "silasScratch", on: "summoned", row: "units" }, // Silly Silas
  "core-083": { sfx: "crucibleChoir", on: "cardPlayed" }, // Transmogulate
  "core-085": { sfx: "hatchWeld", on: "trapFired" }, // Unlicensed Experimentation
  "core-087": { sfx: "pocketZip", on: "cardPlayed" }, // Pocket Chaos
  "core-093": { sfx: "stadiumCheer", on: "cardPlayed" }, // Combo-Index
  "core-095": { sfx: "jackpotReels", on: "cardPlayed" }, // Call to Chaos (Core Edition)
  "core-096": { sfx: "pawnWaltz", on: "trapFired" }, // My Pawn
  "core-097": { sfx: "wishWind", on: "cardPlayed" }, // Zephrys
  "core-098": { sfx: "heroicFanfare", on: "cardPlayed" }, // Heroic Power
  "core-099": { sfx: "forgeAnvil", on: "cardPlayed" }, // Craft a Card
  // R3407001: Stinkyscape.
  // MELCHIOR: a sterile beep count, a ballot clack, a rising tone.
  "stinkyscape-018-1": { sfx: "ballotBeep", on: "summoned", row: "backrow" },
  // BALTHASAR: a soft monitor blip, a lullaby hum, a worried sigh.
  "stinkyscape-018-2": { sfx: "lullabySigh", on: "summoned", row: "backrow" },
  // CASPER: a compact snapping shut, a sultry chime, a tut.
  "stinkyscape-018-3": { sfx: "compactTut", on: "summoned", row: "backrow" },
  // xxFreshJack: a glitch stutter, a crowd roar, a bass drop.
  "stinkyscape-109": { sfx: "glitchDrop", on: "summoned", row: "units" },
  // iKwzki: a harp glissando, a warm two-note chime.
  "stinkyscape-110": { sfx: "harpGliss", on: "summoned", row: "units" },
  // Shockwave447: fast-forwarded chatter, a marker squeak, an electric thunderclap.
  "stinkyscape-111": { sfx: "markerThunder", on: "summoned", row: "units" },
  // Job Application: a pneumatic thunk, a desk-phone trill, a rubber stamp.
  "stinkyscape-112": { sfx: "stampTrill", on: "summoned", row: "backrow" },
  // Paycheck: a paper feed, a roulette rattle, a till drawer.
  "stinkyscape-112-1": { sfx: "tillRattle", on: "cardPlayed" },
  // Scornboi: a gavel crack, a scoff, a heavy hatch slam.
  "stinkyscape-115": { sfx: "gavelSlam", on: "summoned", row: "units" },
  // Megabase: a deep docking clank, a lobby chime, a coin cascade.
  "stinkyscape-116": { sfx: "dockingChime", on: "summoned", row: "units" },
  // IN33DH3: a keyboard flurry, a chirpy AI ping, a smug click.
  "stinkyscape-147": { sfx: "keyboardPing", on: "summoned", row: "units" },
  // IN33DH3 Prime: a low synth swell, a confident snap, a drum tap.
  "stinkyscape-147-1": { sfx: "synthSnap", on: "summoned", row: "units" },
  // Wardrum Blob: a squishy plop, a two-beat hand drum, a happy hum.
  "stinkyscape-147-2": { sfx: "blobDrum", on: "summoned", row: "backrow" },
  // Alteran Reach: a warp whoosh, a sparkling chime per node.
  "stinkyscape-148": { sfx: "warpSparkle", on: "summoned", row: "backrow" },
  // Edge of the Universe: paper tearing, a long held choir note, then silence.
  "stinkyscape-148-1": { sfx: "tearChoir", on: "cardPlayed" },
  // Wildar: a cackle, a crack of thunder, two coin flicks.
  "stinkyscape-149": { sfx: "cackleThunder", on: "cardPlayed" },
  // Call to Chaos (Stinky Edition): a wet squelch, a spinning ratchet, a kazoo fanfare.
  "stinkyscape-150": { sfx: "kazooRatchet", on: "cardPlayed" },
  // Stinkyscape Golem: a grinding rock thud, a thruster roar, a loud sniff.
  "stinkyscape-150-1": { sfx: "thrusterSniff", on: "summoned", row: "units" },
  // Call to Chaos (Scape Edition): a deep tilting groan, rolling marbles, a cosmic gong.
  "stinkyscape-151": { sfx: "marbleGong", on: "cardPlayed" },
  // Sanctum: a hushed choir breath, a page turn, a soft bell.
  "stinkyscape-152": { sfx: "sanctumBell", on: "summoned", row: "backrow" },
  // Stimmycheck: a cash-drawer ping, then a slowly rising whistle.
  "stinkyscape-153": { sfx: "cashWhistle", on: "cardPlayed" },
  // Narcor: a whispered breath, a shimmer, a soft clink.
  "stinkyscape-t-narcor": { sfx: "whisperClink", on: "cardPlayed" },
  // Red Narcor: a deep resonant hum rising to a bright crystal ring.
  "stinkyscape-t-red-narcor": { sfx: "crystalRise", on: "cardPlayed" },
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
  "meditative-039-5": { sfx: "jadeZither", on: "summoned", row: "units" },
  // Feng Shui: a wind chime, a soft gong, one guqin string.
  "meditative-040": { sfx: "windChime", on: "cardPlayed" },
  // CN Flea Market: a gong, crowd chatter, one abacus clack.
  "meditative-042": { sfx: "marketGong", on: "cardPlayed" },
  // Knowledge Breaker: chalk snapping, a long hush, a cracked porcelain chime.
  "meditative-045": { sfx: "chalkHush", on: "summoned", row: "units" },
  // Knowledge Breaker Prime: porcelain shattering, a deep roar, a rolling boom.
  "meditative-045-1": { sfx: "porcelainRoar", on: "summoned", row: "units" },
  // 饕餮: a deep bronze bell, a long inhale, a gulp.
  "meditative-047": { sfx: "bronzeGulp", on: "summoned", row: "units" },
  // YileGPT: a brush stroke, a soft modem chirp, a guqin note.
  "meditative-049": { sfx: "modemGuqin", on: "summoned", row: "units" },
  // YileGPT Prime: a thunderous brush stroke, a server hum, a choir of chirps.
  "meditative-049-1": { sfx: "serverChoir", on: "summoned", row: "units" },
  // Yile's Virus: a dial-up screech chopped into stutters.
  "meditative-049-2": { sfx: "dialupStutter", on: "summoned", row: "units" },
  // AI Girlfriend: a hologram chime, a soft bow, palms snapping a block.
  "meditative-049-3": { sfx: "holoBow", on: "summoned", row: "units" },
  // Devin Bot: a synth hum, then a quick run of pops.
  "meditative-051": { sfx: "synthPops", on: "summoned", row: "units" },
  // Skull of J'Nari: a rattling jaw, a ghostly cough, an interrupted mutter.
  "meditative-063": { sfx: "skullMutter", on: "cardPlayed" },
  // Keymaster Keenus: a key ring's jangle, then locks clicking open in turn.
  "meditative-065": { sfx: "keyJangle", on: "summoned", row: "units" },
  // Pareto Optimality: a cane whistling through the air, a sharp crack, a tut.
  "meditative-071": { sfx: "caneCrack", on: "cardPlayed" },
  // The Cane: thunder rolling, a lacquer creak, a sharp rap on wood; as it lands, since Pareto puts it there without a play.
  "meditative-071-1": { sfx: "caneThunder", on: "summoned", row: "backrow" },
  // Ever Growing Tree: creaking wood, a deep root groan, rustling leaves.
  "meditative-075": { sfx: "rootCreak", on: "cardPlayed" },
  // Touched by KY: a chalk squeak, a gasp, a rush of pages.
  "meditative-079": { sfx: "chalkGasp", on: "summoned", row: "units" },
  // Aluneth: whispered pages, a frosty chime, three wingbeats.
  "meditative-080": { sfx: "frostPages", on: "cardPlayed" },
  // Mayor Medinamogger: a coin's ping, a distant crowd cheering.
  "meditative-086": { sfx: "coinCheer", on: "summoned", row: "units" },
  // Tatches the Totem: five different drums, one after another.
  "meditative-087": { sfx: "totemDrums", on: "summoned", row: "units" },
  // Call to Chaos (Meditative Edition): a temple bell cut short by a crash, a rattling die.
  "meditative-095": { sfx: "templeDie", on: "cardPlayed" },
  // CN Golem: stone blocks locking, a lacquer creak, a ground-shaking stomp.
  "meditative-095-1": { sfx: "golemStomp", on: "summoned", row: "units" },
  // Jlockeed's Evil Blueprints: paper unrolling, a klaxon, gauntlets slamming down on steel.
  "meditative-097": { sfx: "klaxonSlam", on: "cardPlayed" },
  // Prison: an iron clang, rattling chains, a cell door slamming.
  "meditative-097-8": { sfx: "cellSlam", on: "summoned", row: "units" },
  // Jlockeed's Headquarters: hydraulic doors grinding, a radar ping, truck engines revving.
  "meditative-097-9": { sfx: "hangarRev", on: "summoned", row: "units" },
  // R3395000. Tylannt: vault tumblers clicking, then a heavy door's hiss.
  "stinkyscape-006": { sfx: "vaultHiss", on: "summoned", row: "units" },
  // The Greatest Heist: safe-dial ticks and a soft alarm chirp.
  "stinkyscape-007": { sfx: "safeChirp", on: "summoned", row: "backrow" },
  // Rod of Ages: a deep stone hum, a chime for each band.
  "stinkyscape-016": { sfx: "stoneChime", on: "summoned", row: "backrow" },
  // Imitated Instrumentality: three rising boot tones, then a cold electronic chord.
  "stinkyscape-018": { sfx: "magiChord", on: "cardPlayed" },
  // ZolarKeth: a lecture-hall bell, then a rocket's distant rumble.
  "stinkyscape-026": { sfx: "lectureRumble", on: "summoned", row: "units" },
  // Drone Core VIII: a factory press stamping faster and faster.
  "stinkyscape-034": { sfx: "pressStamp", on: "cardPlayed" },
  // Drone Core IX: three deep engine roars, one after another.
  "stinkyscape-035": { sfx: "corvetteRoar", on: "cardPlayed" },
  // Kwzka: a crowd's gasp and a bright pop-chorus sting.
  "stinkyscape-037": { sfx: "popGasp", on: "summoned", row: "units" },
  // The Narcor Project: glass clinking, a Geiger crackle, a low hum.
  "stinkyscape-039": { sfx: "geigerHum", on: "summoned", row: "backrow" },
  // Great Paperclip Theory: a soft metallic twang, then a rising, endless ratchet.
  "stinkyscape-044": { sfx: "wireRatchet", on: "cardPlayed" },
  // Beam v5: a rising whine, a deep beam hum, a cooling tick.
  "stinkyscape-058": { sfx: "beamWhine", on: "summoned", row: "units" },
  // Repeater v5: rapid rattling bursts rising in pitch, a steam hiss.
  "stinkyscape-059": { sfx: "rattleSteam", on: "summoned", row: "units" },
  // Cannon v5: a breech clank and one thunderous boom.
  "stinkyscape-060": { sfx: "breechBoom", on: "summoned", row: "units" },
  // Flak v5: three rolls of flak, then silence and a thin wind.
  "stinkyscape-061": { sfx: "flakWind", on: "summoned", row: "units" },
  // Miner v5: a grinding crunch, then a bright jackpot chime.
  "stinkyscape-066": { sfx: "crunchJackpot", on: "summoned", row: "units" },
  // Z'Zoth: a low choir hum and creaking hulls.
  "stinkyscape-070": { sfx: "choirHulls", on: "summoned", row: "units" },
  // A'Zoth: a brassy horn blast echoing back in layers.
  "stinkyscape-071": { sfx: "hornEcho", on: "summoned", row: "units" },
  // Paragon Mk2: a jet scream and a shield's glassy snap.
  "stinkyscape-073": { sfx: "jetSnap", on: "summoned", row: "units" },
  // Paragon-X: a white-noise roar, then a clean high ring.
  "stinkyscape-074": { sfx: "whiteRing", on: "summoned", row: "units" },
  // Galaxy: a deep hull groan, two drills whining up.
  "stinkyscape-080": { sfx: "hullDrills", on: "summoned", row: "units" },
  // Prototype-X1: a test-range klaxon and a sputtering engine catching.
  "stinkyscape-085": { sfx: "klaxonSputter", on: "summoned", row: "units" },
  // Prototype-X2: a molten roar, shrieking metal, a furious engine snarl.
  "stinkyscape-086": { sfx: "moltenSnarl", on: "summoned", row: "units" },
  // Siva 1: camera shutters and a smug purr.
  "stinkyscape-087": { sfx: "shutterPurr", on: "summoned", row: "units" },
  // Siva 2: a dark reboot sting and a hissing drip.
  "stinkyscape-088": { sfx: "venomDrip", on: "summoned", row: "units" },
};

const NONE: readonly SoundCue[] = [];

/** R3400001, R3405000, R3407001: the signature sound `event` starts, or none: another event or row, another card, or a card the viewer cannot read. */
export function signatureCues(event: GameEvent): readonly SoundCue[] {
  if (event.type !== "cardPlayed" && event.type !== "summoned" && event.type !== "trapFired") return NONE;
  if (event.defId === HIDDEN_DEF_ID) return NONE;
  // R3402001: a firing again has sounded its signature already (R2657001); a set face down is a set (R227).
  if (event.type === "trapFired" ? event.again === true : event.formerId !== undefined) return NONE;
  const own = Object.prototype.hasOwnProperty.call(SIGNATURE_SFX, event.defId) ? SIGNATURE_SFX[event.defId] : undefined;
  if (own === undefined || own.on !== event.type) return NONE;
  if (own.on === "summoned" && event.type === "summoned" && own.row !== event.row) return NONE;
  return [{ kind: "sfx", id: own.sfx, delayMs: 0 }];
}
