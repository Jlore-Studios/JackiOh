// `@jackioh/run` for the web client: the dungeon run's types (`crates/run`, docs/dungeon-run.md DR14),
// as ts-rs writes them from the Rust into `generated/`. The run's rules stay the Rust crate's, which
// the browser reaches through `../wasm` (DR14 part C); this module names their shapes and nothing else.
//
// Regenerate them with `cargo test -p jackioh-run --features ts` and commit the diff. CI does not run
// that command yet, so nothing holds these files to the Rust until `ci.yml` does.

export type { RunAction } from "./generated/RunAction.ts";
export type { RunArtifact } from "./generated/RunArtifact.ts";
export type { RunCard } from "./generated/RunCard.ts";
export type { RunConstants } from "./generated/RunConstants.ts";
export type { RunContent } from "./generated/RunContent.ts";
export type { RunCurseDef } from "./generated/RunCurseDef.ts";
export type { RunEffect } from "./generated/RunEffect.ts";
export type { RunEnemy } from "./generated/RunEnemy.ts";
export type { RunEnemyDef } from "./generated/RunEnemyDef.ts";
export type { RunEventDef } from "./generated/RunEventDef.ts";
export type { RunEventOption } from "./generated/RunEventOption.ts";
export type { RunFightSetup } from "./generated/RunFightSetup.ts";
export type { RunFile } from "./generated/RunFile.ts";
export type { RunNode } from "./generated/RunNode.ts";
export type { RunNodeKind } from "./generated/RunNodeKind.ts";
export type { RunPackKind } from "./generated/RunPackKind.ts";
export type { RunPassiveDef } from "./generated/RunPassiveDef.ts";
export type { RunPassiveKind } from "./generated/RunPassiveKind.ts";
export type { RunPhase } from "./generated/RunPhase.ts";
export type { RunPoolCard } from "./generated/RunPoolCard.ts";
export type { RunSetup } from "./generated/RunSetup.ts";
export type { RunShopGood } from "./generated/RunShopGood.ts";
export type { RunShopItem } from "./generated/RunShopItem.ts";
export type { RunState } from "./generated/RunState.ts";
export type { RunStats } from "./generated/RunStats.ts";
export type { RunTier } from "./generated/RunTier.ts";
export type { RunUnlocks } from "./generated/RunUnlocks.ts";
