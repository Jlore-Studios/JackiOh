# Patch v0.2.0 rulings: the cards-plus-d workstream, Classic+ #40–#78 and the AI cards (R580–R599)

Rows in SPEC §11's format, for the docs workstream to port into §11. Each is proved by a test named
after it and indexed in `packages/engine/test/rulings.test.ts`.

| R | Topic | Ruling | Cards affected |
| --- | --- | --- | --- |
| R596 | "It" after a draw is the card that draw put in the hand | A text that draws and then acts on "it" or "the drawn card" — prices it (C+ #65.2 Normal Grape's "It costs (1) less", C+ #65.3 Large Grape's "It costs (0)") or reads it (T-AI-4 Chain of Thought's "If it costs (1) or less, repeat this") — means the card that one draw itself put in its player's hand. A card cast on draw never reaches the hand (R58), and the card the cast-on-draw chain's repeat then brings is that repeat's draw, not this one's; a burned card is in the graveyard (R4), a fatigue draw brings none (§2.4), a draw a limit stops is no draw (R457): each of these leaves "it" nothing, so nothing is priced and Chain of Thought's chain ends. Hearthstone's reading of "draw a card; it costs less" | C+ #65.2, C+ #65.3, T-AI-4; §2.4, R4, R58, R457 |
