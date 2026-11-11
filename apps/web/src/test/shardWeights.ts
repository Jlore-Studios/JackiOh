// How `web (unit k/8)` (.github/workflows/ci.yml's `web-unit`, #3666) deals the web's, the Worker's and
// the dev dashboard's unit-test files over its shards: longest file first, each to the shard with the
// least load so far, by WEIGHTS, as e2e/scripts/shard-specs.mjs does for the specs. shardSequencer.ts
// plugs `dealShards` into Vitest, whose own `--shard=k/K` cuts the files, hashed by path, into K slices
// of equal count whatever each takes. The count is at its cap of 8 (.github/ci/matrix.py's
// MAX_SHARDS), so a cluster of heavy files in one slice, as in 7/8 (9m 41s), could be cured only by
// dealing them by time.
//
// Every file lands in exactly one shard for any count, so a new or renamed file is never left out: one
// not listed counts DEFAULT_WEIGHT_S and still runs. Refresh the table when a shard drifts, not the
// count: run each shard of a clean checkout with
//   pnpm exec vitest run --project web --project edge --project admin --shard=k/8 --reporter=json
//     --outputFile=before-k.json   (and ci.yml's `--exclude`s)
// and list every file whose `endTime - startTime` is 3 s or more, in whole seconds. Only the ratios
// between the files matter, so the machine need not be CI's.
//
// No imports and only syntax Node can strip, so the sequencer, the test and the root config load it as
// it is.

/** Seconds a test file counts when WEIGHTS does not list it: the many small files it leaves out. */
export const DEFAULT_WEIGHT_S = 1;

/**
 * Seconds per test file of 3 s or more, by its path from the repository root: each file's duration in
 * a run of the eight shards of main at 13bd66f on a 4-core machine (#3666). A file that sweeps
 * (src/test/daily.ts) counts its short run, the one a pull request makes.
 */
