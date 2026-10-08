// Every number the effects layer uses (CLAUDE.md rule 9; docs/polish/1-animations.md, Surface S2).
// Particle counts per recipe live in `TUNING` in `cues.ts`, and preset physics in
// `PARTICLE_PRESETS` in `presets.ts`. The values below are binding: other slices compile and test
// against them.

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
export const FX_LUNGE_STANDOFF = 0.55;        // the lunge stops this many combined half-extents short of the target's centre (< 1: they overlap)
export const FX_LUNGE_MIN_PX = 26;            // = animations.css --lunge-distance (BUILD: at least 20 px)
export const FX_LUNGE_MAX_PX = 520;
export const FX_LUNGE_CONTACT_AT = 0.7;       // the contact beat inside attackDeclared (jk-lunge's 70% keyframe)
export const FX_HEAL_SPLAT_AT = 0.2;
export const FX_DEATH_EMBER_AT = 0.3;
export const FX_DEATH_SMOKE_AT = 0.5;
export const FX_RADIANT_BURST_AT = 0.4;
export const FX_TRAP_BURST_AT = 0.2;
export const FX_BURN_AT = 0.25;
export const FX_FATIGUE_STREAK_AT = 0.35;      // the fatigue hit leaves the empty library at 0.35 D…
export const FX_FATIGUE_FLIGHT_FRACTION = 0.6; // …and reaches the hero at 0.95 D, inside the entry (R200)
export const FX_OVERFLOW_FIZZLE_AT = 0.45;     // a refused card fizzles at 0.45 D, after it bounces off the pile
export const FX_MANA_STAGGER_MS = 40;
export const FX_MANA_MAX_SPARKS = 10;
export const FX_SHAKE_MIN_DAMAGE = 3;
export const FX_TRAUMA_PER_DAMAGE = 0.15;
export const FX_SHAKE_MAX_TRAUMA = 0.8;
export const FX_HERO_TRAUMA_MULT = 1.25;
export const FX_SLAM_STATS_MIN = 10;          // printed attack + health
export const FX_SLAM_TRAUMA_PER_STAT = 0.06;
export const FX_SLAM_MAX_TRAUMA = 0.5;
export const FX_LEGENDARY_TRAUMA = 0.5;
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

