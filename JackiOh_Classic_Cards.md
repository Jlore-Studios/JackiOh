The designer's card list for the Classic and Classic+ sets, as written on 2026-09-30, reconstructed from the **Designer** quotes of `docs/classic-sets.md` (B6 and B7), one entry per card: Classic in the designer's order, Classic+ in the brief's order, each token under the card that makes it. The brief flattened the list's line breaks and bullets onto one line; this file restores them, and writes out again the Radiant faces the brief shortened to "(the same)" or "(…)". The text is the designer's, verbatim, typos included, and each header carries the designer's own name, number, type and tags, so Classic+ keeps the colliding numbers that B2.3 renumbers. A `~~~` paragraph separates a base face from its Radiant face; where the designer wrote `~~`, so does this file. SPEC.md §8.6 and §8.7 and `packages/cards/catalog.json` are the implemented reading, not this file: Classic #55 ships as Book of Wildfire and #72 as Grand Counterspell (R381). The ten AI generated cards the designer asked for (Claude's Datacenter, AI Slop) were written by the implementer and are specified in `docs/classic-sets.md` B8, not here.

# Classic

**(1) Curse of the Forgotten Classic Classic, Spell, Rare, \#1**

Deal 1 damage to the opposing Hero for each card in their Exile. Draw 1

\~\~\~

Deal 1 damage to the opposing Hero for each card in their Exile. Also Recruit a card from their Exile. If it’s a Unit, it attacks them right away.

&nbsp;

**(1) The Trickster Classic, Unit, Human, Common, \#2**

**2/1**

Cry: Your next Trap or Field Spell costs (2) less.

\~\~\~

**4/2**

Cry: Your next Trap or Field Spell costs (0).

&nbsp;

**(1) Book of Heal Classic, Spell, Book, Common, \#3**

Heal 9

\~\~\~

Heal 18

&nbsp;

**(1) Palantir Classic, Field Spell, Legendary, \#4**

Your opponent is limited to 1 Draw per turn.

When your opponent casts a Book, Tribute this to Steal it.

\~\~\~

Your opponent is limited to 1 Draw per turn.

When your opponent casts a Spell, Steal it. Tribute this once it has stolen (2) or more cost worth of Spells.

&nbsp;

**(2) Tesla Classic, Field Trap, Epic, \#5**

**1/4**

Animated Lifesteal

Activates when an opponent summons a Unit. Deal 4 damage to it. Then summon this in Defense Position.

\~\~\~

**2/8**

Animated Lifesteal

Activates when an opponent summons a Unit. Deal 8 damage to it. Then summon this in Defense Position.

&nbsp;

**(2) Cloaked Toe Cracker Classic, Unit, Human, Common, \#6**

**3/4**

Your Traps cost (0).

\~\~\~

**6/8**

Your Traps cost (0). Gain +1 Mana after playing one.

&nbsp;

**(1) InfiniScepter Classic, Field Spell, Legendary, \#7**

Cry: Exile a (0) or (1) cost Spell from your hand.

Activate: Cast a copy of that Spell.

\~\~\~

Cry: Exile a (0)-(2) cost Spell.

Activate: Cast a copy of that Spell.

&nbsp;

**(1) Pickle Classic, Spell, Rare, \#8**

Your opponent chooses 3 times:

- They Discard 1
- They Exile the bottom card of their Library
- You Draw 1

\~\~\~

Your opponent chooses 3 times:

- They Discard 2
- They Exile the bottom 2 cards of their Library
- You Draw 2

&nbsp;

**(2) Income Tax Classic, Trap, Common, \#9**

Activates when your opponent draws the 2nd card on a turn. They give you all but one card of their choice from their hand.

\~\~\~

Activates when your opponent draws the 2nd card on a turn. They give you all but one card of their choice from their hand and the cost of those cards are all reduced by (1).

&nbsp;

**(2) Exile Classic, Trap, Common, \#10**

Activates when your opponent plays a card that costs (1) or less. Counter and Exile it.

\~\~\~

Activates when your opponent plays a card that costs (3) or less. Counter and Exile it. If it costs less than (3), Exile random Permanents from your opponent’s Field until the difference in cost is made up (but never exceeded).

{AI please find a way to clean up this text}

&nbsp;

**(1) Mind Melt Classic, Spell, Common, \#11**

Look at your opponent’s hand. Exile a card in it.

\~\~\~

Look at your opponent’s hand. Exile all cards of a single cost in it.

&nbsp;

**(1) Book of Blood Classic, Spell, Book, Common, \#12**

Deal 5 damage to a Unit. Lifesteal.

\~\~\~

Deal 10 damage to a Unit. Lifesteal.

&nbsp;

**(1) Boots on the Ground Classic, Unit, Human, Common, \#13**

**2/1**

Charge

After this attacks, Draw 1.

\~\~\~

**4/2**

Charge

After this attacks, Recruit.

&nbsp;

**(2) Shadowstep Classic, Trap, Common, \#14**

Activates when any number of your Units die. Return them all to your hand. They cost (0).

\~\~\~

Activates when any number of your Units die. Flicker them so that they survive and add copies to your hand that cost (0).

&nbsp;

**(1) Nose Hunter Classic, Unit, Human, Common, \#15**

**3/1**

Discard a random card: Exile the bottom card of each opponent's Deck.

\~\~\~

**6/2**

Discard a random card: Exile the bottom card of each opponent's Deck and a random card in their hand.

&nbsp;

**(1) Book of Flame Classic, Spell, Book, Common, \#16**

Deal 4 damage

\~\~\~

Deal 8 damage

&nbsp;

**(2) Counterspell Classic, Trap, Common, \#17**

Activates when your opponent casts a Spell. Counter it.

\~\~\~

Activates when your opponent casts a Spell. Counter it. Add a (0) cost copy to your hand.

&nbsp;

**(3) Glitch in the System Classic, Spell, Common, \#18**

Pick a number. Exile all cards that cost that much.

\~\~\~

Pick a number. Exile all cards from your opponent’s Field, Hand, and Deck that cost that much.

&nbsp;

**(1) Lizard’s Breath Classic, Spell, Rare, \#19**

Deal 2 damage. Gains one of the additional effects based on which Pile has the most cards:

- Deck: Draw 1
- GY: +2 Mana
- Exile: Deal +4 damage.

{Note: Ties are broken from top to bottom}

\~\~\~

Deal 4 damage. Gains two of the additional effects based on the two Piles with the most cards:

- Deck: Draw 1
- GY: +2 Mana
- Exile: Deal +4 damage.

&nbsp;

**(2) The Power to Punish Classic, Field Spell, Rare, \#20**

Activate: Choose one:

- Deal 2 damage
- Opponent Discards 1
- Choose a Unit. It is Destroyed at the start of your next turn.

\~\~\~

Activate: Choose one:

- Deal 4 damage
- Opponent Discards 2
- All opposing Units are Destroyed at the start of your next turn.

&nbsp;

**(2) Turtinator Classic, Unit, Common, \#21**

**5/4**

Activate ♾️: Tribute a Unit. Deal damage equal to its attack to any target.

\~\~\~

**10/8**

Activate ♾️: Tribute a Unit. Deal damage equal to twice its attack to any target.

&nbsp;

**(1) Mid Runner Classic, Unit, Human, Common, \#22**

**2/1**

Cry: If this was Summoned in midlane, Tribute it. If you had (4)+ Mana when cast, Bounce 2 random enemy cards.

\~\~\~

**4/2**

Cry: If this was Summoned in midlane, Tribute it. If you had (4)+ Mana when cast, Bounce 2 random enemy cards.

