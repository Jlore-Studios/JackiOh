// The Luau browser module (#919, SPEC §12.10, R915009): `crates/wasm/luau`, the engine with the card
// kit, built into `./pkg/luau` by `scripts/build-wasm.sh`. `index.ts` imports this file dynamically, so
// nothing fetches the module until a practice game with a custom card or the creator asks for it.
// The module is an emscripten `MODULARIZE` factory with a C boundary (v0.4.0's, `da817ba6`): strings
// cross as pointers, each freed with `jackioh_free`, and a failing binding answers NULL (or a negative
// status) with its message in `jackioh_last_error`. `adapt` gives it the normal module's wasm-bindgen
// shape (`LuauBindings`), throwing that message, so `index.ts` calls either module the same way.

import type { JackiohWasmLuauModule } from "./pkg/luau/jackioh_wasm_luau.js";

export type LuauBindings = {
  init(): void;
  catalog(): string;
  catalog_version(): string;
  carried_versions(): string;
  create_game(args: string): string;
  begin_game(state: string): string;
  reduce(state: string, action: string): string;
  legal_actions(state: string, player: string): string;
  view_for(state: string, player: string): string;
  seat_to_act(state: string): string;
  hash_state(state: string): string;
  fold(args: string): string;
  replay_open(args: string, record: string): string;
  replay_page(checkpoints: string, seat: string, from: number, count: number): string;
  last_board_for(state: string, seat: string): string;
  seat_played_by(state: string, home: string): string;
  find_instance(state: string, instance: string): string;
  deal_emote_hand(seed: string, seat: string): string;
  preview_sets(sets: string): void;
  ai_to_act(state: string, seat: string): boolean;
  ai_decide(state: string, seat: string, options: string, deadlineMs: number): string;
  lethal_hint(state: string, seat: string, deadlineMs: number): boolean;
  build_ai_deck(options: string): string;
  choose_action(state: string, seat: string, rngSeed: string, rngCursor: number): string;
  constants(): string;
  craft_preview(recipe: string, cost: number, prompt: string): string;
  engine_tables(): string;
  validator(call: string, input: string): string;
  check_custom_card(card: string): string;
};

function adapt(m: JackiohWasmLuauModule): LuauBindings {
  const fail = (): never => {
    throw new Error(m.UTF8ToString(m._jackioh_last_error()));
  };
  /** Read the answer at `ptr` and free it; NULL is the binding's failure. */
  const take = (ptr: number): string => {
    if (ptr === 0) return fail();
    const text = m.UTF8ToString(ptr);
    m._jackioh_free(ptr);
    return text;
  };
  /** Copy `args` in, run `call` on their pointers, free them after, thrown or returned. */
  const withArgs = <T>(args: readonly string[], call: (ptrs: number[]) => T): T => {
    const ptrs = args.map((arg) => m.stringToNewUTF8(arg));
    try {
      return call(ptrs);
    } finally {
      for (const ptr of ptrs) m._jackioh_free(ptr);
    }
  };
  const one = (call: (a: number) => number) => (a: string): string => withArgs([a], ([p = 0]) => take(call(p)));
  const two =
    (call: (a: number, b: number) => number) =>
    (a: string, b: string): string =>
      withArgs([a, b], ([p = 0, q = 0]) => take(call(p, q)));
  return {
    init: () => m._init(),
    catalog: () => take(m._catalog()),
    catalog_version: () => take(m._catalog_version()),
    carried_versions: () => take(m._carried_versions()),
    create_game: one((a) => m._create_game(a)),
    begin_game: one((a) => m._begin_game(a)),
    reduce: two((a, b) => m._reduce(a, b)),
    legal_actions: two((a, b) => m._legal_actions(a, b)),
    view_for: two((a, b) => m._view_for(a, b)),
    seat_to_act: one((a) => m._seat_to_act(a)),
    hash_state: one((a) => m._hash_state(a)),
    fold: one((a) => m._fold(a)),
    replay_open: two((a, b) => m._replay_open(a, b)),
    replay_page: (checkpoints, seat, from, count) =>
      withArgs([checkpoints, seat], ([p = 0, q = 0]) => take(m._replay_page(p, q, from, count))),
    last_board_for: two((a, b) => m._last_board_for(a, b)),
    seat_played_by: two((a, b) => m._seat_played_by(a, b)),
    find_instance: two((a, b) => m._find_instance(a, b)),
    deal_emote_hand: two((a, b) => m._deal_emote_hand(a, b)),
    preview_sets: (sets) =>
      withArgs([sets], ([p = 0]) => {
        if (m._preview_sets(p) !== 0) fail();
      }),
    ai_to_act: (state, seat) =>
      withArgs([state, seat], ([p = 0, q = 0]) => {
        const answer = m._ai_to_act(p, q);
        if (answer < 0) fail();
        return answer !== 0;
      }),
    ai_decide: (state, seat, options, deadlineMs) =>
      withArgs([state, seat, options], ([p = 0, q = 0, r = 0]) => take(m._ai_decide(p, q, r, deadlineMs))),
    lethal_hint: (state, seat, deadlineMs) =>
      withArgs([state, seat], ([p = 0, q = 0]) => {
        const answer = m._lethal_hint(p, q, deadlineMs);
        if (answer < 0) fail();
        return answer !== 0;
      }),
    build_ai_deck: one((a) => m._build_ai_deck(a)),
    choose_action: (state, seat, rngSeed, rngCursor) =>
      withArgs([state, seat, rngSeed], ([p = 0, q = 0, r = 0]) => take(m._choose_action(p, q, r, rngCursor))),
    constants: () => take(m._constants()),
    craft_preview: (recipe, cost, prompt) =>
      withArgs([recipe, prompt], ([p = 0, q = 0]) => take(m._craft_preview(p, cost, q))),
    engine_tables: () => take(m._engine_tables()),
    validator: two((a, b) => m._validator(a, b)),
    check_custom_card: one((a) => m._check_custom_card(a)),
  };
}

let loading: Promise<LuauBindings> | null = null;

/**
 * Fetch and instantiate the Luau module and run its own `init` (the catalog and the official cards'
 * scripts). Idempotent; a failed fetch is not cached. `bytes`: the module's, in jsdom tests.
 */
export function loadLuauBindings(bytes?: BufferSource): Promise<LuauBindings> {
  if (loading === null) {
    loading = (async (): Promise<LuauBindings> => {
      const { default: createModule } = await import("./pkg/luau/jackioh_wasm_luau.js");
      let wasmBinary = bytes;
      if (wasmBinary === undefined) {
        // The literal `new URL(…, import.meta.url)` is what Vite emits as an asset of this chunk alone.
        const response = await fetch(new URL("./pkg/luau/jackioh_wasm_luau.wasm", import.meta.url));
        if (!response.ok) throw new Error(`the Luau module could not be fetched: ${response.status}`);
        wasmBinary = await response.arrayBuffer();
      }
      const bindings = adapt(await createModule({ wasmBinary }));
      bindings.init();
      return bindings;
    })().catch((cause: unknown) => {
      loading = null;
      throw cause;
    });
  }
  return loading;
}