// R502: a cast on draw. The card bursts out of its drawer's Deck pile as its `cardPlayed` starts.
export const FX_CAST_ON_DRAW_TRAUMA = 0.25;
// R502: #21 Hinder's refresh loss. A frost bolt flies from the caster's hero to the victim's crystal
// tray, lands at FX_CRACK_HIT_AT of the entry, and each crystal the next refresh loses fractures from
// there, FX_CRACK_STAGGER_MS apart but never past the entry's end (R200).
export const FX_CRACK_HIT_AT = 0.45;
export const FX_CRACK_STAGGER_MS = 50;
export const FX_FRACTURE_TAIL_MS = 800;       // a fracture lasts (D − delay) + this, inside FX_MAX_TAIL_MS
export const FX_CRACK_TRAUMA = 0.3;
/** The id the engine gives the next refresh's rider (`NEXT_REFRESH_MODIFIER_ID`, R169): #21 Hinder's badge. */
export const FX_NEXT_REFRESH_MODIFIER_ID = "nextTurnMana";
// R502: #27 Blood Ridden Glowy Jelly Bean. A crimson stream leaves the caster's hero, reaches the card
// made Radiant at FX_BLOOD_FLIGHT_FRACTION of the entry and bursts gold there; on the other seat it
// lands on a back chosen by a counter of the planner's own (R202), never the hidden card's place.
export const FX_BLOOD_FLIGHT_FRACTION = 0.6;
export const FX_BLOOD_PICK_BASE = 1;          // the first back a hidden pick lands on (modulo the hand)
export const FX_BLOOD_PICK_STRIDE = 2;        // how far each later pick of the same play moves on
export const FX_BLOOD_TRAUMA = 0.2;
// Patch v0.2.7: Classic+ #24 Crushing Walls. At the first card its play destroys, two spiked walls
// slide in from the board's left and right edges over lanes 1 and 5, meet the cards at
// FX_WALLS_HIT_AT of the cue (fx.css's `fx-walls-close` stop of the same name), shake and slide back
// out. The cue lasts D + FX_WALLS_TAIL_MS, inside FX_MAX_TAIL_MS (R200).
export const FX_WALLS_HIT_AT = 0.3;
export const FX_WALLS_TAIL_MS = 850;
export const FX_WALLS_REACH = 0.2;            // how far each wall reaches in, of the board's width: one lane of five
export const FX_WALLS_TRAUMA = 0.45;
// R437: a mark branded onto a card. The sigil slams on at FX_BRAND_SLAM_AT of the entry and fades
// over FX_BRAND_TAIL_MS after it.
export const FX_BRAND_SLAM_AT = 0.35;
export const FX_BRAND_TAIL_MS = 700;
// R1363 (MN05): a shield flashes up as Armor takes a hit, from the moment the hit lands, and fades
// over FX_SHIELD_TAIL_MS after the entry.
export const FX_SHIELD_TAIL_MS = 450;
// R436: Call to Chaos's reveal. Line i lands at (FX_CHAOS_LAND_AT + i × stagger) of the entry, the
// stagger shrinking so the last line lands by FX_CHAOS_LAND_LAST; each reel runs past
// FX_CHAOS_REEL_DECOYS other names first. The reveal lasts D + FX_BANNER_TAIL_MS.
export const FX_CHAOS_LAND_AT = 0.4;
// Patch v0.2.0's events (docs/classic-sets.md B3, B5), each a fraction of its entry (R200).
export const FX_FLICKER_RETURN_AT = 0.5;       // a flickered card comes back through the void halfway in
export const FX_REDIRECT_FLIGHT_FRACTION = 0.6; // a redirected hit flies from its old target to its new one
export const FX_COUNTER_TRAUMA = 0.25;         // a countered card shatters with a small shake
export const FX_REWIND_TRAUMA = 0.35;          // the board rewinding (Classic+ #35 Rollback)
export const FX_CHAOS_STAGGER = 0.2;
export const FX_CHAOS_LAND_LAST = 0.85;
export const FX_CHAOS_REEL_DECOYS = 6;
export const FX_CHAOS_DECOY_STEP = 3;          // how far apart in the names table a reel's decoys are
// Issue #124: a sweep's fog rolls over its row (one entry, SWEEP_MS), each unit's hit landing as the
// fog reaches its lane, from FX_FOG_HIT_FROM to FX_FOG_HIT_TO of the entry; a zone wave's count pops
// at FX_ZONE_COUNT_AT. A fog is FX_FOG_PUFFS puffs of cloud and FX_FOG_ICONS icons, spread over the
// row grown by FX_FOG_PAD of its height on every side; both trail FX_FOG_TAIL_MS after the entry.
export const FX_FOG_HIT_FROM = 0.2;
export const FX_FOG_HIT_TO = 0.75;
export const FX_FOG_TAIL_MS = 500;
export const FX_FOG_PUFFS = 7;
export const FX_FOG_ICONS = 6;
export const FX_FOG_PAD = 0.15;
export const FX_ZONE_COUNT_AT = 0.35;
export const FX_ZONE_TAIL_MS = 400;
// Issue #124: a cast another card made flares at its caster's hero, bigger with each cast of the same
// burst (FX_CAST_SCALE_STEP more per cast, up to FX_CAST_SCALE_MAX), and the hero jolts a little.
export const FX_CAST_SCALE_STEP = 0.06;
export const FX_CAST_SCALE_MAX = 1.6;
export const FX_CAST_TRAUMA = 0.12;
export const FX_SPEED_MIN = 0.25;              // R435: the slider runs from a quarter speed…
export const FX_SPEED_MAX = 3;                 // …to three times the table's speed
export const FX_SPEED_DEFAULT = 1;
export const FX_SPEED_STEP = 0.25;             // the settings panel's slider moves by this
export const FX_INTENSITY_SCALE = { off: 0, low: 0.45, normal: 1, high: 1.6 } as const;
export const FX_SETTINGS_KEY = "jackioh.fx.v1";
export const FX_CENTER = { x: 0.5, y: 0.45 } as const;     // viewport anchor for banners and shuffles
export const FX_TEXT = {
  yourTurn: "Your turn",
  opponentTurn: "Opponent's turn",
  /** R845: an extra turn's banner. */
  yourExtraTurn: "Your extra turn",
  opponentExtraTurn: "Opponent's extra turn",
  autoEnded: "No moves left",
  /** R436: the title over the effects Call to Chaos rolled. */
  chaosRolled: "Call to Chaos:",
  /** B5 E10: an effect ended the turn. */
  turnCutShort: "Turn cut short",
  victory: "Victory",
  defeat: "Defeat",
  draw: "Draw",
} as const;