&nbsp;

**(2) Devil’s Pact Classic, Field Spell, Rare, \#23**

Cry: Discard 666

Activate: All cards you play are this turn are replaced by Book of Flame.

\~\~\~

Cry: Discard 6

Activate: All cards you play this turn are replaced by Radiant Book of Flame.

&nbsp;

**(1) Book of Knowledge Classic, Spell, Book, Epic, \#24**

Draw 3

\~\~\~

Draw 6

&nbsp;

**(0) Lag in the System Classic, Spell, Common, \#25**

Exile all (0) and (1) cost cards.

\~\~\~

Exile all enemy (0) and (1) cost cards.

&nbsp;

**(0) Rapid Draw Classic, Spell, Common, \#26**

Draw 4

Discard 4

\~\~\~

Draw 5

Discard 4

&nbsp;

**(0) Pestilent Slime Classic, Unit, Common, \#27**

**1/1**

Double Plague Tokens placed on this.

\~\~\~

**2/2**

Triple Plague Tokens placed on this.

&nbsp;

**(0) Second Wind Classic, Field Spell, Epic, \#28**

Cry: Exile your Deck. Discard your hand.

Aura: You can play cards from your GY. When a card enters your GY, Exile it.

\~\~\~

Cry: Exile your Deck. Discard your hand.

Aura: You can play cards from your GY as long as (1)+ Mana is spent.

&nbsp;

**(1) Book of Vital Kill Classic, Spell, Book, Epic, \#29**

Set a Hero’s health to 13.

\~\~\~

Set a Hero’s health to 13. Add a Book of Flame to your hand.

&nbsp;

**(1) Recycle Classic, Spell, Rare, \#30**

Shuffle your GY into your Library. Draw 1.

\~\~\~

Shuffle your GY into your Library. Reduce shuffled cards’ cost by (1). Draw 1.

&nbsp;

**(2) Cookie Guild Classic, Unit, Human, Common, \#31**

**2/4**

Cry: Recruit a Unit that costs (2) or less.

\~\~\~

**4/8**

Cry: Recruit 3 Units that cost (2) or less.

&nbsp;

**(0) Felinor Feelings Classic, Spell, Felinor, Rare, \#32**

Take control of any card in a lane with a (1) cost Unit you control.

\~\~\~

Summon a Felinor Token. Take control of any card in a lane with a (1) cost Unit you control.

&nbsp;

**(0) Joro Classic, Unit, Legendary, \#33**

**1/1**

After a friendly Unit is targeted by an opponent, this is instead summoned to be the new target.

\~\~\~

**1/1**

Indestructible

After a friendly Unit is targeted by an opponent, this is instead summoned to be the new target.

&nbsp;

**(1) Ancient Acquisition Classic, Spell, Rare, \#34**

Return 2 cards from your GY to your hand.

\~\~\~

Return 4 cards from your GY or Exile to your hand.

&nbsp;

**(0) Prep Classic, Spell, Common, \#35**

Your next Spell this turn costs (2) less.

\~\~\~

Your next Spell this turn costs (4) less.

&nbsp;

**(0) Burn Classic, Spell, Common, \#36**

Deal 2 damage. Draw 1 if you have at 4 or more remaining Mana.

\~\~\~

Deal 4 damage. Draw 1 if you have at 4 or more Total Mana.

&nbsp;

**(0) Last Hurrah Classic, Spell, Epic, \#37**

Draw your deck. At the end of this turn, Discard your hand.

\~\~\~

Draw your deck. At the end of your next turn, Discard your hand.

&nbsp;

**(2) Jackiestan Auctioneer Classic, Field Trap, Human, Rare, \#38**

**4/4**

Animated

Activates/summons when a player plays their 3rd card on a turn.

Whenever a player plays a card, you Draw 1 and deal 2 damage to each enemy hero.

\~\~\~

**8/8**

Animated

Activates/summons when a player plays their 2nd card on a turn.

Whenever a player plays a card, you Draw 1 and deal 4 damage to each enemy hero.

&nbsp;

**(1) Outbreak Classic, Spell, Epic, \#39**

Place a Plague Token. If the number of Plague Tokens is greater than or equal to its cost and it belongs to an enemy, Steal it. Otherwise, Draw equal to the number of Plague Tokens on it.

\~\~\~

Place 2 Plague Tokens. If the number of Plague Tokens on the Permanent is greater than or equal to its cost and it belongs to an enemy, Steal it. Otherwise, Draw equal to the number of Plague Tokens on it.

&nbsp;

**(1) MC Tech Classic, Unit, Rare, \#40**

**3/3**

Cry: If an opponent controls 4 or more Permanents, take control of one of them at random.

\~\~\~

**6/6**

Cry: If an opponent controls 4 or more Permanents, take control of one of them of your choice.

&nbsp;

**(1) State of the Game Classic, Unit, Common, \#41**

**3/3**

Indestructible

\~\~\~

**6/6**

Indestructible

&nbsp;

**(2) Transmutable Toxins Classic, Field Spell, Rare, \#42**

Activate: Place a 2 Plague Tokens at random on Units.

Ally Units gain +1/+1 per Plague Token on them. Enemy Units gain -1/-1 per Plague Token on them.

\~\~\~

Activate: Place 2 Plague Tokens at random on Units.

Ally Units gain +2/+2 per Plague Token on them. Enemy Units gain -2/-2 per Plague Token on them.

&nbsp;

**(3) Plague Nuke Classic, Spell, Epic, \#43**

Destroy all Units. +1 Mana for each Plague Token on them.

\~\~\~

Destroy all Units. +1 Mana for each Plague Token on them and those with Plague Tokens are resummoned from the GY under your control.

&nbsp;

**(4) Back from the GY Classic, Spell, Legendary, \#44**

Summon up to (5) mana worth of Units from your GY.

Exile this.

\~\~\~

Summon all Units from your GY.

Exile this.

&nbsp;

**(2) Nature Titan Classic, Unit, Legendary, \#45**

**6/6**

Tribute 1

Cry and on Attack: Draw 1, your Hero Heals 3.

\~\~\~

**12/12**

Tribute 1

Cry and on Attack: Draw 2, your Hero Heals 6.

&nbsp;

**(1) Divine Favor Classic, Spell, Rare, \#46**

Draw until you have as many cards as your opponent.

\~\~\~

Draw until you have twice as many cards as your opponent.

&nbsp;

**(2) Recurring Felinor Classic, Unit, Felinor, Rare, \#47**

**3/2**

Cry: Cast Ancient Acquisition

If this is in your GY and you activate a Trap, return this to your hand.

\~\~\~

**6/4**

Cry: Cast Ancient Acquisition

If this is in your GY and you activate a Trap, return this to your hand. It costs (0).

&nbsp;

**(2) Hired Shrimp Classic, Unit, Common, \#48**

**4/3**

Cry: Destroy a card that takes more lines of code to implement.

\~\~\~

**8/6**

Cry: Destroy a card that takes more lines of code to implement. Valid targets are highlighted.

&nbsp;

**(3) Anti-Greed Machine Classic, Unit, Common, \#49**

**9/9**

Rush

Draw is limited to 1 per turn.

\~\~\~

**18/18**

Rush

Enemy Draw is limited to 1 per turn.

&nbsp;

**(2) Voidwalker Classic, Unit, Rare, \#50**

**6/3**

Cry: Exile all GYs.

Whenever a card is sent to a GY, Exile it.

\~\~\~

**12/6**

Cry: Exile the enemy GYs.

Whenever an enemy card is sent to a GY, Exile it.

&nbsp;

