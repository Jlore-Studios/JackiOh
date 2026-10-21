// R724000: the Patch notes page groups patches by release line. A line is the major version that
// brought a set (a Z), and lists the patches from it up to the next major; it carries a summary and
// the technical changes players noticed in that major version, which ship no card data (v0.3.0, the
// Rust rewrite, is Meditative's). Every patch's own note (R724001) reads under fixed headings.
//
// Presentation only, over public data (CLAUDE.md rule 7). A version is compared for equality and
// never parsed or ordered (R105): a line starts at its major (R1369000), its `tier` in patches.json or,
// for a patch shipped before tiers were recorded, the one that adds a card of a set no earlier patch
// held, in patches.json's own order, so a lettered patch ("v0.2.10b") sits in its line like any other.

import type { Patch, PlayerNote, TechnicalEntry } from "./source.ts";

/** One release line: a set and the patches that came with it. Its first patch is the line's major. */
export type Release = {
  readonly id: string;
  readonly name: string;
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

/** R724000: the patches before every major, should there be any. */
export const EARLIER_RELEASE: Release = { id: "earlier", name: "Earlier patches", summary: "", technical: [] };

/**
 * R724000, R1369000: the release lines, oldest first, the n-th opened by the n-th major. A major
 * version that brings a set adds its line here.
 */
export const RELEASES: readonly Release[] = [
  {
    id: "core",
    name: "Core",
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
    summary:
      "Two new sets, Classic and Classic+, then their first balance patch and the Heroic Power rework. From v0.2.5f to v0.2.7b, cards of the earlier sets were reworded and Nerf and Buff could tune more.",
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
    summary: "The Meditative set, 102 cards and 30 tokens, shipped in v0.3.0.",
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
      "Every token says so: its type line reads Token Unit or Token Spell, and on the board and on small cards it wears a Token mark.",
    ],
  },
  {
    id: "meditativeplus",
    name: "Meditative+",
    summary: "The Meditative+ set, 26 cards and 4 tokens, shipped in v0.4.0, with a new tag, Potion.",
    technical: [
      "The economy is live: a new account starts with the 39 cards of the three practice decks and earns the rest from games, quests, set tracks, packs and tokens. Every card you own stays yours; the six full-art faces are earned like any Epic cosmetic.",
    ],
  },
];

/** The title the page shows for a patch: its player note's, or its record's when it has no note. */
export function playerTitle(patch: Patch): string {
  return patch.player?.title ?? patch.title;
}

/** A card id's set, as an id spells it: the part before its first dash. */
function idSet(id: string): string {
  const dash = id.indexOf("-");
  return dash === -1 ? id : id.slice(0, dash);
}

/**
 * R1369000: which patches are majors, a Z's first version: those whose recorded `tier` says so, and
 * one with no tier that adds a card of a set no earlier patch held.
 */
export function majors(patches: readonly Patch[]): boolean[] {
  const seen = new Set<string>();
  return patches.map((patch) => {
    const fresh = patch.changes.some((change) => change.kind === "added" && !seen.has(idSet(change.id)));
    for (const change of patch.changes) seen.add(idSet(change.id));
    return patch.tier === undefined ? fresh : patch.tier === "major";
  });
}

/** A line for a major whose line `RELEASES` does not write yet, named by its note. */
function unwrittenRelease(patch: Patch): Release {
  return { id: patch.version, name: playerTitle(patch), summary: "", technical: [] };
}

/**
 * R724000, R1369000: the patches by release line, newest line first and each line's patches newest
 * first. The n-th major opens the n-th line; patches before every major open Earlier patches; every
 * other patch joins the line open before it.
 */
export function releaseGroups(patches: readonly Patch[], releases: readonly Release[]): ReleaseGroup[] {
  const groups: { release: Release; patches: Patch[] }[] = [];
  const major = majors(patches);
  let opened = 0;
  patches.forEach((patch, at) => {
    let open = groups.at(-1);
    if (major[at] === true || open === undefined) {
      const release = major[at] === true ? (releases[opened] ?? unwrittenRelease(patch)) : EARLIER_RELEASE;
      if (major[at] === true) opened += 1;
      open = { release, patches: [] };
      groups.push(open);
    }
    open.patches.push(patch);
  });
  return groups.reverse().map((group) => ({ release: group.release, patches: group.patches.reverse() }));
}

/**
 * R1369000: the technical log's lines players read, by the version of the X they were logged on:
 * App and AI entries that have a player line, in the log's order. Internal entries are the record's.
 */
export function technicalLines(entries: readonly TechnicalEntry[]): ReadonlyMap<string, readonly string[]> {
  const lines = new Map<string, string[]>();
  for (const entry of entries) {
    if (entry.kind === "internal" || entry.player === undefined || entry.player.trim() === "") continue;
    const list = lines.get(entry.version) ?? [];
    list.push(entry.player);
    lines.set(entry.version, list);
  }
  return lines;
}
