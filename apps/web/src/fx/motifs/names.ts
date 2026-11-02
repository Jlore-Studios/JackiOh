// R2784000: the names a card's effects table may give a motif (docs/polish/9-card-animations.md), and
// the hooks it may give one to. A name is accepted before its part draws it (`aura` waits for #2787):
// such an entry plays its event rows only, as a card with no entry does.

/** Every motif name a table may use: the ten damage and removal motifs drawn here, then those still to come. */
export const FX_MOTIF_NAMES = ["bolt", "volley", "barrage", "chain", "lance", "nova", "wipe", "vanish", "scorch", "negate", "aura"] as const;

/** The motifs `MOTIF_PLANNERS` draws (#2784's damage and removal family). */
export const FX_DRAWN_MOTIFS = ["bolt", "volley", "barrage", "chain", "lance", "nova", "wipe", "vanish", "scorch", "negate"] as const;

/**
 * Where on its card a motif plays: `cast` a non-Unit's play, `cry` a Unit's; `death`, `trigger` and
 * `activate` wait for #2783's `hookResolving` marker and plan nothing until it lands.
 */
export const FX_CARD_HOOKS = ["cast", "cry", "death", "trigger", "activate"] as const;

export type MotifName = (typeof FX_MOTIF_NAMES)[number];
export type DrawnMotif = (typeof FX_DRAWN_MOTIFS)[number];
export type FxCardHook = (typeof FX_CARD_HOOKS)[number];

export function isMotifName(name: string): name is MotifName {
  return (FX_MOTIF_NAMES as readonly string[]).includes(name);
}

export function isDrawnMotif(name: string): name is DrawnMotif {
  return (FX_DRAWN_MOTIFS as readonly string[]).includes(name);
}

export function isCardHook(name: string): name is FxCardHook {
  return (FX_CARD_HOOKS as readonly string[]).includes(name);
}