**(1) Back Breaker Classic, Unit, Common, \#51**

**3/2**

Stack

Death: Destroy all backrow.

\~\~\~

**6/4**

Stack

Death: Destroy all enemy backrow.

&nbsp;

**(2) Final Gambit Classic, Trap, Epic, \#52**

Activate when you would take lethal damage. Redirect it to your opponent, heal 10, and Draw 3.

\~\~\~

Activate when you would take lethal damage. Redirect it to your opponent, heal 20, and Draw your deck.

&nbsp;

**(1) Plague Crawler Classic, Unit, Common, \#53**

**2/2**

Cry: Place a Plague Token on another card.

When a Plague Token is placed on this, Draw 1.

\~\~\~

**4/4**

Cry: Place 2 Plague Tokens on another card.

When a Plague Token is placed on this, Draw 2.

&nbsp;

**(1) Rewind Classic, Spell, Common, \#54**

Trigger the Cry of a friendly Unit on the Field or in the GY.

\~\~\~

Trigger the Cry of a Unit twice on the Field or in the GY.

&nbsp;

**(1) Book of Flame Classic, Spell, Book, Epic, \#55**

Deal 4 damage.

\~\~\~

Deal 8 damage.

&nbsp;

**(4) Spell Tyrant Classic, Unit, Legendary, \#56**

**5/5**

Cry: Cast 3 Spells from your GY. Exile them afterward.

\~\~\~

**10/10**

Cry: Cast all Spells from your GY. Exile them afterward.

&nbsp;

**(1) Echo Classic, Spell, Epic, \#57**

Has the text of the last played Spell (from either player).

\~\~\~

Has the text of the last played Spell (from either player). Echo 1.

&nbsp;

**(2) Common Resources Classic, Field Spell, Common, \#58**

Start of Turn: Draw the bottom card of the enemy Deck.

\~\~\~

Start of Turn and End of Turn: Draw the bottom card of the enemy Deck.

&nbsp;

**(1) Plague Doctor Classic, Unit, Human, Common, \#59**

**2/3**

Cry: Deal damage equal to the number of Plague Tokens on the field.

\~\~\~

**4/6**

Cry: Place 2 Plague Tokens on this. Deal damage equal to the number of Plague Tokens on the field.

&nbsp;

**(5) Pile On Classic, Spell, Rare, \#60**

Recruit your entire Deck.

If this is put in your GY, put it at the bottom of your Library instead.

\~\~\~

Recruit your entire Deck.

&nbsp;

**(3) Plague Bringer Goliath Classic, Unit, Legendary, \#61**

**7/7**

Tribute 1 Rush Trample

Cry: Place 3 Plague Tokens. Draw 1.

\~\~\~

**14/14**

Tribute 1 Rush Trample

Cry: Place 3 Plague Tokens. Draw 3.

&nbsp;

**(1) Living Bomb Classic, Field Spell, Rare, \#62**

At the start of each player’s turn, they destroy all cards with a Plague Counter on them.

\~\~\~

At the start of each enemy’s turn, they destroy all cards with a Plague Counter on them.

&nbsp;

**(2) Crop Dusting Classic, Trap, Common, \#63**

Start of Turn: activate. Put a Plague Token on each Permanent. Draw 1.

*(no Radiant face written)*

&nbsp;

**(2) Malzahar’s Recycler Classic, Field Spell, Rare, \#64**

End of Turn: Discard 2

Whenever you Discard, Draw the same number.

\~\~\~

End of Turn: Discard 2

Whenever you Discard, Draw your entire deck.

&nbsp;

**(2) Ace in the Hole Classic, Trap, Common, \#65**

End of Turn: Flip a coin. If heads, activate. Recruit a card.

\~\~\~

End of Turn: Flip a coin. If tails, Recruit a card. If heads, activate. Recruit 3 cards.

&nbsp;

**(2) EU Striker Classic, Unit, Human, Common, \#66**

**5/4**

Summon this from your hand if you play a Unit. If you play a card, Bounce this.

\~\~\~

**10/8**

Rush

Summon this from your hand if you play a Unit. If you play a card, Bounce this.

&nbsp;

**(1) Felinor Feeler Classic, Unit, Human, Common, \#67**

**2/4**

Pierce

Cry: Set all enemy Units to Defense Position.

\~\~\~

**4/8**

Pierce Rush

Cry: Set all enemy Units to Defense Position.

&nbsp;

**(4) Small Card Lobbyist Classic, Unit, Common, \#68**

**11/13**

(3)+ cost cards cost (1) more.

\~\~\~

**22/26**

Enemy (3)+ cost cards cost can’t be played.

&nbsp;

**(2) Plague Charger Classic, Unit, Rare, \#69**

**4/2**

Charge

If this has a Plague Token on it, First Strike

+2 Attack for each Plague Token on this

\~\~

**8/4**

Charge

If this has a Plague Token on it, First Strike

+4 Attack for each Plague Token on this

&nbsp;

**(1) Book of Plague Classic, Spell, book, Epic, \#70**

Place 5 Plague Tokens.

\~\~\~

Place 10 Plague Tokens.

&nbsp;

**(3) Lane Eater Classic, Unit, Common, \#71**

**4/4**

Destroy other cards in and Lock this Lane.

\~\~\~

**8/8**

Destroy enemy cards in and Lock the enemy Lane.

&nbsp;

**(2) Counterspell Classic, Trap, Rare, \#72**

Activate when enemy casts any kind of Spell or Trap. Counter that card.

\~\~\~

Activate when enemy casts any kind of Spell or Trap. Steal that card.

&nbsp;

**(2) Nurse Cleaver Classic, Unit, Common, \#73**

**3/6**

Rush Cleave Lifesteal

\~\~\~

**6/12**

Charge Cleave Lifesteal

&nbsp;

**(2) Corpse Plantation Classic, Field Spell, Epic, \#74**

Cry: Place 2 Plague Tokens on this.

You may spend Plague Tokens on this as mana to cast Units from your GY. You must spend at least (1) Plague Token.

\~\~\~

Cry: Place 4 Plague Tokens on this.

You may spend Plague Tokens on this as mana to cast Units from your GY. You must spend at least (1) Plague Token.

&nbsp;

**(1) Argusland Classic, Field Spell, Rare, \#75**

Your Hero takes half damage rounded up.

\~\~\~

Your Hero takes quarter damage rounded up.

&nbsp;

**(2) Plague Bringer Classic, Unit, Common, \#76**

**4/4**

Rush

Cry: Place 2 Plague Tokens. Draw 1.

\~\~\~

**8/8**

Rush

Cry: Place 4 Plague Tokens. Draw 2.

&nbsp;

**(2) Anti-Magic Monkey Classic, Unit, Common, \#77**

**5/5**

Stack

Spells cost (1) more.

\~\~\~

**10/10**

Stack

Spells cost (2) more.

&nbsp;

**(1) Mutate Spell Classic, Spell, Rare, \#78**

Active ♾️: Consume a Plague Token on a card for the following effect:

- Enemy card: Exile it.
- Ally Backrow: Draw 2.
- Ally Unit: Attack a random enemy.

\~\~\~

Activate ♾️: Consume a Plague Token on a card for the following effect:

- Enemy card: Fuse it with a valid card in your Field, hand, or Library (Exile otherwise).
- Ally Backrow: Draw 4.
- Ally Unit: Attack a random enemy twice.

&nbsp;

**(1) Risky Die Classic, Spell, Common, \#79**

Draw 3

Reduce their cost by (1). Exile all that cost more than (0).

\~\~\~

Draw 3

Reduce their cost by (1). Exile all that cost more than (1).

