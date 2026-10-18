The designer's list for the Meditative+ set and the four rule changes beside it, as written on 2026-10-18 in issue #1173, verbatim (typos, numbering and formatting as the designer wrote them; `~~~` separates a base face from its Radiant face); `docs/meditative-plus-set.md` reads it card by card, and SPEC (§7, §8.9) and `crates/cards/catalog.json` are the implemented reading, not this file.

```text
* ALL CARDS IN CHINESE GAIN the CN tribute
* Make sure all tokens are labeled as tokens properly 
* Standardize the multiple cost cards to have it be that they are called embiggened cards and that the more expensive cost is considered embiggened version and the cheaper cost is considered normal version


* Freeze
   * Unit loses its next attack/change battle position opportunity
   * Unfreezes at end of turn where it could’ve done the action
      * Ex. Unit is frozen on opponents turn. It can’t do anything on your turn and unfreezes at the end of your turn.
      * Ex. Unit is frozen on your turn after you attacked with it. It does NOT unfreeze at the end of your turn.
      * Hearthstone-style


(1) Bob’s Plan, Quickdraw, Spell, Mythic, Meditative+, #1
Add a Wincon card to your hand & supporting cards to help you achieve it
{for AI creating this: you determine the easiest way to achieve it and help out, you may make some AI cards to help if you deem necessary}
~~~
Add a Radiant Wincon card to your hand & supporting cards to help you achieve it

(1) NA Tech, Spell, Epic, Meditative+, #2
Fuse the lowest winrate card & highest winrate card, add it to your hand, it costs (2)
~~~
Fuse the Radiant lowest winrate card & Radiant highest winrate card, add it to your hand, it costs (0)

(2 embiggen 4) Wondrous Wand, Spell, Epic, Meditative+, #3
Draw 1 embiggen 2 cards, they cost (0)
~~~
Draw 2 embiggen 5 cards, they cost (0)

(2 embiggen 4) Supply Drop, Spell, Jlockheed, Epic, Meditative+, #4
Choose Combat or Supplies:
Combat: Pierce, Lifesteal, In 3 turns add deal 4 damage to all enemies, embiggen 5 turns 9 damage
Supplies: In 3 turns add a Radiant Jlockheed card to your hand, it costs (2) embiggen 5 turns 5 Radiant Jlockheed cards
~~~
Choose Combat or Supplies:
Combat: Pierce, Lifesteal, In 3 turns add deal 9 damage to all enemies, embiggen 5 turns 15 damage
Supplies: In 3 turns add 2 Radiant Jlockheed card to your hand, it costs (0) embiggen 5 turns fill your hand with Radiant Jlockheed cards

(0 embiggen 1) Spirit Lash, Spell, Rare, Meditative+, #5
Deal 1 damage to all permanents embiggen Lifesteal, Pierce
~~~
Deal 1 damage to all permanents embiggen Lifesteal, Pierce
Echo 1

(0) Bloodbloom, Spell, Epic, Meditative+, #6
Your next spell costs [spell_cost * 3] health instead of mana
~~~
Your next spell costs [spell_cost * 3] health instead of mana, it also gains Echo

(1) Life Mage, Unit, Common, Human, Meditative+, #7
Divine Shield
[1/2]
Aura: Your spells have lifesteal
~~~
Divine Shield
[2/4]
Aura: Your spells have lifesteal, and healing is twice as effective

(2) Arch Mage, Unit, Common, Human, Meditative+, #8
[4/7]
Spell damage +3
~~~
[8/14]
Spell damage +6

(1) Magic Amplifier, Field Spell, Common, Meditative+, #9
Spell damage +2 for both players
~~~
Spell damage +5 for both players

(1) Medina Grifter, Unit, Human, Common, Meditative+, #10
[4/1]
Cry: Steal a card from your opponents hand
~~~
[8/2]
Cry: Steal a card from your opponents hand, it costs (0)

(1) Living Mana Potion, Unit, Rare, Potion, Meditative+, #11
[5/1]
Activate: tribute this and gain +5 Spell damage for the rest of the turn
~~~
[10/2]
Activate: gain +5 Spell damage for the rest of the turn

(1) Living Armor Potion, Unit, Rare, Potion, Meditative+, #12
[1/5]
Activate: tribute this and your hero gains +5 Armor unit the start of your next turn
~~~
[2/10]
Activate: your hero gains +5 Armor unit the start of your next turn

(1) Living Health Potion, Unit, Rare, Potion, Meditative+, #13
[3/3]
Activate: tribute this and heal all allies 5 health
~~~
[6/6]
Activate: heal all allies 5 health

(1) Potion Pouch, Spell, Rare, Meditative+, #14
Add 2 Potions to your hand
~~~
Add 2 Radiant Potions to your hand

(1) Book of Potions, Spell, Epic, Book, Meditative+, #15
Add 1 Radiant Potion to your hand
~~~
Add 2 Radiant Potion to your hand

(1) Jazakus, Unit, Epic, Human, Catalyst, Meditative+, #16
[2/2]
Cry: Craft a custom potion 
{costs (1)}
Death: Shuffle Jazakus Prime into your deck
~~~
[4/4]
Cry: Craft a Radiant custom potion 
{costs (1)}
Death: Shuffle Radiant Jazakus Prime into your deck

(1) Jazakus Prime, Unit, Epic, Human, Prime, Token, Meditative+, #16.1
[8/8]
Cry: Craft a HUGE custom potion 
{costs (3) ~ about 5 times better than the normal custom potion}
~~~
[16/16]
Cry: Craft a Radiant HUGE custom potion 
{costs (3) ~ about 5 times better than the Radiant normal custom potion}

(1) Pile On, Spell, Rare, Meditative+, #17
After a spell targeting an enemy resolves, this targets them dealing 2 damage
(from anywhere in your hand or deck)
~~~
After a spell targeting an enemy resolves, this targets them dealing 5 damage
(from anywhere in your hand or deck)

(1) Cognitive Offloading, Spell, Epic, Meditative+, #18
Cast when drawn: Gain 1 mana & draw 1 card; AI completes the rest of your turn
~~~
Cast when drawn: Gain 3 mana & draw 2 card; AI completes the rest of your turn

(1 embiggen 2) Grubby Gloves, Spell, Common, Meditative+, #19
Steal 2 cards from your opponents deck embiggen their hand instead
~~~
Steal 3 cards from your opponents deck embiggen their hand instead, they cost (0)

(2) KY’s Competition, Spell, Rare, KY, Meditative+, #20
Solve questions as you can, summon a [X/X] Lil’ Kenny when you are finished or fail to answer a question, where X=number of correct answers in a row
~~~
Solve easy questions as you can, summon a [2X/2X] Lil’ Kenny when you are finished or fail to answer a question, where X=number of correct answers in a row

{normal questions triple digit addition / subtraction, easy single or double digit addition / subtraction}

(1) Lil’ Kenny, Unit, Rare, KY, Meditative+, Token, #20.1
[X/X]
Has the following keywords based on the value of X:
4: Rush / 8: Divine Shield / 12: First Strike / 16: Cleave / 20: Lifesteal / 30: Indestructible 
~~~
[2X/2X]
Has the following keywords based on the value of X:
4: Rush / 8: Divine Shield / 12: First Strike / 16: Cleave / 20: Lifesteal / 30: Indestructible 

(2) Harmony Spirit, Unit, Rare, CN, Meditative+, #21
[3/7]
Taunt
Cry: Set the stats of all other units to 3/3
~~~
[6/14]
Taunt
Cry: Set the stats of all other units to 3/3, fill your board with Rush Tokens {they are in Chinese and gain the CN keyword}

(3) Mountaintop Garden, Field Spell, Epic, Meditative+, #22
Aura: All embiggened innately gain their Embiggened effect (without paying the extra cost); cards instead of an embiggened (cost +2) effect to make them Radiant (if they are not Radiant already)
~~~
Aura: All embiggened innately gain their Embiggened effect (without paying the extra cost); cards instead of an embiggened (cost +1) effect to make them Radiant (if they are not Radiant already)

(2 embiggen 4) Enormous Elderberry, Spell, Fruit, Epic, Meditative+, #23
Add a random embiggened card to your hand, its embiggened form costs (0); embiggen add 3 instead of 1
~~~
Add a random radiant embiggened card to your hand, its embiggened form costs (0); embiggen add 3 instead of 1

(3) Bob, Wisconsin's Strongest Soldier, Unit, Spell, Legendary, Meditative+, #24
[5/5]
Cry: Choose one

* Freeze all enemy units
* Add an enemy unit to your hand and give your opponent 3 coins
* Discover a random unit, make it radiant
* Draw 1, fill your hand with copies of it

~~~
[10/10]
Cry: Do all of the following in order

* Freeze all enemy units
* Add an enemy unit to your hand and give your opponent 3 coins
* Discover a random unit, make it radiant
* Draw 1, fill your hand with copies of it


(1) KY’s Intuition, Spell, Rare, KY, Meditative+, #25
Discover one of 3 code blocks. Add the Radiant card with that code to your hand.
{should come from an actual coded card}
~~~
Discover one of 3 code blocks. Add the Radiant card with that code to your hand. It costs (0)
{should come from an actual coded card}

(0) Scratch Pad, Spell, Meditative+, #26
Code a card from scratch. Exile this.
~~~
Code a card from scratch, make it radiant. Exile this.
{no tool tips or hints}
```
