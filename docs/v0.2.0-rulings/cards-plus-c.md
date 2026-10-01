# Patch v0.2.0 rulings: the Classic+ #1–#39 workstream (R560–R579)

Rows in SPEC §11's format, for the docs workstream to port into §11. Each is proved by a test named
after it and indexed in `packages/engine/test/rulings.test.ts`.

| R | Topic | Ruling | Cards affected |
| --- | --- | --- | --- |
| R572 | C+ #12.8 Frostspatula: a kill in its own last combat | "Every Unit this destroyed" counts a Unit whose death it caused in the same state-check pass as its own death (a mutual kill): its Death adds the Units this action's `destroyed` events name it the killer of (R42) to the ones it remembered, each once, so a 10/3 that trades with a 3/3 Rush Token still summons a fresh Rush Token for its controller | C+ #12.8; R42, R89, R409 |
| R578 | C+ #37 Wardrum: which play is the 3rd | Its controller's Spell, Field Spell and Trap plays of the turn (casts included, R70) are counted in the order they were played, a cast made inside a play's resolution coming after that play; Wardrum answers the resolution of the play whose place in that order is its threshold (3) and no other, so a 4th cast that resolves inside the 3rd's resolution does not set it off and the 3rd's own resolution does | C+ #37; R70, R464 |
