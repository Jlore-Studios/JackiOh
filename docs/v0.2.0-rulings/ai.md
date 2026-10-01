# Patch v0.2.0 rulings: the AI workstream (R600–R609)

Rows in SPEC §11's format, for the docs workstream to port into §11. Each is proved by a test named
after it and indexed in `packages/engine/test/rulings.test.ts`.

| R | Topic | Ruling | Cards affected |
| --- | --- | --- | --- |
| R600 | Who stays on the shadow-ban watch list | R390's `SHADOW_WATCH` holds the cards a sweep found at risk by their own numbers and did not ban: some tier's pass-1 or pass-2 numbers meet a flag's condition at half strength (affordable on `minAffordableTurns` turns and played at most once, or a mean evaluation change below half of `selfHarmDelta`). A card that was at risk only because it stood on the old ban or the old watch list, and whose numbers in both passes are clean this time, comes off: it is no longer on track to be banned. So a card cleared once stays watched for one more sweep, and the list does not only grow. Each entry names the numbers that put it there, per tier and pass | §9.9, R186, R390 |
| R601 | The numbers a judgement ban reads | A `neverPlayed` or `selfHarm` ban reads pass 2's numbers at one tier and nothing else: the card's games of that tier in pass 2, the ones it was forced into and the ones it was dealt as at-risk filler, summed. `neverPlayed` needs `banAffordableTurns` (6) affordable turns and no play in any of them; `selfHarm` needs `banHarmPlays` (8) plays whose mean evaluation change is below `selfHarmDelta`. Pass 1's numbers are never added in: they only put a card at risk, so a card at risk that pass 2 did not sweep is not banned for either. `error` and `timeout` count the card's own forced games of both passes, as R186 always did | §9.9, R186, R390 |