&nbsp;

**(4) BOOM! Big Max Classic, Unit, Legendary, \#80**

**26/8**

Tribute 2 Rush Trample Indestructible

\~\~\~

**26/16**

Tribute 2 Rush Trample Indestructible

&nbsp;

**(2) The Power to Thrive Classic, Field Spell, Rare, \#81**

Active: Choose one:

- You heal 3
- Draw 1
- +1 Mana

\~\~\~

Active: Choose one:

- You heal 6
- Draw 2
- +2 Mana

&nbsp;

**(1) Sheeople Classic, Unit, Common, \#82**

**1/1**

Death: Draw 2

Counts as 2 Tribute

\~\~\~

**2/2**

Death: Draw 3

Counts as 3 Tribute

&nbsp;

**(3) Flame Lance Classic, Spell, Common, \#83**

Trample

Deal 11 damage to a Unit.

\~\~\~

Trample

Deal 22 damage to a Unit.

&nbsp;

**(2) Lockdown Classic, Field Spell, Rare, \#84**

Indestructible

Aura: When a Permanent is cast, Lock that space

Active: Tribute this

\~\~\~

Indestructible

Aura: When an enemy Permanent is cast, Lock that space

Active: Tribute this

&nbsp;

**(4) King Wagtoggle Classic, Unit, Legendary, \#85**

**5/5**

Cry: Swap Decks with the enemy.

\~\~\~

**10/10**

Cry: Swap Decks with the enemy. Recruit a card.

&nbsp;

**(4) Genn Classic, Unit, Common, \#86**

**14/14**

\~\~\~

**14/14**

&nbsp;

**(X) Plague Chalice Classic, Field Spell, Epic, \#87**

Enters with X Plague Tokens on it.

Counter all cards with cost equal to the number of Plague Tokens on this.

\~\~\~

Enters with X Plague Tokens on it.

Counter all enemy cards with cost equal to the number of Plague Tokens on this.

&nbsp;

**(2) Siphon Squad Classic, Field Trap, Rare, \#88**

Enemy Units have -X attack where X is twice the number of Units they control.

Tribute this if an enemy ever has 0 Units.

\~\~\~

Enemy Units have 0 attack.

Tribute this if an enemy ever has 0 Units.

&nbsp;

**(2) Paul Allen’s Ghost Classic, Unit, Rare, \#89**

**5/6**

Divine Shield

As an additional cost to target this with anything but an attack, Discard 2.

\~\~\~

**10/12**

Divine Shield Reborn

As an additional cost to target this with anything but an attack, Discard 2.

&nbsp;

**(1) In Too Deep Classic, Field Spell, Quickdraw, Mythic, \#90**

Indestructible

Go on a quest!

Quests

- Quest 1: Draw 2 cards
- Quest 2: Destroy 2 cards
- Quest 3: Control 3 cards
- Quest 4: Float 3 mana
- Quest 5: Deal 12 damage
- Quest 6: Control 10 attack and 10 health work of stats
- Quest 7: Float 5 mana
- Quest 8: Exile 3 cards
- Quest 9: Draw your entire Deck
- Quest 10: Have 6 Units in your GY

Rewards

- Reward A: Heal 6
- Reward B: Deal 3 damage
- Reward C: Return 2 random cards from your GY to your hand
- Reward D: Place 3 Plague Tokens
- Reward E: Give a random friendly Unit +3/+3
- Reward F: Bounce a card
- Reward G: Enemy Discards 2
- Reward H: Draw 2
- Reward I: Recruit a card
- Reward J: Gain 100 mana
- Reward K: Exile the enemy Deck
- Reward L: Aura: You may play cards from the GY
- Reward M: Aura: Your Units have Indestructible

Progression paths

- Quest 1 => Reward A or B
- Quest 2 => Reward C or D
- Quest 3 => Reward D or E
- Quest 4 => Reward F or G
- Quest 5 => Reward G or H
- Quest 6 => Reward H or I
- Quest 7 => Reward J
- Quest 8 => Reward K
- Quest 9 => Reward L
- Quest 10 => Reward M
- Reward A => Quest 2
- Reward B => Quest 3
- Reward C => Quest 4
- Reward D => Quest 5
- Reward E => Quest 6
- Reward F => Quest 7
- Reward G => Quest 8
- Reward H => Quest 9
- Reward I => Quest 10

\~\~\~

Radiant: You don’t have to choose a path, you get them all.

{after completing a quest, choose a reward, which also sends you down another quest path. when actually in game, only shows the active quest and the current reward options}

&nbsp;

# Classic+

**(3) Doom Shroom Classic+, Trap, Epic, \#1**

Activate when your Hero is attacked: Exile all Units. Lock this slot.

\~\~\~

Activate when your Hero is attacked: Exile all enemy Units. Lock this slot.

&nbsp;

**(3) Groom Shroom Classic+, Trap, Felinor, Epic, \#2**

Activate when your Hero is attacked: Fill your board with random Felinors. Give them Taunt.

\~\~\~

Activate when your Hero is attacked: Fill your board with random Radiant Felinors. Give them Taunt.

&nbsp;

**(2) Second Amendment Snake Classic+, Unit, Rare, \#3**

**1/6**

End of Turn: Gain 2 Plague Tokens

Death: Deal 1 damage split among enemies for each Plague Token on this.

\~\~\~

**2/12**

End of Turn: Gain 3 Plague Tokens

Death: Deal 1 damage split among enemies for each Plague Token on this.

&nbsp;

**(3) Juhan Biggest Bat Classic+, Unit, CN, Common, \#4**

**9/6**

Stack

Cards under this are transformed into copies of this.

\~\~\~

**18/12**

Stack First Strike

Cards under this are transformed into copies of this.

&nbsp;

**(2) Guy Att Classic+, Unit, Human, Common, \#5**

**6/8**

Cry: Destroy all your backrow.

\~\~\~

**12/16**

Cry: Destroy ALL backrow.

&nbsp;

**(1) Wrong-House Attacker Classic+, Unit, Human, Common, \#6**

**1/1**

Rush, Lifesteal, Poisonous

\~\~\~

**2/2**

Rush, Lifesteal, Poisonous, Reborn

&nbsp;

**(3) The House Classic+, Field Spell, Rare, \#7**

Cry and start of turn summon either a Right-House Protector (⅔) or Wrong House Attacker (⅓)

\~\~\~

Radiant: both

&nbsp;

**(2) Withering Storm Classic+, Spell, Rare, \#8**

Degrade 4 random cards in your opponents deck

Draw 1

\~\~\~

Degrade all cards in your opponents deck

Draw 1

&nbsp;

**(0) Silence Classic+, Spell, Common, \#9**

Vanilla a Unit

\~\~\~

Vanilla a Permanent

&nbsp;

**(0) New Wraps Classic+, Spell, Common, \#10**

Give a Unit Reborn.

\~\~\~

Give a Unit Reborn. Make it Radiant.

&nbsp;

**(2) Anime Armor Classic+, Unit, Rare, \#11**

**4/4**

Your Hero can only take up to 1 damage at a time.

\~\~\~

**8/8**

Reborn

Year Hero can only take up to 1 damage at a time.

&nbsp;

**(3) The Mother Pancake Classic+, Unit, Pancake, Legendary, \#12**

**8/8**

Taunt

End of turn: Add a Pancake card tokens to your hand

\~\~\~

**16/16**

Taunt

End of turn: Add 2 Pancake card tokens to your hand

&nbsp;

**(0) Devour Classic+, Spell, Pancake, Legendary, Token, \#12.1**

