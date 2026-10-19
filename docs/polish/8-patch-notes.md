# Patch notes: what's wrong, the options, and what each costs

A design note (not yet a plan). It covers both halves of "patch notes": what a **player** sees (`/patch-notes`,
the History section of every card, the New ribbon) and what the **repo** keeps to produce it (snapshots,
`patches.json`, fragments, `patches ship`, the replay catalogs). Numbers were measured on `main` at
0b35f1a5 (2026-10-19), with the v0.3.8–v0.3.15 queue still pending.

## TL;DR

The system records every version **three times**:
- as a full copy of the catalog (95.9% of the 6.76 MB patch directory);
- as derived lists (`changes`, `index.json`, `commits`, `blob`);
- as the per-version deltas `crates/cards/build.rs` recomputes on every build.

The browser then recomputes the diffs a fourth time, from whole catalogs. Players get a page in which 32% of
the changed-card entries are internal bookkeeping ("Lines of code: not recorded → 15") and 40% are wording
passes, each drawn as two full card faces.

Most of the bloat goes away with four moves:

1. **Players see only what prints.** Internal fields are hidden, wording passes are collapsed, and the note
   says *why* while the diff says *what*.
2. **Store deltas, not copies.** 6.49 MB becomes about 0.6 MB, and a ship PR goes from +146,681 lines to a few
   thousand.
3. **Ship the browser small, precomputed notes.** Each opened patch costs tens of KB, not two 360 KB catalogs.
4. **Name the version when the card change merges, not after.** Today every card change runs on `main`
   under the *previous* version's name until the promotion PR lands. That was four days for Oct 15–19, and
   games from those days will refuse to replay once v0.3.15 ships.

