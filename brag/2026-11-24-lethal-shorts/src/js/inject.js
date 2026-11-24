async () => {
  const H = { deckSize: 6, manaBonus: 3, manaCap: 4, extraOpeningCards: 0, extraDrawsPerTurn: 0 };
  const inj = {
    seed: "SEED",
    decks: {
      "brag-a": ["core-008", "meditative-100", "traininggrounds-032", "traininggrounds-008", "core-045", "traininggrounds-003"],
      "brag-b": ["core-096", "core-041", "meditative-002", "stinkyscapeplus-002", "meditative-073", "meditativeplus-008"],
    },
    handicaps: { p1: H, p2: { ...H, heroHealth: 3 } },
  };
  localStorage.setItem("jackioh.e2e.decks", JSON.stringify(inj));
  return "ok";
}