Destroy a Unit. Take damage equal to its remaining health.

\~\~\~

Destroy a Unit. Gain health equal to its remaining health.

&nbsp;

**(1) Death Boil Classic+, Spell, Pancake, Legendary, Token, \#12.2**

Target a Unit or Hero. Deal 6 damage if an enemy and Heal 6 if an ally.

\~\~\~

Target a Unit or Hero. Deal 12 damage if an enemy and Heal 12 if an ally.

&nbsp;

**(1) Fluffy Grip Classic+, Spell, Pancake, Legendary, Token, \#12.3**

Steal a Unit from your opponent's deck Deck add it to your hand. It costs (0).

\~\~\~

Steal a Unit from your opponent's Deck and add it to your hand. It costs (0) and becomes Radiant.

&nbsp;

**(1) Powder Spray Classic+, Spell, Pancake, Legendary, Token, \#12.4**

Deal 3 damage to all enemies.

\~\~\~

Deal 6 damage to all enemies.

&nbsp;

**(1) Anti-Waffle Shell Classic+, Field Spell, Pancake, Legendary, Token, \#12.5**

Cry: Give your Units Divine Shield

Aura: Your Units have +2/+2

\~\~\~

Cry: Give your Units Divine Shield

Aura: Your Units have +4/+4

&nbsp;

**(2) Frozen Wastes Classic+, Field Spell, Pancake, Legendary, Token, \#12.6**

Destroy all Units. Exile a card from your Deck for each one.

\~\~\~

Destroy all Units. Your opponent Exiles a card from their Deck for each one.

&nbsp;

**(2) Legion of the Hungry Classic+, Field Spell, Pancake, Legendary, Token, \#12.7**

Exile 5 random cards from your Deck. Summon any Units among them.

\~\~\~

Exile 5 random cards from your Deck. Summon any Units among them. Make them Radiant.

&nbsp;

**(2) Frostspatula Classic+, Field Spell, Pancake, Legendary, Token, \#12.8**

**10/3**

Animated on your turn

Rush

Death: Resummon all Units destroyed by this

\~\~\~

**20/6**

Animated on your turn

Rush

Death: Resummon all Units destroyed by this. Make them Radiant.

&nbsp;

**(1) Mommy Barker Classic+, Unit, Human, Pancake, Legendary, \#13**

**2/2**

Death: Add a Pancake card token to your hand

\~\~\~

**4/4**

Reborn

Death: Add a Pancake card token to your hand

&nbsp;

**(1) Forever& Classic+, Spell, Epic, \#14**

Then next spell you play gains “When this leaves your hand, add it right back to your hand (it can’t cost less than (2) mana)”

\~\~\~

Then next spell you play gains “When this leaves your hand, add it right back to your hand (it can’t cost less than (1) mana)”

Draw 1

[fix wording]

&nbsp;

**(1) Conjure Rush Token Classic+, Spell, Common, \#15**

Summon a 3/3 rush token, it gains a random keyword

\~\~\~

Summon 3 of them

&nbsp;

**(2) Conjure Rush Token+ Classic+, Spell, Rare, \#16**

Summon a 3/3 rush token, it gains 3 random keywords

\~\~\~

Summon 3 of them

&nbsp;

**(4) Conjure Rush Token++ Classic+, Spell, Epic, \#17**

(4) Conjure Tush Token++, spell, Epic

Summon a 3/3 rush token, it gains 3 random keywords

\~\~\~

Summon 3 of them

&nbsp;

**(2) Gullible Treatler Classic+, Unit, Human, Common, \#18**

**8/9**

Start of Turn: Tribute this if you don’t control a Field Spell or Trap

\~\~\~

**16/18**

Start of Turn: Tribute this if no player controls a Field Spell or Trap

&nbsp;

**(4) League of Losers Classic+, Spell, Legendary, \#19**

Summon the five stack. Summon Top Loser, Jungle Loser, Mid Loser, Support Loser, and Bot Loser into your Unit Zones 1–5, in that order. (Occupied zones are skipped.)

\~\~\~

Summon the Radiant five stack. Summon Radiant Top Loser, Radiant Jungle Loser, Radiant Mid Loser, Radiant Support Loser, and Radiant Bot Loser into your Unit Zones 1–5, in that order. (Occupied zones are skipped.)

&nbsp;

**(2) Top Loser Classic+, Unit, Legendary, Token, \#19.1**

**5/5**

Armor 3

Cannot be in Defense Position. Can only be attacked by Units in this lane.

\~\~\~

**10/10**

Armor 6

Cannot be in Defense Position. Can only be attacked by Units in this lane, Immune to Spells.

&nbsp;

**(2) Jungle Loser Classic+, Unit, Legendary, Token, \#19.2**

**5/5**

End of Turn: 25% chance to attack a random enemy Unit. If this Destroys the enemy Unit in Bot Loser's lane, Bot Loser goes Berserk.

\~\~\~

**10/10**

End of Turn: 50% chance to attack a random enemy Unit. If this Destroys the enemy Unit in Bot Loser's lane, Bot Loser gets the kill instead (trigger its "When this destroys a Unit" effect).

&nbsp;

**(2) Mid Loser Classic+, Unit, Legendary, Token, \#19.3**

**5/5**

Cry: Flip a coin. Heads (Fed): Gain +5/+5. Tails (Int): Gain -3/-3. Your opponent gains 1 mana next turn.

\~\~\~

**10/10**

Lucky 1

Cry: Flip a coin. Heads (Fed): Gain +10/+10. Tails (Int): Gain -3/-3. Your opponent gains 1 mana next turn.

&nbsp;

**(2) Support Loser Classic+, Unit, Legendary, Token, \#19.4**

**0/5**

Divine Shield

End of Turn: Heal your Hero and all your Units by 3.

\~\~\~

**0/10**

Divine Shield, Reborn

End of Turn: Heal your Hero and all your Units by 6.

&nbsp;

**(2) Bot Loser Classic+, Unit, Legendary, Token, \#19.5**

**5/5**

Rush, First Strike

When this destroys a Unit, gain +5 Attack.

While Berserk: At the Start and End of Turn, attack your Hero.

\~\~\~

**10/10**

Charge, First Strike

When this destroys a Unit, gain +10 Attack.

Tranquility makes this unit not be able to go Berserk

&nbsp;

**(1) Mushroom Power Classic+, Unit, Common, \#20**

**2/2**

Cry: Give adjacent Units +2/+2

\~\~\~

**4/4**

Cry: Give adjacent Units +4/+4

&nbsp;

**(0) Whirlwind Classic+, Spell, Common, \#21**

Pierce

Deal 1 damage to all Units

\~\~\~

Pierce

Deal 1 damage to all Units

End of turn: Return this to your hand

&nbsp;

**(1) Blood Moon Classic+, Trap, Rare, \#22**

Activates when an enemy is healed: All enemy healing this turn is converted to Pierce damage instead.

\~\~\~

{Becomes a Field Trap}

Activates when an enemy is healed: All enemy healing is converted to Pierce damage instead.

&nbsp;

**(1) Dropshipping Classic+, Spell, CN, Epic, \#23**

Add 3 random cards (including tokens) to your hand. Give them Brittle 2.

\~\~\~

Add 3 random cards (including tokens) to your hand. Give them Brittle 2 and set their Cost to (1).

&nbsp;

**(3) Crushing Walls Classic+, Spell, Epic, \#24**

Destroy all cards in each player’s leftmost and rightmost lanes for each player.

\~\~\~

Destroy all cards in the enemy’s leftmost and rightmost lanes.

&nbsp;