See the [cost-benefit table](#cost-benefit) and the [recommended order](#recommended-order).

---

## 1. How it works today

| Piece | What it is | Size |
|---|---|---|
| `crates/cards/patches/v*.json` | a **full** catalog per version (30 files) | 6.49 MB (≈55 KB early, ≈360 KB now) |
| `patches.json` | version, date, title, player note, `commits`, derived `changes` | 211 KB (143 KB of it derived `changes`) |
| `index.json` | card id → versions it changed in (derived) | 28 KB |
| `shipped.json` | version → commit + git blob hash of its snapshot | 4 KB |
| `pending/*.json` | one fragment per card-changing branch (prose + claimed cards) | 9 files, 31 KB |
| `crates/tools/src/patches.rs` | `patches check`/`ship`/new: own JSON writer, own SHA-1, ship ordering from git | 4,688 lines (2,169 code, 2,521 tests) |
| `.github/workflows/patches-ship.yml` | after merge: ship, open a `patches/ship-*` PR with the bot token, auto-merge | 123 lines |
| `apps/web/src/patches/` | page, card history, diff, ribbon | 3,859 lines (1,346 tests, 789 CSS) |
| Rules | R105, R375, R388, R507, R646, R650, R743, R724000, R724001, R767000, R1266000, R1280000, R1383, plus ~10 touching | 13 core notes; ≈180 lines of docs elsewhere |

**The flow:**
1. A branch edits `catalog.json` and adds a fragment.
2. After the merge, `patches ship` turns each fragment into a version: a full snapshot, then `patches.json`,
   `shipped.json` and `index.json` are rewritten, and `CATALOG_VERSION` is bumped in `render.yaml` and
   `.env.example`.
3. A second PR carries that and merges itself.
4. `build.rs` parses every snapshot on every build to make the replay catalogs (R767000). The browser keeps
   only versions from v0.3.7 on.
5. The web page lazy-loads two whole snapshots per opened patch and diffs them in TypeScript (`diff.ts`), a
   second implementation of `patches.rs`'s `changed_fields`.

## 2. The problems

### Players
- **Internal fields shown as card changes.**
  - 234 of 723 changed-card entries (32%) change only lines of code (`loc`), card references (`refs`) or the
    tuning metadata of `params`.
  - v0.2.7b, v0.3.2 (80 of 80), v0.3.3 and v0.3.7b are *entirely* that kind of change, and v0.3.7b is the
    patch the page opens on.
  - Card history draws two full faces for each such entry.
- **Wording passes drown balance changes.**
  - 290 of 723 entries (40%) are text-only.
  - Opening v0.1.1 draws 105 card faces, v0.2.0's button reads "Show the 317 cards", and v0.3.8 (pending) is a
    72-card wording pass.
  - The page can't tell a wording pass from a real text change: Hinder's added "Discard 1." is "text only"
    by field.
- **Notes repeat the diff.** v0.2.0's note says "Hit Job costs (3), up from (2)." and the diff under it says
  "Cost (2) Cost → (3) Cost". Notes rarely say *why*.
- **Too many versions.**
  - 30 versions in 27 days, 7 on Oct 5 alone, and 9 lettered micro versions.
  - v0.1.0–v0.2.0 (6 versions) shipped on one commit, so no one ever played them apart.
  - v0.3.8–v0.3.14 will be versions no one played at all.
- **Heavy for a read-only page.**
  - The route sits behind `EngineGate`, so the 2.31 MB-gzipped engine loads before anything paints, though
    nothing in `patches/` calls the engine directly. One of the card helpers it uses might, which needs a
    check.
  - Every page of the site imports the 211 KB `patches.json` into the main bundle (`wire/cards.ts:5`,
    `ribbon.ts:10`), only to read one version string and the ribbon's new-card ids.
  - One card's history loads up to 13 whole catalogs: 2.25 MB raw for core-024, 604 KB for the median card.
- **Bugs found on the way.**
  - v0.2.8 shows an empty counts line, then "Show the 1 card", then "This patch changed no card." The toggle
    counts Glitch and the counts line doesn't (`PatchNotes.tsx:41,298`).
  - In zh-CN, old faces print *today's* Chinese text, because the Chinese table is keyed by card id.
  - The "Number" label is translated as 数值 ("numeric value").
  - The "Meditative+" release name has no zh-CN entry.
- **Hard to find.** The only link is in the footer of the landing and login pages. There is no menu entry and
  no "new patch" marker.

### Authors and bots
- A card PR needs:
  - a fragment whose **version number** the author picks, and renames if another in-flight PR took it (R743);
  - hand-written `player` JSON in five fixed headings, plus a date argument that is validated and then
    discarded;
  - `KEPT_SCRIPTS` entries when a script's behaviour changes.
- Some lists are kept in sync by hand:
  - `BROWSER_CARRIED_FROM`;
  - the 22-snapshot list in `crates/cards/tests/cross/card_text.rs`;
  - `RELEASES` in `releases.ts`;
  - `win_rates.json` naming a pending version;
  - about 15 tests pinned to old history.
- Two diff implementations (Rust `changed_fields`, TS `diffCard`) drift apart. This bit today: v0.3.8 is the
  first patch to change a face's `grants`, and the TS side never read it (#1332).
- Docs have drifted:
  - `docs/ADDING_CARDS.md:153` still describes the pre-R1280000 ship;
  - comments cite TypeScript files that no longer exist (`patches-io.ts`, `patches.ts`, `versions.ts`);
  - fragment titles use three different styles.

### Infrastructure
- **The window.** Render deploys `main` on every push, but `CATALOG_VERSION` moves only when the *second*
  PR merges.
  - Every game played between the two is recorded under the old version's name, while playing the new cards.
  - `catalog_for` (`crates/engine/src/replay.rs:875`) folds such a game under the old version's snapshot
    once the new version ships. The hash differs, so the replay is refused as `rules_changed`, and the
    game's stats sit under the wrong patch.
  - Normally the window is 10–30 minutes; the Oct 15–19 wedge stretched it to four days.
- **Huge PRs.** Each ship adds ≈380 KB per version. The queue now open (#1330) is +146,681 / −704 lines over
  23 files.
- **Fragile shipping.** It needs `fetch-depth: 0` and first-parent commit lookups for order and dates, the
  bot token, an auto-merging second PR, and a run on every push touching `pending/**`. One refusal blocked
  the queue for four days (fixed by R1280000; the new failure step now opens an issue).
- **Build cost.** Every build of `jackioh_cards` parses 6.5 MB of snapshots. Editing a fragment's *prose*
  rebuilds the crate and changes the server's rules fingerprint (`crates/server/build.rs:20` hashes
  `patches/` including `pending/`), which invalidates checkpoints.

## 3. What others do

These were gathered through search extracts: page fetches failed in this environment, so check exact wording
before quoting.

- **Card games show old → new text with a designer's reason**, grouped, and call wording-only edits out as
  such:
  - Hearthstone: `Old:`/`New:` plus a comment; "a text-only change for clarity".
    <https://hearthstone.blizzard.com/en-us/news/24293284/3622-patch-notes>
  - Marvel Snap: dated balance updates, the reasoning first, then `[Old]`/`[New]`.
    <https://marvelsnap.com/balance-update-september-10-2026/>
  - Legends of Runeterra: old → new card text with an arrow. <https://wiki.leagueoflegends.com/en-us/LoR:Patch>
- **Hotfixes hang off their patch, not beside it.**
  - Dota 2 letters them (7.39b–e) and serves its notes as JSON the site renders.
    <https://www.dota2.com/patches>
  - TFT has "B-patches"; Overwatch posts dated hotfix entries on a monthly page.
- **Changelog tools use per-PR fragments plus a release step:**
  - Changesets: randomly named `.changeset/*.md`, one rolling "Version Packages" PR, and no fragment for a
    change users can't see. <https://changesets.dev/guide/automating>
  - towncrier: fragments to avoid merge conflicts, a CI check, an empty release reads "No significant
    changes". <https://towncrier.readthedocs.io/en/stable/>
  - Common Changelog: one entry per change; changes that cancel out are dropped.
    <https://common-changelog.org/>
- **Version data is pinned per build, without copies in the tree:**
  - Riot Data Dragon and HearthstoneJSON publish an immutable file per build.
  - Git already deduplicates identical blobs and delta-compresses near-identical ones, so in-tree copies only
    cost checkout and bundle size.
  - Event-sourcing guidance: snapshots are an optimisation, never the source of truth.
    <https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing>
  - JSON Merge Patch (RFC 7396) fits a catalog keyed by id: a changed entry is its new value, a removed one
    is `null`. <https://datatracker.ietf.org/doc/html/rfc7396>
- **Two identifiers:**
  - an internal build id for replays, such as the HSReplay `build` attribute or SemVer `+build` metadata
    (ignored for precedence, <https://semver.org/>);
  - a short name for players.
  - LoL's season-name versions (25.S1.x) were reverted within weeks: keep player names short and stable.

## 4. The options

Effort: **S** is under a day and ~10 files; **M** is 1–3 days and 10–30 files; **L** is more, or needs a
repository setting only a person can change. Each option names the rulings it would amend (CLAUDE.md rule 3).

### Player-facing (P)

- **P1. Hide what doesn't print.**
  - Lines of code, `refs` and tuning metadata leave the player's view. A param's *value* still shows, filled
    into the text.
  - A patch with nothing printed says "Internal changes only" with no card list, and card history skips such
    entries.
  - Removes 32% of entries, and the default-open patch stops being a wall of metadata.
  - *Cost S* (`diff.ts`, `PatchNotes.tsx`, `CardHistory.tsx` and their tests; R507).
  - *Risk:* developers lose a view they used. Mitigation: a `?details=1` query keeps it.
- **P2. Collapse wording passes.**
  - A fragment's claims get a kind; the minimum is `wording: [ids]` beside `cards`.
  - Wording-kind cards render as one collapsed line: "Card text cleanup: 72 cards (no rule changes)". This
    is Hearthstone's "text-only change for clarity".
  - It has to be declared, because "text-only" by field includes real changes like Hinder's.
  - *Cost S–M* (fragment schema in `patches.rs`, `patches check`, page; R724001, R646).
- **P3. "The note says why, the diff says what."**
  - Claims become `{id, kind: new|balance|rework|wording|fix, why?}`.
  - The page groups cards by kind and prints each card's `why` next to its old → new face.
  - The five hand-written note headings shrink to one summary line, since the numbers are no longer
    restated.
  - *Cost M* (it supersedes P2's schema, plus the bot instructions; R724001, R724000).
  - *Benefit:* the convention every card game uses, and less prose to write.
- **P4. Hotfixes nest under their patch.**
  - At ship, `patches.json` records `parent` for a micro version and `release` for versions shipped on one
    commit.
  - The page shows v0.2.10b–d as dated lines inside v0.2.10, and v0.1.0–v0.2.0 as one release.
  - It is display only, with no history rewrite, and it parses no version strings, which R105 forbids.
  - *Cost M* (`patches.rs` ship, one migration of the 30 entries, page; R650, R743).
- **P5. Precomputed notes for the browser.**
  - A build step (`cargo jackioh patches notes`, run by `scripts/build-wasm.sh` like the WASM module, not
    committed) writes:
    - one small JSON per version: added defs, removed ids, and each changed card's before and after;
    - one JSON per card's history.
  - The page fetches roughly 5–50 KB per patch instead of two catalogs, and card history fetches one file
    instead of up to 13.
  - With one diff in Rust, the TypeScript `diffCard` shrinks to rendering, which ends the Rust/TS drift
    (#1332's class of bug).
  - *Cost M–L* (tool, `source.ts`, `history.ts`, `diff.ts`, tests; R388).
- **P6. `patches.json` out of the main bundle.**
  - The version string and the ribbon's new-card ids become a small generated constant.
  - Saves 211 KB raw (≈23 KB gzipped) on every page of the site.
  - *Cost S* (`wire/cards.ts`, `ribbon.ts`, a generated file checked like `apps/web/src/wire/`; R1383).
- **P7. Drop `EngineGate` from `/patch-notes`.**
  - Saves 2.31 MB gzipped on the first visit to the page.
  - *Cost S*, if the check that nothing under the page calls the engine holds; otherwise M.
- **P8. Fix the four bugs.**
  - The Glitch count.
  - zh-CN old faces: print an older face in English with a note, or keep its Chinese text in the history.
  - The 数值 label.
  - The missing "Meditative+" release name.
  - *Cost S.*
- **P9. Make it findable.** A "Patch notes" entry in the main menu, plus a dot when the newest version is
  unseen (localStorage). *Cost S.*

### Stored data (D)

- **D1. Deltas instead of full snapshots.**
  - Each `v*.json` holds only what that version changed, as a JSON Merge Patch: a changed entry's new value,
    `null` for a removed one.
  - The whole history is ≈0.6 MB (measured: 594 KB compact against 6.49 MB), about 11× smaller.
  - A ship adds a few KB instead of ≈380 KB per version, and the queue now open would have been ≈+10k lines,
    not +146k.
  - `build.rs` already reduces snapshots to deltas, so it folds them directly.
  - A one-time conversion proves itself: a test rebuilds each of the 30 catalogs and checks it equals the
    old snapshot byte for byte, and the golden replays run on top.
  - *Cost M–L* (`build.rs`, `history.rs`, `patches.rs`, `source.ts` or P5, the server and `card_text.rs`
    tests that read snapshots; R388, R646, R767000, R1280000).
- **D2. Stop committing derived data.**
  - Remove `index.json`, `patches.json`'s `changes` and `commits`, and `shipped.json`'s `blob`, and with it
    the hand-written SHA-1.
  - Whoever needs them derives them at build time. "History is never edited" becomes a CI check that a PR
    touches no shipped version file.
  - Saves ≈175 KB and a lot of merge noise, and removes ≈300 lines of `patches.rs`.
  - *Cost S–M* (R388, R646).
- **D3. Carry only versions that were deployed, and only for as long as replays live.**
  - Versions that shipped on one commit, or in one batch, share a single replay catalog.
  - A version older than `MATCH_ACTION_RETENTION_DAYS` (90) leaves the carried set, and its games refuse as
    `earlier_patch`, as the browser's already do.
  - Of little use today, since all history is under 90 days old, but it caps growth.
  - *Cost S–M* (R767000).
- **D4. One version per release, not per fragment.**
  - Every fragment pending when ship runs becomes **one** version with a merged note, as Changesets does.
  - The queue now open would have been one patch, not nine, with no unplayed intermediates and no synthetic
    snapshots, which also retires most of R1280000's machinery.
  - *Cost M* (R646, R743, R1280000).
  - *Trade:* a patch can mix topics, but P3's per-kind grouping keeps it readable.

### Flow (F)

- **F1. Authors never pick a number.**
  - A fragment is named after its issue or branch (`pending/1187.json`), as towncrier and Changesets do, and
    ship assigns the number. That ends R743's rename-on-collision and pending names in `win_rates.json`.
  - `patches check --fix` writes the claimed cards from the branch's catalog diff, so the author writes only
    prose.
  - *Cost S–M* (`patches.rs`, `docs/issues-and-patches.md`, `docs/ADDING_CARDS.md`; R743, R646).
- **F2. Close the window.** Three ways, cheapest first:
  - **F2a. Tag the in-between catalog honestly.**
    - While fragments are pending, the build's version is `<last shipped>+<catalog hash>`, SemVer build
      metadata and the "two identifiers" convention.
    - Games in the window refuse as `earlier_patch` (honest), not `rules_changed` (looks like a bug), and
      their stats stop landing under the wrong patch.
    - *Cost S* (`build.rs`, `catalog_for`, stats keys; R105, R767000).
  - **F2b. The PR ships its own version.**
    - With D1's small deltas, the card PR appends its delta and `patches.json` entry itself, so a version
      exists from the merge commit on.
    - This deletes the promotion workflow, the bot-token second PR, `fetch-depth: 0`, the first-parent
      ordering and the window.
    - Two card PRs in flight then conflict on the history list. GitHub's merge queue resolves that, but it's
      a repository setting only a person can turn on, and the bots would rebase.
    - *Cost L* (R646, R1280000, `patches-ship.yml`, the bots' instructions).
  - **F2c. Ship on the production cadence** (daily, with #1234).
    - Versions would match what jackioh.com got. But Render deploys the server on every push, so the window
      remains unless the server deploys daily too, and that is a deploy change a person owns (CLAUDE.md
      rule 13).
    - *Not recommended alone.*
- **F3. Shrink `patches.rs`.** After D1, D2, D4 and F1, the claims-built snapshots, the SHA-1, the version
  resolver and most ordering code go. An estimated −1,000 to −1,500 lines, tests included. *It comes free
  with those.*
- **F4. One home for the rules.**
  - Fold the 13 patch rulings' current behaviour into one spec section that the rulings point to, and fix the
    doc drift: `ADDING_CARDS.md:153`, the stale TS file references, the fragment title styles.
  - Exclude fragment prose from the server's rules fingerprint, after confirming why `pending/` is in it.
  - *Cost S.*

## Cost-benefit

Benefit: ●●● large, ●● clear, ● small. Effort: S, M, L as in §4. Risk is what could break and how it's
caught.

| # | Change | Players | Repo/bots | Effort | Risk | Needs |
|---|---|---|---|---|---|---|
| P1 | Hide non-printing fields | ●●● | ● | S | low: tests pin the page | — |
| P2 | Collapse wording passes | ●●● | ● | S–M | low | fragment field |
| P3 | Why-notes + per-kind grouping | ●●● | ●● (less prose) | M | low–med: bot instructions change | P2 (supersedes) |
| P4 | Hotfixes nest under their patch | ●● | — | M | low: display only | ship records `parent` |
| P5 | Precomputed notes for the browser | ●● (speed) | ●● (one diff) | M–L | med: page rewiring | best after D1 |
| P6 | `patches.json` out of the main bundle | ● (every page) | ● | S | low | — |
| P7 | No engine for `/patch-notes` | ●● (first visit) | — | S–M | low | a check |
| P8 | Fix the four bugs | ●● | — | S | low | — |
| P9 | Menu entry + "new" dot | ●● | — | S | low | — |
| D1 | Deltas, not snapshots | — | ●●● (11× smaller, small PRs, faster builds) | M–L | med: replays; proven by a byte-for-byte rebuild test + golden | — |
| D2 | Stop committing derived data | — | ●● | S–M | low | — |
| D3 | Carry only deployed / in-retention catalogs | — | ● now, ●● later | S–M | low: graceful refusals | — |
| D4 | One version per release | ●● (fewer, meaningful versions) | ●● | M | med: changes numbering rules | — |
| F1 | Number at ship; claims auto-filled | — | ●● (no renames, less hand JSON) | S–M | low | — |
| F2a | Honest in-between version | ● (replays refuse honestly) | ●● (stats per patch right) | S | low–med: version-string consumers | — |
| F2b | PR ships its own version | ● | ●●● (no promotion system, no window) | L | med: CI/bot flow | D1; merge queue (a person) |
| F2c | Ship daily with production | ● | ● | L | high: deploy | a person; Render cadence |
| F3 | Shrink `patches.rs` | — | ●● | free | — | D1, D2, D4, F1 |
| F4 | One home for the rules + drift fixes | — | ●● | S | low | — |

**Not worth it:**
- MTG Arena's separate "A-" cards for rebalances (heavy, and Wizards is retiring it).
- CalVer or season names (LoL reverted them).
- Moving notes to GitHub Releases: the page and the static page read the notes as data, and Releases would
  split the source.
- Rewriting history to merge the old micro versions: P4 gets the same result on screen, and R743's renames
  already needed a migration (`0025_patch_retcon.sql`).

## Recommended order

1. **Clean the page (one PR, S): P1, P6, P7, P8, P9 and F4.**
   - Biggest player-visible gain for the least risk.
   - Nothing changes in the data, the ship or the replays.
2. **Say why, group by kind (one PR, M): P3 (which includes P2) and P4.**
   - Changes the fragment schema and the bots' instructions.
   - Old patches keep their five-heading notes and render as today.
3. **Small history (two PRs, M–L): D1 and D2, then P5.**
   - The conversion PR proves every catalog byte for byte.
   - P5 then makes the browser fetch only what a patch changed.
   - F3's shrink lands across both.
4. **Version at merge (decide first): F1, D4 and F2a now; F2b if you will turn on the merge queue.**
   - F2a alone fixes stats and makes replays honest.
   - F2b removes the promotion machinery outright.

Steps 1 and 2 are independent of 3 and 4, and can run in parallel with them.

## Decisions for you

1. **Internal fields:** hide them outright (P1), or keep a `?details=1` view for development?
2. **Versions:** one version per release (D4), or keep one per fragment? D4 changes how numbers read from
   now on, not history.
3. **The window:** is F2a (cheap, honest) enough, or do you want F2b, which needs the merge queue enabled
   in the repository settings?
4. **The queue now open (#1330):** ship v0.3.8–v0.3.15 as nine versions, as planned (my recommendation:
   the names are already cited in titles, rulings and `win_rates.json`, and waiting widens the window), or
   hold it and ship it as one under D4?
