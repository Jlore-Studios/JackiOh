// The one place `apps/web` names the engine's deck constants. BUILD §2 puts `DECK_SIZE` and
// `MAX_COPIES` in the engine's config (`crates/engine/src/config.rs`) and says nothing restates them,
// so the deckbuilder's counters and its test fixtures read them from there. The import is the constants
// generated from that config (`src/wire/engineConfig.ts`), and not the engine: the engine is the
// WebAssembly module, which this client does not want to load for two settled numbers.
export { DECK_SIZE, MAX_COPIES } from "@jackioh/engine/config";
