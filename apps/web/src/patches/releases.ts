// R724000: the Patch notes page groups patches by release line. A line is the major version that
// brought a set, and lists the patches from its first card patch up to the next line's first; it
// carries a summary and the technical changes players noticed in that major version, which ship no
// card data, those before its first card patch included (v0.3.0, the Rust rewrite, is Meditative's).
// Every patch's own note (R724001) reads under fixed headings.
//
// Presentation only, over public data (CLAUDE.md rule 7). A version is compared for equality and
// never parsed or ordered (R105): a line starts at the patch named by its `first`, in patches.json's
// own order, so a lettered patch ("v0.2.10b") sits in its line like any other.

import type { Patch, PlayerNote } from "./source.ts";

/** One release line: a set and the patches that came with it. */
export type Release = {
  readonly id: string;
  readonly name: string;
  /** The version of the line's first patch. */
  readonly first: string;
  readonly summary: string;
  /** What players noticed in the line's major version that shipped no card data, one line each. */
  readonly technical: readonly string[];
};

export type ReleaseGroup = { readonly release: Release; readonly patches: readonly Patch[] };

export type NoteSection = Exclude<keyof PlayerNote, "title">;

/** R724000: a player note's headings, in the order the page shows them. */
export const NOTE_SECTIONS: readonly { readonly key: NoteSection; readonly heading: string }[] = [
  { key: "new", heading: "New" },
  { key: "balance", heading: "Balance" },
  { key: "changes", heading: "Changes" },
  { key: "text", heading: "Card text" },
  { key: "fixes", heading: "Fixes" },
];

/** R724000: the patches before every line's first, should there be any. */
export const EARLIER_RELEASE: Release = { id: "earlier", name: "Earlier patches", first: "", summary: "", technical: [] };

/** R724000: the release lines, oldest first. A major version that brings a set adds its line here. */
export const RELEASES: readonly Release[] = [
  {
    id: "core",
    name: "Core",
    first: "v0.1.0",
    summary: "The first set: 100 cards and 9 tokens, joined by The Coin in v0.1.0c and the Ghoul Token in v0.1.1.",
    technical: [
      "Hearthstone-style mana, combat and keywords on five lanes, with a backrow where Traps wait face-down.",
      "Online play in Best of 1, Conquest (three decks, with a sealed deck pick before each game) and All Random.",
      "Practice against an AI with three difficulties, with no account, and a four-lesson tutorial with a coach.",
      "Procedural sound effects and a voice line for every card.",
    ],
  },
  {
    id: "classic",
    name: "Classic & Classic+",
    first: "v0.2.0",
    summary: "Two new sets, Classic and Classic+, then their first balance patch and the Heroic Power rework.",
    technical: [
      "A ranked ladder with Grape tiers and a global leaderboard.",
      "The Card Almanac: browse every card, tokens included, with no account.",
      "Patch notes and card history: this page, and a History section in every card's detail view.",
      "A public stats page with card win rates.",
      "Background music and a main menu theme.",
      "Emotes, Units that land with a slam, and rematch and double-or-nothing offers after a game.",
      "Settings in Gameplay, Visuals and Audio tabs, kept with your account.",
    ],
  },
  {
    id: "meditative",
    name: "Meditative",
    first: "v0.3.1",
    summary:
      "The Meditative set, 102 cards and 30 tokens, shipped in v0.3.5. Before it, v0.3.1 to v0.3.4 reworded cards of the earlier sets and changed what Nerf and Buff can tune.",
    technical: [
      "The Rust rewrite (v0.3.0): the rules engine, the cards, the AI and the server were rewritten in Rust, and your browser runs the same engine as WebAssembly. Games play as they did before.",
      "The practice AI kept training: no Core, Classic or Classic+ card is barred from its decks any more, and it plays most Meditative cards too.",
      "Usernames: every player has one, shown wherever a player is named, and you can sign in with it.",
      "Leaving a practice game can save it, to resume later from the practice menu.",
      "24 emotes, 8 of them dealt to each player each game.",
      "Every Legendary and Mythic opens its play with a few bars of music of its own, and every Unit has a line to speak as it attacks.",
      "The music no longer switches to a theme when a Legendary or Mythic is played: its own few bars play, then the station carries on.",
      "Replays: watch your finished games again, step by step, share a turn as a GIF, and download a game as a file to attach to a bug report.",
      "Play together: friends, chat, challenges, watching and shared replays, and Duos co-op against an AI team.",
    ],
  },
];

/** The title the page shows for a patch: its player note's, or its record's when it has no note. */
export function playerTitle(patch: Patch): string {
  return patch.player?.title ?? patch.title;
}

/**
 * R724000: the patches by release line, newest line first and each line's patches newest first. A
 * patch named by a line's `first` opens that line; one before every line's first opens Earlier
 * patches; every other patch joins the line open before it.
 */
export function releaseGroups(patches: readonly Patch[], releases: readonly Release[]): ReleaseGroup[] {
  const groups: { release: Release; patches: Patch[] }[] = [];
  for (const patch of patches) {
    const opens = releases.find((release) => release.first === patch.version);
    let open = groups.at(-1);
    if (opens !== undefined || open === undefined) {
      open = { release: opens ?? EARLIER_RELEASE, patches: [] };
      groups.push(open);
    }
    open.patches.push(patch);
  }
  return groups.reverse().map((group) => ({ release: group.release, patches: group.patches.reverse() }));
}
