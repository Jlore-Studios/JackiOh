// R3337000: Stinkyscape's ten Anomalies as the client draws them: the code each board's `boardReplaced`
// names (`crates/cards/src/scripts/stinkyscape/t_anomaly_found.rs`, `Board.code`), its name and whether it
// is one of the Radiant face's three rich boards. Presentation only (CLAUDE.md rule 7): the board laid is
// public, and nothing here decides which one a card lays. `anomalies.test.ts` holds it to the cards crate.

/** The card that holds an Anomaly (Stinkyscape's token Anomaly Found!), which the Scanning Probes find. */
export const ANOMALY_FOUND_ID = "stinkyscape-t-anomaly-found";

/** The ten codes, in the cards crate's `ALL_BOARDS` order. */
export const ANOMALY_CODES = ["WH", "BF", "AL", "AT", "SP", "MT", "DH", "FO", "AN", "AA"] as const;

export type AnomalyCode = (typeof ANOMALY_CODES)[number];

/** One Anomaly: its name as its board prints it, and whether it is a rich board (a Radiant face's, R2247000). */
export type AnomalySpec = { name: string; rich: boolean };

/** R3337000: every Anomaly by its code. */
export const ANOMALIES: { readonly [C in AnomalyCode]: AnomalySpec } = {
  WH: { name: "Wormhole", rich: false },
  BF: { name: "Battlefields", rich: false },
  AL: { name: "Asteroid Field: Large", rich: false },
  AT: { name: "Asteroid Field: Small", rich: false },
  SP: { name: "Wrecked Spice Platform", rich: false },
  MT: { name: "Monument", rich: false },
  DH: { name: "Drone Hideouts", rich: false },
  FO: { name: "Frontier Outpost", rich: true },
  AN: { name: "Asteroid Field: Narcour", rich: true },
  AA: { name: "Asteroid Field: Axnit", rich: true },
};

/** Whether `code` is one of the ten (a `boardReplaced.anomaly`, a held card's label). */
export function isAnomalyCode(code: unknown): code is AnomalyCode {
  return typeof code === "string" && (ANOMALY_CODES as readonly string[]).includes(code);
}