export const WEIGHTS: Readonly<Record<string, number>> = {
  "apps/admin/src/Dashboard.test.tsx": 3,
  "apps/web/src/auth/UsernamePrompt.test.tsx": 6,
  "apps/web/src/cards/CardFace.test.tsx": 62,
  "apps/web/src/cards/CardMarks.test.tsx": 3,
  "apps/web/src/cards/art/art.test.ts": 22,
  "apps/web/src/cards/art/classicAltScenes.test.tsx": 14,
  "apps/web/src/cards/art/classicPlusAltScenes.test.tsx": 12,
  "apps/web/src/cards/art/coreAltScenes.test.tsx": 16,
  "apps/web/src/cards/art/coreScenes.test.ts": 3,
  "apps/web/src/cards/art/meditativeAltScenes.test.tsx": 14,
  "apps/web/src/cards/art/meditativeScenes.test.ts": 3,
  "apps/web/src/cards/art/motifs.test.ts": 3,
  "apps/web/src/cards/art/pictures.test.ts": 5,
  "apps/web/src/cards/art/scene.test.ts": 6,
  "apps/web/src/cards/art/setAltScenes.test.tsx": 9,
  "apps/web/src/cards/art/stinkyscape91AltScenes.test.tsx": 15,
  "apps/web/src/cards/art/stinkyscapeScenes.test.ts": 12,
  "apps/web/src/cards/cardState.test.tsx": 4,
  "apps/web/src/cards/defaultLooks.test.tsx": 10,
  "apps/web/src/cards/depth.test.tsx": 3,
  "apps/web/src/cards/diorama.test.tsx": 9,
  "apps/web/src/cards/fullArtOwnership.test.tsx": 3,
  "apps/web/src/cards/fullArtTiers.test.tsx": 129,
  "apps/web/src/cards/inspect/CardDetail.test.tsx": 23,
  "apps/web/src/cards/inspect/inspect.test.tsx": 4,
  "apps/web/src/cards/placeholders.test.tsx": 33,
  "apps/web/src/cards/quest.test.tsx": 3,
  "apps/web/src/cards/radiantDiff.test.ts": 19,
  "apps/web/src/cards/references.test.tsx": 17,
  "apps/web/src/cards/starship.test.tsx": 3,
  "apps/web/src/cards/variantLevels.test.tsx": 3,
  "apps/web/src/cards/versionLooks.test.tsx": 64,
  "apps/web/src/cards/versionLooks3.test.tsx": 49,
  "apps/web/src/cards/versionLooks4.test.tsx": 35,
  "apps/web/src/cards/versionLooks5.test.tsx": 15,
  "apps/web/src/cards/versionLooks6.test.tsx": 7,
  "apps/web/src/collection/TokenOffer.test.tsx": 7,
  "apps/web/src/collection/collection.test.tsx": 22,
  "apps/web/src/dungeon/fight/FightScreen.test.tsx": 4,
  "apps/web/src/dungeon/start/StartScreen.test.tsx": 10,
  "apps/web/src/duos/DuosBoard.test.tsx": 15,
  "apps/web/src/duos/DuosReplay.test.tsx": 5,
  "apps/web/src/duos/DuosRoute.test.tsx": 4,
  "apps/web/src/emotes/PortraitPicker.test.tsx": 3,
  "apps/web/src/game/ActivateControl.test.tsx": 6,
  "apps/web/src/game/Board.test.tsx": 11,
  "apps/web/src/game/Card.test.tsx": 11,
  "apps/web/src/game/Game.test.tsx": 5,
  "apps/web/src/game/PileBrowser.test.tsx": 4,
  "apps/web/src/game/PromptLane.test.tsx": 3,
  "apps/web/src/game/TakeoverStill.test.tsx": 4,
  "apps/web/src/game/TerminusBoard.test.tsx": 17,
  "apps/web/src/game/WipeBoard.test.tsx": 4,
  "apps/web/src/game/animations.window.test.ts": 34,
  "apps/web/src/game/cosmetics.test.tsx": 3,
  "apps/web/src/game/deckbuilder/DeckPatchBadge.test.tsx": 4,
  "apps/web/src/game/deckbuilder/DeckWorkshop.test.tsx": 58,
  "apps/web/src/game/deckbuilder/PoolGrid.virtual.test.tsx": 5,
  "apps/web/src/game/deckbuilder/browse.test.tsx": 11,
  "apps/web/src/game/embiggen-real.test.tsx": 21,
  "apps/web/src/game/emotes.test.tsx": 15,
  "apps/web/src/game/overflow.test.tsx": 3,
  "apps/web/src/i18n/i18n.test.ts": 3,
  "apps/web/src/net/ci-scope.test.ts": 21,
  "apps/web/src/net/gate-session-changes.test.tsx": 4,
  "apps/web/src/net/server-image.test.ts": 5,
  "apps/web/src/packs/PackOpening.test.tsx": 4,
  "apps/web/src/patches/CardHistory.test.tsx": 3,
  "apps/web/src/patches/PatchNotes.test.tsx": 6,
  "apps/web/src/patches/github.test.ts": 4,
  "apps/web/src/patches/placeholders.test.tsx": 32,
  "apps/web/src/patches/releaseArt.test.ts": 6,
  "apps/web/src/practice/claim.test.ts": 6,
  "apps/web/src/practice/driver-run.test.ts": 7,
  "apps/web/src/practice/driver.test.ts": 84,
  "apps/web/src/practice/replays.test.ts": 9,
  "apps/web/src/practice/todaysDeal.test.ts": 5,
  "apps/web/src/replay/ReplayPlayer.test.tsx": 12,
  "apps/web/src/replay/ShareGif.test.tsx": 5,
  "apps/web/src/routes/account.test.tsx": 3,
  "apps/web/src/routes/almanac-preview-stinkyscape.test.tsx": 33,
  "apps/web/src/routes/almanac-preview-traininggrounds.test.tsx": 17,
  "apps/web/src/routes/almanac-preview.test.tsx": 20,
  "apps/web/src/routes/almanac.test.tsx": 157,
  "apps/web/src/routes/almanac.virtual.test.tsx": 3,
  "apps/web/src/routes/collection.test.tsx": 5,
  "apps/web/src/routes/command-menu.test.tsx": 12,
  "apps/web/src/routes/decks.test.tsx": 14,
  "apps/web/src/routes/dev/fx.test.tsx": 11,
  "apps/web/src/routes/engine-gate.test.tsx": 3,
  "apps/web/src/routes/gate.test.tsx": 6,
  "apps/web/src/routes/invite-feedback.test.tsx": 4,
  "apps/web/src/routes/landing-news.test.tsx": 3,
  "apps/web/src/routes/landing.test.tsx": 12,
  "apps/web/src/routes/landingFan.test.ts": 4,
  "apps/web/src/routes/login-flows.test.tsx": 4,
  "apps/web/src/routes/match.test.tsx": 9,
  "apps/web/src/routes/play.test.tsx": 22,
  "apps/web/src/routes/practice-tutorial.test.tsx": 4,
  "apps/web/src/routes/practice.test.tsx": 23,
  "apps/web/src/routes/privacy.test.tsx": 3,
  "apps/web/src/routes/replays.test.tsx": 5,
  "apps/web/src/routes/shell-gate.test.tsx": 4,
  "apps/web/src/routes/shop.test.tsx": 4,
  "apps/web/src/routes/sign-in-landing.test.tsx": 3,
  "apps/web/src/routes/stats.test.tsx": 11,
  "apps/web/src/settings/wiring.test.tsx": 3,
  "apps/web/src/shop/Shop.test.tsx": 3,
  "apps/web/src/shortcuts/shortcuts.test.tsx": 5,
  "apps/web/src/static-pages.test.ts": 3,
  "apps/web/src/test/ux/attack-pickup.test.tsx": 3,
  "apps/web/src/test/ux/board-ux.test.tsx": 4,
  "apps/web/src/test/ux/drag-continue.test.tsx": 3,
  "apps/web/src/test/ux/drag-layer.test.tsx": 9,
  "apps/web/src/test/ux/settings.test.tsx": 3,
  "apps/web/src/together/FriendsButton.test.tsx": 4,
  "apps/web/src/together/TogetherRoute.test.tsx": 5,
  "apps/web/src/tutorial/discovery-play.test.ts": 10,
  "apps/web/src/tutorial/lessons.test.ts": 25,
  "apps/web/src/tutorial/scripts/backrow.test.ts": 32,
  "apps/web/src/tutorial/scripts/basics.test.ts": 17,
  "apps/web/src/tutorial/scripts/casting.test.ts": 14,
  "apps/web/src/tutorial/scripts/finale.test.ts": 25,
  "apps/web/src/tutorial/scripts/keywords.test.ts": 20,
};

/**
 * The `count` shards of `paths` (repository-root-relative, `/`-separated): the heaviest path first,
 * each to the shard with the least load so far (the lowest index on a tie). The paths are put in a
 * fixed order first, so the same set deals the same way whatever order it arrives in, which every
 * shard's job must agree on. The shards together hold each path once, so none is skipped.
 */
export function dealShards(
  paths: readonly string[],
  count: number,
  weights: Readonly<Record<string, number>> = WEIGHTS,
): string[][] {
  const weightOf = (path: string): number => weights[path] ?? DEFAULT_WEIGHT_S;
  const shards: string[][] = Array.from({ length: count }, () => []);
  const loads: number[] = Array.from({ length: count }, () => 0);
  const ordered = [...paths].sort((a, b) => weightOf(b) - weightOf(a) || (a < b ? -1 : a > b ? 1 : 0));
  for (const path of ordered) {
    let lightest = 0;
    for (let i = 1; i < count; i++) {
      if ((loads[i] ?? 0) < (loads[lightest] ?? 0)) lightest = i;
    }
    shards[lightest]?.push(path);
    loads[lightest] = (loads[lightest] ?? 0) + weightOf(path);
  }
  return shards;
}
