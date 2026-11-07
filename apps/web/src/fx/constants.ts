// Every number the effects layer uses (CLAUDE.md rule 9; docs/polish/1-animations.md, Surface S2), binding:
// other slices test against them. Recipe counts live in `cues.ts`'s `TUNING`, preset physics in `presets.ts`.

export const FX_MAX_TAIL_MS = 900;            // T: nothing an entry starts outlives entry end + T
export const FX_MAX_PARTICLE_LIFE_MS = 900;   // every preset's max life, ≤ FX_MAX_TAIL_MS
export const FX_SPLAT_HOLD_MS = 650;          // splat duration = (D − delay) + hold
export const FX_RAYS_TAIL_MS = 600;           // rays duration  = (D − delay) + tail
export const FX_ARROWS_TAIL_MS = 300;
export const FX_CRACK_TAIL_MS = 400;
export const FX_BANNER_TAIL_MS = 900;         // entry banner duration = D + tail
export const FX_RING_MS = 500;                // ring duration = min(FX_RING_MS, D − delay + T)
export const FX_HANDOVER_BANNER_MS = 1400;
export const FX_RESULT_MS = 3200;             // every planResult cue ends by this
export const FX_PROJECTILE_FLIGHT_FRACTION = 0.55;
export const FX_MIND_CONTROL_FLIGHT_FRACTION = 0.7;
export const FX_FUSE_FLIGHT_FRACTION = 0.5;
export const FX_SLAM_AT = 0.6;
export const FX_HOLD_MAX_MS = 6000;           // a stage effect's safety cap; it normally ends when the board shows the next view (R200)
export const FX_CONCEAL_AT = 0.9;             // an "after" conceal is set at 0.9 D, while the card's own motion still runs
export const FX_LUNGE_STANDOFF = 0.55;        // the lunge stops this many combined half-extents short of the target's center (< 1: they overlap)
export const FX_LUNGE_MIN_PX = 26;            // = animations.css --lunge-distance (BUILD: at least 20 px)
export const FX_LUNGE_MAX_PX = 520;
export const FX_LUNGE_CONTACT_AT = 0.7;       // the contact beat inside attackDeclared (jk-lunge's 70% keyframe)
export const FX_HEAL_SPLAT_AT = 0.2;
export const FX_DEATH_EMBER_AT = 0.3;
export const FX_DEATH_SMOKE_AT = 0.5;
export const FX_RADIANT_BURST_AT = 0.4;
export const FX_TRAP_BURST_AT = 0.3;           // trapFired's reveal, jk-trap-flip's 30% stop, after its beat of anticipation (R2111000)
export const FX_BURN_AT = 0.25;
export const FX_FATIGUE_STREAK_AT = 0.35;      // the fatigue hit leaves the empty library at 0.35 D…
export const FX_FATIGUE_FLIGHT_FRACTION = 0.6; // …and reaches the hero at 0.95 D, inside the entry (R200)
export const FX_OVERFLOW_FIZZLE_AT = 0.45;     // a refused card fizzles at 0.45 D, after it bounces off the pile
export const FX_MANA_STAGGER_MS = 40;
export const FX_MANA_MAX_SPARKS = 10;
export const FX_SLAM_STATS_MIN = 10;          // printed attack + health
export const FX_SLAM_TRAUMA_PER_STAT = 0.06;
export const FX_SLAM_MAX_TRAUMA = 0.5;
/** R2722001: a Legendary's or a Mythic's entrance lands this hard (0.5 until R2722001 made it bigger). */
export const FX_LEGENDARY_TRAUMA = 0.7;
/** R2722001: a Mythic's takeover lasts its entry and this much more, inside FX_MAX_TAIL_MS (R200). */
export const FX_TAKEOVER_TAIL_MS = 700;
/** R2722001: the takeover's shake as its card lands (R2803000: eased for the phone frame budget; the result still thumps hardest). */
export const FX_TAKEOVER_TRAUMA = 0.6;
export const FX_TRAP_TRAUMA = 0.4;
export const FX_RESULT_TRAUMA = 0.9;
export const FX_LETHAL_LEAD_MAX_MS = 600;     // the killing blow replayed before the result plays in at most this (R200)
export const FX_SHAKE_MAX_PX = 18;
export const FX_SHAKE_MAX_DEG = 1.2;
export const FX_SHAKE_FREQ_HZ = 18;
export const FX_TRAUMA_DECAY = 1.2;           // trauma per second; a full shake (1) is spent in 833 ms, inside FX_MAX_TAIL_MS
export const FX_PARTICLE_CAP = 600;
export const FX_PARTICLE_CAP_MOBILE = 260;
export const FX_PARTICLE_CAP_MIN = 120;
export const FX_MOBILE_WIDTH = 600;           // CSS px
export const FX_MAX_DPR = 2;
export const FX_MAX_DT_MS = 50;
export const FX_ADAPT_WINDOW = 30;            // frames
export const FX_ADAPT_SLOW_MS = 24;           // the least mean raw frame time that counts as slow
export const FX_ADAPT_SLOW_FACTOR = 1.4;      // …and slow also means this many times the display's own frame interval
export const FX_ADAPT_DISPLAY_MAX_MS = 34;    // the slowest interval taken for a display's refresh (30 Hz); slower is load
export const FX_ADAPT_MIN_INTERVAL_MS = 4;    // shorter raw intervals are not a display's refresh (240 Hz is 4.2 ms)
export const FX_ADAPT_RECOVER_WINDOWS = 3;    // healthy windows in a row that double a lowered cap back up
export const FX_DEFAULT_SEED = 0x5eed;
export const FX_MEMORY_LIMIT = 64;            // entries each FxMemory map keeps (oldest evicted)
export const FX_MEMORY_RECENT = 16;           // R502: the last events the planner keeps in order (a cast on draw is read off them)
export const FX_MEMORY_RESOLVING = 8;         // R502: the plays still resolving it keeps, innermost last
export const FX_MEMORY_HITS = 44;             // R2784000: the hit ids each keeps, a damage and a removal of all 20 zones' cards and both heroes