**(2) Soul Shot Classic+, Spell, Common, \#25**

Destroy a random enemy Unit

\~\~\~

Lucky 1

Destroy a random enemy Unit

&nbsp;

**(3) Tommy Tempo Classic+, Unit, Human, Common, \#26**

**9/9**

Taunt

Cast on Draw: End your turn immediately

\~\~\~

**18/18**

Taunt

Cast on Draw: You can take one more action, then your turn ends

&nbsp;

**(4) Zephrys Zealotism Classic+, Spell, Mythic, \#27**

Replace your Hand with the perfect Hand. Refresh your mana.

\~\~\~

Replace your Hand with the perfect Radiant Hand. Refresh your mana. (only using cards from Classic & Classic+)

&nbsp;

**(2) Nuestro hogar, nuestras tumbas Classic+, Unit, Common, \#28**

**3/4**

Taunt, Reborn

Death: Your hero heals 3

\~\~\~

**6/8**

Taunt, Reborn, Divine Shield

Death: Your hero heals 8

&nbsp;

**(3) Portal to the Past Classic+, Spell, Epic, \#29**

Discover a card from your last game’s board (as it was when that game ended). It costs (0).

\~\~\~

Add 3 random cards from your last game’s board (as it was when that game ended) to your hand. They cost (0).

&nbsp;

**(3) Felinor Fuser Classic+, Unit, Felinor, Epic, \#30**

**3/3**

Cry: Discover 2 Felinors. Fuse them into this.

\~\~\~

**6/6**

Cry: Discover 2 Radiant Felinors. Fuse them into this.

&nbsp;

**(2) Fusion Lab Classic+, Field Spell, Epic, \#31**

Cry and End of Turn: Fuse a random card into a card in your hand. Its cost stays the same.

\~\~\~

Cry and End of Turn: Fuse a random Radiant card into a card in your hand. Its cost stays the same.

&nbsp;

**(2) Otherworldly Removal Classic+, Spell, Epic, \#32**

Add an Execute, a Brawl, and a Blade Storm to your hand.

\~\~\~

Add a Radiant Execute, Brawl, and Blade Storm to your hand.

&nbsp;

**(1) Execute Classic+, Spell, Epic, Token, \#32.1**

Destroy a damaged Unit.

\~\~\~

Destroy all damaged enemy Units.

&nbsp;

**(2) Brawl Classic+, Spell, Epic, Token, \#32.2**

Destroy all Units except one chosen at random.

\~\~\~

Destroy all Units except one of your choice.

&nbsp;

**(1) Blade Storm Classic+, Spell, Epic, Token, \#32.3**

Deal 1 damage to all Units. Repeat until a Unit dies (up to 30 times).

\~\~\~

Deal 1 damage to all enemy Units. Repeat until a Unit dies (up to 30 times).

&nbsp;

**(2) Ivory Tower Classic+, Field Spell, Rare, \#33**

Your cards gain Stack.

A Unit may be played on top of this. That Unit can't attack or be attacked.

\~\~\~

Your cards gain Stack.

A Unit may be played on top of this. That Unit can't attack or be attacked, and becomes Radiant.

&nbsp;

**(3) Memory Leak Classic+, Field Spell, Epic, \#34**

Choose one:

End of Turn: Lock a random Zone on your opponent's side.

After your opponent plays a Field Spell or Unit, Lock that Zone.

\~\~\~

End of Turn: Lock a random Zone on your opponent's side.

After your opponent plays a Field Spell or Unit, Lock that Zone.

&nbsp;

**(4) Rollback Classic+, Spell, Legendary, \#35**

Return the board to its state from 1, 2, or 3 turns ago.

\~\~\~

Return the board to its state from 1, 2, or 3 turns ago. Choose whether this affects only your side, only your opponent's side, or both.

&nbsp;

**(2 embiggen 4) Conjure Bones Classic+, Spell, Rare, \#36**

Shuffle 7 embiggen 17 Bone Storm into your deck.

\~\~\~

Shuffle 7 embiggen 17 Radiant Bone Storm into your deck.

&nbsp;

**(1) Bone Storm Classic+, Spell, Rare, Token, \#36.1**

Cast on Draw: Deal 1 damage to all enemies.

\~\~\~

Echo

Cast on Draw: Deal 1 damage to all enemies.

&nbsp;

**(5) Wardrum Classic+, Unit, Quickdraw, Legendary, \#37**

**5/5**

After you play 3 or more Spells, Field Spells, or Traps in a turn: Summon this from your hand or deck.

End of Turn: Cast a random Spell, Field Spell, or Trap you played this turn.

\~\~\~

**10/10**

After you play 3 or more Spells, Field Spells, or Traps in a turn: Summon this from your hand or deck.

End of Turn: Cast all Spells, Field Spells, and Traps you played this turn.

&nbsp;

**(2) Solarius Classic+, Unit, Epic, \#38**

**3/2**

Spell Damage +2

Cry: Draw 1.

Death: Shuffle a Solarius-Prime into your deck.

\~\~\~

**6/4**

Spell Damage +5

Cry: Draw 2.

Death: Shuffle a Radiant Solarius-Prime into your deck.

&nbsp;

**(4) Solarius-Prime Classic+, Unit, Epic, Token, \#38.1**

**9/5**

Spell Damage +3

Cry: Cast 5 random Spells. They target enemies when possible.

\~\~\~

**18/10**

Spell Damage +7

Cry: Cast 5 random Radiant Spells. They target enemies when possible.

&nbsp;

**(1) Book Worm Classic+, Unit, Common, \#39**

**1/4**

Death: Add 1 random Book to your hand.

Start of Turn: Increase the number of Books this adds by 1.

\~\~\~

**2/8**

Death: Add 1 random Radiant Book to your hand.

Start of Turn: Increase the number of Books this adds by 1.

&nbsp;

**(X) Appropriations Classic+, Spell, Epic, \#40**

Choose one:

Military: Units on your board, in your deck, and in your hand gain +2X Attack and Rush.

Education: Shuffle 2X Radiant Books into your deck. They gain Cast on Draw and target enemies when possible.

Culture: Cards on your board, in your deck, and in your hand have a 10X% chance to become Radiant.

Healthcare: Units on your board, in your deck, and in your hand gain +2X Health and Armor X.

\~\~\~

Choose one:

Military: Units on your board, in your deck, and in your hand gain +5X Attack and Rush.

Education: Shuffle 5X Radiant Books into your deck. They gain Cast on Draw and target enemies when possible.

Culture: Cards on your board, in your deck, and in your hand have a 25X% chance to become Radiant.

Healthcare: Units on your board, in your deck, and in your hand gain +7X Health and Armor 2X.

&nbsp;

**(1) KY’s Constant Classic+, Spell, KY, Rare, \#41**

Change a random number on a card in your hand to 3.

\~\~\~

Discover a number number on a card in your hand to 3.

&nbsp;

**(1) KY’s Test Classic+, Spell, KY, Legendary, \#42**

Offer three multiple-choice problems: one Easy, one Medium, and one Hard. Each shows a random reward from its difficulty’s list. Choose one to answer. If you answer correctly, gain its reward.

Easy (simple addition): 3 Coins, a random (2) cost KY card, a random Legendary card that costs (0), or 2 random Books.

Medium (relatively simple double integrals): 2 random (4) cost cards that cost (1), 5 random Books, 5 random KY cards, or fill your hand with Books.

Hard (rigorous proofs, complex Markov chains, statistics, PDEs, linear algebra): Add KY’s Gift to your hand. It costs (0).

\~\~\~

