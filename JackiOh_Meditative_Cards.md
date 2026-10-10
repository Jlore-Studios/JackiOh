The designer's card list for the Meditative set and the visuals pass that ships with it, as written on 2026-10-08 in issue #496, verbatim: typos, the numbering (two #28s, two #93s, two #19-family entries, no #62, two unnumbered stubs) and the formatting kept as the designer wrote them. `docs/meditative-set.md` is the implementer's reading of it, card by card, and SPEC.md (§6, §7 and §8.8) and `crates/cards/catalog.json` are the implemented reading, not this file. A `~~~` line separates a base face from its Radiant face. The text sits in a fenced block so that Markdown does not fold its single line breaks.

```text
# New Card Sets

## Meditative

### Key words
- [ ] Degrade gets renamed to **Nerf**
- [ ] Upgrade gets renamed to **Buff**

### Cosmetics
- [ ] Hero portraits can be clicked on
- [ ] Hero portraits have more vivid art
- [ ] There are more emotes, but emotes are randomized each game

### Sound effects
- [ ] All legendary & mythic cards have an intro music snippet [similar to Hearthstone] when played
- [ ] More sound effects for nicher interactions, such as:
  - [ ] When 50% (or more) of a damage is negated by armor
  - [ ] When all damage is negated by armor
  - [ ] Any other sound effects or cases you can find

### AI
- [ ] Update AI such that it can beat the previous AI 40 out of 100 games with the new cards
  - [ ] At least 80% of the new cards need to be in the pool of allowed cards

### Randomized Decks
- [ ] Ability to pick a skewed option in randomized decks (against AI + randomized duels) where there are more cards of the new set (At least 50% of cards in your deck)

### Misc
- [ ] Update almanac with new cards
- [ ] Any other small polish you find

-----

(2) Disruptive Disruptor Meditative, Field Spell, Common, #1
You cannot be forced to discard cards on your opponents turn
~~~ 
You cannot be forced to discard cards on your opponents turn
Cry: Draw 1 and gain 2 mana

(2) Rampaging Rhino Meditative, Unit, Common, #2
5/9
When this takes an instance of damage damage, discard a card
~~~
11/20
When this takes an instance of damage damage, discard a card

(3) Jlockwork Machine Meditative, Unit, Rare, #3
10/10
When your opponent plays a card exile the top 3 card of your library
~~~
20/20
Rush
When your opponent plays a card exile the top 3 card of your library

(4) Juicy Kumquat Melon Meditative, Epic, Spell, #4
Draw a (0), (1), (2), (3), (4), and (5) cost Meditative card 
~~~
Draw a (0), (1), (2), (3), (4), and (5) cost Meditative card 
They cost (2) less

(1) Death by 1000 cuts Meditative, Spell, Rare, #5
Deal 1 damage to a unit, echo (2 * [current max mana]{ie turn 1=1, turn 4=4, turn 4< = 4} 
~~~
Deal 1 damage to a unit, echo (4 * [current max mana]{ie turn 1=1, turn 4=4, turn 4< = 4} 
Lifesteal

(0) Me no Likey Meditative, Epic, Spell, #6
Give a unit you control to an enemy; Gain mana equal to its cost 
~~~
Give a unit you control to an enemy; Gain mana equal to twice its cost

(4) Introspection Meditative, Field Spell, Legendary, #7
End of turn: Heal all allies 3, draw 1, reduce the cost of cards in your hand by (1), deal 2 damage to all enemies 
~~~
End of turn: Heal all allies 8, draw 1, reduce the cost of cards in your hand by (2), deal 4 damage to all enemies

(0) Reach the Summit Meditative, Quickdraw, Wincon, Mythic, #8
Current ascent level: 0
End of turn: Return this to your hand with cost equal to its ascent level
Gain the effects of your ascent level:
Ascent 0: heal 2 to your hero
Ascent 1: deal 2 damage
Ascent 2: draw 2 cards
Ascent 3: your opponent discards 2 cards randomly 
Ascent 4: exile two random enemy Permian’s 
Ascent 5: add two random Radiant cards to your hand, they cost (0)
Ascent 6: reach for the stars 
Ascent 7: and when they’re near…
Ascent 8: push even harder
Ascent 9: and you might just
Ascent 10: win the game
~~~
Current ascent level: 0
End of turn: Return this to your hand with cost equal to its [ascent level - 1]
Gain the effects of your ascent level:
Ascent 0: heal 3 to your hero
Ascent 1: deal 3 damage
Ascent 2: draw 3 cards
Ascent 3: your opponent discards 3 cards randomly 
Ascent 4: exile three random enemy Permian’s 
Ascent 5: add three random Radiant cards to your hand, they cost (0)
Ascent 6: reach for the stars 
Ascent 7: and when they’re near…
Ascent 8: push even harder
Ascent 9: and you might just
Ascent 10: win the game

(2) Joint Filing Meditative, Epic, Field Spell, #9
Your start and end of turn effect trigger an additional time
~~~ 
Your start and end of turn effects trigger two additional times

(2) Double Counting Meditative, Epic, Field Spell, #10
Your cry and death effects trigger an additional time
~~~ 
Your cry and death effects trigger two additional times

(4) Double Header Meditative, Field Spell, Legendary, #11
The first card you play each turn adds a (0) cost copy non-Radiant of it to your hand
~~~
The first card you play each turn adds a (0) cost Radiant copy of it to your hand

(2) Fear Mongerer Meditative, Epic, Human, Unit, #12
6/8
Cry: trigger your end of turn effects
~~~ 
6/8
Cry: trigger your end of turn effects twice

(2) Gatling Pea Meditative, Epic, Unit, #13
2/6
Armor 2
End of turn: deal 1 damage to your opponent and permanently increase this damage by 1 
~~~ 
4/12
Armor 4
End of turn: deal 2 damage to your opponent and permanently increase this damage by 2

(2) Prime Time Meditative, Epic, Spell, #14
Draw 3 prime indexed cards from your deck
~~~
Draw all prime indexed cards from your deck

(2) Smelly Steven Meditative, Unit, Human, Common, #15
5/5
Cry: your opponents spells cost (1) more next turn
~~~
10/10
Cry: your opponents spells cost (2) more next turn

(2) Trenful Trickster Meditative, Unit, Rare, #16
1/5
End of turn: Summon a random Trap
~~~
2/10
End of turn: Summon a random Traps

(0) True Craft a Card Meditative, Spell, Mythic, #17
Craft a card
(Physically pull up a scratch esque gui with resources allocation and lines of code, should be representative of how the codebase actually is and let them create a custom card)
~~~
Craft a Radiant card 
(Radiant here is arbitrary, it’s more so it will be about %100-150 stronger on average and have the Radiant tag)

(1) Expedition12 Meditative, QuickDraw, Epic, Spell, #18
Lose all mana next turn. Choose a card in your hand to become Radiant
~~~
Lose all mana next turn. Choose a card in your hand become Radiant. Echo

(1) Expedition1234 Meditative, QuickDraw, Epic, Spell, #19
Lose all mana next three turns. Add a Temporal Rift to your hand
~~~
Lose all mana next three turns. Add a Radiant Temporal Rift to your hand

(2) Temporal Rift Meditative, Token, Epic, Spell, #19.1
Gain an Extra Turn 

(Players can only gain at most one Extra Turn from the temporal rift card)
~~~
Gain an Extra Tura, 2 mana, 2 mana next turn, and draw 2

(Players can only gain at most one Extra Turn from the temporal rift card)

(1) Aestheticize the Game Meditative, Quickdraw, Spell, Wincon, Mythic, #20
Choose an alternative Wincon:
Have 100 player health
Have 100 cards in your GY
Have 100 attack and 100 health combined on your board
~~~
Choose an alternative Wincon:
Have 90 player health
Have 90 cards in your GY
Have 90 attack and 90 health combined on your board

(0 embiggen 1) API Key Fishing Meditative, Spell, Rare, #21
You have a 2% embiggen 5% chance to steal your opponents hand.
End of turn: Add a API Key Fishing to your hand
~~~
You have a 5% embiggen 12% chance to steal your opponents hand.
End of turn: Add a API Key Fishing to your hand

(2) Mind Games Meditative, Epic, Spell, #22
Choose one secretly: Greed, Attack, Defend
Gain a reward on your next turn depending on what you played:
Greed: gain 2 mana and draw 2
Attack: deal 8 damage to all enemies
Defend: all allies heal 8 and gain 2 armor
Add a Fortify Mind to your opponents hand
~~~
Choose one secretly: Greed, Attack, Defend
Gain a reward on your next turn depending on what you played:
Greed: gain 2 mana and draw 2
Attack: deal 8 damage to all enemies
Defend: all allies heal 8 and gain 2 armor
Add a Fortify Mind to your opponents hand, it costs (2)

(Players can only gain at most one Extra Turn from the temporal rift card)

(0) Fortify Mind Meditative, Token, Epic, Spell, #22.1
Temporary (discards at the end of your turn)
Choose one: Greed, Attack, Defend
If you successfully predict your opponent e.g., Attack when they Greed, Greed when they Defend, Defend when they Attack; counter their gain
If you pick the same thing as your opponent, do nothing.
If you failed to predict your opponent e.g, Greed when they Attack, Defend when they Greed, Attack when they Defend; one of the negative effects happens depending on what you picked
Greed: lose 2 mana next turn and discard 2 random cards now
Attack: all allies take 8 damage
Defend: all enemies heal 8 and gain 2 armor
If you discard this trigger all of the negative effects happen
~~~
Temporary (discards at the end of your turn)
Choose one: Greed, Attack, Defend
If you successfully predict your opponent e.g., Attack when they Greed, Greed when they Defend, Defend when they Attack; counter their gain
If you pick the same thing as your opponent, do nothing.
If you failed to predict your opponent e.g, Greed when they Attack, Defend when they Greed, Attack when they Defend; one of the negative effects happens depending on what you picked
Greed: lose 2 mana next turn and discard 2 random cards now
Attack: all allies take 8 damage
Defend: all enemies heal 8 and gain 2 armor

(1) Golly Bob Howdy, Unit, Common, Human, Meditative, #23
3/5
Cry: Transform adjacent Units into Sheep Tokens
~~~
6/10
Cry: Transform adjacent Units into Radiant Sheep Tokens

(3) Polymorph, Spell, Common, Meditative, #24
Transform a Unit into a Sheep Token
~~~
Transform a Unit and adjacent ones into Sheep Tokens

(1) Blue-Eyes White Felinor, Unit, Rare, Felinor, Meditative, #25
12/9
Tribute 2 \n Immune to tribal tag based hate.
~~~
24/18
Tribute 2 \n Immutable \n Immune to tribal tag based hate.

(4) Alternate Fate, Field Spell, Legendary, Meditative, #26
Aura: Players don’t generate mana naturally.
End of Turn: Summon a Mana Well in ALL open backrow.
~~~
Aura: Players don’t generate mana naturally.
End of Turn: Summon a Mana Well in ALL open backrow. Make yours Radiant.

(2) Clip-Farming Lawyer, Unit, Human, Common, Meditative, #27
6/5
End of Turn: Unlock a random zone. If you do, add 1 Coin to your hand.
~~~
12/10
End of Turn: Unlock a random zone. If you do, add 2 Coins to your hand. If there are no locked zones, lock a random enemy one.

(2) Showdown, Spell, Meditative, #28
Choose a lane. Lock all other lanes until the start of your next turn.
~~~
Choose a lane. Lock all other lanes until the start of your next turn. Cast Book of Buff on any cards you place in it this turn.

(2) Clip-Farming Lawyer, Unit, Human, Common, Meditative, #27
6/5
End of Turn: Unlock a random zone. If you do, add 1 Coin to your hand.
~~~
12/10
End of Turn: Unlock a random zone. If you do, add 2 Coins to your hand. If there are no locked zones, lock a random enemy one.

(2) Shade-iris, Unit, Common, Meditative, #28
7/4
Cry: Shuffle 1 Ancient Curse into the enemy Deck.
~~~
14/8
Cry: Shuffle 2 Ancient Curse into the enemy Deck.

(2) Ancient Curse, Spell, Token, Common, Meditative, #28.1
Cast on Draw: Take 7 damage.
~~~
Pierce
Cast on Draw: Take 7 damage.

(3) Forbiddenous Factory, Field Spell, Plague, Rare, Meditative, #29
Cry: Place 3 Plague Counters on this.
End of Turn: Spend a Plague Counter to shuffle 1 Ancient Curse into the enemy Deck.
~~~
Cry: Place 3 Plague Counters on this.
End of Turn: Spend a Plague Counter to shuffle 2 Ancient Curse into the enemy Deck.

(1) Fickle E-Kitten, Unit, Epic, Felinor, Meditative, #30
3/4
Start of Turn: If your opponent has a more expensive permanent than you, they gain control of this. Otherwise, shuffle a Love Bomb into your deck.
~~~
6/8
Start of Turn: If your opponent has a more expensive permanent than you and a larger Deck, they gain control of this. Otherwise, shuffle a Radiant Love Bomb into your deck.

(1) Love Bomb, Spell, Token, Epic, Meditative, #30.1
Cast on Draw: Heal your Hero 7.
~~~
Cast on Draw: Heal your Hero 14.

(3) The Conductor, Unit, Human, Rare, Meditative, #31
7/9
Deft
Cry: Draw all Created cards from your Deck.
~~~
14/18
Deft
Cry: Shuffle a Prime card into your Deck then draw all Created cards from your Deck.

REMARK ALL THE FOLLOWING CARDS SHOULD NOT SPECIFY THAT THE TEXT IS CONVERTED TO CHINESE IT SHOULD JUST DO IT

(2) Spiritually 中国, Spell, CN, Epic, Meditative, #32
Randomly do one of three things:
Get ready to learn Chinese (Translate all cards in your and your opponents hand and deck into Chinese)
Convert all cards into your hand into the same CN card (they cost 0)
Summon 5 random CN units
~~~
Randomly do one of three things:
Get ready to learn Chinese (Translate all cards in your opponents hand and deck into Chinese)
Convert all cards into your hand into the same Radiant CN card (they cost 0)
Summon 5 random Radiant CN units
(For the AI building this card: This cards text is in Chinese)

(0) First Day of 学校, Spell, CN, Rare, Meditative, #33
Add two random CN cards to your hand; they cost (1) less
(For the AI building this card: The cards added to your hand should have their text be in Chinese)
~~~
Add two random Radiant CN cards to your hand; they cost (1) less
(For the AI building this card: The cards added to your hand should have their text be in Chinese)

(4) 高考, Spell, CN, KY, Rare, Meditative, #34
Destroy all non-CN or KY permanents. Buff all CN & KY permanents twice.  
~~~
Destroy all enemy non-CN or KY permanents. Buff all friendly CN & KY permanents thrice.  

(1) RCTA (CN), Spell, CN, Common, Meditative, #35
Give a non-CN permanent the CN tag. Convert its text to Chinese. Give it +2/+2 and buff it.
~~~
Give a non-CN permanent the CN tag. Convert its text to Chinese. Give it +5/+5 and buff it twice.

(1) CN Peptides, Spell, CN, Common, Meditative, #36
Choose a unit; it has a 50% chance to be buffed 7 times and a 50% chance to die.
~~~
Choose a unit; it has a LUCKY 1 50% chance to be buffed 10 times and a 50% chance to die. 
(Luck depending on if its an ally or enemy)

(X) CN in a bottle, Spell, CN, Epic, Meditative, #37
When this enters your hand: Replaced with a random Radiant card. Convert its text to Chinese.
~~~
When this enters your hand: Replaced with a random Radiant card; it costs(0). Convert its text to Chinese.

(2) H1B Printer, Field Spell, CN, KY, Rare, Meditative, #38
Start of turn: Add a random Radiant KY or CN card to your hand
~~~
Start of turn: Add a random Radiant KY or CN card to your hand; it costs (0)

(2) 赌石 Addict, Unit, CN, Rare, Meditative, #39
[4/4]
Cry: Add an Auspicious Rock to your hand
~~~
[8/8]
Cry: Add a Radiant Auspicious Rock to your hand

(0) Auspicious Rock, Spell, CN, Rare, Token, Meditative, #39.1
When played lose 2 health and randomly gain:
(20%) Dud
(70%) Jade
(10%) Red Jade
~~~
When played lose 2 health and Lucky 2 randomly gain:
(20%) Dud
(70%) Jade
(10%) Red Jade

(0) Jade, Spell, CN, Rare, Token, Meditative, #39.2
Add 1 to your Jade Counter
Gain 1 mana
~~~
Add 2 to your Jade Counter
Gain 2 mana

(0) Dud, Spell, CN, Common, Token, Meditative, #39.3
Deal 2 damage
~~~
Deal 5 damage

(0) Red Jade, Spell, CN, Mythic, Token, Meditative, #39.4
Add 5 to your Jade Counter
~~~
Add 10 to your Jade Counter

(10) Jade Beauty, Unit, CN, Mythic, Token, Meditative #39.5
(Summons when your Jade Counter reaches 5)
[20/20]
Can’t Attack; Indestructable; Immutable 
End of turn: Allure all enemy units to join your side, at the start of your next turn they join your side (or die of heartbreak if there is not space)
~~~
(Ascends to radiant when your Jade Counter reaches 10)
[40/40]
Indestructable; Immutable
End of turn: Allure all enemy units to join your side, at the start of your next turn they join your side (or die of heartbreak if there is not space)

(2) Feng Shui, Field Spell, CN, Legendary, #40
All cards gain the tag 水，木，火，金，土
Auspicious behavior will be rewarded and inauspicious behavior will be punished
Last played tag opponent:
Last played tag you:
(hidden text: if a card reacts positively with your last played card it becomes Radiant; if it reacts negatively you take 10 damage and it gains Brittle 2)
Aura: gain Luck 1
~~~
All cards gain the tag 水，木，火，金，土
Auspicious behavior will be rewarded for you; and inauspicious behavior will be punished heavily for your opponent
Last played tag opponent:
Last played tag you:
(hidden text: if a card reacts positively with your last played card it becomes Radiant; if it reacts negatively you take 10 damage and it gains Brittle 2)
Aura: gain Luck 1

(2) CN Smuggler, Unit, CN, Rare, Meditative, #41
[4/5]
Start of Turn: Add an Auspicious Rock to your hand and a random CN card
~~~
[8/10]
Start of Turn: Add a Radiant Auspicious Rock to your hand and a random Radiant CN card

(0) CN Flea Market, Spell, CN, Legendary, Meditative, #42
Open up a night market; you gain 50 yuan to buy cards
(For AI offer primarily CN cards Auspicious Rocks and other AI generated cards that are related to CN)
~~~
Open up a night market; you gain 80 yuan to buy cards; you may barter
(For AI offer primarily CN cards Auspicious Rocks and other AI generated cards that are related to CN; Barter system is up for your interpretation)

(X) CN Jade Market, Spell, CN, Rare, Meditative, #43
Add X Auspicious Rocks to your hand
~~~
Add X Radiant Auspicious Rocks to your hand

(3) CN Jade Well, Spell, CN, Rare, Meditative, #44
Start of turn: Add a Auspicious Rocks to your hand
~~~
Start of turn: Add a Radiant Auspicious Rocks to your hand

(1) Knowledge Breaker, Unit, CN, KY, Catalyst, Legendary, Meditative, #45
[1/3]
Cry: Degrade all other CN & KY cards
Aura: your units may be played as field Animated Field Traps that activate at the Start of your turn
Death: Shuffle Knowledge Breaker Prime into your Deck
~~~
[2/6]
Cry: Degrade all other CN & KY cards twice
Divine Shield
Aura: your units may be played as field Animated Field Traps that activate at the Start of your turn
Death: Shuffle Radiant Knowledge Breaker Prime into your Deck

(4) Knowledge Breaker Prime, Unit, CN, KY, Prime, Legendary, Meditative, #45.1
[6/18]
Rush
Cry: Summon 5 Random Traps; Destroy all other CN & KY cards
Aura: your units may be played as field Animated Traps that activate at the Start of your turn
~~~
[12/36]
Rush, Poisonous 
Cry: Summon 5 Random Radiant Traps; Exile all other CN & KY cards
Aura: your units may be played as field Animated Traps that activate at the Start of your turn

(2 embiggen 4) Conjure Intellect, Spell, Epic, Meditative, #46
Add a random a KY, CN, and Book to your hand; embiggen they cost (0)
~~~
Add a random a Radiant KY, Radiant CN, and Radiant Book to your hand; embiggen they cost (0)

(4) 饕餮, Unit, CN, Legendary Meditative, #47
[8/8]
End of turn: Fuse a random enemy permanent into this
~~~
[16/16]
End of turn: Fuse a random enemy permanent into this; and one from their deck

(2) Tranquility, Spell, Epic, Meditative, #48
Immediately end your turn; your hero is immune to damage until the start of your turn turn
~~~
Your hero is immune to damage until the start of your turn turn

(2) YileGPT Tamed, Unit, CN, Meditative, Mythic, Acclaimed, #49
[2/6]
Cry: summon a Virus in the opposite lane of this 
Death: place YileGPT Unleashed at the bottom of your deck
~~~
[4/12]
Cry: summon a Radiant Virus in the opposite lane of this 
Death: place Radiant YileGPT Unleashed at the bottom of your deck

(10) YileGPT Unleased, Unit, CN, Token, Meditative, Mythic, Acclaimed, #49.1
(Excess mana you end your turn with decrease the cost of this card)
[8/20]
Cry: Randomly picks 2 out of 5
Summon 2 Claude’s Datacenter
Add 3 Radiant Glitch in the systems to your hand, they cost (0)
Summon an AI Girlfriend
Fill your opponents board with Virus 
Degrade all cards in your opponents, board, hand, and deck twice.
Can’t Attack
Start of turn: make a random friendly permanent radiant
~~~
(Excess mana you end your turn with decrease the cost of this card)
[16/40]
Cry: Do the following:
Summon 2 Claude’s Datacenter
Add 3 Radiant Glitch in the systems to your hand, they cost (0)
Summon an AI Girlfriend
Give your opponent two Viruses 
Degrade all cards in your opponents, board, hand, and deck twice.
Can’t Attack; Armor 5
Start of turn: make two random friendly permanent radiant

(2) Yile’s Virus, Unit, CN, Token, Meditative, Mythic, Acclaimed, #49.2
[0/8]
Can’t attack; Can’t be in defense position
Activate: Destroy this
Death: Discard a random card
Start of turn: Deal 2 damage to adjacent non Virus units & to your hero
~~~
[0/8]
Can’t attack; Can’t be in defense position
Activate: Destroy this
Death: Discard 2 random cards
Start of turn: Deal 4 damage to adjacent non Virus units & to your hero

(4) AI Girlfriend, Field Spell, Token, CN, Mythic, Meditative, Acclaimed, #49.3
[2/30]
Can’t attack
Start of turn: all enemy units attack this unit
After this is attacked: degrade the attacking unit
~~~
[4/60]
Can’t attack
Start of turn: all enemy units attack this unit
After this is attacked: degrade the attacking unit twice

(0) CN Tech, Spell, CN, Meditative, Rare, #50
Add the highest win rate card to your hand
~~~
Add the highest win rate Radiant card to your hand

(2) Devin Bot, Unit, Legendary, Meditative, #51
1/1
Reborn
Cry: Fill your board with random 1/1s. {for AI: there are cards with a 1/1 stat line, including Tokens}
~~~
2/2
Reborn
Cry: Fill your board with random Radiant 1/1s. {for AI: there are cards with a 1/1 stat line, including Tokens}

(1) Economic Anxiety, Field Spell, Meditative, Rare, #52
Units gain +3 Attack when attacking a Unit that doesn’t share one of their tags. {for AI: highlight valid targets in yellow when dragging for the attack}
~~~
Your Units gain +3 Attack and Poisonous when attacking a Unit that doesn’t share one of their tags. {for AI: highlight valid targets in yellow when dragging for the attack}

(1) Prestige, Spell, Meditative, Common, #53
De-Radiant a card. Add an AI-Generated card to your hand. {for AI: if no valid targets, can be played for just the AI-Generated card}
~~~
De-Radiant all enemy cards. Add a Radiant AI-Generated card to your hand. {for AI: if no valid targets, can be played for just the AI-Generated card}

(1) Money Machine, Field Spell, Meditative, Common, #54
End of Turn: Shuffle a Coin into each player’s deck.
Activate: Spend 2 mana to Tribute this. Either player may Activate this. {for AI: your “Coin” has a sneaky 10% chance to be Jade and a 1% chance to be Red Jade}
~~~
End of Turn: Shuffle a Coin into your opponent’s deck and a Radiant Coin into yours.
Activate: Spend 2 mana to Tribute this. Either player may Activate this. {for AI: your Radiant  “Coin” has a sneaky 10% chance to be Radiant Jade and a 1% chance to be Radiant Red Jade}

(2) Dragon Fruit, CN, Fruit, Spell, Meditative, #55
Add a random Prime card to your hand
~~~
Add a random Radiant Prime card to your hand

(3) House Party, Spell, Meditative, #56
Fill your board with Right-House Defenders
~~~
Fill your board with Radiant Right-House Defenders

(1) Clip-Farming Critikal, Unit, Common, Meditative, #57
[5/1]
Combo: add the previous card you played back to your hand
~~~
[10/2]
Combo: add the previous card you played back to your hand, make it Radiant

(2) Permanent Underclassman, Unit, Human, Common, Meditative, #58
3/3
Cry: Summon a Random (1) cost Unit with “Death: Add a Book to your hand.”
~~~
6/6
Cry: Summon a Random (1) cost Radiant Unit with Death: Add a Radiant Book to your hand.

(3) Permanent Upperclassman, Unit, Human, Rare, Meditative, #59
3/3
Cry: Summon a Random (4) cost Unit with “Death: Add a Book Fused with an AI-Generated card to your hand.”
~~~
3/3
Cry: Summon a Random (4) cost Radiant Unit with “Death: Add a Radiant Book Fused with a Radiant AI-Generated card to your hand.”

(3) Eschews, Field Spell, Epic, Meditative, #60
End of Turn: Cast a random Human, Book, CN, or AI-Generated card.
Tribute this after 5 ally Units have died.
~~~
End of Turn: Cast a random Radiant Human, Book, CN, or AI-Generated card.
Tribute this after 50 ally Units have died.

(3) Joon Jorker, Unit, Human, Common, Meditative, #61
7/5
End of Turn: Buff the Unit to the right 5 times.
~~~
14/10
End of Turn: Buff the Unit to the right 10 times.

(3) Skull of J’Nari, Field Spell, Legendary, Meditative, #63
Start of Turn: Summon a random Unit from your hand.
~~~
Start of Turn: Summon a random Unit from your hand. Make it Radiant.
{for AI: multiple activation voicelines: “This one will prove very useful! ...Probably.”, “there IS a method to my madness!”, “Good units are hard to find.”}

(1) Traitorous Blood, Trap, Common, Meditative, #64
Reveals when an opposing Unit attacks and has a neighbor: The attack is redirected to the neighbor.
~~~
Reveals when an opposing Unit attacks and has a neighbor: The attack is redirected to the neighbor. Summon a copy of any of the Units destroyed.

(4) Keymaster Keenus, Unit, Legendary, Meditative, #65
1/1
Has every Keyword.
~~~
2/2
Has every Keyword.
Death: Give them all to another one of your Units.

(2) Fiery Waraxe, Field Spell, Common, Meditative, #66
3/2
Animated on your turn
~~~
6/4
Animated on your turn
Pierce

(1) Sentient Cat Ears, Unit, Felinor, Epic, Meditative, #67
1/1
Magnetic {For AI: can optionally Stack on to one of your Units. If it does, Fuse with it}
End of Turn: Shuffle a Love Bomb into your Deck.
~~~
2/2
Magnetic 
End of Turn: Shuffle 2 Love Bombs into your Deck.

(1) Catnip, Field Trap, Felinor, Rare, Meditative, #68
5/5
Animated
Reveal after an enemy casts a Spell.
~~~
10/10
Animated
Taunt
Reveal after an enemy casts a Spell.

(4) The Maestro, Unit, Human, Rare, Meditative, #69
4/4
For every mana you spend, put a Plague Counter on this.
Activate: Spend 4 Plague Tokens. Exile a random card from each of the enemy Field, Hand, Deck, and GY.

(1) I’M WILL BE YOUR DOOM, Unit, Common, Meditative, #70
5/8
Cry: Summon a “Ready… I’m” token for your enemy.
~~~
10/16
Cry: Summon a “Ready… I’m” token for your enemy and a Radiant “Ready… I’m” token for yourself.
{for AI: voice line is “I’M WILL BE YOUR DOOM”}

(1) Ready… I’m, Unit, Common, Meditative, #70.1
2/1
Poisonous
~~~
4/2
Poisonous
Divine Shield
{for AI: voice line is “Ready… I’m”}

(2) Pareto Optimality, Field Spell, Mythic, Meditative, #71
Cry: Summon The Cane.
Whenever the enemy plays a card that isn’t the AI optimal move, The Cane attacks a random enemy.
~~~
Cry: Summon Radiant The Cane.
Whenever the enemy emotes or plays a card that isn’t the AI optimal move, The Cane attacks a random enemy.

(2) The Cane, Field Spell, CN, Mythic, Meditative, #71.1
3/1
Animated on your turn and when Pareto Optimal
Indestructible
Pierce
~~~
6/2
Animated on your turn and when Pareto Optimal
Indestructible
Pierce
Trample

(1) The Banisher, Unit, Common, Meditative, #72
2/1
When this damages a Unit, exile that Unit.
~~~
4/2
Cleave
When this damages a Unit, exile that Unit.

(2) Plate Packer, Unit, Human, Common, Meditative, #73
6/6
Cry: Gain +1/+1 for each Armor on the Field, including Heroes.
~~~
12/12
Cry: Gain +2/+2 for each Armor on the Field, including Heroes.
{for AI: voice line is “305 is the new 225”}

(8) Montaña Giant, Unit, Epic, Meditative, #74
8/8
Costs (1) less for each card in your Hand.
End of Turn: Radiant this if your Hand is full at the end of your turn.
~~~
16/16
Costs (1) less for each card in your Hand.
End of Turn: +8/+8 this if your Hand is full at the end of your turn.

(4) Ever Growing Tree, Field Spell, Legendary, Meditative, #75
Activate: Fuse a random Field Spell into this.
~~~
Activate: Fuse a random Radiant Field Spell into this.

(1) Do or Die, Spell, Common, Meditative, #76
Mark 2 random cards in the enemy hand. If they aren’t played, steal them at the start or your turn.
~~~
Mark every card in the enemy hand. If they aren’t played, steal them at the start or your turn.

(3) Bulk Booster, Spell, Common, Meditative, #77
Fill your hand with random Common cards.
~~~
Fill your hand with random Radiant Common cards.

(4) Occidentless Mandate, CN, Spell, Meditative, Epic, #78
Exile all non-CN permanents; add a random CN card for each friendly permanent you exiled.
~~~
Exile all non-CN permanents; add a random Radiant CN card for each friendly permanent you exiled, they cost (2) less.

(3) Touched by KY, KY, Unit, Meditative, Legendary, #79
Cry: Draw 4 cards, for the rest of the game your handsize is 12
[4/4]
~~~
Cry: Draw 4 cards, make them Radiant, for the rest of the game your handsize is 12
[8/8]
{for ai implementing: voiceline should be “We must expand our minds”}

(3) Aluneth, Field Spell, Quickdraw, Legendary, Meditative #80
Indestructible, Untributable, Immutable
End of turn: Draw 3
~~~
Indestructible, Immutable
Activate: Exile this
End of turn: Draw 3
{for ai implementing: voiceline should be “Come, child. Let us wreak havoc.”}

(0 embiggen 2) Deadman’s Hand, Spell, Rare, Meditative, #81
Shuffle a copy of your hand {including this} into your deck, embiggen cards have a 25% chance of becoming Radiant
~~~
Shuffle a copy of your hand {including this} into your deck, embiggen cards have a 100% chance of becoming Radiant
Draw 1

(1) Medina Outfitter, Unit, Common, Meditative, #82
[1/1]
Cry: Buff every card in your hand
~~~ 
[2/2]
Cry: Buff every card in your hand, thrice

(2) Medina Enforcer, Unit, Rare, Meditative, #83
4/4
End of Turn: Buff every card in your hand
~~~
4/4
End of Turn: Buff every card in your hand, thrice

(1) Volatility, Unit, Common, Meditative, #84
3/3
Buffs & Nerfs are twice as effective on this.
~~~
6/6
Buffs are three times as effective on this.

(2) Playtester, Unit, Rare, Meditative, #85
[4/5]
Start of turn: Add either a Book of Buff or a Book of Nerf to your hand
~~~
[8/10]
Start of turn: Add either a Radiant Book of Buff or a Radiant Book of Nerf to your hand

(2) Mayor Medinamogger, Unit, Legendary, Meditative, #86
5/4
All targets are random.
~~~
10/8
Lucky 1
All targets are random.

(0) Small Time Recruits
Something with 1 costs

(1) Tatches the Totem, Unit, All Tribes, Legendary, Meditative, #87
0/3
Summon this from your Deck if you play a card with a Tribal tag.
End of Turn: Buff a random card in your Hand or Deck.
~~~
0/6
Summon this from your Deck if you play a card with a Tribal tag.
End of Turn: Buff a random card in your Hand or Deck and adjacent Units.

(4) The True Sheep, Unit, Epic, Meditative #88
[1/1]
Cry: Discover a tribute card to replace this with
Worth 5 tributes
~~~
[2/2]
Cry:  Discover a Radiant tribute card to replace this with
Worth 500 tributes


(2) Jlarna, Field Spell, Rare, Meditative, #89
Combo 2: This costs (2) less.
Aura: You can spend mana from next turn. {for AI: lock your next turn’s mana crystals as this happens}
If either player doesn’t play a card on a turn, Tribute this.
~~~
Combo 2: This costs (2) less.
Aura: You can spend mana from next turn.

Note (kept with the designer's text above, verbatim): the designer then made four later changes,
all in the build: the brief's "pay in 4" stays as the Aura's four instalments; the Combo line is
gone and the cost is (3); a missed instalment is forgiven; and the Tribute condition is "If you
don't use your credit line", checked on the turn Jlarna is played too.

(1) Spell Basket, Spell, Common, Meditative, #90
Add 3 random spells to your hand
~~~
Add 3 random Radiant spells to your hand

(1) Windfast, Catalyst, Unit, Epic, Meditative, #91
1/1
Tribute 1
Windfury
Instead of attacking itself, this summons a Unit from your Hand to do the attack, then Bounces it if it survives.
Death: Shuffle Windfurious Prime into your Deck.
~~~
2/2
Tribute 1
Windfury
Instead of attacking itself, this summons a Unit from your Hand to do the attack.
Death: Shuffle Radiant Windfurious Prime into your Deck.

(3) Windfurious Prime, Unit, Prime, Epic, Medidative, #91.1
5/10
Rush
Windfury
In addition to attacking itself, this summons 2 Units from your Hand or Deck to join the attack as well. {for AI: these units attack first}
~~~
10/20
Rush
Windfury
First Strike
In addition to attacking itself, this summons 2 Units from your Hand or Deck to join the attack as well.

(3) Unan, Unit, Rare, Meditative, #92
[3/13]
Armor 3
Fatal damage a friendly ally would take is redirected to this
~~~
[6/26]
Armor 7
Fatal damage a friendly ally would take is redirected to this

(2) Paranoia, Field Spell, Epic, Meditative, #93
Your Spells may be played as Traps that activate at the End of your turn, Start of your next turn, or End of your next turn
Draw 1
~~~
Your Spells may be played as Traps that activate at the End of your turn, Start of your next turn, or End of your next turn
They also gain Echo +1
Draw 1

(1) Growing Felinor, Unit, Felinor, Common, Meditative, #93
[1/1]
Cannot be in defense position
Death: Summon a Growing Felinor Sr
~~~
[1/1]
Divine Shield, Rush
Cannot be in defense position
Death: Summon a Radiant Growing Felinor Sr

(1) Growing Felinor Sr, Unit, Felinor, Common, Meditative, #93.1
[2/2]
Cannot be in defense position
Death: Summon a Growing Felinor Sr
~~~
[2/2]
Divine Shield, Rush
Cannot be in defense position
Death: Summon a Radiant Growing Felinor Sr Sr

(1) Growing Felinor Sr Sr, Unit, Felinor, Common, Meditative, #93.2
[3/3]
Cannot be in defense position
Death: Summon a Growing Felinor Sr
~~~
[3/3]
Divine Shield, Rush
Cannot be in defense position
Death: Summon a Radiant Growing Felinor Super Senior

(1) Growing Felinor Super Senior, Unit, Felinor, Common, Meditative, #93.3
[4/4]
Cannot be in defense position
~~~
[4/4]
Divine Shield, Rush
Cannot be in defense position

(4) Shrinking Felinor, Unit, Felinor, Rare, Meditative, #94
[9/11]
Death: Summon a Shrinking Felinor with base stats [-3/-3]
~~~
[18/22]
Death: Summon a Shrinking Felinor with base stats [-2/-3]

(4) Call to Chaos (Meditative Edition) Spell, Meditative, Call to Chaos, Legendary, #95
???
(One of the following random effects)
Fuse your entire hand into one card, add two more copies of it to your hand, they costs(0)
Add 3 CN cards to your hand, they cost (0)
Add 2 Prime cards to your hand, they cost (0)
Your hero gains 8 Armor & heal your hero 8 health
Summon a Jade Beauty
Summon 3 random Acclaimed cards
Bounce your opponents field, then degrade all the cards 
Summon a CN Golem
For the rest of the game: Start of turn cast a random Call to Chaos
Cast a random Call to Chaos
~~~
!!!
(three effects)

(4) CN Golem Unit, CN, Token, Meditative, Legendary, #95.1
10/10
Rush, Poisonous, Cleave, Pierce
On kill shuffle a CN-Virus into your opponents deck
~~~ 
20/20
Rush, Poisonous, Cleave, Pierce, Windfury
On kill shuffle a Radiant CN-Virus into your opponents deck

(2) Meditative Journey, Spell, Rare, Meditative, #96
Select up to two cards in your hand to go on a journey {exile them and a shuffle a Journey Complete into your deck}
~~~
Select up to five cards in your hand to go on a journey {exile them and a shuffle a Journey Complete into your deck}
Draw 1

(2) Journey Complete, Spell, Rare, Meditative, #96.1
Cast of draw: add a ascended cards back to your hand [Radiant versions of the two cards that were exiled]
~~~
Cast of draw: add a ascended cards back to your hand, they cost (1) less [Radiant versions of the cards that were exiled]

(0) Jlockheed’s Evil Blueprints, Spell, Mythic, Jlockheed, Meditative #97
Piece together the blueprint  
~~~
Piece together the Radiant blueprint  
{for ai: discover one of the following 9 units}

(0) Empty Plot, Unit, Common, Meditative #100.1
[0/3]
Can’t attack
Cards may be stacked on this {they have stack}
~~~
[0/8]
Can’t attack
Cards may be stacked on this {they have stack}

(1) Wishing Well, Unit, Common, Meditative #97.2
[0/6]
Can’t attack
Activate: 10% chance to add a random Radiant card to your hand
~~~
[0/12]
Can’t attack
Activate: Lucky 1 12% chance to add a random Radiant card to your hand

(2) School, Unit, Rare, Meditative #97.3
[0/8]
Can’t attack
Activate: summon a random (1) cost unit
~~~
[0/16]
Can’t attack
Activate: summon a random Radiant (1) cost unit

(3) Mega Church, Unit, Rare, Meditative #97.4
[0/5]
Can’t attack
Tribute 5 (can tribute any card {owned by any player} that costs (1) or less)
Activate: Take control of an enemy Permanent
~~~
[0/10]
Can’t attack, Divine Shield
Tribute 5 (can tribute any card {owned by any player} that costs (1) or less)
Activate: Take control of an enemy Permanent, make it Radiant

(2) Bunker, Unit, Epic, Meditative #97.5
Tribute 2
[5/10]
Armor 3, Can’t attack, First strike
Has triple the attack against units in the same lane
~~~
Tribute 2
[5/30]
Armor 5, Can’t attack, First strike
Has triple the attack against units in the same lane

(4) University, Unit, Epic, Meditative #97.6
[0/12]
Can’t attack
Activate: Buff all friendly permanents twice
~~~ 
[0/24]
Can’t attack
Activate: Buff all friendly permanents five times

(2) The Great Wall, Unit, Epic, Meditative #97.7
Tribute 3
Immutable
Can’t attack
[0/50]
Cry: Lock all friendly unit tiles
~~~
Tribute 3
Immutable
Armor 2
Can’t attack
[0/100]
Cry: Lock all friendly unit tiles

(4) Prison, Unit, Legendary, Meditative #97.8
Tribute 2
[0/16]
Every Unit your opponent plays has a 50% chance of being stacked under this {for the ai: giving you control of it}
~~~
[0/32]
Every Unit your opponent plays has a Lucky 50% chance of being stacked under this {for the ai: giving you control of it}

(4) Jlockheed’s Headquarters, Unit, Jlockheed, Mythic, Meditative #97.9
Tribute 5
Indestructable
Can’t attack
[0/20]
Activate: Fill your board with random Jlockheed cards
~~~
Tribute 5
Indestructable
Can’t attack
[0/50]
Activate 2: Fill your board with random Radiant Jlockheed cards
```

Added by the designer after the brief (issue #571), verbatim:

```text
(1) Gachaholic, Unit, Human, CN, Common, Meditative, #98
1/1
Cry: Add a random Luck-based card to your hand. Give it Lucky 1.
~~~
2/2
Activate: Add a random Luck-based card to your hand. Give it Lucky 1.

(1) Catboy Maid SSR+, Unit, Felinor, CN, Common, Meditative, #99
1/1
Lucky 1
Cry: Draw 1-2. +1-4 to your Jade Counter.
~~~
2/2
Lucky 2
Cry: Draw 1-2. +1-10 to your Jade Counter.

(2) Greaser, Unit, Common, Meditative, #100
7/7
~~~
21/21

any card that flips coins can by default get Lucky (heads is the Lucky side)
```
