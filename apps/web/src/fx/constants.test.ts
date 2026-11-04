// docs/polish/1-animations.md S2: the effects layer's constants, whose values the design fixes
// ("values binding"): every bound in the doc, and the settings panel's speed steps and intensity
// scale, is stated in these names, so a silent change to one is a contract change.

import { describe, expect, it } from "vitest";

import * as constants from "./constants.ts";

const S2 = {
  FX_MAX_TAIL_MS: 900,
  FX_MAX_PARTICLE_LIFE_MS: 900,
  FX_SPLAT_HOLD_MS: 650,
  FX_RAYS_TAIL_MS: 600,
  FX_ARROWS_TAIL_MS: 300,
  FX_CRACK_TAIL_MS: 400,
  FX_BANNER_TAIL_MS: 900,
  FX_RING_MS: 500,
  FX_HANDOVER_BANNER_MS: 1400,
  FX_RESULT_MS: 3200,
  FX_PROJECTILE_FLIGHT_FRACTION: 0.55,
  FX_MIND_CONTROL_FLIGHT_FRACTION: 0.7,
  FX_FUSE_FLIGHT_FRACTION: 0.5,
  FX_SLAM_AT: 0.6,
  FX_HOLD_MAX_MS: 6000,
  FX_CONCEAL_AT: 0.9,
  FX_LUNGE_STANDOFF: 0.55,
  FX_LUNGE_MIN_PX: 26,
  FX_LUNGE_MAX_PX: 520,
  FX_LUNGE_CONTACT_AT: 0.7,
  FX_HEAL_SPLAT_AT: 0.2,
  FX_DEATH_EMBER_AT: 0.3,
  FX_DEATH_SMOKE_AT: 0.5,
  FX_RADIANT_BURST_AT: 0.4,
  FX_TRAP_BURST_AT: 0.3,
  FX_BURN_AT: 0.25,
  FX_FATIGUE_STREAK_AT: 0.35,
  FX_FATIGUE_FLIGHT_FRACTION: 0.6,
  FX_OVERFLOW_FIZZLE_AT: 0.45,
  FX_MANA_STAGGER_MS: 40,
  FX_MANA_MAX_SPARKS: 10,
  FX_SLAM_STATS_MIN: 10,
  FX_SLAM_TRAUMA_PER_STAT: 0.06,
  FX_SLAM_MAX_TRAUMA: 0.5,
  FX_LEGENDARY_TRAUMA: 0.7,
  FX_TAKEOVER_TAIL_MS: 700,
  FX_TAKEOVER_TRAUMA: 1,
  FX_TRAP_TRAUMA: 0.4,
  FX_RESULT_TRAUMA: 0.9,
  FX_LETHAL_LEAD_MAX_MS: 600,
  FX_SHAKE_MAX_PX: 18,
  FX_SHAKE_MAX_DEG: 1.2,
  FX_SHAKE_FREQ_HZ: 18,
  FX_TRAUMA_DECAY: 1.2,
  FX_PARTICLE_CAP: 600,
  FX_PARTICLE_CAP_MOBILE: 260,
  FX_PARTICLE_CAP_MIN: 120,
  FX_MOBILE_WIDTH: 600,
  FX_MAX_DPR: 2,
  FX_MAX_DT_MS: 50,
  FX_ADAPT_WINDOW: 30,
  FX_ADAPT_SLOW_MS: 24,
  FX_ADAPT_SLOW_FACTOR: 1.4,
  FX_ADAPT_DISPLAY_MAX_MS: 34,
  FX_ADAPT_MIN_INTERVAL_MS: 4,
  FX_ADAPT_RECOVER_WINDOWS: 3,
  FX_DEFAULT_SEED: 0x5eed,
  FX_MEMORY_LIMIT: 64,
  // R502's cast on draw, #21 Hinder's crack and #27 Blood Ridden's drain; R437's brand; R436's Call to Chaos.
  FX_MEMORY_RECENT: 16,
  FX_MEMORY_RESOLVING: 8,
  FX_MEMORY_HITS: 44,
  FX_CAST_ON_DRAW_TRAUMA: 0.25,
  FX_CRACK_HIT_AT: 0.45,
  FX_CRACK_STAGGER_MS: 50,
  FX_FRACTURE_TAIL_MS: 800,
  FX_CRACK_TRAUMA: 0.3,
  FX_NEXT_REFRESH_MODIFIER_ID: "nextTurnMana",
  FX_BLOOD_FLIGHT_FRACTION: 0.6,
  FX_BLOOD_PICK_BASE: 1,
  FX_BLOOD_PICK_STRIDE: 2,
  FX_BLOOD_TRAUMA: 0.2,
  // Classic+ #24 Crushing Walls.
  FX_WALLS_HIT_AT: 0.3,
  FX_WALLS_TAIL_MS: 850,
  FX_WALLS_REACH: 0.2,
  FX_WALLS_TRAUMA: 0.45,
  FX_ROLLBACK_STEP: 0.08,
  FX_INK_TEAR_TRAUMA: 0.2,
  // #2796: Meditative's Flourishes.
  FX_JADE_CRACK_TRAUMA: 0.3,
  FX_HARMONY_TRAUMA: 0.15,
  FX_CN_SWEEP_TRAUMA: 0.35,
  // Classic #85 King Wagtoggle's crown swap (R2794000).
  FX_CROWN_SWAP_TRAUMA: 0.2,
  FX_BRAND_SLAM_AT: 0.35,
  FX_BRAND_TAIL_MS: 700,
  // MN05: Armor's shield flash (R1363).
  FX_SHIELD_TAIL_MS: 450,
  FX_CHAOS_LAND_AT: 0.4,
  FX_CHAOS_STAGGER: 0.2,
  FX_CHAOS_LAND_LAST: 0.85,
  FX_CHAOS_REEL_DECOYS: 6,
  FX_CHAOS_DECOY_STEP: 3,
  // Issue #2652: a batch of coins or dice (R2652001).
  FX_ROLL_LAND_AT: 0.5,
  FX_ROLL_STAGGER: 0.05,
  FX_ROLL_LAND_LAST: 0.7,
  FX_ROLL_TUMBLE_FACES: 8,
  FX_ROLL_TUMBLE_STRIDE: 3,
  // Issue #124: the sweep's fog, the whole-pile wave and the multicast flare.
  FX_FOG_HIT_FROM: 0.2,
  FX_FOG_HIT_TO: 0.75,
  FX_FOG_TAIL_MS: 500,
  FX_FOG_PUFFS: 7,
  FX_FOG_ICONS: 6,
  FX_FOG_PAD: 0.15,
  FX_ZONE_COUNT_AT: 0.35,
  FX_ZONE_TAIL_MS: 400,
  FX_CAST_SCALE_STEP: 0.06,
  FX_CAST_SCALE_MAX: 1.6,
  FX_CAST_TRAUMA: 0.12,
  FX_FLAVOR_LIVE: true,
  FX_SHOT_LAUNCH_AT: 0.24,
  FX_SHOT_SHAKE_TRAUMA: 0.3,
  FX_FLAVOR_VOLLEY_MAX: 4,
  // Issue #2784: the motifs (R2784000) and the gallery's scales.
  FX_MOTIF_PHASES: { gather: 0, travel: 0.2, land: 0.6, linger: 0.75 },
  FX_MOTIF_SCALE_PER_COST: 0.08,
  FX_MOTIF_COST_CAP: 7,
  FX_MOTIF_X_COST: 3,
  FX_MOTIF_SCALE_MAX: 2,
  FX_MOTIF_RARITY_SCALE: { Common: 1, Rare: 1.1, Epic: 1.25, Legendary: 1.4, Mythic: 1.6, Token: 0.9 },
  FX_MOTIF_VOLLEY_MAX: 5,
  FX_MOTIF_BARRAGE_MAX: 8,
  FX_MOTIF_BARRAGE_STEP: 0.02,
  FX_MOTIF_CENTER_LANE: 3,
  FX_MOTIF_TRAUMA: {
    bolt: 0.25,
    volley: 0.3,
    barrage: 0.35,
    chain: 0.3,
    lance: 0.4,
    nova: 0.45,
    wipe: 0.6,
    vanish: 0,
    scorch: 0.3,
    negate: 0.25,
    drain: 0.3,
    bless: 0.2,
    curse: 0.25,
    ward: 0.2,
    conjure: 0.3,
    rise: 0.35,
    offering: 0.3,
    shift: 0.15,
    bind: 0.2,
    tether: 0.25,
    mirror: 0.25,
    transmute: 0.3,
    flight: 0,
    glimpse: 0.1,
    mana: 0,
    tithe: 0.1,
    tally: 0.15,
    snap: 0.4,
    send: 0,
    radiance: 0.15,
    mend: 0,
    deal: 0,
    seal: 0.2,
    // Issue #2794: recall (R2794000).
    recall: 0,
    hex: 0.15,
  },
  // Issue #2797: Stinkyscape's Flourishes (R2797000).
  FX_TERMINUS_TRAUMA: 0.5,
  FX_MAGI_STAGGER: 0.05,
  FX_MAGI_BOOTS: 3,
  FX_MAGI_TRAUMA: 0.3,
  FX_WARP_TRAUMA: 0.6,
  FX_WARP_FROM: { x: 0.5, y: 0 },
  FX_MOTIF_TITHE_MAX: 5,
  FX_MOTIF_DEAL_MAX: 5,
  // Issue #2793: Core's flourishes, and a set pass's floor.
  FX_TIDE_TRAUMA: 0.35,
  FX_WISH_TRAUMA: 0.3,
  FX_FORGE_TRAUMA: 0.4,
  FX_FORGE_REACH: 0.2,
  // Issue #2798: Stinkyscape's flourishes.
  FX_FLASH_FREEZE_TRAUMA: 0.35,
  FX_DUO_BOND_TRAUMA: 0.3,
  FX_STRATOS_SWARM_TRAUMA: 0.3,
  // Issue #2799: Training Grounds' flourishes.
  FX_CELESTIAL_TRAUMA: 0.35,
  FX_DINO_STOMP_TRAUMA: 0.55,
  FX_GAME_DESIGN_TRAUMA: 0.45,
  FX_SET_PASS_FLOOR: 0.55,
  FX_GALLERY_SCALES: [0.5, 1, 1.5, 2],
  // Issue #2787: the auras (R2787000).
  AURA_MAX_PER_ZONE: 2,
  AURA_MOTES_PER_AURA: 24,
  AURA_PARTICLE_SHARE: 0.25,
  AURA_FADE_IN_MS: 600,
  AURA_FADE_OUT_MS: 500,
  AURA_BURST_SPEED: 220,
  AURA_BURST_DECAY: 4,
  AURA_SOURCES_LIMIT: 64,
  FX_CALLOUT_MS: 600,
  FX_CALLOUTS_PER_HIT: 2,
  FX_BEAT_STAGGER_MS: 60,
  FX_BEAT_STAGGER_MAX: 0.5,
  FX_BEAT_TRAUMA: 0.25,
  FX_FIRST_STRIKE_BREAK_AT: 0.4,
  FX_FIRST_STRIKE_BREAK_REACH: 0.6,
  FX_PIERCE_REACH: 0.8,
  FX_FLYING_LUNGE_LIFT: 0.18,
  // Issue #2791: the tempo keyword beats (R2791000).
  FX_STACK_LAND: { x: 0.62, y: 0.35 },
  FX_DEFT_FEINT_STEP: 0.18,
  FX_CHARGE_LINE_SPREAD: 0.22,
  FX_SUMMON_BRUTE_TRAUMA: 0.5,
  FX_SUMMON_FLAVOR_TUNING: {
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
  },
  FX_DEATH_BOLT_FLIGHT_FRACTION: 0.5,
  FX_DEATH_BILL_COLORS: ["#e9f7d9", "#3fae5a", "#1f6b35"],
  FX_DEATH_LIGHTS_OUT_COLORS: ["#4a4a57", "#26262e", "#0b0b0f"],
  FX_DEATH_FLAVOR_TUNING: {
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
  },
  FX_SPEED_MIN: 0.25,
  FX_SPEED_MAX: 3,
  FX_SPEED_DEFAULT: 1,
  FX_SPEED_STEP: 0.25,
  FX_FLICKER_RETURN_AT: 0.5,
  FX_REDIRECT_FLIGHT_FRACTION: 0.6,
  FX_COUNTER_TRAUMA: 0.25,
  FX_REWIND_TRAUMA: 0.35,
  FX_INTENSITY_SCALE: { off: 0, low: 0.45, normal: 1, high: 1.6 },
  FX_QUEST_HOLD_MS: 1_500,
  FX_QUEST_TRAUMA: 0.5,
  FX_SYSTEM_TRAUMA: 0.3,
  FX_STARSHIP_FUSE_STAGGER_MS: 250,
  FX_STARSHIP_FUSE_LAND_AT: 0.6,
  FX_STARSHIP_FUSE_TRAUMA: 0.25,
  FX_STARSHIP_COUNT_AT: [0, 0.25, 0.5],
  FX_STARSHIP_IGNITE_AT: 0.75,
  FX_STARSHIP_LIFT_AT: 0.45,
  FX_STARSHIP_LAND_AT: 0.8,
  FX_STARSHIP_BADGE_AT: { x: 0.5, y: 0.36 },
  FX_STARSHIP_TAIL_MS: 400,
  FX_STARSHIP_STILL_MS: 1_600,
  FX_JADE_SCALE_STEP: 0.15,
  FX_JADE_SCALE_MAX: 2,
  FX_SETTINGS_KEY: "jackioh.fx.v1",
  FX_CENTER: { x: 0.5, y: 0.45 },
  FX_TEXT: {
    yourTurn: "Your turn",
    opponentTurn: "Opponent's turn",
    yourExtraTurn: "Your extra turn",
    opponentExtraTurn: "Opponent's extra turn",
    autoEnded: "No moves left",
    auspicious: "吉",
    inauspicious: "凶",
    chaosRolled: "Call to Chaos:",
    coinsFlipped: "Coin flip",
    diceRolled: "Dice roll",
    turnCutShort: "Turn cut short",
    autopilot: "The AI takes the turn",
    questComplete: "Quest complete",
    votedOut: "Voted out",
    escaped: "Escaped the universe",
    victory: "Victory",
    defeat: "Defeat",
    draw: "Draw",
  },
};