// R502: a cast on draw. The card bursts out of its drawer's Deck pile as its `cardPlayed` starts.
export const FX_CAST_ON_DRAW_TRAUMA = 0.25;
// R502: #21 Hinder's refresh loss. Each crystal the next refresh loses fractures from the frost bolt's
// landing, FX_CRACK_STAGGER_MS apart but never past the entry's end (R200).
export const FX_CRACK_HIT_AT = 0.45;
export const FX_CRACK_STAGGER_MS = 50;
export const FX_FRACTURE_TAIL_MS = 800;       // a fracture lasts (D − delay) + this, inside FX_MAX_TAIL_MS
export const FX_CRACK_TRAUMA = 0.3;
/** The id the engine gives the next refresh's rider (`NEXT_REFRESH_MODIFIER_ID`, R169): #21 Hinder's badge. */
export const FX_NEXT_REFRESH_MODIFIER_ID = "nextTurnMana";
// R502: #27 Blood Ridden Glowy Jelly Bean. On the other seat its stream lands on a back chosen by a
// counter of the planner's own (R202), never the hidden card's place.
export const FX_BLOOD_FLIGHT_FRACTION = 0.6;
export const FX_BLOOD_PICK_BASE = 1;          // the first back a hidden pick lands on (modulo the hand)
export const FX_BLOOD_PICK_STRIDE = 2;        // how far each later pick of the same play moves on
export const FX_BLOOD_TRAUMA = 0.2;
// Classic+ #24 Crushing Walls: the walls meet the cards at FX_WALLS_HIT_AT of the cue (fx.css's
// `fx-walls-close` stop of the same name); the cue lasts D + FX_WALLS_TAIL_MS, inside FX_MAX_TAIL_MS (R200).
export const FX_WALLS_HIT_AT = 0.3;
export const FX_WALLS_TAIL_MS = 850;
export const FX_WALLS_REACH = 0.2;            // how far each wall reaches in, of the board's width: one lane of five
export const FX_WALLS_TRAUMA = 0.45;
// R2795000: Classic+ #35 Rollback rewinds the lanes from 5 to 1, each ring this fraction of D after the
// last (all of them inside Travel to Land); Classic+ #56 Book of Pain shakes this hard at its first tear.
export const FX_ROLLBACK_STEP = 0.08;
export const FX_INK_TEAR_TRAUMA = 0.2;
// #2796: Meditative's Flourishes (fx/cards/meditative/, meditativeplus/): each one's shake at Land.
export const FX_JADE_CRACK_TRAUMA = 0.3;
export const FX_HARMONY_TRAUMA = 0.15;
export const FX_CN_SWEEP_TRAUMA = 0.35;
// R2794000: Classic #85 King Wagtoggle's crown swap shakes as the two decks land in each other's place.
export const FX_CROWN_SWAP_TRAUMA = 0.2;
// R437: a mark branded onto a card; its sigil fades over FX_BRAND_TAIL_MS after the entry.
export const FX_BRAND_SLAM_AT = 0.35;
export const FX_BRAND_TAIL_MS = 700;
// R1363 (MN05): Armor's shield flashes from the hit's landing and fades over this after the entry.
export const FX_SHIELD_TAIL_MS = 450;
// R436: Call to Chaos's reveal. Line i lands at (FX_CHAOS_LAND_AT + i × stagger) of the entry, the
// stagger shrinking so the last line lands by FX_CHAOS_LAND_LAST. The reveal lasts D + FX_BANNER_TAIL_MS.
export const FX_CHAOS_LAND_AT = 0.4;
// docs/classic-sets.md B3, B5's events, each a fraction of its entry (R200).
export const FX_FLICKER_RETURN_AT = 0.5;       // a flickered card comes back through the void halfway in
export const FX_REDIRECT_FLIGHT_FRACTION = 0.6; // a redirected hit flies from its old target to its new one
export const FX_COUNTER_TRAUMA = 0.25;         // a countered card shatters with a small shake
export const FX_REWIND_TRAUMA = 0.35;          // the board rewinding (Classic+ #35 Rollback)
export const FX_CHAOS_STAGGER = 0.2;
export const FX_CHAOS_LAND_LAST = 0.85;
export const FX_CHAOS_REEL_DECOYS = 6;
export const FX_CHAOS_DECOY_STEP = 3;          // how far apart in the names table a reel's decoys are
// R2652001: a batch of coin flips or die rolls, one panel. Item i of n lands at (FX_ROLL_LAND_AT +
// i × stagger) of the entry, the stagger shrinking so the last lands by FX_ROLL_LAND_LAST, every coin
// spinning and every die tumbling through FX_ROLL_TUMBLE_FACES faces first. It lasts D + FX_BANNER_TAIL_MS.
export const FX_ROLL_LAND_AT = 0.5;
export const FX_ROLL_STAGGER = 0.05;
export const FX_ROLL_LAND_LAST = 0.7;
export const FX_ROLL_TUMBLE_FACES = 8;
export const FX_ROLL_TUMBLE_STRIDE = 3;        // a die's walk through its faces, item i stepping this + i
// Issue #124: a sweep's fog rolls over its row in one entry (SWEEP_MS), each unit's hit landing as the
// fog reaches its lane; it spreads over the row grown by FX_FOG_PAD of its height on every side.
export const FX_FOG_HIT_FROM = 0.2;
export const FX_FOG_HIT_TO = 0.75;
export const FX_FOG_TAIL_MS = 500;
export const FX_FOG_PUFFS = 7;
export const FX_FOG_ICONS = 6;
export const FX_FOG_PAD = 0.15;
export const FX_ZONE_COUNT_AT = 0.35;
export const FX_ZONE_TAIL_MS = 400;
// Issue #124: a cast another card made flares at its caster's hero, bigger with each cast of the same burst.
export const FX_CAST_SCALE_STEP = 0.06;
export const FX_CAST_SCALE_MAX = 1.6;
export const FX_CAST_TRAUMA = 0.12;
// #1442, #1444: the shot leaves at jk-lunge's 24% keyframe and lands at the entry's end
export const FX_FLAVOR_LIVE: boolean = true;
export const FX_SHOT_LAUNCH_AT = 0.24;
export const FX_SHOT_SHAKE_TRAUMA = 0.3;
// #1443: the per-entry cap on non-combat damage shots (one volley starts together).
export const FX_FLAVOR_VOLLEY_MAX = 4;
// R2784000: a card's motif (fx/motifs/). Its four phases start at these fractions of the entry, as
// FX_SLAM_AT does; its scale is the card's rarity × (1 + capped cost × step), at most FX_MOTIF_SCALE_MAX,
// times the intensity (an X cost counts as FX_MOTIF_X_COST); each motif's shake is FX_MOTIF_TRAUMA's.
export const FX_MOTIF_PHASES = { gather: 0, travel: 0.2, land: 0.6, linger: 0.75 } as const;
export const FX_MOTIF_SCALE_PER_COST = 0.08;
export const FX_MOTIF_COST_CAP = 7;
export const FX_MOTIF_X_COST = 3;
export const FX_MOTIF_SCALE_MAX = 2;
export const FX_MOTIF_RARITY_SCALE = { Common: 1, Rare: 1.1, Epic: 1.25, Legendary: 1.4, Mythic: 1.6, Token: 0.9 } as const;
export const FX_MOTIF_VOLLEY_MAX = 5;          // volley: the shots one entry fires
export const FX_MOTIF_BARRAGE_MAX = 8;         // barrage: the hits that land one by one…
export const FX_MOTIF_BARRAGE_STEP = 0.02;     // …this fraction of D apart, never past linger
export const FX_MOTIF_CENTER_LANE = 3;         // nova: the lane a one-sided blast centres on
export const FX_MOTIF_TRAUMA = {
  bolt: 0.25,
  volley: 0.3,
  barrage: 0.35,
  lance: 0.4,
  nova: 0.45,
  wipe: 0.6,
  vanish: 0,
  scorch: 0.3,
  negate: 0.25,
  drain: 0.3,
  bless: 0.2,
  curse: 0.25,
  conjure: 0.3,
  rise: 0.35,
  offering: 0.3,
  shift: 0.15,
  bind: 0.2,
  tether: 0.25,
  transmute: 0.3,
  flight: 0,
  mana: 0,
  tithe: 0.1,
  tally: 0.15,
  snap: 0.4,
  send: 0,
  radiance: 0.15,
  mend: 0,
  deal: 0,
  seal: 0.2,
  recall: 0,
  hex: 0.15,
} as const;
// R2803000: Armor's and a Divine Shield's shake where the folded `ward` drew (cues.ts armor,
// shieldBreak), in Holy. A judged guess's shake (bespoke `mindJudge`) stays `glimpse`'s.
export const FX_ARMOR_TRAUMA = 0.2;
export const FX_MIND_JUDGE_TRAUMA = 0.1;
// R2797000: Stinkyscape's three Flourishes (fx/cards/stinkyscape/). Olivian Expanse's rift shakes as it
// gives up its card; Imitated Instrumentality's three boots leave FX_MAGI_STAGGER of D apart, at most
// FX_MAGI_BOOTS of them staggered, the first shaking the board; Drone Core X's Battleship warps in from
// FX_WARP_FROM of the board (its top edge) and shakes as it lands.
export const FX_TERMINUS_TRAUMA = 0.5;
export const FX_MAGI_STAGGER = 0.05;
export const FX_MAGI_BOOTS = 3;
export const FX_MAGI_TRAUMA = 0.3;
export const FX_WARP_TRAUMA = 0.6;
export const FX_WARP_FROM = { x: 0.5, y: 0 } as const;
/** R2784000: `tithe` glints by count on hidden backs, at most this many (R202). */
export const FX_MOTIF_TITHE_MAX = 5;
export const FX_MOTIF_DEAL_MAX = 5; // deal: the cards one entry deals
// #2793: Core's flourishes (fx/cards/core/): each shake at Land; how far in from the board's edges Craft a Card's halves start.
export const FX_TIDE_TRAUMA = 0.35;
export const FX_WISH_TRAUMA = 0.3;
export const FX_FORGE_TRAUMA = 0.4;
export const FX_FORGE_REACH = 0.2;
// #2798: Stinkyscape's flourishes (fx/cards/stinkyscape/): each shake at Land.
export const FX_FLASH_FREEZE_TRAUMA = 0.35;
export const FX_DUO_BOND_TRAUMA = 0.3;
export const FX_STRATOS_SWARM_TRAUMA = 0.3;
// #2799: Training Grounds' flourishes (fx/cards/traininggrounds/): each shake at Land.
export const FX_CELESTIAL_TRAUMA = 0.35;
export const FX_DINO_STOMP_TRAUMA = 0.55;
export const FX_GAME_DESIGN_TRAUMA = 0.45;
// #2793: the share of a set's non-token cards its pass gives an entry other than `none`.
export const FX_SET_PASS_FLOOR = 0.55;
// R2803000: the audit's floor. At least this share of the catalog's non-token cards wears an entry
// other than `none`, so the audit's coverage test holds every pass to half the catalog.
export const FX_COVERAGE_FLOOR = 0.5;
// R2803000: the audit's motif floor. A drawn motif fewer than this many table entries use is folded
// into bespoke work or its nearest motif, as the design doc's rule says.
export const FX_MOTIF_MIN_CARDS = 3;
// R2803000: the audit's phone budget. The heaviest set pieces and a board of four auras hold this
// mean frame time at the phone particle cap with mobile emulation.
export const FX_PHONE_FRAME_BUDGET_MS = 50;
// R2784000: the scales the /dev/fx gallery offers.
export const FX_GALLERY_SCALES = [0.5, 1, 1.5, 2] as const;
// R3332000: the signature tier's loudness budgets, inside R200's FX_MAX_TAIL_MS, one set piece per action
// and SET_PIECE_HOLD_CAP_MS (R2785000). A Legendary's signature ends within SIGNATURE_LEGENDARY_MAX_MS of
// its play's start, a Mythic's, its takeover's hold included, within SIGNATURE_MYTHIC_MAX_MS. A Spell's, seen
// only briefly, ends within SIGNATURE_SPELL_SHARE of its rarity's and shakes the loudest
// (SIGNATURE_SPELL_MAX_TRAUMA); a permanent's (a Unit, a Field Spell, a Trap) at most
// SIGNATURE_PERMANENT_MAX_TRAUMA. A signature aura's look stays within AURA_SIGNATURE_MAX_INTENSITY in its
// mote alpha, its tint and its edge glow, the quietest once in play.
export const SIGNATURE_LEGENDARY_MAX_MS = 1_600;
export const SIGNATURE_MYTHIC_MAX_MS = 2_600;
export const SIGNATURE_SPELL_SHARE = 0.75;
export const SIGNATURE_SPELL_MAX_TRAUMA = 1;
export const SIGNATURE_PERMANENT_MAX_TRAUMA = 0.8;
export const AURA_SIGNATURE_MAX_INTENSITY = 0.8;
// R3332001: Felinor Fiender's assembly (fx/cards/core/felinorFiender.ts). The first Felinor's streak of
// cat light leaves at FX_FELINOR_STREAK_AT of D, each next one FX_FELINOR_STREAK_STEP later, all landing
// at FX_SLAM_AT; at most FX_FELINOR_STREAKS_MAX streak, one per unit zone of a side.
export const FX_FELINOR_STREAK_AT = 0.1;
export const FX_FELINOR_STREAK_STEP = 0.08;
export const FX_FELINOR_STREAKS_MAX = 5;
// R2787000: a card's aura (fx/aura/), the standing look of a Field Spell or a lasting modifier. At most
// AURA_MAX_PER_ZONE cover a zone; each draws at most AURA_MOTES_PER_AURA motes (times the intensity),
// all of them together at most AURA_PARTICLE_SHARE of the particle cap. It fades in over AURA_FADE_IN_MS
// with a burst (px/s, decaying by AURA_BURST_DECAY per second) and out over AURA_FADE_OUT_MS; the layer
// remembers the source of at most AURA_SOURCES_LIMIT modifiers (oldest evicted).
export const AURA_MAX_PER_ZONE = 2;
export const AURA_MOTES_PER_AURA = 24;
export const AURA_PARTICLE_SHARE = 0.25;
export const AURA_FADE_IN_MS = 600;
export const AURA_FADE_OUT_MS = 500;
export const AURA_BURST_SPEED = 220;
export const AURA_BURST_DECAY = 4;
export const AURA_SOURCES_LIMIT = 64;
// R2789000: the combat keyword beats (fx/motifs/beats.ts). A callout is the keyword's icon over the unit; a
// hit's beats start FX_BEAT_STAGGER_MS apart (a later one at most FX_BEAT_STAGGER_MAX of D in), at most
// FX_CALLOUTS_PER_HIT of them each with a callout. The fractions are of D, the lengths in ms, and all of it
// lies inside the entry (R200).
export const FX_CALLOUT_MS = 600;
export const FX_CALLOUTS_PER_HIT = 2;
export const FX_BEAT_STAGGER_MS = 60;
export const FX_BEAT_STAGGER_MAX = 0.5;
export const FX_BEAT_TRAUMA = 0.25;
export const FX_FIRST_STRIKE_BREAK_AT = 0.4;   // the dying defender's counter-blow breaks off this far into the entry…
export const FX_FIRST_STRIKE_BREAK_REACH = 0.6; // …this many card heights toward the attacker
export const FX_PIERCE_REACH = 0.8;            // a Pierce's spear line runs this many card heights past each edge
export const FX_FLYING_LUNGE_LIFT = 0.18;      // a Flying attacker's lunge scales up this much more, over the lane
// R2791000: the tempo keyword beats (fx/motifs/beats.ts). A Stack landing bursts up-right of the
// pile top; a Deft feint sidesteps this many card widths; a Charge landing's speed lines spread
// this far across the hand region they leave from.
export const FX_STACK_LAND = { x: 0.62, y: 0.35 } as const;
export const FX_DEFT_FEINT_STEP = 0.18;
export const FX_CHARGE_LINE_SPREAD = 0.22;
// #1445: a Unit's summon entrance takes its flavor, at these counts at intensity "normal" (a Brute's with
// a Legendary's weight). Every count is at least 8, so low plans strictly fewer particles and high more.
export const FX_SUMMON_BRUTE_TRAUMA = 0.5;
export const FX_SUMMON_FLAVOR_TUNING = {
  physicalSpark: { count: 8, power: 1.2 },
  bruteCloud: { count: 48, power: 1.3 },
  bruteHeavy: { count: 64, power: 1.6 },
  beastClaw: { count: 22, power: 1.2 },
  beastScratch: { count: 14, power: 1 },
  fireFlame: { count: 32, power: 1 },
  fireEmber: { count: 18, power: 0.9 },
  darkRise: { count: 16, power: 0.8 },
  darkSmoke: { count: 12, power: 0.7 },
  holyLight: { count: 26, power: 0.9 },
  nerdPages: { count: 14, power: 0.9 },
  nerdMotes: { count: 10, power: 0.7 },
  techScan: { count: 12, power: 0.8 },
  techBoot: { count: 10, power: 0.8 },
  natureSprout: { count: 16, power: 1 },
  natureBloom: { count: 12, power: 0.8 },
  toxicBubbles: { count: 20, power: 0.8 },
  toxicPop: { count: 12, power: 0.7 },
  snackCrumbs: { count: 18, power: 0.9 },
  snackFall: { count: 12, power: 0.8 },
  hustleShower: { count: 40, power: 1.3 },
  hustleCoins: { count: 16, power: 1 },
  cloutConfetti: { count: 26, power: 1 },
  cloutFlash: { count: 14, power: 1.2 },
  spiritFade: { count: 12, power: 0.8 },
  spiritWisps: { count: 10, power: 0.7 },
} as const;
// #1446: a face-up death's flavor exit, on the death's beats (FX_DEATH_EMBER_AT, FX_DEATH_SMOKE_AT).
// Colors run hot first; every count is at least 8, as the summon's (rule 9).
export const FX_DEATH_BOLT_FLIGHT_FRACTION = 0.5;
export const FX_DEATH_BILL_COLORS = ["#e9f7d9", "#3fae5a", "#1f6b35"] as const;
export const FX_DEATH_LIGHTS_OUT_COLORS = ["#4a4a57", "#26262e", "#0b0b0f"] as const;
export const FX_DEATH_FLAVOR_TUNING = {
  physicalSlump: { count: 10, power: 0.6 },
  physicalDust: { count: 18, power: 0.5 },
  bruteRubble: { count: 30, power: 1.2 },
  bruteDust: { count: 24, power: 0.8 },
  beastKick: { count: 14, power: 1.3 },
  fireFlame: { count: 28, power: 0.9 },
  fireAsh: { count: 16, power: 0.5 },
  darkSink: { count: 18, power: 0.7 },
  darkSmoke: { count: 16, power: 0.6 },
  holyLight: { count: 24, power: 0.9 },
  nerdPages: { count: 14, power: 0.8 },
  nerdSnap: { count: 10, power: 1 },
  techSparks: { count: 20, power: 1.3 },
  techFade: { count: 10, power: 0.5 },
  natureWilt: { count: 16, power: 0.6 },
  natureFall: { count: 10, power: 0.5 },
  toxicDrip: { count: 12, power: 0.6 },
  toxicPuddle: { count: 22, power: 0.7 },
  snackBite: { count: 16, power: 1 },
  snackCrumbs: { count: 18, power: 0.6 },
  hustleBills: { count: 30, power: 1.2 },
  hustleCoins: { count: 12, power: 0.9 },
  cloutFlash: { count: 16, power: 1.4 },
  cloutDark: { count: 20, power: 0.5 },
  spiritRise: { count: 14, power: 0.6 },
  spiritWisps: { count: 10, power: 0.5 },
} as const;
export const FX_SPEED_MIN = 0.25;              // R435: the slider runs from a quarter speed…
export const FX_SPEED_MAX = 3;                 // …to three times the table's speed
export const FX_SPEED_DEFAULT = 1;
export const FX_SPEED_STEP = 0.25;             // the settings panel's slider moves by this
export const FX_INTENSITY_SCALE = { off: 0, low: 0.45, normal: 1, high: 1.6 } as const;
// R2785000: the beat a quest's set piece asks for at speed 1, scaled and capped by `setPieceHoldMs`.
export const FX_QUEST_HOLD_MS = 1_500;
// R2785000: a quest's set piece lands this hard; a system's Signature this hard.
export const FX_QUEST_TRAUMA = 0.5;
export const FX_SYSTEM_TRAUMA = 0.3;
// R2801000: the Starship (fx/starship.ts). A fuse run's pieces fly in this far apart, each landing at
// FX_STARSHIP_FUSE_LAND_AT of its share and bolting on this hard.
export const FX_STARSHIP_FUSE_STAGGER_MS = 250;
export const FX_STARSHIP_FUSE_LAND_AT = 0.6;
export const FX_STARSHIP_FUSE_TRAUMA = 0.25;
// R2801000: the launch's countdown, as fractions of its hold, the count shown being the counts left
// (3, 2, 1); its engines ignite at FX_STARSHIP_IGNITE_AT of the hold.
export const FX_STARSHIP_COUNT_AT = [0, 0.25, 0.5] as const;
export const FX_STARSHIP_IGNITE_AT = 0.75;
// R2801000: after the hold, as fractions of the launch's own motion: the contrail's flight up, and the landing.
export const FX_STARSHIP_LIFT_AT = 0.45;
export const FX_STARSHIP_LAND_AT = 0.8;
// R2801000: where on the Starship its Dormant badge burns away as it lifts off (the seal's moon, DormantSeal.tsx).
export const FX_STARSHIP_BADGE_AT = { x: 0.5, y: 0.36 } as const;
// R2801000: the launch cue outlasts its entry by this much (inside FX_MAX_TAIL_MS, R200); under Reduce
// Motion its still frame stands this long and holds nothing.
export const FX_STARSHIP_TAIL_MS = 400;
export const FX_STARSHIP_STILL_MS = 1_600;
// R3336000: the Drone Battleship (fx/battleship.ts). The Shield's armor plates glow at most this many; where on
// each layer's card its dome, its reactor core and its bay doors sit (fractions of the card).
export const FX_SHIP_PLATES_MAX = 8;
export const FX_SHIP_DOME_AT = { x: 0.5, y: 0.22 } as const;
export const FX_SHIP_CORE_AT = { x: 0.5, y: 0.62 } as const;
export const FX_SHIP_BAY_AT = { x: 0.5, y: 0.5 } as const;
// R3336000: a hit the dome takes ripples this many rings, FX_SHIP_RIPPLE_STEP of the entry apart; a drone leaves
// its bay at FX_SHIP_LAUNCH_AT of the entry and flies this much of it (an Interceptor fast, a Corvette heavy);
// the deck beneath a breaking layer lights up at FX_SHIP_LIGHTS_AT.
export const FX_SHIP_RIPPLES = 2;
export const FX_SHIP_RIPPLE_STEP = 0.2;
export const FX_SHIP_LAUNCH_AT = 0.1;
export const FX_SHIP_INTERCEPTOR_FLIGHT = 0.45;
export const FX_SHIP_CORVETTE_FLIGHT = 0.85;
export const FX_SHIP_LIGHTS_AT = 0.5;
// R3336000: a Battleship cue outlasts its entry by this much (inside FX_MAX_TAIL_MS, R200); the arrival is this
// many beats, one per layer summoned (Drone Core X's warp, cards/stinkyscape/battleshipWarp.ts).
export const FX_SHIP_TAIL_MS = 400;
export const FX_SHIP_ARRIVAL_BEATS = 3;
// R3336000: the board's shake: the hangar locking on, the dome's flare, an ordinary hit on the dome, a Corvette
// landing, the reactor's pulse (times how far it has melted, 0 to 1), a deck shearing off and the reactor's blast.
export const FX_SHIP_LOCK_TRAUMA = 0.35;
export const FX_SHIP_DOME_TRAUMA = 0.3;
export const FX_SHIP_HIT_TRAUMA = 0.25;
export const FX_SHIP_CORVETTE_TRAUMA = 0.2;
export const FX_SHIP_TICK_TRAUMA = 0.3;
export const FX_SHIP_BREAK_TRAUMA = 0.45;
export const FX_SHIP_BLAST_TRAUMA = 0.95;
// R3338000: the Terminus cards' set pieces (fx/terminus/). A set piece's beats fall evenly from
// FX_TERMINUS_BEATS_FROM to FX_TERMINUS_BEATS_TO of its span (its hold, or, unheld, its swallow); its cue
// outlasts its entry by FX_TERMINUS_TAIL_MS (inside FX_MAX_TAIL_MS, R200), and under Reduce Motion its
// still frame stands FX_TERMINUS_STILL_MS and holds nothing.
export const FX_TERMINUS_BEATS_FROM = 0.25;
export const FX_TERMINUS_BEATS_TO = 0.85;
export const FX_TERMINUS_TAIL_MS = 600;
export const FX_TERMINUS_STILL_MS = 1_600;
// R3338000: where on its zone Ceaseless Void's 10/10 burns once the fold settles, and the point of the
// board Farewell's lamp hangs from.
export const FX_TERMINUS_STATS_AT = { x: 0.85, y: 0.88 } as const;
export const FX_TERMINUS_LAMP_AT = { x: 0.5, y: 0.05 } as const;
// R3338000: the most graveyard cards Farewell lifts into its light, one a beat; Dice of Doom's Terminus
// cards rain into the hand this share of the entry's motion apart.
export const FX_TERMINUS_LIFT_MAX = 12;
export const FX_TERMINUS_RAIN_STEP = 0.06;
// R3338000: Event Horizon's extra turn opens with the board snapping back this hard; Last Word's turn
// counts down 3, 2, 1 at these fractions of its opening entry.
export const FX_TERMINUS_SNAP_TRAUMA = 0.5;
export const FX_TERMINUS_COUNT_AT = [0.2, 0.45, 0.7] as const;
// R961: the jade tally grows this much per step, at most this big.
export const FX_JADE_SCALE_STEP = 0.15;
export const FX_JADE_SCALE_MAX = 2;
export const FX_SETTINGS_KEY = "jackioh.fx.v1";
export const FX_CENTER = { x: 0.5, y: 0.45 } as const;     // viewport anchor for banners and shuffles
export const FX_TEXT = {
  yourTurn: "Your turn",
  opponentTurn: "Opponent's turn",
  /** R845: an extra turn's banner. */
  yourExtraTurn: "Your extra turn",
  opponentExtraTurn: "Opponent's extra turn",
  autoEnded: "No moves left",
  /** R983: what the turn banner shows for a Feng Shui judgement. */
  auspicious: "吉",
  inauspicious: "凶",
  /** R436: the title over the effects Call to Chaos rolled. */
  chaosRolled: "Call to Chaos:",
  /** R2652001: the title over a batch of coins, and over a batch of dice. */
  coinsFlipped: "Coin flip",
  diceRolled: "Dice roll",
  /** B5 E10: an effect ended the turn. */
  turnCutShort: "Turn cut short",
  /** R1183002: the AI plays the rest of the turn (Meditative+ #18). */
  autopilot: "The AI takes the turn",
  /** R2785000: a quest's set-piece banner, the system's climax. */
  questComplete: "Quest complete",
  /** R1843000: a full tally removes a player. */
  votedOut: "Voted out",
  /** R1843002: a player escaped the universe. */
  escaped: "Escaped the universe",
  victory: "Victory",
  defeat: "Defeat",
  draw: "Draw",
} as const;