Offer three multiple-choice problems: one Easy, one Medium, and one Hard. Each shows a random reward from its difficulty’s list. Choose one to answer. If you answer correctly, gain its reward.

Easy (simple addition): 3 Coins, a random (2) cost KY card, a random Legendary card that costs (0), or 2 random Books.

Medium (relatively simple double integrals): 2 random (4) cost cards that cost (1), 5 random Books, 5 random KY cards, or fill your hand with Books.

Hard (rigorous proofs, complex Markov chains, statistics, PDEs, linear algebra): Add KY’s Gift to your hand. It costs (0).

Rewards are Radiant.

&nbsp;

**(4) KY’s Gift Classic+, Field Spell, KY, Legendary, Token, \#42.1**

Start of Turn: Gain 1 mana. Your opponent discards a card. Heal your Hero for 5. Add a random Book, KY card, Legendary card, and (4) cost card to your hand. They cost (0).

\~\~\~

Start of Turn: Gain 2 mana. Your opponent discards 2 cards. Heal your Hero for 10. Add a random Radiant Book, KY card, Legendary card, and (4) cost card to your hand. They cost (0).

&nbsp;

**(4) AI Slop Classic+, Spell, Legendary, \#43**

Add 3 AI generated cards to your hand, fuse them, it costs (0)

(you [the ai programming this] create the AI generated cards, make 10 of them with any effect you want, they should be the same as Claude’s Datacenter)

\~\~\~

Add 3 Radiant AI generated cards to your hand, fuse them, it costs (0)

(you [the ai programming this] create the AI generated cards, make 10 of them with any effect you want, they should be the same as Claude’s Datacenter)

&nbsp;

**(2) Simplicity Audit Classic+, Spell, Rare, \#44**

Exile every card in the field with LESS lines of code than this

\~\~\~

Choose if its all on only your opponents; Exile every card in the field with LESS lines of code than this (highlight targets)

&nbsp;

**(2) Complexity Audit Classic+, Spell, Rare, \#45**

Exile every card in the field with MORE lines of code than this

\~\~\~

Choose if its all on only your opponents; Exile every card in the field with MORE lines of code than this (highlight targets)

&nbsp;

**(2) Felinor Flagbearer Classic+, Unit, Felinor, Legendary, \#46**

**4/4**

Rush, Cleave

Cry: Permanently your hero gains +1 Armor

Aura: Your other Felinors have +1/+1

Death: Shuffle Felinor Flagbearer Prime into your deck

\~\~\~

**8/8**

Rush, Cleave

Cry: Permanently your hero gains +2 Armor

Aura: Your Felinors have +2/+2

Death: Shuffle Felinor Flagbearer Prime into your deck

&nbsp;

**(2) Felinor Flagbearer Prime Classic+, Unit, Felinor, Legendary, Token, \#47**

**5/5**

Rush

Cry: Fill your board with copies of this

Aura: Your other Felinors have +1/+1

\~\~\~

**10/10**

Rush

Cry: Fill your board with copies of this

Aura: Your other Felinors have +2/+2

&nbsp;

**(4) Jogg’s Box Classic+, Spell, Legendary, \#11**

Cast 10 random spells.

\~\~\~

Echo 1

Cast 10 random spells.

&nbsp;

**(1) Jlockheed’s Lobbyist Classic+, Unit, Jlockheed, Legendary, \#48**

**0/3**

Can’t be in defense position

Death: Add a random Jlockheed card into your hand, it costs (0)

\~\~\~

**0/6**

Death: Add a random Radiant Jlockheed card into your hand, it costs (0)

&nbsp;

**(2) Jay Fungus Classic+, Unit, Rare, \#49**

**3/6**

Taunt

End of Turn: Reduce a random card in your hands’ costs by (2)

\~\~\~

**6/12**

Taunt

End of Turn: Reduce a random card in your hands’ costs by (20)

&nbsp;

**(1) Adaptive Growth Classic+, Spell, Epic, \#50**

Cast when Drawn: If you have less units than your opponent, give all units -3/-3, if not give all units +2/+2

\~\~\~

Cast when Drawn: If you have less units than your opponent, give all enemy units -4/-4, if not give all friendly units +3/+3

&nbsp;

**(3) Jlockheed’s J15 Fighter Classic+, Unit, Jlockheed, Epic, \#51**

**7/2**

First Strike, Rush

Cant be in a defense position. Can’t be attacked

\~\~\~

**14/4**

First Strike, Rush, Divine Shield

Cant be in a defense position. Can’t be attacked

&nbsp;

**(2) Jlockheed’s Permanent Defense Contract Classic+, Spell, Jlockheed, Epic, \#52**

For the rest of the game, at the start of your turn add a random Jlockheed card to your hand.

\~\~\~

For the rest of the game, at the start of your turn add a random Radiant Jlockheed card to your hand, it costs (1) less.

&nbsp;

**(1) Book of Tokens Classic+, Spell, Book, Epic, \#53**

Summon 2 Rush Tokens

\~\~

Summon 2 Radiant Rush Tokens

&nbsp;

**(1) Book of Books Classic+, Spell, Book, Epic, \#54**

Add 2 Random Books to your hand, they cost (0)

\~\~

Add 2 Random Radiant Books to your hand, they cost (0)

&nbsp;

**(1) Book of Greed Classic+, Spell, Book, Epic, \#55**

Add 3 Random Legendary or Mythic cards to your hand

\~\~

Add 3 Random Radiant Legendary or Mythic cards to your hand

&nbsp;

**(1) Book of Pain Classic+, Spell, Book, Epic, \#56**

Your opponent discards 2 cards

\~\~

Your opponent discards 4 cards

&nbsp;

**(1) Book of Stats Classic+, Spell, Book, Epic, \#57**

Give a unit +5/+5

\~\~

Give a unit +10/+10

&nbsp;

**(1) Fruit Basket Classic+, Spell, Fruit, Rare, \#58**

Add 3 Random Fruits to your hand

\~\~

Add 3 Random Radiant Fruits to your hand

&nbsp;

**(1) All Purpose Apple Classic+, Spell, Fruit, Rare, \#59**

Summon a rush token, heal 2 to your hero, deal 1 damage

\~\~

Summon a Radiant rush token, heal 4 to your hero, deal 2 damage

&nbsp;

**(1) Doctors Orders Classic+, Unit, Spell, Rare, \#60**

Cry and Start of Turn: Add an All Purpose Apple to your hand

\~\~\~

Cry and Start of Turn: Add a Radiant All Purpose Apple to your hand

&nbsp;

**(1) Bauble Bubble Classic+, Field Spell, Fruit, Rare, \#61**

Death: Add 2 (0) cost Stockpile to your hand.

\~\~

Death: Add 2 (0) cost Radiant Stockpile to your hand.

&nbsp;

**(1) KY’s Papaya Classic+, Spell, Fruit, KY, Epic, \#62**

Create a coordinate plane of the board from your hero wherein your lane 1, your backrow corresponds to <0,0> and lane 5 back their backrow corresponds to <4,3>. Create up to a 3rd degree polynomial equation. All cards that would pass through this line get exiled.

\~\~

Create a coordinate plane of the board from your hero wherein your lane 1, your backrow corresponds to <0,0> and lane 5 back their backrow corresponds to <4,3>. Create up to a 3rd degree polynomial equation. All enemy cards that would pass through this line get exiled.

&nbsp;

**(2) Fruit Tree Classic+, Field Spell, Fruit, Rare, \#63**

Start of your turn: Add a random Fruit to your hand it costs (0).

\~\~

Start of your turn: Add a random Fruit to your hand it costs (0).

&nbsp;