describe("S2 the effects constants", () => {
  it("S2 constants.ts exports exactly the S2 table, with S2's values", () => {
    expect({ ...constants }).toEqual(S2);
  });

  it("S2 no particle outlives the tail, and every fraction of D lands inside the entry", () => {
    expect(constants.FX_MAX_PARTICLE_LIFE_MS).toBeLessThanOrEqual(constants.FX_MAX_TAIL_MS);
    const fractions = [
      constants.FX_PROJECTILE_FLIGHT_FRACTION,
      constants.FX_MIND_CONTROL_FLIGHT_FRACTION,
      constants.FX_FUSE_FLIGHT_FRACTION,
      constants.FX_SLAM_AT,
      constants.FX_HEAL_SPLAT_AT,
      constants.FX_DEATH_EMBER_AT,
      constants.FX_DEATH_SMOKE_AT,
      constants.FX_RADIANT_BURST_AT,
      constants.FX_TRAP_BURST_AT,
      constants.FX_BURN_AT,
      constants.FX_FATIGUE_STREAK_AT,
      constants.FX_FATIGUE_FLIGHT_FRACTION,
      constants.FX_OVERFLOW_FIZZLE_AT,
      constants.FX_CONCEAL_AT,
      constants.FX_LUNGE_STANDOFF,
      constants.FX_LUNGE_CONTACT_AT,
      constants.FX_FLICKER_RETURN_AT,
      constants.FX_REDIRECT_FLIGHT_FRACTION,
      constants.FX_WALLS_HIT_AT,
      constants.FX_WALLS_REACH,
      constants.FX_FORGE_REACH,
      constants.FX_DEATH_BOLT_FLIGHT_FRACTION,
      // R2789000: the beats' stagger cap and a First Strike's break-off.
      constants.FX_BEAT_STAGGER_MAX,
      constants.FX_FIRST_STRIKE_BREAK_AT,
      // R2797000: Imitated Instrumentality's stagger, and where Drone Core X's Battleship warps in from.
      constants.FX_MAGI_STAGGER,
      constants.FX_WARP_FROM.x,
      // R2801000: a fuse's landing, the launch's ignition, contrail flight and landing.
      constants.FX_STARSHIP_FUSE_LAND_AT,
      constants.FX_STARSHIP_IGNITE_AT,
      constants.FX_STARSHIP_LIFT_AT,
      constants.FX_STARSHIP_LAND_AT,
    ];
    for (const fraction of fractions) {
      expect(fraction).toBeGreaterThan(0);
      expect(fraction).toBeLessThanOrEqual(1);
    }
    // R2784000: a motif's phases run in order from the entry's start, each inside it.
    const phases = [constants.FX_MOTIF_PHASES.gather, constants.FX_MOTIF_PHASES.travel, constants.FX_MOTIF_PHASES.land, constants.FX_MOTIF_PHASES.linger];
    expect(phases[0]).toBe(0);
    for (let k = 1; k < phases.length; k += 1) {
      expect(phases[k]).toBeGreaterThan(phases[k - 1] ?? 1);
      expect(phases[k]).toBeLessThanOrEqual(1);
    }
    // R2801000: the countdown runs in order from the hold's start, every count before the ignition, and
    // the launch's tail stays inside FX_MAX_TAIL_MS (R200).
    const counts = constants.FX_STARSHIP_COUNT_AT;
    expect(counts[0]).toBe(0);
    for (let k = 1; k < counts.length; k += 1) expect(counts[k]).toBeGreaterThan(counts[k - 1] ?? 1);
    expect(counts[counts.length - 1]).toBeLessThan(constants.FX_STARSHIP_IGNITE_AT);
    expect(constants.FX_STARSHIP_TAIL_MS).toBeLessThanOrEqual(constants.FX_MAX_TAIL_MS);
    // R318: the fatigue streak leaves the library and lands on the hero inside its entry.
    expect(constants.FX_FATIGUE_STREAK_AT + constants.FX_FATIGUE_FLIGHT_FRACTION).toBeLessThan(1);
    // #1446: a Beast's streak leaves at the embers' beat and lands inside its entry.
    expect(constants.FX_DEATH_EMBER_AT + constants.FX_DEATH_BOLT_FLIGHT_FRACTION).toBeLessThan(1);
  });

  it("R435 the slider runs from 0.25x to 3x in steps that land on both ends and on the default", () => {
    expect(constants.FX_SPEED_MIN).toBe(0.25);
    expect(constants.FX_SPEED_MAX).toBe(3);
    const steps = (constants.FX_SPEED_MAX - constants.FX_SPEED_MIN) / constants.FX_SPEED_STEP;
    expect(Number.isInteger(steps)).toBe(true);
    const toDefault = (constants.FX_SPEED_DEFAULT - constants.FX_SPEED_MIN) / constants.FX_SPEED_STEP;
    expect(Number.isInteger(toDefault)).toBe(true);
  });

  it("S2 the particle caps shrink from desktop to phone to the adaptive floor", () => {
    expect(constants.FX_PARTICLE_CAP_MIN).toBeLessThan(constants.FX_PARTICLE_CAP_MOBILE);
    expect(constants.FX_PARTICLE_CAP_MOBILE).toBeLessThan(constants.FX_PARTICLE_CAP);
  });

  it("S2 the shake caps stay inside [0, 1] trauma", () => {
    for (const trauma of [
      constants.FX_SLAM_MAX_TRAUMA,
      constants.FX_LEGENDARY_TRAUMA,
      constants.FX_TRAP_TRAUMA,
      constants.FX_RESULT_TRAUMA,
      constants.FX_TAKEOVER_TRAUMA,
      constants.FX_BEAT_TRAUMA,
      // #2796: Meditative's Flourishes.
      constants.FX_JADE_CRACK_TRAUMA,
      constants.FX_HARMONY_TRAUMA,
      constants.FX_CN_SWEEP_TRAUMA,
      // R2797000: Stinkyscape's Flourishes.
      constants.FX_TERMINUS_TRAUMA,
      constants.FX_MAGI_TRAUMA,
      constants.FX_WARP_TRAUMA,
    ]) {
      expect(trauma).toBeGreaterThan(0);
      expect(trauma).toBeLessThanOrEqual(1);
    }
    // R2784000: a motif's shake, none for vanish.
    for (const trauma of Object.values(constants.FX_MOTIF_TRAUMA)) {
      expect(trauma).toBeGreaterThanOrEqual(0);
      expect(trauma).toBeLessThanOrEqual(1);
    }
  });

  it("S2 the fullest shake (trauma 1) is spent inside FX_MAX_TAIL_MS, so a shake started inside an entry ends inside its tail (R200)", () => {
    expect((1 / constants.FX_TRAUMA_DECAY) * 1000).toBeLessThanOrEqual(constants.FX_MAX_TAIL_MS);
  });

  it("S2 the result thumps hardest of all, and a trap reveal is still felt (B52)", () => {
    // Peak offset is FX_SHAKE_MAX_PX · trauma² (shake.ts). A damage hit's shake is its tier's
    // `shakePx` (game/damageFeel.ts, #57), which cues.test.ts holds.
    const peak = (t: number): number => constants.FX_SHAKE_MAX_PX * t * t;
    expect(peak(constants.FX_RESULT_TRAUMA)).toBeGreaterThanOrEqual(12);
    expect(peak(constants.FX_TRAP_TRAUMA)).toBeGreaterThanOrEqual(2);
  });
});

