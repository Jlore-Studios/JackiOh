// The GitHub releases (github.ts): one per shipped version, read as the Patch notes page reads it, a
// pre-release while only staging has it, each tagged at the commit that brought it to main.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { GITHUB_RELEASE_BODY_LIMIT } from "./constants.ts";
import { FIXTURE_PATCHES, FIXTURE_SNAPSHOTS, V1, V2, V3 } from "./fixtures.ts";
import { githubReleases, markdownText, type GithubRelease, type ShippedRecord } from "./github.ts";
import type { Release } from "./releases.ts";
import type { Patch, Snapshot, TechnicalEntry } from "./source.ts";

const SITES = { production: "https://prod.example", staging: "https://staging.example", patchNotes: "/patch-notes" };
const SHIPPED: readonly ShippedRecord[] = [
  { version: V1, commit: "c1" },
  { version: V2, commit: "c2" },
  { version: V3, commit: "c3" },
];
const TECHNICAL: readonly TechnicalEntry[] = [
  { version: V3, commit: "t1", pr: 1, title: "faster menus", kind: "app", player: "Menus open **faster**." },
  { version: V3, commit: "t2", pr: 2, title: "a refactor", kind: "internal", player: "Nothing a player sees." },
];

function fixtureReleases(options: { live?: readonly string[]; patches?: readonly Patch[]; releases?: readonly Release[]; limit?: number } = {}) {
  return githubReleases({
    patches: options.patches ?? FIXTURE_PATCHES,
    shipped: SHIPPED,
    technical: TECHNICAL,
    snapshot: (version) => FIXTURE_SNAPSHOTS[version] ?? null,
    live: new Set(options.live ?? [V1, V2, V3]),
    sites: SITES,
    ...(options.releases === undefined ? {} : { releases: options.releases }),
    ...(options.limit === undefined ? {} : { limit: options.limit }),
  });
}

function release(releases: readonly GithubRelease[], tag: string): GithubRelease {
  const found = releases.find((candidate) => candidate.tag === tag);
  if (found === undefined) throw new Error(`no release ${tag}`);
  return found;
}

describe("GitHub releases", () => {
  it("are one per shipped version in patches.json's order, each tagged at the commit shipped.json records", () => {
    const { releases } = fixtureReleases();
    expect(releases.map(({ tag, target }) => [tag, target])).toEqual([
      [V1, "c1"],
      [V2, "c2"],
      [V3, "c3"],
    ]);
    // A patch with no player note is named by its record's title, as the page names it.
    expect(releases.map((entry) => entry.name)).toEqual([`${V1} — First cards`, `${V2} — The knight grows`, `${V3} — Two fixture sets`]);
  });

  it("make a version only staging has a pre-release, and mark the newest one production has Latest", () => {
    const { latest, releases } = fixtureReleases({ live: [V1, V2] });
    expect(latest).toBe(V2);
    expect(releases.map((entry) => entry.prerelease)).toEqual([false, false, true]);
    const staged = release(releases, V3).body;
    expect(staged).toContain("> [!NOTE]");
    expect(staged).toContain("[staging.example](https://staging.example)");
    expect(staged).toContain("Patch notes on [staging.example](https://staging.example/patch-notes)");
    expect(release(releases, V2).body).not.toContain("[!NOTE]");
    expect(release(releases, V2).body).toContain("Patch notes on [prod.example](https://prod.example/patch-notes)");
    expect(fixtureReleases({ live: [] }).latest).toBeNull();
  });

  it("read a patch's note under the page's headings in its order, then its technical lines players read", () => {
    const body = release(fixtureReleases().releases, V3).body;
    const at = (text: string): number => body.indexOf(text);
    expect(body.startsWith("<!-- Written by .github/workflows/releases.yml")).toBe(true);
    expect(body).toContain("**2026-03-04** · Earlier patches · Micro patch");
    expect(at("### ✨ New")).toBeGreaterThan(0);
    expect(at("### ⚖️ Balance")).toBeGreaterThan(at("### ✨ New"));
    expect(at("### 🩹 Fixes")).toBeGreaterThan(at("### ⚖️ Balance"));
    expect(at("### ⚙️ Technical changes")).toBeGreaterThan(at("### 🩹 Fixes"));
    expect(body).toContain("- Test Bolt costs (2), up from (1).");
    // A line a person wrote keeps its emphasis; an Internal entry is the record's alone.
    expect(body).toContain("- Menus open **faster**.");
    expect(body).not.toContain("Nothing a player sees.");
    expect(body).not.toContain("### 🔄 Changes");
    expect(release(fixtureReleases().releases, V2).body).not.toContain("### ✨ New");
  });

  it("list a patch's cards as the page does: what changed on each, the details it doesn't print, the added by set and the removed", () => {
    const { releases } = fixtureReleases();
    const third = release(releases, V3).body;
    expect(third).toContain("**3 cards added · 2 cards changed**");
    expect(third).toContain("<summary>Show the 5 cards</summary>");
    expect(third).toContain("#### Changed (1)\n\n- **Test Bolt**\n  - Cost:");
    expect(third).toContain("  - Text: Deal <ins>4</ins> damage. <ins>Your</ins> Spells cost (1) less.<br>_Was:_ Deal <del>3</del> damage. Spells cost (1) less <del>this turn</del>.");
    expect(third).toContain("#### Changed only in details the card doesn't print (1)\n\n- **Quiet Golem**: Lines of code:");
    expect(third).toContain("#### Added (3)\n\n**Classic** (1): Fresh Face\n\n**Classic+** (1): Pancake Pal\n\n**Classic+ tokens** (1): Syrup Token");
    const second = release(releases, V2).body;
    expect(second).toContain("#### Removed (1)\n\nOld Relic");
  });

  it("open a release line's major with the line's summary and technical changes, which no other patch repeats", () => {
    const line: Release = { id: "fixture", name: "Fixture line", summary: "Four test cards, then more.", technical: ["A test screen."] };
    const patches = FIXTURE_PATCHES.map((patch, at) => (at === 0 ? { ...patch, tier: "major" as const } : patch));
    const { releases } = fixtureReleases({ patches, releases: [line] });
    const major = release(releases, V1).body;
    expect(major).toContain("**2026-01-02** · Fixture line · Major version");
    expect(major).toContain("## Fixture line\n\n_Four test cards, then more._\n\n**Technical changes**\n\n- A test screen.\n\n---");
    expect(release(releases, V3).body).toContain("· Fixture line ·");
    expect(release(releases, V3).body).not.toContain("## Fixture line");
  });

  it("list the cards more briefly while a body is over the limit, down to its counts", () => {
    const full = release(fixtureReleases().releases, V3).body;
    const summary = release(fixtureReleases({ limit: full.length - 1 }).releases, V3).body;
    expect(summary).toContain("- **Test Bolt**: Cost:");
    expect(summary).not.toContain("<ins>");
    const counts = release(fixtureReleases({ limit: 1 }).releases, V3).body;
    expect(counts).toContain("**3 cards added · 2 cards changed**");
    expect(counts).not.toContain("<details>");
  });

  it("escape what Markdown would read in a card's words, and keep its line breaks", () => {
    expect(markdownText("*Star* <b>_x_</b> [1]\nDraw ~2~ | `go`")).toBe("\\*Star\\* \\<b\\>\\_x\\_\\</b\\> \\[1\\]<br>Draw \\~2\\~ \\| \\`go\\`");
  });
});