**(10) Mulch Muncher Classic+, Unit, Rare, \#64**

**9/9**

Rush, Trample

Costs (1) less per fruit you’ve played this game

\~\~\~

**18/18**

Rush, Trample, Divine Shield

Costs (1) less per fruit you’ve played this game

&nbsp;

**(1) Two Grapes Classic+, Spell, Fruit, Rare, \#65**

Add 3 Graphes to your hand (12% rotten) (60% normal) (20% large) (7% golden) (1% mythic)

\~\~

Add 3 Radiant Graphes to your hand, roles are Lucky 1 (12% rotten) (60% normal) (20% large) (7% golden) (1% mythic)

&nbsp;

**(1) Rotten Grape Classic+, Spell, Fruit, Common, Token, \#65.1**

(Despite being a token can be generated by any Fruit card)

Your hero loses 5 hp

\~\~\~

Your hero loses 1 hp

&nbsp;

**(1) Normal Grape Classic+, Spell, Fruit, Common, Token, \#65.2**

(Despite being a token can be generated by any Fruit card)

Deal 2 damage if played on an enemy, heal 2 hp if played on an ally

Draw 1, the card costs (1) less

\~\~\~

Deal 4 damage if played on an enemy, heal 4 hp if played on an ally

Draw 2, the card costs (1) less

&nbsp;

**(3) Large Grape Classic+, Spell, Fruit, Rare, Token, \#65.3**

(Despite being a token can be generated by any Fruit card)

Deal 5 damage if played on an enemy, heal 5 hp if played on an ally

Draw 1, the card costs (0)

\~\~\~

Deal 10 damage if played on an enemy, heal 10 hp if played on an ally

Draw 2, the card costs (0)

&nbsp;

**(1) Golden Grape Classic+, Spell, Fruit, Legendary, Token, \#65.4**

(Despite being a token can be generated by any Fruit card)

Make a card in your board or hand Radiant

\~\~\~

Make a card in your board or hand Radiant, Cleave (Cleave targets adjacent cards on board OR in hand)

&nbsp;

**(0) Mythic Grape Classic+, Spell, Fruit, Mythic, Token, \#65.5**

(Despite being a token can be generated by any Fruit card)

Replace your hand with Random Mythic cards. They cost (0)

\~\~\~

Replace your hand with Random Radiant Mythic cards. They cost (0)

&nbsp;

**(3) Vine of Grapes Classic+, Spell, Fruit, Rare, \#66**

Add 5 Graphes to your hand (12% rotten) (60% normal) (20% large) (7% golden) (1% mythic)

\~\~

Add 3 Radiant Graphes to your hand, roles are Lucky 1 (12% rotten) (60% normal) (20% large) (7% golden) (1% mythic) (uses graphs as above)

&nbsp;

**(2) Pear Classic+, Spell, Fruit, Rare, \#67**

Summon 2 random (1) cost Common units

\~\~\~

Summon 2 random (1) cost Radiant Common units

&nbsp;

**(4) Organic Produce Classic+, Field Spell, Fruit, Epic, \#68**

Cry: Add a random Fruit to your hand

Aura: Fruit you play is Radiant

\~\~\~

Cry: Add two random Fruit to your hand, they cost (0)

Aura: Fruit you play is Radiant

&nbsp;

**(X) Buff Billy Classic+, Unit, Human, Rare, \#69**

**[3X / 3X]**

Cry: Upgrade this X times

\~\~\~

**[7X / 7X]**

Cry: Upgrade this 2X times

&nbsp;

**(2) Chaos Machine Classic+, Field Spell, Rare, \#70**

Start & End of turn: Upgrade a random card in your hand or you control, and Degrade a random card in your opponents hand or under their control

\~\~\~

Start & End of turn: Upgrade two random cards in your hand or you control, and Degrade two random cards in your opponents hand or under their control.

&nbsp;

**(1) Book of Buff Classic+, Spell, Book, Epic, \#71**

Upgrade a card 5 times (Can target cards in your hand)

\~\~

Upgrade a card 10 times (Can target cards in your hand)

&nbsp;

**(1) Book of Nerf Classic+, Spell, Book, Epic, \#72**

Degrade a card 5 times

\~\~

Degrade a card 10 times (Can target cards in your hand)

&nbsp;

**(4) Call to Chaos (Classic+ Edition) Classic+, Spell, Call to Chaos, Legendary, \#73**

???

(One of the following random effects)

- Add 5 fruit to your hand, they cost (0)
- Add 3 books to your hand, they cost (0)
- Destroy all enemy permanents
- Add 3 Classic cards to your hand, they cost (0)
- Upgrade all cards in your hand & deck twice
- Fuse all cards in your deck with a random card, they maintain their original cost
- Degrade all cards in your opponents board and hand three times
- Summon a Classic Golem
- Replace your deck with random Call to Chaos, they cost (0)
- Cast a random Call to Chaos

\~\~\~

!!!

(three effects)

&nbsp;

**(4) Classic Golem Classic+, Unit, Legendary, Token, \#73.1**

**10/10**

Rush, Firststrike, Trample

After this kills a unit this may attack again.

When this attacks a unit, it transforms into a random Classic or Classic+ card

\~\~\~

**20/20**

Rush, Firststrike, Trample, Divine Shield

After this kills a unit this may attack again.

When this attacks a unit, it transforms into a random Classic or Classic+ card

&nbsp;

**(2) Twice Forward One Step Backwards Classic+, Trap, Mythic, \#74**

Brittle 4

Every (2) cards your opponent plays, fuse it into this card and gain +1 Brittle

\~\~\~

Brittle 10

Every (2) cards your opponent plays, fuse a radiant version into this card and gain +1 Brittle

&nbsp;

**(2) J-lease J-Jungle EX-plorer Classic+, Unit, Legendary, \#75**

**[5/5]**

Cry: Shuffle a J-lease J-Jungle EX-plorer Pack into your deck.

\~\~\~

**[10/10]**

Cry: Shuffle a J-lease J-Jungle EX-plorer Pack into your deck.

&nbsp;

**(2) J-lease J-Jungle EX-plorer Pack Classic+, Spell, Legendary, Token, \#75**

Cast on Draw: Add 5 random Radiant Classic or Classic+ cards to your hand.

\~\~\~

Cast on Draw: Add 5 random Radiant Classic or Classic+ cards to your hand, they cost (0)

&nbsp;

**(1) Brother Lar Classic+, Unit, CN, Human, Rare, \#25**

**1/1**

Death: Summon a Brother Ping token

\~\~\~

**2/2**

Death: Summon a Radiant Brother Ping token

&nbsp;

**(2) Brother Ping Classic+, Unit, CN, Human, Rare, Token, \#25.1**

**4/4**

Pierce

Activate: Deal 1 damage

\~\~\~

**8/8**

Pierce

Activate 2: Deal 1 damage

&nbsp;

**(2) Anti-Softlock Classic+, Spell, Rare, \#28**

Draw 1. All cards on both players' boards, hands, and decks gain Stack and Pierce. Unlock all slots.

\~\~\~

Draw 2. Choose one: all cards on both players' boards, hands, and decks, or only yours, gain Stack and Pierce. Unlock all slots.

&nbsp;

**(2) Claude’s Datacenter Classic+, Field Spell, Legendary, \#42**

End of turn add a random AI generated card to your hand, it costs (0)

(you [the ai programming this] create the AI generated cards, make 10 of them with any effect you want)

\~\~\~

End of turn add a random Radiant AI generated card to your hand, it costs (0)

(you [the ai programming this] create the AI generated cards, make 10 of them with any effect you want)
