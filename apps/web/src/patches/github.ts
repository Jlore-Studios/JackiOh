// GitHub releases: the Patch notes page in Markdown, one release per shipped version, each reading as
// its patch reads on the page (R724000, R724001), with the technical log's lines players read for it
// (R1369000). `.github/workflows/releases.yml` publishes them through `apps/web/scripts/github-releases.ts`.
//
// A version only main's patches.json holds, not production's, is a pre-release: it is on staging until
// the next production merge (docs/deploy-cloudflare.md). Every tag is the commit that brought the patch
// to main, as `shipped.json` records it (R646), so the releases sit in the order and on the days the
// patches shipped. Public data only (CLAUDE.md rule 7).

import type { TextRange } from "../cards/radiantDiff.ts";
import { changeSummary } from "./ChangeList.tsx";
import { GITHUB_RELEASE_BODY_LIMIT } from "./constants.ts";
import type { CardDelta, FieldChange } from "./diff.ts";
import { cardCount, countsLine, patchCards, type PatchCards } from "./history.ts";
import { NOTE_SECTIONS, RELEASES, playerTitle, releaseGroups, technicalLines, type NoteSection, type Release } from "./releases.ts";
import type { Patch, Snapshot, TechnicalEntry } from "./source.ts";

/** One shipped version's record in `shipped.json`: the commit that brought it to main. */
export type ShippedRecord = { readonly version: string; readonly commit: string };

/** Where the game is served, and the Patch notes page's path on both hosts. */
export type Sites = { readonly production: string; readonly staging: string; readonly patchNotes: string };

export type GithubRelease = {
  readonly tag: string;
  /** The commit the tag names. */
  readonly target: string;
  readonly name: string;
  /** True while only staging has it. */
  readonly prerelease: boolean;
  readonly body: string;
};

export type GithubReleases = {
  /** The newest version production holds, the one GitHub marks Latest; null when it holds none. */
  readonly latest: string | null;
  readonly releases: readonly GithubRelease[];
};

/**
 * R388: the patches rebuilt from the catalog's git history. `shipped.json` records the commit that
 * rebuilt them, v0.2.0's, so each is tagged at the commit its record's source names instead.
 */
const REBUILT_COMMITS: Readonly<Record<string, string>> = {
  "v0.1.0": "46266903212386e2d73f7b1eee8c4b8b5d73fa9b",
  "v0.1.0b": "cd780db96d226816d8051e756f9bda0314c10207",
  "v0.1.0c": "a17a9e8ddc7ac1272861ce93a80e65e1e906f356",
  "v0.1.0d": "c219bb4ecbdc625d448fd558f68fc185c1fa1517",
  "v0.1.1": "1005c50df56d244075879403230fa448259e7702",
};

const SECTION_ICON: Readonly<Record<NoteSection, string>> = { new: "✨", balance: "⚖️", changes: "🔄", text: "📝", fixes: "🩹" };
const TECHNICAL_ICON = "⚙️";
const CARDS_ICON = "🃏";

const TIER_NAME: Readonly<Record<NonNullable<Patch["tier"]>, string>> = { major: "Major version", patch: "Patch", micro: "Micro patch" };

/** The first line of every body: a person editing a release on GitHub reads that the next run puts it back. */
const WRITTEN_BY = "<!-- Written by .github/workflows/releases.yml from crates/cards/patches/: an edit here is overwritten on its next run. -->";

/** How much of each card a release prints, most first: the next is tried while a body is over GitHub's limit. */
type CardDetail = "full" | "summary" | "names" | "counts";
const CARD_DETAILS: readonly CardDetail[] = ["full", "summary", "names", "counts"];

