# Patch v0.2.0 rulings: the Classic #1–#45 workstream (R520–R539)

Rows in SPEC §11's format, for the docs workstream to port into §11. Each is proved by a test named
after it and indexed in `packages/engine/test/rulings.test.ts`.

| R | Topic | Ruling | Cards affected |
| --- | --- | --- | --- |
| R520 | C #7 InfiniScepter holding an X-cost Spell | An X-cost Spell costs (0) in a hand (R65), so C #7's Cry may exile one. Its Activate casts the copy with the X its caster chooses as the cast begins, from 1 to their current mana (R348), as a cast's other choices are its caster's (R70, R81); the cast pays nothing, so the X is not paid, and bounding it by current mana keeps the choice enumerable for `legalActions` and the prompt | C #7; R65, R70, R81, R348 |
| R521 | C #9 Income Tax: which draws count | The opponent's draws in a turn are every draw that happened that turn, whoever's turn it is: the start-of-turn draw, a draw an effect makes, a card burned on a full hand and a card cast on draw alike, since each left the deck by a draw (as R55's drawn counter counts them). A draw a draw limit stops never happened and does not count (§2.4). A draw from an empty deck happened and counts, but draws no card, so it gives nothing by itself. The trap fires on the draw that brings the count to its number (2, tunable), and on no later one that turn | C #9; §2.4, R55, R58 |