describe("R200 the v0.2.0 tails fit inside T", () => {
  it("R502 R437 every new tail and landing fraction keeps its cue inside its entry plus FX_MAX_TAIL_MS", () => {
    expect(constants.FX_FRACTURE_TAIL_MS).toBeLessThanOrEqual(constants.FX_MAX_TAIL_MS);
    expect(constants.FX_BRAND_TAIL_MS).toBeLessThanOrEqual(constants.FX_MAX_TAIL_MS);
    expect(constants.FX_WALLS_TAIL_MS).toBeLessThanOrEqual(constants.FX_MAX_TAIL_MS);
    expect(constants.FX_TAKEOVER_TAIL_MS, "R2722001").toBeLessThanOrEqual(constants.FX_MAX_TAIL_MS);
    expect(constants.FX_CALLOUT_MS, "R2789000").toBeLessThanOrEqual(constants.FX_MAX_TAIL_MS);
    for (const at of [constants.FX_CRACK_HIT_AT, constants.FX_BLOOD_FLIGHT_FRACTION, constants.FX_BRAND_SLAM_AT, constants.FX_CHAOS_LAND_AT, constants.FX_CHAOS_LAND_LAST, constants.FX_ROLL_LAND_AT, constants.FX_ROLL_LAND_LAST]) {
      expect(at).toBeGreaterThanOrEqual(0);
      expect(at).toBeLessThanOrEqual(1);
    }
  });
});