describe("GitHub releases of the real history", () => {
  const dir = join(dirname(fileURLToPath(import.meta.url)), "../../../../crates/cards/patches");
  const read = <T>(file: string): T => JSON.parse(readFileSync(join(dir, file), "utf8")) as T;
  const patches = read<Patch[]>("patches.json");
  const shipped = read<ShippedRecord[]>("shipped.json");
  const { latest, releases } = githubReleases({
    patches,
    shipped,
    technical: read<TechnicalEntry[]>("technical.json"),
    snapshot: (version) => read<Snapshot>(`${version}.json`),
    live: new Set(patches.map((patch) => patch.version)),
    sites: SITES,
  });

  it("give every shipped version one release within GitHub's limit, the newest Latest", () => {
    expect(releases.map((entry) => entry.tag)).toEqual(patches.map((patch) => patch.version));
    expect(latest).toBe(patches.at(-1)?.version);
    for (const entry of releases) {
      expect(entry.body.length, entry.tag).toBeLessThanOrEqual(GITHUB_RELEASE_BODY_LIMIT);
      expect(entry.target, entry.tag).toMatch(/^[0-9a-f]{40}$/u);
    }
  });

  it("tag the patches rebuilt from git history at their own commits, and every other at shipped.json's", () => {
    const commit = new Map(shipped.map((record) => [record.version, record.commit]));
    const rebuilt = releases.filter((entry) => entry.target !== commit.get(entry.tag));
    expect(rebuilt.map(({ tag, target }) => [tag, target.slice(0, 7)])).toEqual([
      ["v0.1.0", "4626690"],
      ["v0.1.0b", "cd780db"],
      ["v0.1.0c", "a17a9e8"],
      ["v0.1.0d", "c219bb4"],
      ["v0.1.1", "1005c50"],
    ]);
  });

  it("open each release line at its major, Core's at the first", () => {
    const opened = releases.filter((entry) => entry.body.includes("\n## ")).map((entry) => entry.tag);
    expect(opened[0]).toBe("v0.1.0");
    expect(opened).toEqual(patches.filter((patch) => patch.tier === "major").map((patch) => patch.version));
  });
});
