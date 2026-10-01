# Patch v0.2.0 rulings: Classic cards #46–#90 (R540–R559)

The cards-classic-b workstream's block (PLAN §5). The docs workstream ports these rows into SPEC §11
at integration, in SPEC §11's format. Each row's proving tests are named `it("R<n> …")` and indexed in
`packages/engine/test/rulings.test.ts`.

| R | Topic | Ruling | Cards affected |
| --- | --- | --- | --- |
| R540 | C #90 reward J is next-turn mana | In Too Deep's reward J, "gain 100 mana", is next-turn mana: a +100 rider on its controller's next mana refresh (§2.3, R364's next-turn mana), not mana for the turn it is granted. Quest 7, the only quest that leads to J, completes as its controller's turn ends (end a turn with 5 or more unspent mana), so mana for that turn would lapse unused; the reward has its intended effect on the next turn. §8.6's row 90 reads "J, gain 100 mana" and is read this way | C #90; §2.3, R364, R404 |
| R541 | What counts as a draw for C #90 | Every draw of its controller that took a card from their deck counts toward quest 1 ("draw 2 cards"), whether the card was kept, burned at the hand cap (R4) or cast on draw (R58), as #100 Ceaseless Void's draw counter counts draws (R55, R225). A draw the draw limit stopped never happened (§2.4) and a fatigue draw takes no card, so neither counts | C #90; §2.4, R4, R55, R58, R225, R404 |
| R542 | Whose damage and whose deaths C #90 counts | Quest 5 ("your cards deal 12 damage to enemies") counts a hit whose source its controller controlled and whose target was the opponent's hero or a Unit the opponent controlled, both read as the hit landed. Quest 2 ("2 enemy permanents destroyed") counts a permanent the opponent controlled as it died, by anything, a stolen card included: the `destroyed` event names the controller a card died under (R172) | C #90; R404 |
| R543 | The order of C #90's Radiant rewards | The Radiant face's completed quest grants every reward it offers in the tree's order (A before B, C before D, …), then opens every quest they lead to. A reward that asks a question (B's target, D's placements, F's target, G's discards) still asks it; only the reward prompt itself is skipped | C #90; R404 |
| R550 | C #63 Crop Dusting places on itself too | C #63's "each permanent" is every permanent on the field as it fires, the firing trap itself included: it takes its placement like any other, and its tokens go with it when it is spent to the graveyard as its firing ends (R78) | C #63; §6.3, R78 |
