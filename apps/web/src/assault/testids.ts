// The Infinite Assault screen's stable testing surface. Part D owns `fight/testids.ts`.

export const assaultTestid = {
  page: "assault-page",
  deckSaved: "assault-deck-saved",
  deckPreset: "assault-deck-preset",
  deckDay: "assault-deck-day",
  start: "assault-start",
  /** R4582001: on the day screen, back to the fight its Menu left. */
  resume: "assault-resume",
  level: (level: number): string => `assault-level-${String(level)}`,
  briefing: "assault-briefing",
  fightStart: "assault-fight-start",
  result: "assault-result",
  record: "assault-record",
  option: (kind: "saved" | "preset" | "day", index: number): string =>
    `assault-option-${kind}-${String(index)}`,
  curve: "assault-curve",
  history: "assault-history",
  startLevel: (level: number): string => `assault-startLevel-${String(level)}`,
} as const;
