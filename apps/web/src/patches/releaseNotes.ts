// Player-facing release notes for `/patch-notes` (R724000): Factorio- and Steam-style history,
// one bubble per set, written by hand. This is presentation only (CLAUDE.md rule 7): the
// machine-readable card patch history in `crates/cards/patches/` (R388, R646) is untouched, and
// `PatchNotes.tsx` falls back to a patch's catalog title and notes when no note below claims it,
// so a patch that ships before its note is written still shows.

export type SubPatchNote = {
  readonly version: string;
  readonly title: string;
  readonly summary: string;
};

export type SetReleaseNotes = {
  readonly id: string;
  readonly name: string;
  readonly versions: string;
  readonly summary: string;
  readonly technicalChanges: readonly string[];
  readonly subPatches: readonly SubPatchNote[];
};

/** Every set's release notes, newest set first. The page opens on the first entry. */
export const ALL_SET_RELEASES: readonly SetReleaseNotes[] = [
  {
    id: "meditative",
    name: "Meditative",
    versions: "v0.3.0 – v0.3.6",
    summary:
      "The Meditative expansion: 102 cards and 30 tokens built around the Zenith, Flow and Focus mechanics, joining every game mode.",
    technicalChanges: [
      "Engine Rewrite: Complete rewrite of the game engine in pure Rust with strict determinism and zero allocations in the combat resolution loop.",
      "AI Upgrades: Heuristic evaluation improvements, dual-lane tactical positioning, and tuned bot difficulty levels.",
      "WebAssembly Performance: Fast client-side match execution and instant turn response powered by WebAssembly.",
      "Card Text & Clarity: Comprehensive card wording overhaul, streamlining keyword rules (Nerf & Buff) and targeting conditions.",
    ],
    subPatches: [
      {
        version: "v0.3.6",
        title: "Zephyrs Cross-Set Expansion",
        summary: "Core Zephyrs and Zephrys Zealotism now draw from all shipped sets, including Meditative.",
      },
      {
        version: "v0.3.5",
        title: "Meditative Expansion Launch",
        summary: "Official release of the Meditative set: 102 cards and 30 tokens available across all game modes.",
      },
      {
        version: "v0.3.4",
        title: "Keyword Refinements: Nerf & Buff",
        summary: "Renamed Degrade to Nerf and Upgrade to Buff across all printed cards for improved clarity.",
      },
      {
        version: "v0.3.3",
        title: "Heroic Power & Face Numbers Tuning",
        summary: "Heroic Power now tunes only its active power; adjusted base-only number distributions.",
      },
      {
        version: "v0.3.2",
        title: "Declared Numbers Expansion",
        summary: "Expanded tunable parameters across 80 cards to support broader stat-scaling interactions.",
      },
      {
        version: "v0.3.1",
        title: "Card Text & Keyword Polish",
        summary: "Refined card text across all sets for standardized Trap triggers and graveyard phrasing.",
      },
    ],
  },
  {
    id: "classic",
    name: "Classic & Classic+",
    versions: "v0.2.0 – v0.2.11",
    summary:
      "The Classic & Classic+ double expansion: 90 Classic cards, 78 Classic+ cards and 38 token units, with new keywords and the first balance passes.",
    technicalChanges: [
      "Ranked Ladder: Introduced competitive ranked queue with seasonal placement, tiers (Grape, Raisin, Jlorious), and leaderboard tracking.",
      "Card Almanac: Launched the public Card Almanac directory with voice line previews and advanced search filters.",
      "Match Replays & Stats: Public card win rate statistics and end-of-game telemetry tracking.",
      "Tavern Audio: Added original dynamic background music, atmospheric tavern SFX, and distinct unit audio.",
      "Match Feel: Implemented unit slam impact effects, board reactions, and turn timer alerts.",
    ],
    subPatches: [
      {
        version: "v0.2.11",
        title: "Heroic Power Rework",
        summary: "Reworked Heroic Power into thirteen distinct activated abilities with Radiant upgrades.",
      },
      {
        version: "v0.2.10d",
        title: "Balance Patch 1 Follow-Up",
        summary: "Adjusted opening hands for Cast on Draw cards, tuned Radiant Divine Favor and Rewind, and refined targeting.",
      },
      {
        version: "v0.2.10c",
        title: "Bounce Keyword Refinement",
        summary: "Restricted Bounce keyword strictly to permanents returned from the field.",
      },
      {
        version: "v0.2.10b",
        title: "Tag Conventions: Catalyst & Prime",
        summary: "Standardized Catalyst, Prime, and Acclaimed tags on legendary units.",
      },
      {
        version: "v0.2.10",
        title: "Classic & Classic+ Balance Pass 1",
        summary: "Major balance pass introducing Bounce, Plague Counters, Recruit rules, and Transform battle position rules.",
      },
      {
        version: "v0.2.9",
        title: "Rarity Pass & Trap Polish",
        summary: "Rarity rebalancing across 15 cards; added Plague Counter visual cues and tuned Pocket Chaos.",
      },
      {
        version: "v0.2.8b",
        title: "Gary the Gambler Balance",
        summary: "Gary the Gambler's Cry now flips a coin for Divine Shield or Rush.",
      },
      {
        version: "v0.2.8",
        title: "Secret Glitch Token",
        summary: "Added the rare hidden glitch occurrence with chaotic board disruption effects.",
      },
      {
        version: "v0.2.7b",
        title: "Condition Glow Indicators",
        summary: "Added yellow condition met highlight cues to ten Core cards.",
      },
      {
        version: "v0.2.7",
        title: "Targeting & Warning Polish",
        summary: "Plastic Surgery requires a valid unit target; added counter warning on Plague Chalice.",
      },
      {
        version: "v0.2.6",
        title: "Book of Wildfire Transformation",
        summary: "Book of Wildfire now periodically transforms into other Books at end of turn.",
      },
      {
        version: "v0.2.5",
        title: "Field Spell Animation Revert",
        summary: "Reverted Animated unit faces from field spells while keeping fusion and guard mechanics.",
      },
      {
        version: "v0.2.4",
        title: "Smart Random Casts & Deft Keyword",
        summary: "Harmful random casts now prioritize enemies; introduced the Deft combat keyword.",
      },
      {
        version: "v0.2.3",
        title: "Masochism Mask & Animated Pass",
        summary: "Adjusted Masochism Mask cost and animated field spell experiments.",
      },
      {
        version: "v0.2.2",
        title: "Mechanics Pass",
        summary: "Core mechanics updates across damage timing and keyword resolution.",
      },
      {
        version: "v0.2.1",
        title: "Classic Text Polish",
        summary: "Card text consistency and syntax alignment across Classic cards.",
      },
      {
        version: "v0.2.0",
        title: "Classic & Classic+ Launch",
        summary: "Released 90 Classic cards, 78 Classic+ cards, and 38 token units.",
      },
    ],
  },
  {
    id: "core",
    name: "Core",
    versions: "v0.1.0 – v0.1.1",
    summary:
      "The base set launch: 100 cards and 9 tokens on the dual-lane battlefield, with the first balance tuning after release.",
    technicalChanges: [
      "Dual-Lane Battlefield: Innovative tactical lane combat with backrow trap staging and Hearthstone-style mana curves.",
      "Collection & Deck Builder: Custom deck construction supporting card trios, radiant upgrades, and card detail inspection.",
      "Tutorial & Matchmaking: Interactive onboarding system, local pass-and-play, and online lobby matchmaking.",
    ],
    subPatches: [
      {
        version: "v0.1.1",
        title: "Core Balance Pass",
        summary: "Initial post-launch balance tuning and mana curve adjustments.",
      },
      {
        version: "v0.1.0d",
        title: "Radiant Card Faces",
        summary: "Introduced Radiant visual treatment and gold border card upgrades.",
      },
      {
        version: "v0.1.0c",
        title: "The Coin & Opening Balance",
        summary: "Added The Coin mechanic for second-player initiative balance.",
      },
      {
        version: "v0.1.0b",
        title: "Day-One Card Adjustments",
        summary: "Stat and cost adjustments for high-impact early drops.",
      },
      {
        version: "v0.1.0",
        title: "Core Set Launch",
        summary: "Initial public launch with 100 cards and 9 tokens.",
      },
    ],
  },
];

/** The hand-written note for a catalog version, if any set claims it. */
export function findSubPatchNote(version: string): SubPatchNote | undefined {
  for (const set of ALL_SET_RELEASES) {
    const found = set.subPatches.find((sub) => sub.version === version);
    if (found !== undefined) return found;
  }
  return undefined;
}
