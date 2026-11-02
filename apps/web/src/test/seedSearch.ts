// The web tests' seed search (#2569), after the Rust testkit's `seed_where` (#848): a test that needs
// a seeded game to land one way finds the seed at run time instead of pinning one that every card
// change re-deals. The search is deterministic, so a test replays the same game every run.

/**
 * How many seeds `seedWhere` tries before it throws. Far below the engine's own `SEED_SEARCH_BOUND`
 * (crates/engine/src/config.rs): each try here plays a whole game inside a test's time.
 */
export const SEED_SEARCH_BOUND = 40;

/**
 * The first seed `<prefix>-0`, `<prefix>-1`, … that `accept` takes, or a throw naming the prefix and
 * the bound. `accept` builds the game from the seed and reads what the test needs of it.
 */
export function seedWhere(prefix: string, accept: (seed: string) => boolean): string {
  for (let index = 0; index < SEED_SEARCH_BOUND; index += 1) {
    const seed = `${prefix}-${String(index)}`;
    if (accept(seed)) return seed;
  }
  throw new Error(
    `no seed among the first ${String(SEED_SEARCH_BOUND)} ("${prefix}-0" to "${prefix}-${String(SEED_SEARCH_BOUND - 1)}") is accepted`,
  );
}
