// The one place `apps/web` names the engine's deck constants.
//
// BUILD §2 puts `DECK_SIZE` and `MAX_COPIES` in the engine's config (`crates/engine/src/config.rs`)
// and says nothing restates them, so the deckbuilder's counters and its test fixtures read them from
// there rather than spelling 20 and 1. The import is `@jackioh/engine/config`, the constants
// generated from that config (`src/wire/engineConfig.ts`), and not the engine: the engine is the
// WebAssembly module, which this client does not want to load for two settled numbers, which is
// what the deep relative path used to be working around.
export { DECK_SIZE, MAX_COPIES } from "@jackioh/engine/config";
