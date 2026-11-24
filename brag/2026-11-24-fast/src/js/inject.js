// The two decks (25 cards each, plainly generic names) as seedGame injects them; no handicap.
async () => {
  const inj = {
    seed: "SEED",
    decks: {
      "fast-a": DECK_A,
      "fast-b": DECK_B,
    },
  };
  localStorage.setItem("jackioh.e2e.decks", JSON.stringify(inj));
  return "ok";
}