/** Text as Markdown prints it: every character that would mark it up is escaped, and a line break is kept. */
export function markdownText(text: string): string {
  return text.replace(/[\\`*_[\]<>~|]/gu, (char) => `\\${char}`).replace(/\r?\n/gu, "<br>");
}

/** A line a person wrote for players (a note's, a release line's, the technical log's): its emphasis is kept, its HTML is not. */
function prose(line: string): string {
  return line.replace(/[<>]/gu, (char) => `\\${char}`);
}

/** A face's text with the stretches a patch added or removed in `<ins>` or `<del>`, as the page marks them. */
function markedText(text: string, ranges: readonly TextRange[], tag: "ins" | "del"): string {
  let out = "";
  let at = 0;
  for (const range of ranges) {
    out += `${markdownText(text.slice(at, range.start))}<${tag}>${markdownText(text.slice(range.start, range.end))}</${tag}>`;
    at = range.end;
  }
  return out + markdownText(text.slice(at));
}

function changeLine(change: FieldChange): string {
  if (change.kind === "value") return `  - ${markdownText(changeSummary(change))}`;
  const now = change.after === "" ? "_No text_" : markedText(change.after, change.added, "ins");
  const was = change.removed.length === 0 ? "" : `<br>_Was:_ ${markedText(change.before, change.removed, "del")}`;
  return `  - ${markdownText(change.label)}: ${now}${was}`;
}

function changedCard(delta: Extract<CardDelta, { kind: "changed" }>, detail: CardDetail): string {
  const name = `**${markdownText(delta.def.name)}**`;
  if (detail === "full") return [`- ${name}`, ...delta.changes.map(changeLine)].join("\n");
  return `- ${name}: ${markdownText(delta.changes.map(changeSummary).join("; "))}`;
}

function names(defs: readonly { readonly name: string }[]): string {
  return defs.map((def) => markdownText(def.name)).join(" · ");
}

function cardsSection(cards: PatchCards, detail: CardDetail): string[] {
  const out: string[] = [];
  const heading = (title: string, count: number): string => `#### ${title} (${String(count)})`;
  if (cards.changed.length > 0) {
    out.push(heading("Changed", cards.changed.length));
    out.push(detail === "names" ? names(cards.changed.map((delta) => delta.def)) : cards.changed.map((delta) => changedCard(delta, detail)).join("\n"));
  }
  if (cards.dataOnly.length > 0) {
    out.push(heading("Changed only in details the card doesn't print", cards.dataOnly.length));
    out.push(detail === "names" ? names(cards.dataOnly.map((delta) => delta.def)) : cards.dataOnly.map((delta) => changedCard(delta, "summary")).join("\n"));
  }
  if (cards.added.length > 0) {
    out.push(heading("Added", cards.added.reduce((sum, group) => sum + group.cards.length, 0)));
    for (const group of cards.added) {
      out.push(`**${markdownText(group.label)}** (${String(group.cards.length)}): ${names(group.cards)}`);
    }
  }
  if (cards.removed.length > 0) {
    out.push(heading("Removed", cards.removed.length));
    out.push(names(cards.removed));
  }
  return out;
}

function listed(cards: PatchCards): number {
  return cards.changed.length + cards.dataOnly.length + cards.added.reduce((sum, group) => sum + group.cards.length, 0) + cards.removed.length;
}

function host(origin: string): string {
  return new URL(origin).host;
}

function link(origin: string, path: string): string {
  return `[${host(origin)}](${origin}${path})`;
}

/** The release line's summary and technical changes, which open the line's major. */
function lineSection(release: Release): string[] {
  if (release.summary === "" && release.technical.length === 0) return [];
  const out = [`## ${markdownText(release.name)}`];
  if (release.summary !== "") out.push(`_${prose(release.summary)}_`);
  if (release.technical.length > 0) {
    out.push("**Technical changes**");
    out.push(release.technical.map((line) => `- ${prose(line)}`).join("\n"));
  }
  out.push("---");
  return out;
}

function lines(heading: string, icon: string, items: readonly string[]): string[] {
  return items.length === 0 ? [] : [`### ${icon} ${heading}`, items.map((line) => `- ${prose(line)}`).join("\n")];
}

type Placed = { readonly release: Release; readonly major: boolean };

function body(patch: Patch, placed: Placed, technical: readonly string[], cards: PatchCards, live: boolean, sites: Sites, detail: CardDetail): string {
  const out = [WRITTEN_BY];
  if (!live) {
    out.push(
      [
        "> [!NOTE]",
        `> **On staging.** Try it at ${link(sites.staging, "")} now; it reaches ${link(sites.production, "")} with the next production merge.`,
      ].join("\n"),
    );
  }
  const tier = patch.tier === undefined ? [] : [TIER_NAME[patch.tier]];
  out.push([`**${patch.date}**`, markdownText(placed.release.name), ...tier].join(" · "));
  if (placed.major) out.push(...lineSection(placed.release));
  for (const { key, heading } of NOTE_SECTIONS) out.push(...lines(heading, SECTION_ICON[key], patch.player?.[key] ?? []));
  out.push(...lines("Technical changes", TECHNICAL_ICON, technical));

  out.push(`### ${CARDS_ICON} Cards`);
  const count = listed(cards);
  if (count === 0) {
    out.push("_No card changed._");
  } else {
    out.push(`**${countsLine(patch)}**`);
    if (detail !== "counts") {
      out.push(
        [
          "<details>",
          `<summary>Show the ${cardCount(count)}</summary>`,
          "",
          cardsSection(cards, detail).join("\n\n"),
          "",
          "</details>",
        ].join("\n"),
      );
    }
  }
  const site = live ? sites.production : sites.staging;
  out.push("---", `<sub>Every patch, card by card: Patch notes on ${link(site, sites.patchNotes)}</sub>`);
  return `${out.join("\n\n")}\n`;
}

/**
 * Every shipped version's release, in patches.json's order (R105), from the record and its
 * snapshots. `snapshot` reads one version's catalog snapshot, each asked for once, in order;
 * `live` holds the versions production's patches.json holds.
 */
export function githubReleases(input: {
  readonly patches: readonly Patch[];
  readonly shipped: readonly ShippedRecord[];
  readonly technical: readonly TechnicalEntry[];
  readonly snapshot: (version: string) => Snapshot | null;
  readonly live: ReadonlySet<string>;
  readonly sites: Sites;
  readonly releases?: readonly Release[];
  readonly limit?: number;
}): GithubReleases {
  const { patches, live, sites, limit = GITHUB_RELEASE_BODY_LIMIT } = input;
  const commits = new Map(input.shipped.map((record) => [record.version, record.commit]));
  const technical = technicalLines(input.technical);
  const placed = new Map<string, Placed>();
  for (const group of releaseGroups(patches, input.releases ?? RELEASES)) {
    const oldest = group.patches.at(-1)?.version;
    for (const patch of group.patches) placed.set(patch.version, { release: group.release, major: patch.version === oldest });
  }

  let before: Snapshot | null = null;
  const releases = patches.map((patch): GithubRelease => {
    const after = input.snapshot(patch.version);
    if (after === null) throw new Error(`no snapshot of ${patch.version}`);
    const target = REBUILT_COMMITS[patch.version] ?? commits.get(patch.version);
    if (target === undefined) throw new Error(`shipped.json records no commit for ${patch.version}`);
    const where = placed.get(patch.version);
    if (where === undefined) throw new Error(`${patch.version} sits in no release line`);
    const cards = patchCards(patch, after, before);
    before = after;
    const isLive = live.has(patch.version);
    const write = (detail: CardDetail): string => body(patch, where, technical.get(patch.version) ?? [], cards, isLive, sites, detail);
    const text = CARD_DETAILS.map(write).find((candidate) => candidate.length <= limit) ?? write("counts");
    return { tag: patch.version, target, name: `${patch.version} — ${playerTitle(patch)}`, prerelease: !isLive, body: text };
  });
  const latest = releases.findLast((release) => !release.prerelease)?.tag ?? null;
  return { latest, releases };
}
