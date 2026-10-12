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

/**
 * Every set's release notes, newest set first. The page opens on the first entry. A sub-patch's
 * summary says only what its catalog patch's notes, card changes and source change say
 * (`patches.json`), in words a player reads; a set's technical changes are what merged to `main`
 * during its version line. A test holds every number a summary states to its patch's notes.
 */
export const ALL_SET_RELEASES: readonly SetReleaseNotes[] = [
  {
    id: "meditative",
    name: "Meditative",
    versions: "v0.3.0 – v0.3.6",
    summary:
      "The Meditative set: 102 cards and 30 tokens, shipped in v0.3.5 into the deck builder, the Almanac, random decks and every card pool that names no set. Among them are Magnetic Units, the Jade Counter, an extra turn, a card that can win the game and a card you craft yourself. Before it shipped, v0.3.1 to v0.3.4 changed cards of the earlier sets: a text pass, more numbers to tune, Heroic Power's numbers tied to the power it has, and Degrade and Upgrade renamed Nerf and Buff.",
    technicalChanges: [
      "The Rust rewrite (v0.3.0): the rules engine, the cards, the AI and the server are rewritten in Rust, and your browser runs the same engine compiled to WebAssembly. It plays exactly as v0.2.11 did: games recorded on the old engine replay step for step on the new one.",
      "The practice AI: its training brought back into its decks every Core card it used to leave out, so it now builds from every Core, Classic and Classic+ card. Its decks leave the Meditative cards out until it has been trained on them.",
      "Usernames: every player has a username, shown wherever a player is named, and you can sign in with it.",
      "Practice and rooms: leaving a practice game now asks whether to save it, a saved game waits on the practice menu with a Resume banner, and an online room can be shared as an invite link.",
      "Emotes and sound: 24 emotes, 8 of them dealt to each player each game; every Legendary and Mythic opens its play with a few bars of music of its own; every Unit speaks as it attacks, and the Meditative cards are voiced.",
    ],
    subPatches: [
      {
        version: "v0.3.6",
        title: "Zephyrs and Zephrys Zealotism draw on every set",
        summary:
          "Zephyrs and Zephrys Zealotism no longer name a set: each draws on every non-token card of every shipped set, Meditative included, leaving out only itself, so Zephyrs may offer Zephrys Zealotism and Zealotism may deal Zephyrs. A later set joins them the day it ships. Only their pools and words change.",
      },
      {
        version: "v0.3.5",
        title: "Meditative ships",
        summary:
          "The Meditative set's 102 cards and 30 tokens join the deck builder, the Almanac, random decks and every card pool that names no set. Call to Chaos (Core Edition) and Call to Chaos (Classic+ Edition) now link the Meditative Edition their text names; no other card of an earlier set changes.",
      },
      {
        version: "v0.3.4",
        title: "Degrade is Nerf, Upgrade is Buff",
        summary:
          "Degrade is renamed Nerf and Upgrade is renamed Buff on both faces of every card that printed them, from Withering Storm to Fine-Tuning, and Heroic Power's Radiant Steady Shot now says it permanently deals two more damage. Only the words change; the cards play as before.",
      },
      {
        version: "v0.3.3",
        title: "Heroic Power's numbers belong to its power",
        summary:
          "Degrade and Upgrade (now Nerf and Buff) and KY's Constant reach only the numbers of the power Heroic Power has now; a number of another power keeps its tuning until a reroll brings that power back. Thirteen numbers that only a base face prints, Life Tap's damage among them, are tuned on the base face only.",
      },
      {
        version: "v0.3.2",
        title: "More numbers to tune",
        summary:
          "111 more numbers on 80 cards can be moved by Degrade and Upgrade (now Nerf and Buff) and KY's Constant: damage, healing, draws, tokens, copies, buffs, Armor, mana, discounts, set costs, thresholds and Heroic Power's powers. Each prints as before.",
      },
      {
        version: "v0.3.1",
        title: "Card text pass",
        summary:
          "Text only; no card does anything new. Every Trap's condition reads “Reveals when”, several cards say plainly what they already did, and “GY” is written out as graveyard.",
      },
    ],
  },
  {
    id: "classic",
    name: "Classic & Classic+",
    versions: "v0.2.0 – v0.2.11",
    summary:
      "Two sets at once: Classic (90 cards) and Classic+ (78 cards and 38 tokens), with the keywords Animated, Activate, Brittle, Degrade and Upgrade (Degrade and Upgrade are now Nerf and Buff). The line went on to the first Classic and Classic+ balance patch and the Heroic Power rework.",
    technicalChanges: [
      "Ranked play: a ranked ladder with Raisin placement games, five Grape tiers, the top 100 shown as numbered Jlorious ranks, seasons with a soft reset, and a global leaderboard.",
      "The Card Almanac: browse every card, tokens included, with no account.",
      "Patch notes and card history: this page, and a History section on every card showing each patch that changed it.",
      "Statistics: a public stats page with card win rates and player summaries (with a privacy opt-out), and a win-rate sort in the deck builder.",
      "Music: background music that follows the game, and a main menu theme.",
      "Settings: Gameplay, Visuals and Audio tabs, and your game settings follow your account.",
      "At the table: emotes, Units that land with a slam that grows with their size, and rematch and double-or-nothing offers after a game.",
    ],
    subPatches: [
      {
        version: "v0.2.11",
        title: "The Heroic Power rework",
        summary:
          "Heroic Power costs (0), and playing it uses nothing. Each of its thirteen powers, from Expedition Map to Terminus Tricks, is an Activate you pay for once per turn, each with a Radiant face. Ping picks its target as it is activated, and in play the card shows only the power it rolled.",
      },
      {
        version: "v0.2.10d",
        title: "Balance patch 1 follow-up",
        summary:
          "A deck too short of other cards fills an opening hand with its Cast on Draw cards, which wait uncast and are cast as the game starts. Divine Favor's and Rewind's numbers are tuned on the Radiant face only, the face that prints them. Radiant The Power to Punish marks every enemy Unit it will destroy, and Core cards no longer say “to a target”.",
      },
      {
        version: "v0.2.10c",
        title: "Bounce is for permanents",
        summary:
          "Bounce now means only a permanent returned to hand from the field. Seven cards that return cards from elsewhere, such as Reoccurring Dream and KY's Math Equation, say “Return … to hand” again and play as before.",
      },
      {
        version: "v0.2.10b",
        title: "Three new tags",
        summary:
          "Catalyst on Solarius and Felinor Flagbearer, Prime on the tokens their Deaths shuffle in, and Acclaimed on BOOM! Big Max and Wardrum. Solarius-Prime is now Solarius Prime. No card plays differently.",
      },
      {
        version: "v0.2.10",
        title: "Classic and Classic+ balance patch 1",
        summary:
          "Discards are random unless a card says you choose; every card that returns a card to hand says Bounce; Plague Counters placed with no card named all go on one chosen permanent; Recruit skips (X) Cost cards; a Locked zone refuses only plays; a transformed Unit keeps its battle position; Reveal shows a backrow card without firing it; and balance and text changes run across Classic and Classic+.",
      },
      {
        version: "v0.2.9",
        title: "Rarity pass",
        summary:
          "Fifteen rarity changes across Classic, Classic+ and three Core cards. Classic and Classic+ Traps say “Reveals when”, Plague Tokens are now Plague Counters, Radiant Crop Dusting places 3 Plague Counters and draws 3, and Pocket Chaos costs (4), each copy it passes on costing (1) less.",
      },
      {
        version: "v0.2.8b",
        title: "Gary the Gambler buffed",
        summary:
          "Gary the Gambler's Cry, on both faces, also flips one more coin: heads gives him Divine Shield, tails gives him Rush.",
      },
      {
        version: "v0.2.8",
        title: "Glitch",
        summary:
          "A hidden token, Glitch: once a card named “… in the System” has been played, any card made into a hand or deck has a tiny chance to be Glitch instead. It is in no pool, nor in the Almanac or the deck builder, and when played it resets the match, swaps the seats, lays two other games' boards on the field, or voids the match.",
      },
      {
        version: "v0.2.7b",
        title: "Condition glow for more Core cards",
        summary:
          "The yellow “condition met” glow now lights for more Core cards, among them Bread and Butter, Bear Honeypot, Spiteful Stab and, in your hand, Quickstriker. No card plays differently.",
      },
      {
        version: "v0.2.7",
        title: "Plastic Surgery, Craft a Card, Crushing Walls and Plague Chalice",
        summary:
          "Plastic Surgery can no longer be played with no Unit to target. Radiant Craft a Card no longer draws a card. Crushing Walls now shows its spiked walls closing in on lanes 1 and 5, and a card in your hand that Plague Chalice would counter now wears a warning.",
      },
      {
        version: "v0.2.6",
        title: "Book of Wildfire becomes a different Book",
        summary:
          "At the end of your turn, a Book of Wildfire in your hand becomes a different random Book (Radiant: a random Radiant Book), never Wildfire and never the Book it already is, and keeps changing each turn.",
      },
      {
        version: "v0.2.5",
        title: "Field Spells stop coming to life",
        summary:
          "The eighteen Field Spells v0.2.3 made Animated lose Animated and their unit faces again. Ivory Tower's fusion and Final Gambit's guard stay, and an Animated card with no printed stats now fights as a 0/1.",
      },
      {
        version: "v0.2.4",
        title: "Aimed random casts and Deft",
        summary:
          "A random cast aims harmful picks at enemies and helpful ones at friends (Jogg's Box is unchanged). The Deft keyword, one attack and one position switch a turn in either order, is printed by Deft Duelist and joins the random keyword pool.",
      },
      {
        version: "v0.2.3",
        title: "Animated Field Spells and Ivory Tower's fusion",
        summary:
          "Eighteen Field Spells, Masochism Mask among them, gain Animated with a unit face (undone in v0.2.5). Ivory Tower fuses the first Unit you stack onto it into itself, and Final Gambit neither heals nor draws once its redirected hit has defeated the enemy hero.",
      },
      {
        version: "v0.2.2",
        title: "Mechanics changes",
        summary:
          "Chaos Machine never Upgrades (now Buffs) itself, Joro answers only a Spell's targeting, a Grape that any random pool picks is re-rolled by the Grapes' own odds, every card that uses Plague Tokens gains the Plague tag, Blade Storm casts Whirlwind until a Unit dies, Adaptive Growth's buff and debuff are both 2 (Radiant 3), and Exile's base face answers (2) Cost or less.",
      },
      {
        version: "v0.2.1",
        title: "Card text pass",
        summary: "Consistent terms and house style across Core, Classic and Classic+ card text.",
      },
      {
        version: "v0.2.0",
        title: "Classic and Classic+",
        summary:
          "Two new sets, Classic (90 cards) and Classic+ (78 cards and 38 tokens). In Core, Hit Job costs (3), Flood, Collateral Damage, Big Felinor, Snom Bunny Mind Control and Twisting Nether cost (4), Masochism Mask costs (1), and seven cards get new text.",
      },
    ],
  },
  {
    id: "core",
    name: "Core",
    versions: "v0.1.0 – v0.1.1",
    summary:
      "The base set: 100 cards and 9 tokens as first built, with The Coin and the Ghoul Token joining them by v0.1.1. Hearthstone-style mana, combat and keywords meet five shared lanes and a hidden backrow of Traps.",
    technicalChanges: [
      "The board: five lanes, each with a Unit zone and a backrow zone on either side, where Traps wait face-down until they fire.",
      "Online play: Best of 1, Conquest (a trio of decks, each game's deck picked in secret) and All Random. Both players mulligan at once, and a draw can be offered.",
      "Practice: an AI opponent with three difficulties and a four-lesson tutorial with a coach, with no account needed.",
      "Cards and board: card faces with procedural art, card inspect, a deck builder, drag to play with green and yellow highlights, and a board that fits a phone.",
      "Sound and effects: sound effects, a voice line for every card, and visual effects over every animated event.",
    ],
    subPatches: [
      {
        version: "v0.1.1",
        title: "Pierce, Deck and Tribute",
        summary:
          "The Pierce keyword arrives, an Indestructible Unit never has Taunt, an X you choose is at least one, the Ghoul Token is added and Heroic Power gains an eighth power, Stitching. The game says Deck for the library and Tribute for a sacrifice wherever you read them, a face-down Trap shows its cost, and 105 cards and tokens change in all.",
      },
      {
        version: "v0.1.0d",
        title: "The Radiant pass",
        summary:
          "Every Radiant face meets one standard and prints its text in full. Cards gain the Jlockeed tag where it applies, and every card lists the cards its text names.",
      },
      {
        version: "v0.1.0c",
        title: "Call to Chaos and The Coin",
        summary:
          "Call to Chaos's Rush Token roll summons five Radiant Rush Tokens, and The Coin is added for the player going second.",
      },
      {
        version: "v0.1.0b",
        title: "Core balance changes",
        summary:
          "Right-house defender gains Taunt. Radiant Saintess gains Reborn and her Death, no longer her Cry, makes your other Units Radiant. The Rush, Sheep, Felinor and Bread Tokens' Radiant faces are stronger, and Twisted Sorcerer's name is spelled right.",
      },
      {
        version: "v0.1.0",
        title: "Core",
        summary: "The Core set as first built: 100 cards and 9 tokens.",
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
