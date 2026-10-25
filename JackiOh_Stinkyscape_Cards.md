The designer's list for the Stinkyscape set and the patches beside it, as uploaded on 2026-10-25, verbatim (typos, numbering and formatting as the designer wrote them, Markdown escapes included; `~~~` separates a base face from its Radiant face); `docs/stinkyscape-set.md` reads it and plans the issues, and SPEC (§7, §8) and `crates/cards/catalog.json` will be the implemented reading, not this file.

```text
# Mechanics

## Keywords

* **Dormant**  
  * Cannot attack, be targeted, exiled, destroyed, or interacted with until a condition is met  
* **Starship & Starship Piece**  
  * **Starship Pieces Keyword does not transfer when equipped,** (i.e., if you equip a starship piece onto a non-starship piece the result is not a starship piece.   
  * When a **Starship Piece** enters the GY a copy of it gets **fused** into a **Dormant Starship.** You may pay (3) to **Awaken** the **Starship.** Upon being **Awoken** trigger all **Cry**  
    * *If you do not have a Dormant Starship summon a Dormant Starship; if your front row board is full, stack it on top of your Lane 3 front row (regardless if its Locked or Not).*   
    * *Note Starships*  
  * **(3) Dormant Starship, Unit, Legendary, Dormant Starship, Stinkyscape**  
    * **Dormant**  
    * **While Dormant** cards cannot be stacked on top of this, if they do the stacked card is **exiled** before its effect resolves   
    * **Activate:** Pay (3) to **Awaken** and trigger all **Cries**  
    * *When awoken gain one of these randomly (names with a tag gain preference and can only appear if the Starship would have that tag)*  
      * **The Ninth Life \~ Felinor**  
      * **Frontier’s Promise \~ Human**  
      * **The Celestial Archive \~ KY**  
      * **The Lantern Ferry \~ CN**  
      * **\~ Jlockeed**  
      * **Ember of Departure \~ Generic**  
      * **The Silent Choir \~ Generic**  
    * *Note that this is NOT a token so it can be shuffled into your GY*  
  * E.G.,  
    * **(3) The Ninth Life, Unit, Legendary, Starship, Stinkyscape**  
* **Tradable**  
  * Once per turn: can shuffle a **Tradable** card in your hand back into the deck to draw a card  
    * Global per-turn limit on ALL **Tradable** cards so you can’t spam a bunch of them for unlimited cycle  
  * Borrow the Duos pop up for sending cards to your partner

## Patches

* **Ceaseless Void**  
  * Gains the Terminus tag  
* **In Too Deep**  
  * Gains the Quest tag  
* **Otherworldly Removal**  
  * Gains the Otherworldly tag  
* **Brawl**  
  * Gains the Other-Origins tag  
* **Blade Storm**  
  * Gains the Other-Origins tag  
* **Execute**  
  * Gains the Other-Origins tag  
* **Showdown**  
  * Gains **Tradable**  
* **Prestige**  
  * Gains **Tradable**  
  * Only can be played if there is a valid target to de-Radiant  
* **Felinor Feelings**  
  * Loses Felinor tag  
* **Friend of Felinors**  
  * Loses Felinor tag  
* **Organic Produce**  
  * Loses Fruit tag  
* **Fruit Tree**  
  * Loses Fruit tag

## Tokens

**(1) Drone Interceptor, Unit, Common, Token, Stinkyscape**  
\[2/2\]  
**Ranged, Flying**  
\<has 1 random combat keyword\>  
\~\~\~  
\[4/4\]  
**Ranged, Flying**  
\<has 2 random combat keyword\>

**(3) Drone Corvette, Unit, Rare, Token, Stinkyscape**  
\[8/8\]  
**Ranged, Flying**  
\<has 2 random combat keyword\>  
\~\~\~  
\[16/16\]  
**Ranged, Flying**  
\<has 4 random combat keyword\>

**Drone Battleship**  
When this enters the field put the summon the **Drone Battleship \- Meltdown Layer** then stack on top of it the **Drone Battleship \- Propagation Layer** then the **Drone Battleship \- Shield Layer**  
*{such that you defeat the shield layer then the propagation layer then the meltdown layer}*  
*{Radiant Drone Battleships make all layers **Radiant**}*

**(5) Drone Battleship \- Shield Layer, Unit, Mythic, Starship, Token, Stinkyscape**  
\[12/12\]  
**Starship**  
**Can’t attack**  
**Taunt, Armor 4**  
Fatal damage a friendly ally would take is redirected to this  
\~\~\~  
\[24/24\]  
**Starship**  
**Can’t attack**  
**Taunt, Armor 8**  
Fatal damage a friendly ally would take is redirected to this

**(5) Drone Battleship \- Propagation Layer, Unit, Mythic, Starship, Token, Stinkyscape**  
\[12/12\]  
**Starship**  
**Can’t attack**  
**Activate 2:** summon a **Drone Interceptor,**  
**Activate 1:** summon a **Drone Corvette**  
\~\~\~  
\[24/24\]  
**Starship**  
**Can’t attack**  
**Activate 2:** summon a **Radiant** **Drone Interceptor,**  
**Activate 1:** summon a **Radiant** **Drone Corvette**

**(5) Drone Battleship \- Meltdown Layer, Unit, Mythic, Starship, Token, Stinkyscape**  
\[12/12\]  
**Starship**  
**Charge, Windfury, Firststrike, Brittle 3**  
**End of turn:** Take 6 damage  
\~\~\~  
\[24/24\]  
**Starship**  
**Charge, Windfury, Firststrike, Brittle 6**  
**End of turn:** Take 6 damage

## Resource Tokens

**Resources** are a kind of Token where they **DO NOT** take up handspace *(but you can only have a max of 8 of them in your hand again until they are auto sold)* **AND** can be sold or bought on the **Marketplace** for **Credits.** *{Prices fluctuate}*

Rarity Table when generating random **Stinkyscape** resources

* Common: 55%  
* Rare: 25%  
* Epic: 14%  
* Legendary: 5%  
* Mythic: 1%

**(0) Reknite, Spell, Common, Token, Stinkyscape**  
*Sells for 1\~3 credits \| buys for 3*  
Your hero gains \+1 Armor until next turn  
\~\~\~  
*Sells for 2\~6 credits \| buys for 6*  
Your hero gains \+2 Armor until next turn

**(0) Korrelite, Spell, Common, Token, Stinkyscape**  
*Sells for 2\~5 credits \| buys for 5*  
Deal 1 damage to a random enemy  
\~\~\~  
*Sells for 4\~10 credits \| buys for 10*  
Deal 1 damage to a random enemy. **Echo 1**

**(0) Gellium, Spell, Rare, Token, Stinkyscape**  
*Sold for 3\~10 credits \| buys for 10*  
Heal 4 hp  
\~\~\~  
*Sold for 6\~20 credits \| buys for 20*  
Heal 8 hp

**(0) Axnit, Spell, Epic, Token, Stinkyscape**  
*Sold for 5\~15 credits \| buys for 15*  
Deal 3 damage, **lifesteal**  
\~\~\~  
*Sold for 10\~30 credits \| buys for 30*  
Deal 3 damage, **lifesteal, Echo 1**

**(0) Narcor, Spell, Legendary, Token, Stinkyscape**  
*Sold for 12\~25 credits \| buys for 25*  
Add a random **Rare** card to your hand  
\~\~\~  
*Sold for 24\~50 credits \| buys for 50*  
Add a **Radiant** random **Rare** card to your hand

**(0) Red Narcor, Spell, Mythic, Token, Stinkyscape**  
*Sold for 20\~40 credits \| buys for 40*  
Add a random **Legendary** or **Mythic** card to your hand, it costs (2) less  
\~\~\~  
*Sold for 40\~80 credits \| buys for 80*  
Add a random **Radiant** **Legendary** or **Mythic** card to your hand, it costs (0)

## Anomalies

Anomalies a special kind of Token that when cast exile your entire board and replace it with a “precreated board” with the following:

**(4) Anomaly Found\!, Spell, Mythic, Token, Stinkyscape**  
*{Name of Anomaly \- everything except AN, AA, FO}*  
\~\~\~  
*{Name of Anomaly – AN, AA, FO}*

*{if lane isn’t specified assume empty}*

**WH \- Wormhole**  
**Lane 1 unit Radiant Drone Interceptor**  
**Lane 2 unit Drone Interceptor**  
**Lane 4 unit Drone Corvette**  
**Lane 1 backrow Wormhole**  
**Lane 2 backrow Radiant Wormhole**  
**Lane 3 backrow The Gauntlet**   
**Lane 4 backrow Wormhole**  
**Lane 5 backrow Wormhole**

**BF \- Battlefields**  
**Lane 2 unit Drone Battleship**  
**Lane 5 unit Radiant Drone Interceptor**  
**Lane 3 backrow Drone Hanger**

**AL \- Asteroid Field: Large**  
**Lane 3 unit Radiant Large Asteroid**  
**Lane 4 unit Huge Asteroid**

**AT \- Asteroid Field: Small**  
**Lane 1 unit Radiant Small Asteroid**  
**Lane 2 unit Radiant Medium Asteroid**  
**Lane 3 unit Small Asteroid**  
**Lane 4 unit Small Asteroid**  
**Lane 5 unit Radiant Small Asteroid**

**SP \- Wrecked Spice Platform**  
**Lane 1 unit Abandoned Spice Tower**  
**Lane 2 unit Abandoned Spice Tower**  
**Lane 3 unit**   
**Lane 4 unit Radiant Abandoned Spice Tower**  
**Lane 5 unit**   
**Lane 1 backrow Drone Interceptor**  
**Lane 2 backrow**   
**Lane 3 backrow Drone Interceptor**  
**Lane 4 backrow Radiant Drone Interceptor**  
**Lane 5 backrow Drone Hanger**

**MT \- Monument**  
**Lane 1 unit Small Asteroid**  
**Lane 2 unit Small Asteroid**  
**Lane 3 unit Small Asteroid**  
**Lane 4 unit Small Asteroid**  
**Lane 5 unit Small Asteroid**  
**Lane 3 backrow Radiant Sanctum**

**DH \- Drone Hideouts**  
**Lane 2 unit Drone Corvette**  
**Lane 4 unit Drone Corvette**  
**Lane 1 backrow Drone Hanger**  
**Lane 3 backrow Radiant Drone Hanger**  
**Lane 5 backrow Drone Hanger**

**FO \- Frontier Outpost**  
**Lane 2 unit Radiant Halo**  
**Lane 4 unit Radiant Phalanx**  
**Lane 3 backrow The Syndicate**

**AN \- Asteroid Field: Narcour**  
**Lane 1 unit Small Asteroid** *{only gives Narcor and red Narcor}*  
**Lane 2 unit Radiant Large Asteroid** *{only gives Narcor and red Narcor}*  
**Lane 3 unit Radiant Small Asteroid** *{only gives Narcor and red Narcor}*  
**Lane 5 unit Large Asteroid** *{only gives Narcor and red Narcor}*

**AA \- Asteroid Field: Axnit** *{only gives Axnit}*  
**Lane 1 unit Radiant Medium Asteroid** *{only gives Axnit}*  
**Lane 2 unit Large Asteroid** *{only gives Axnit}*  
**Lane 3 unit Radiant Large Asteroid** *{only gives Axnit}*  
**Lane 4 unit Radiant Medium Asteroid** *{only gives Axnit}*  
**Lane 5 unit Radiant Small Asteroid** *{only gives Axnit}*

## Marketplace

**Marketplace** your spend **Credits** to buy on of the following cards *(Radiant versions appear in place of a normal one 20% of the time)*:

**Marketplace** cycles offerings every 6 turns (The stock *{max amount of cards of a certain type}* a player can buy from the marketplace for a given cycle is 1\~3 with resources being 4\~8 and scanning probes being 6 and 3 for Mk1 and Mk2 respectively); but **Fluctuate prices every turn**

It should roughly be:  
10 **Credits** \~ **1 mana**  
With 0 mana cards costing around **5 Credits,** and the cost of every mana above 1 being 5 **Credits** *(e.g, a 5 mana card should cost around 45 Credits)*  
Radiant versions should have a 2x price premium   
With a **SD** of \~ (1/10th of the mana cost)  
With the exception of **Resources** whose prices are always the same

**Four of the Resources:**  
**Three of the following:**  
Beam v3  
Repeater v3  
Cannon v3  
Flak v3  
Beam v4  
Repeater v4  
Cannon v4  
Flak v4  
Beam v5  
Repeater v5  
Cannon v5  
Flak v5  
**Always:**  
Warp Disruptor  
Warp Disruptor II  
Scanning Probe Mk1  
Scanning Probe Mk2  
**Two of the following:**  
Drone Core I  
Drone Core II  
Drone Core III  
Drone Core IV  
Drone Core V  
Drone Core VI  
Drone Core VII  
Drone Core VIII  
Drone Core IX  
Drone Core X  
**One of the following:**  
Rush Token Launcher  
Fruit Launcher  
Jade Launcher  
Book Launcher  
**Two random Starship piece cards**  
**Two random Stinkyscape cards**  
**Create 10 AI Stinkyscape cards and always offer one of those**

# Cards

**(0) Freeze Ray, Spell, Common, Stinkyscape, \#1**  
**Freeze** a Unit and its adjacent Units.  
\~\~\~  
**Freeze** all enemy Units.

**(2) Absolute Zero, Spell, Rare, Stinkyscape, \#2**  
**Freeze** a Unit forever.  
\~\~\~  
**Freeze** a Unit forever. You may give it **Indestructible**.

**(3) Meteor, Spell, Epic, Stinkyscape, \#3**  
Deal 15 damage to a Unit. Lock its zone. **Tramples** onto adjacent Units.  
\~\~\~  
Deal 20 damage to a Unit. Lock its zone. **Tramples** onto adjacent Units and the enemy hero.

**(4) Olivian Expanse, Spell, Mythic, Stinkyscape, \#4**  
**Discover** a Terminus card. If your Deck is empty, it costs (0).  
\~\~\~  
**Discover** a Terminus card. If your Deck is empty, it costs (0). If this is also the last card in your hand, it gains **Win the Game**.

**(4) Farewell, Spell, Mythic, Terminus, Token, Stinkyscape, \#4.1**  
Exile all permanents and graveyards.  
\~\~\~  
Exile all permanents, graveyards, hands, and decks.

**(4) KY’s Last Exam, Spell, Mythic, Terminus, KY, Token, Stinkyscape, \#4.2**  
Solve an impossible problem. If you get it right, deal 30 damage to the enemy hero and **Echo** this.  
\~\~\~  
Solve an impossible problem. If you get it right, deal 30 damage to the enemy hero and **Echo** this. You snuck in a cheat sheet.

**(4) Event Horizon, Spell, Mythic, Terminus, Token, Stinkyscape, \#4.3**  
Take an extra turn.  
\~\~\~  
**Echo**. Take an extra turn.

**(4) Last Word, Trap, Mythic, Terminus, Token, Mythic, Stinkyscape, \#4.4**  
**Reveal** when you take lethal damage: **End the Turn**. You then take an extra turn where your Units have **Charge**. Lose the game at the end of that turn.  
\~\~\~  
**Reveal** when you take lethal damage: **End the Turn**. You then take an extra turn where you have 100 mana and your Units have **Charge**. Lose the game at the end of that turn.

**(4) 1v1 Final Destination, Trap, Mythic, Terminus, Token, Mythic, Stinkyscape, \#4.5**  
**Lock** all but one random lane. Set each hero’s health to 1\. Both players discard all non-permanents.  
\~\~\~  
**Lock** all but one random lane. Set each hero’s health to 1\. Both players discard all non-permanents. Fill your hand with (0) cost **Book of Buffs**.

**(4) The Big Crunch, Spell, Mythic, Terminus, Token, Mythic, Stinkyscape, \#4.6**  
Cast every **Call to Chaos**.  
\~\~\~  
Cast every Radiant **Call to Chaos**.

**(1) Masochism Unmasked, Field Trap, Rare, Stinkyscape, \#5**  
**Reveals** when you take damage on your turn: Redirect all damage you take on your turn to your opponent (up to 20).  
\~\~\~  
**Reveals** when you take damage on your turn: Redirect all damage you take on your turn to your opponent and double it (up to 40).

**(2) Tylannt, Unit, Mythic, Acclaimed, Human, Mythic, Stinkyscape, \#6**  
6/4  
**Tradable**  
**Cry**: Complete the current stage on all your Quests.  
\~\~\~  
12/8  
**Tradable**  
**Cry**: Summon **The Greatest Heist**. Complete the current stage on all your Quests.

**(3) The Greatest Heist, Field Spell, Quest, Legendary, Stinkyscape, \#7**  
**Cry**: Discover an \[Radiant\] Acclaimed Unit or a Jellybean.  
Empty the VC vault\!

* Stage 1: Initiation  
  * Quest: Make an Acclaimed Unit Radiant.  
  * Reward: Summon a random \[Radiant\] Acclaimed Unit.  
* Stage 2: Comradery  
  * Quest: Have 2 Acclaimed Units you control die.  
  * Reward: Add 3 random \[Radiant\] **Anomalies** or **Resources** to your hand.  
* Stage 3: Empty the Vaults  
  * Quest: Steal an enemy card.  
  * Reward: Steal your enemy’s mana crystals.

**(2) Conjure Rosemary Triscuit, Spell, Rare, Stinkyscape, \#8**  
Add a **Rosemary Triscuit** to your hand. Shuffle a **Wardrum** into your deck.  
\~\~\~  
Add a Radiant **Rosemary Triscuit** to your hand. Shuffle a Radiant **Wardrum** into your deck.

**(0) Rosemary Triscuit, Spell, Token, Rare, Stinkyscape, \#8.1**  
Refresh (2) mana.  
\~  
Refresh (4) mana.

**(2) Ticketed, Field Spell, Common, Stinkyscape, \#9**  
**Brittle 3**  
**Cry: Stack** a permanent under this.  
**Death**: return that permanent to its controller.  
\~\~\~  
**Cry: Stack** a permanent under this.  
**Death**: return that permanent to its controller.

**(2) Otherworldly Endurance, Stinkyscape, Spell, Otherworldly, Epic, \#10**  
Add an **Inner Fire**, a **Divine Spirit**, and a **Power Word Shield** to your hand.  
\~\~\~  
Add a Radiant **Inner Fire**, a **Divine Spirit**, and a **Power Word Shield** to your hand.

**(1) Inner Fire, Stinkyscape, Spell, Token, Other-Origins, Epic, \#10.1**  
Set a units attack equal to its health  
\~\~\~  
Set a units attack equal to its \[health \+ 4 (if ally)\] or \[health \- 4 (if enemy)\]

**(2) Divine Spirit, Stinkyscape, Spell, Token, Other-Origins, Epic, \#10.2**  
Set a units health to double its health  
\~\~\~  
Set a units health to triple its health

**(0) Power Word Shield Stinkyscape, Spell, Token, Other-Origins, Epic, \#10.3**  
Give a unit \+2 health; Draw 1  
\~\~\~  
Give a unit \+4 health; Draw 2

**(1) Otherworldly Magic, Stinkyscape, Spell, Otherworldly, Epic, \#11**  
Add a **Frostbolt**, **Arcane Missiles**, and **Flame Strike** to your hand.  
\~\~\~  
Add a Radiant **Frostbolt**, **Arcane Missiles**, and **Flame Strike** to your hand.

**(1) Frostbolt, Stinkyscape, Spell, Token, Other-Origins, Epic, \#11.1**  
Deal 3 damage to a target; **Freeze** it  
\~\~\~  
Deal 5 damage to a target; **Freeze** it; **Cleave**

**(0) Arcane Missiles, Stinkyscape, Spell, Token, Other-Origins, Epic, \#11.2**  
Deal 1 damage to a random enemy; **echo 2**  
\~\~\~  
Deal 1 damage to a random enemy; **echo 5**

**(2) Flame Strike Stinkyscape, Spell, Token, Other-Origins, Epic, \#11.3**  
Deal 5 damage to all enemy units   
\~\~\~  
Deal 10 damage to all enemy units 

**(1) Otherworldly Nightmare, Stinkyscape, Spell, Otherworldly, Epic, \#12**  
Add an **Shadow Word Death**, a **Shadow Word Pain**, and a **Psychic Scream** to your hand.  
\~\~\~  
Add a Radiant Execute, Brawl, and Blade Storm to your hand.

**(1) Shadow Word Death, Stinkyscape, Spell, Token, Other-Origins, Epic, \#12.1**  
Destroy a unit with (5) or more attack  
\~\~\~  
Destroy a unit with (5) or more attack; **Cleave**

**(0) Shadow Word Pain, Stinkyscape, Spell, Token, Other-Origins, Epic, \#12.2**  
Destroy a unit with (3) or less attack  
\~\~\~  
Destroy a unit with (3) or less attack; **Cleave**

**(3) Psychic Scream Stinkyscape, Spell, Token, Other-Origins, Epic, \#12.3**  
Shuffle all permanents into your opponents deck  
\~\~\~  
Shuffle all permanents into your opponents deck; they cost (2) more *{max (4)}*

**(2) Asphyxiants Cloud, Stinkyscape, Spell, Rare, \#13**  
Deal 1 damage to all enemies, draw 1, your next **spell** you play this turn gains poisonous   
\~\~\~  
Deal 1 damage to all enemies, draw 1, your next **spell** you play this turn gains poisonous   
**Echo 1**   
*{should deal 1 **poisonous** damage to all enemies, if it doesn’t please reword so it does}*

**(2) Arm of the Jempire, Unit, Human, Common, Stinkyscape, \#14**  
4/5  
When one of your **Taunt** Units is attacked, give it \+2 attack.  
\~\~\~  
8/10  
When one of your **Taunt** Units is attacked, give it \+4 attack.

**(1) Arg Arg Defender, Unit, Human, Common, Stinkyscape, \#15**  
2 /3  
**Cry**: Give adjacent Units \+1/+1 and **Taunt**.  
\~\~\~  
4/6  
**Cry**: Give adjacent Units \+2/+2, **Divine Shield**, and **Taunt**.

**(1) Rod of Ages, Field Spell, Quickdraw, Mythic, Stinkyscape, \#16**  
**Animated, Flying**  
The first unit played on this is **Equipt**  
**End of turn:** Gain a random **Combat Keyword**  
Upon reaching **10 Keywords** make this card **Radiant**  
\~\~\~  
**Animated, Flying, Indestructable**  
The first unit played on this is **Equipt**  
**End of turn:** Gain two random **Combat Keyword**

**(1) Imprisoned Ship Smith, Unit, Human, Common, Stinkyscape, \#17**  
\[2/2\]  
**Dormant** for 2 turns. When this **awakens** summon a **Fiery Waraxe**  
\~\~\~  
\[4/4\]  
**Dormant** for 2 turns. When this **awakens** summon a **Radiant** **Fiery Waraxe**  
*{voice line is: They can not stop my work}*

**(3) Imitated Instrumentality, Spell, Legendary, Stinkyscape, \#18**  
Summon the 3 Brains. They offer great strategic value but they can vote to remove you. {For AI: if all 3 Brains vote to remove you from the game, an animation plays where they all agree to remove you and you disconnect from the game after. This is treated as a loss}  
\~\~\~  
Summon the 3 Radiant Brains. They offer great strategic value but they can vote to remove you.

**(1) MELCHIOR, Field Spell, Token, Legendary, Stinkyscape, \#18.1**  
**Indestructible**  
**Start of Turn:** Votes to remove you if the enemy has more Units than you.  
**Activate:** Discover a Combat Keyword. Give it to all your Units.  
\~\~\~  
**Indestructible**  
**Start of Turn:** Votes to remove you if the enemy has more Units than you.  
**Activate:** Give your Units 3 random Combat Keywords.

**(2) BALTHASAR, Field Spell, Token, Legendary, Stinkyscape, \#18.2**  
**Indestructible**  
**Start of Turn:** Votes to remove you if you have less than half your starting life.  
**Activate:** Add a copy of the last card you played to your hand. It costs (1) more.  
\~\~\~  
**Indestructible**  
**Start of Turn:** Votes to remove you if you have less than half your starting life.  
**Activate:** Add a Radiant copy of the last card you played to your hand. It costs (1) more.

**(3) CASPER, Field Spell, Token, Legendary, Stinkyscape, \#18.3**  
**Indestructible**  
**Start of Turn:** Votes to remove you if you **Floated** last turn.  
**Activate:** **Allure** a random enemy Unit.  
\~\~\~  
**Indestructible**  
**Start of Turn:** Votes to remove you if you **Floated** last turn.  
**Activate:** **Allure** a random enemy Unit on the field and hand.

**(2) Imprisoned Cosmality, Unit, Common, Stinkyscape, \#19**  
10/6  
**Pierce**  
**Dormant** for 2 turns. When this **awakens,** deal 10 damage split among enemies.  
\~\~\~  
20/12  
**Pierce**  
**Dormant** for 2 turns. When this **awakens,** deal 20 damage split among enemies.

**(2) Tnack Trav, Unit, Epic, Stinkyscape, \#20**  
6/6  
**Cry**: Copy the text of a Legendary or Mythic Unit on the Field.  
\~\~\~  
12/12  
**Cry**: Copy the Radiant text of a Legendary or Mythic Unit on the Field.

**(1) Moonberry, Spell, Fruit, Common, Stinkyscape, \#21**  
**Flashback**  
Add a random Other-Origins card to your hand.  
\~\~\~  
**Flashback**  
Add a random Radiant Other-Origins card to your hand. It costs (1) less.

**(4) Aerelret, Trap, Epic, Stinkyscape, \#21**  
30% chance to **Reveal** when an opponent attacks  
**On reveal: AMBUSH** summon a **Warp Disruptor** then 5 **Paragon Mk1**  
\~\~\~  
**On reveal: AMBUSH** summon a **Radiant Warp Disruptor** then 5 **Radiant** **Paragon Mk1**

**(4) Gatinsir, Trap, Epic, Stinkyscape, \#22**  
30% chance to **Reveal** when an opponent attacks  
**On reveal: AMBUSH** summon two **Warp Disruptor** then **2 Phalanx**  
\~\~\~  
**On reveal: AMBUSH** summon two Radiant **Warp Disruptor** then 2 **Radiant Phalanx**

**(1) Warp Disruptor, Field Spell, Rare, Stinkyscape, \#23**  
**Stack**  
**Brittle 3**  
**Aura:** Units cannot be in defense position, Units have \+2 attack, when the enemy summons a unit a random unit from your side attacks it  
\~\~\~  
**Stack**  
**Brittle 3**  
**Aura:** Units cannot be in defense position, Units have \+2 attack, your units have **First strike**, when the enemy summons a unit a random unit from your side attacks it

**(2) Warp Disruptor II, Field Spell, Rare, Stinkyscape, \#24**  
**Stack**  
**Brittle 5**  
**Aura:** Units cannot be in defense position, Units have \+3 attack, when the enemy summons a unit all unit from your side attacks it  
\~\~\~  
**Stack**  
**Brittle 5**  
**Aura:** Units cannot be in defense position, Units have \+3 attack, your units have **First strike**, when the enemy summons a unit all unit from your side attacks it

**(2) KY’s Tricknology, Spell, KY, Epic, Sinkyscape, \#25**  
Choose a zone. Its Jacobian stretches a unit square. A **Starship Piece** lands on each zone it covers. *F(x, y) \= (x² − y, xy)*  
\~\~\~  
Choose a zone. Its Jacobian stretches a unit square. A Radiant **Starship Piece** lands on each of your zones it covers. *F(x, y) \= (x² − y, xy)*

**(4) ZolarKeth, Unit, Human, Acclaimed, Legendary, Sinkyscape, \#26**  
2/9  
**Cry**: Summon a Radiant **University**. Its Activate is locked.  
**Activate**: Summon a random **Starship Piece** or **Activate** your Universities.  
\~\~\~  
4/18  
**Cry**: Summon a Radiant **University**. Its Activate is locked.  
**Activate**: Summon a random Radiant **Starship Piece** then **Activate** your Universities.

**(0) Drone Core I, Spell, Drone Core, Common, Stinkscape \#27**  
Discard a card. Summon a **Drone Interceptor**  
**\~\~\~**  
Discard a card. Summon a **Radiant** **Drone Interceptor** 

**(1) Drone Core II, Spell, Drone Core, Common, Stinkscape \#28**  
Summon a **Drone Interceptor**  
**\~\~\~**  
Summon a **Radiant** **Drone Interceptor** 

**(2) Drone Core III, Spell, Drone Core, Common, Stinkscape \#29**  
Summon 2 **Drone Interceptor**  
**\~\~\~**  
Summon 2 **Radiant** **Drone Interceptor** 

**(3) Drone Core IV, Spell, Drone Core, Rare, Stinkscape \#30**  
Summon 3 **Drone Interceptor**  
**\~\~\~**  
Summon 3 **Radiant** **Drone Interceptor** 

**(3) Drone Core V, Spell, Drone Core, Rare, Stinkscape \#31**  
Summon a **Drone Corvette**  
**\~\~\~**  
Summon a **Radiant Drone Corvette**

**(4) Drone Core VI, Spell, Drone Core, Epic, Stinkscape \#32**  
Summon 5 **Drone Interceptor**  
**\~\~\~**  
Summon 5 **Radiant** **Drone Interceptor** 

**(4) Drone Core VII, Spell, Drone Core, Epic, Stinkscape \#33**  
Summon a **Drone Corvette** & Summon 2 **Drone Interceptors**  
**\~\~\~**  
Summon a **Radiant Drone Corvette** & Summon 2 **Radiant** **Drone Interceptors**

**(6) Drone Core VIII, Spell, Drone Core, Legendary, Stinkscape \#34**  
**Float** reduces cost  
Fill your board with **Drone Interceptor**  
**\~\~\~**  
**Float** reduces cost  
Fill your board with **Radiant** **Drone Interceptor** 

**(6) Drone Core IX, Spell, Drone Core, Legendary, Stinkscape \#35**  
**Float** reduces cost  
Summon 3 **Drone Corvette**  
**\~\~\~**  
**Float** reduces cost  
Summon 3 **Radiant Drone Corvette**

**(8) Drone Core X, Spell, Drone Core, Mythic, Stinkscape \#36**  
**Float** reduces cost  
Summon the **Drone Battleship**  
\~\~\~  
**Float** reduces cost  
Summon the **Radiant** **Drone Battleship**

**(3) Kwzka, Unit, Human, Acclaimed, Legendary, Stinkyscape, \#37**  
6/9  
**Aura**: **Allure** all enemy Units played that cost less than this.  
**Death**: Remove all **Allures** on the field.  
\~\~\~  
12/18  
**Divine Shield**  
**Aura**: **Allure** all enemy Units played that cost less than this.  
**Death**: Remove all **Allures** on the field.

**(1) Shield Amplifier, Unit, Common, Stinkyscape, \#38**  
1/1  
**Starship Piece**  
**Equip**  
**Divine Shield**  
When this survives an attack, give it \+3/+3.  
\~\~\~  
2/2  
**Starship Piece**  
**Equip**  
**Divine Shield**  
When this survives an attack, give it \+6/+6.

**(4) The Narcor Project, Field Spell, Legendary, Stinkyscape, \#39**  
**Brittle 4**  
**End of Turn:** Add a **Narcor** to your hand.  
**Activate:** Discard a **Narcor** to reduce **Brittle** by 1\.  
**Death**: If this died to **Brittle**, its **End of Turn** effect doubles and becomes permanent.  
\~\~\~  
**Brittle 4**  
**End of Turn:** Add a **Red** **Narcor** to your hand.  
**Activate:** Discard a **Narcor** or **Red Narcor** to reduce **Brittle** by 1\.  
**Death**: If this died to **Brittle**, its **End of Turn** effect doubles and becomes permanent.

**(3)** **Poison Capsule, Unit, Rare, Stinkyscape, \#40**  
2/2  
**Starship Piece**  
**Cry:** Place 2 Plague Counters.  
**End of Turn:** Deal 1 damage to all enemies per Plague Counter on this.  
\~\~\~  
4/4  
**Starship Piece**  
**Cry:** Place 4 Plague Counters.  
**End of Turn:** Deal 1 damage to all enemies per Plague Counter on this.

**(1)** **Drone Sympathizer, Spell, Common, Stinkscape \#41**  
Add 2 random **Drone Cores** to your hand  
\~\~\~  
Add 2 **Radiant** random **Drone Cores** to your hand

**(2)** **Core Researcher, Unit, Human, Rare, Stinkscape \#42**  
\[4/5\]  
**Cry:** add a **Radiant Drone Core** to your hand  
\~\~\~  
\[8/10\]  
**Cry:** add 2 **Radiant Drone Core** to your hand

**(1)** **Healing Capsule, Unit, Rare, Stinkyscape, \#43**  
2/2  
**Starship Piece**  
**End of Turn:** Heal all allies 2\.  
\~\~\~  
4/4  
**Starship Piece**  
**End of Turn:** Heal all allies 4\.

**(1)** **Great Replacement Theory, Spell, Quickdraw, Mythic, Stinkscape, \#44**  
Add a **Drone Core I** to your hand. The first time you play each **Drone Core** tier add the subsequent tier **Drone Core** to your hand.   
\~\~\~  
Add a **Radiant** **Drone Core I** to your hand. The first time you play each **Drone Core** tier add the **Radiant** **Drone Core** to your hand. 

**(4)** **Drone Commander, Unit, Human, Epic, Stinkscape, \#45**  
\[8/12\]  
Add 6 random **Drone Cores** to your hand, they cost (1) less  
\~\~\~  
\[16/24\]  
Add 6 **Radiant** random **Drone Cores** to your hand, they cost (1) less

**(2)** **Circadian Halfcycler, Field Spell, Epic, Stinkscape, \#46**  
**Aura:** the first 3 actions of your turn are played by an **AI**  
**End of turn:** Summon a **Drone Interceptor**  
\~\~\~  
**Aura:** the first 3 actions of your turn are played by an **AI**  
**End of turn:** Summon a **Radiant** **Drone Interceptor**

**(2)** **Circadian Fullcycler, Field Spell, Epic, Stinkscape, \#47**  
**Brittle 5**  
**Aura:** your turn is played by an **AI**  
**End of turn:** Summon a **Drone Corvette**  
\~\~\~  
**Brittle 5**  
**Aura:** your turn is played by an **AI**  
**End of turn:** Summon a **Radiant** **Drone Corvette**

**(1) Ultralight Chassis, Unit, Common, Stinkyscape, \#48**  
4/1  
**Starship Piece**  
**Equip**  
**Rush**  
**\~\~\~**  
8/2  
**Starship Piece**  
**Equip**  
**Rush**

**(2) Spacemine, Unit, Common, Stinkyscape, \#49**  
5/1  
**Starship Piece**  
**Death:** Deal attack as damage to a random enemy.  
**\~\~\~**  
10/2  
**Starship Piece**  
**Death:** Deal attack as damage to 2 random enemies.

**(2) Beam v3, Unit, Rare, Weapon, Stinkyscape, \#50**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 6 damage, **Cleave**, **Pierce** *{has an extra turn cooldown}*  
*{deals half damage to heroes}*  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 12 damage, **Cleave**, **Pierce** *{has an extra turn cooldown}*  
*{deals half damage to heroes}*

**(2) Repeater v3, Unit, Rare, Weapon, Stinkyscape, \#51**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 4:** Deal 1 damage, **Pierce**   
*{deals half damage to heroes}*  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 4:** Deal 2 damage, **Pierce**  
*{deals half damage to heroes}*

**(2) Cannon v3, Unit, Rare, Weapon, Stinkyscape, \#52**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 5 damage  
*{deals half damage to heroes}*  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 10 damage  
*{deals half damage to heroes}*

**(2) Flak v3, Unit, Rare, Weapon, Stinkyscape, \#53**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 2 damage to all enemies  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 4 damage to all enemies

**(3) Beam v4, Unit, Epic, Weapon, Stinkyscape, \#54**  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 9 damage, **Cleave**, **Pierce** *{has an extra turn cooldown}*  
*{deals half damage to heroes}*  
\~\~\~  
\[0/12\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 18 damage, **Cleave**, **Pierce** *{has an extra turn cooldown}*  
*{deals half damage to heroes}*

**(3) Repeater v4, Unit, Epic, Weapon, Stinkyscape, \#55**  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 6:** Deal 1 damage, **Pierce**   
*{deals half damage to heroes}*  
\~\~\~  
\[0/12\]  
**Starship Piece**  
**Equip**  
**Activate 6:** Deal 2 damage, **Pierce**  
*{deals half damage to heroes}*

**(3) Cannon v4, Unit, Epic, Weapon, Stinkyscape, \#56**  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 7 damage  
*{deals half damage to heroes}*  
\~\~\~  
\[0/12\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 14 damage  
*{deals half damage to heroes}*

**(3) Flak v4, Unit, Epic, Weapon, Stinkyscape, \#57**  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 2 damage to all enemies, twice  
\~\~\~  
\[0/12\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 4 damage to all enemies, twice

**(4) Beam v5, Unit, Legendary, Weapon, Stinkyscape, \#58**  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 15 damage, **Cleave**, **Pierce** *{has an extra turn cooldown}*  
*{deals half damage to heroes}*  
\~\~\~  
\[0/16\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 30 damage, **Cleave**, **Pierce** *{has an extra turn cooldown}*  
*{deals half damage to heroes}*

**(4) Repeater v5, Unit, Legendary, Weapon, Stinkyscape, \#59**  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 6:** Deal 2 damage, **Pierce**  
*{deals half damage to heroes}*  
\~\~\~  
\[0/16\]  
**Starship Piece**  
**Equip**  
**Activate 6:** Deal 4 damage, **Pierce**  
*{deals half damage to heroes}*

**(4) Cannon v5, Unit, Legendary, Weapon, Stinkyscape, \#60**  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 10 damage  
*{deals half damage to heroes}*  
\~\~\~  
\[0/16\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 20 damage  
*{deals half damage to heroes}*

**(4) Flak v5, Unit, Legendary, Weapon, Stinkyscape, \#61**  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Deal 2 damage to all enemies, thrice  
\~\~\~  
\[0/16\]  
**Starship Piece**  
**Equip**  
**Activate 3:** Deal 4 damage to all enemies, thrice

**(0) Miner v1, Unit, Common, Stinkyscape, \#62**  
\[0/1\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Gain a random **Stinkyscape** resource. *{has an extra turn cooldown}*  
\~\~\~  
\[0/2\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Gain a random Radiant **Stinkyscape** resource. *{has an extra turn cooldown}*

**(1) Miner v2, Unit, Common, Stinkyscape, \#63**  
\[0/2\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Gain a random **Stinkyscape** resource.  
\~\~\~  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Gain a random Radiant **Stinkyscape** resource.

**(2) Miner v3, Unit, Rare, Stinkyscape, \#64**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 2:** Gain a random **Stinkyscape** resource.  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 2:** Gain a random Radiant **Stinkyscape** resource.

**(3) Miner v4, Unit, Epic, Stinkyscape, \#65**  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 3:** Gain a random **Stinkyscape** resource.  
\~\~\~  
\[0/12\]  
**Starship Piece**  
**Equip**  
**Activate 3:** Gain a random Radiant **Stinkyscape** resource.

**(4) Miner v5, Unit, Legendary, Stinkyscape, \#66**  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 5:** Gain a random **Stinkyscape** resource.  
\~\~\~  
\[0/16\]  
**Starship Piece**  
**Equip**  
**Activate 5:** Gain a random Radiant **Stinkyscape** resource.

**(1)** **Tractor Beam, Unit, Rare, Stinkyscape, \#67**  
\[0/2\]  
**Starship Piece**  
**Equip**  
**Start of your turn:** Gain a random **Stinkyscape** resource.  
\~\~\~  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Start of your turn:** Gain a random **Radiant** **Stinkyscape** resource.

**(1) Startos, Unit, Common, Stinkyscape, \#68**  
**Starship Piece**  
**Flying**  
\[3/3\]  
\~\~\~  
**Starship Piece**  
**Flying**  
\[6/6\]

**(2) Falcon, Unit, Rare, Stinkyscape, \#69**  
\[4/1\]  
**Starship Piece**  
**Charge, Flying**  
\~\~\~  
\[8/1\]  
**Starship Piece**  
**Charge, Flying, Pierce**

**(4) Z’Zoth, Unit, Legendary, Stinkyscape, \#70**  
5/7  
**Cry:** Resummon all **Death** Units from your graveyard.  
\~\~\~  
10/14  
**Cry:** Resummon all **Death** Units from your graveyard. Give them **Reborn**.

**(4) A’Zoth, Unit, Legendary, Stinkyscape, \#71**  
7/5  
**Cry:** Resummon all **Cry** Units from your graveyard. Activate each **Cry** with random targets.  
\~\~\~  
14/10  
**Cry:** Resummon all **Cry** Units from your graveyard. Activate each **Cry** twice with random targets.

**(3) Paragon Mk1, Unit, Epic, Stinkyscape, \#72**  
\[6/1\]  
**Starship Piece**  
**Charge, Flying, Ranged**  
\~\~\~  
\[12/2\]  
**Starship Piece**  
**Charge, Flying, Ranged**, **Pierce**

**(4) Paragon Mk2, Unit, Legendary, Stinkyscape, \#73**  
\[7/1\]  
**Starship Piece**  
**Charge, Flying, Ranged, Divine Shield**  
\~\~\~  
\[14/2\]  
**Starship Piece**  
**Charge, Flying, Ranged, Divine Shield, Pierce**

**(4) Paragon-X, Unit, Mythic, Stinkyscape, \#74**  
\[10/1\]  
**Starship Piece**  
**Tribute 2**  
**Charge, Flying, Ranged, Divine Shield, Immutable, Immune to Spell Damage**  
\~\~\~  
\[20/2\]  
**Starship Piece**  
**Tribute 2**  
**Charge, Flying, Ranged, Divine Shield, Immutable, Immune to Spell Damage**, **Pierce**

**(1) Badger, Unit, Common, Stinkyscape, \#75**  
\[2/3\]  
**Starship Piece**  
When this attacks add gain a random **Stinkyscape** resource.   
\~\~\~  
\[4/6\]  
**Starship Piece**  
When this attacks add gain a random **Radiant** **Stinkyscape** resource. 

**(2) Honey Badger, Unit, Rare, Stinkyscape, \#76**  
\[4/6\]  
**Starship Piece**  
**Rush**  
When this attacks add gain a random **Stinkyscape** resource.   
\~\~\~  
\[8/12\]  
**Starship Piece**  
**Rush**  
When this attacks add gain a random **Radiant** **Stinkyscape** resource. 

**(2) Yuma, Unit, Epic, Stinkyscape, \#77**  
1/8  
**Starship Piece**  
**Flying**  
**Cry:** Trade offer your opponent at the start of their turn. You give them any card in your hand and they have to Discover a card in their own to give to you.  
\~\~\~  
2/16  
**Starship Piece**  
**Flying**  
**Lucky 1**  
**Cry:** Trade offer your opponent at the start of their turn. You give them any card in your hand and they have to Discover a card in their own to give to you.

**(3) Ozark, Unit, Epic, Stinkyscape, \#78**  
\[4/4\]  
**Starship Piece**  
**Cry:** A random \[Radiant\] **Miner vX**s is **Equip** this.  
\~\~\~  
\[8/8\]  
**Starship Piece**  
**Cry:** A random \[Radiant\] **Miner vX**s is **Equip** this, twice.

**(4) \[Embiggen 5\] Orca, Unit, Epic, Stinkyscape, \#79**  
8/8  
**Starship Piece**  
**Flying**  
**Cry**: Choose between 2 \[Radiant\] **Miner vX**s to **Equip** this.  
\~\~\~  
16/16  
**Starship Piece**  
**Flying**  
**Cry**: Choose between 2 \[Radiant\] **Miner vX**s to **Equip** this, twice

**(4) Galaxy, Unit, Legendary, Stinkyscape, \#80**  
10/10  
**Starship Piece**  
**Flying**  
**Tribute 2**  
**Cry**: Discover 2 Radiant **Miner vX**s to **Equip** this.  
\~\~\~  
20/20  
**Starship Piece**  
**Flying**  
**Tribute 2**  
**Cry**: **Equip** this with each Radiant **Miner vX**.

**(3) Radix, Unit, Rare, Stinkyscape, \#81**  
\[2/2\]  
**Starship Piece**  
**Cry:** Discover 2 **Rare** **Weapons**; **Equip** them into this  
\~\~\~  
\[4/4\]  
**Starship Piece**  
**Cry:** Discover 2 **Epic** **Weapons**; **Equip** them into this

**(4) Phalanx, Unit, Epic, Stinkyscape, \#82**  
\[4/4\]  
**Starship Piece**  
**Armor 3**  
**Cry:** Discover 2 **Rare** **Weapons**; **Equip** them into this  
\~\~\~  
\[8/8\]  
**Starship Piece**  
**Armor 6**  
**Cry:** Discover 2 **Epic** **Weapons**; **Equip** them into this

**(4) Halo, Unit, Epic, Stinkyscape, \#83**  
\[2/2\]  
**Starship Piece**  
**Cry:** Discover 3 **Rare** **Weapons**; **Equip** them into this  
\~\~\~  
\[4/4\]  
**Starship Piece**  
**Cry:** Discover 3 **Epic** **Weapons**; **Equip** them into this

**(1) Power Cell, Spell, Common, Stinkyscape, \#84**  
Shuffle 4 **Metal Scraps** into your deck.  
\~\~\~  
Shuffle 8 **Metal Scraps** into your deck.

**(1) Metal Scraps, Spell, Token, Common, Stinkyscape, \#84.1**  
**Cast on Draw:** Gain a **Stinkyscape** resource.  
\~\~\~  
**Cast on Draw:** Gain a Radiant **Stinkyscape** resource.

**(4) Prototype-X1, Unit, Legendary, Stinkyscape, \#85**  
**Tribute 2**  
**Starship Piece**  
\[6/6\]  
**Cry:** Discover 2 **Epic** **Weapons**; **Equip** them into this  
\~\~\~  
**Tribute 2**  
**Starship Piece**  
\[12/12\]  
**Cry:** Discover 2 **Legendary** **Weapons**; **Equip** them into this

**(4) Prototype-X2, Unit, Mythic, Stinkyscape, \#86**  
**Float** reduces **Tribute** requirement by 1  
**Tribute 5**  
**Starship Piece**  
\[8/8\]  
**Cry:** Discover 3 **Epic** **Weapons**; **Equip** them into this  
\~\~\~  
**Float** reduces **Tribute** requirement by 1  
**Tribute 5**  
**Starship Piece**  
\[8/8\]  
**Cry:** Discover 3 **Legendary** **Weapons**; **Equip** them into this

**(2) Siva 1, Unit, Felinor, Acclaimed, Legendary, Stinkyscape, \#87**  
2/2  
**Tradable**  
**Cry**: **Allure** a random enemy Acclaimed Unit.  
\~\~\~  
4/4  
**Tradable**  
**Cry**: **Allure** a random enemy Acclaimed Unit. If there isn’t one, summon a **Frozen** one to **Allure**.

**(3) Siva 2, Unit, Human, Acclaimed, Legendary,** **Stinkyscape, \#88**  
4/4  
**Aura:** Your Humans have **Poisonous**.  
\~\~\~  
8/8  
**Aura:** Your Humans have **Poisonous**.  
**Cry:** Fill your hand with **Bigots**.

**(1) Spice, Spell, Rare, Stinkyscape, \#89**  
Can be sold on the Resource marketplace. {For AI: Its price is determined like all other Resources; only appearing once a **Spice** is potentially relevant}  
Discover a **Stinkyscape** card.  
\~\~\~  
Can be sold on the Resource marketplace.  
Discover a Radiant **Stinkyscape** card.

**(2) After Burner, Unit, Common, Stinkyscape, \#90**  
4/8  
**Starship Piece**  
When launched, gain 2 random Combat Keywords.  
\~\~\~  
8/16  
**Starship Piece**  
When launched, gain 4 random Combat Keywords.

**(1) The Propagation's Wasp, Swarm, Common, Stinkyscape, \#91**  
\[3/3\]  
After these attacks, summon a copy of this, then **evolve.**  
**Evolve:** gain **Flying**  
\~\~\~  
\[6/6\]  
After these attacks, summon a copy of this, then **evolve.**  
**Evolve:** gain **Flying**

**(2) The Propagation's Hornet, Swarm, Rare, Stinkyscape, \#92**  
\[4/4\]  
**Rush**  
After these attacks, summon a copy of this, then **evolve.**  
**Evolve:** gain **Flying**  
\~\~\~  
\[8/8\]  
**Rush**  
After these attacks, summon a copy of this, then **evolve.**  
**Evolve:** gain **Flying**

**(2) Patcher Nanobots, Unit, Common, Stinkyscape, \#93**  
3/4  
**Starship Piece**  
**Taunt**  
**Death:** Heal your hero 6\.  
\~\~\~  
6/8  
**Starship Piece**  
**Taunt**  
**Death:** Heal your hero 12\.

**(3) Tmack Trav, Unit, Epic, Human, Stinkyscape, \#94**  
9/9  
**Rush**  
After this kills a unit, this may attack again.   
\~\~\~  
18/18  
**Rush, Pierce**  
After this kills a unit, this may attack again. 

**(3) Vanguard Coalition, Spell, Rare, Stinkyscape, \#95**  
Add a **Scornboi, Ocean Tsunami, Tylannt, Vxyl, Siva 1,** and **Siva 2** to your hand  
\~\~\~  
Add a **Radiant** **Scornboi, Ocean Tsunami, Tylannt, Siva 1,** and **Siva 2** to your hand

**(3) Ocean Tsunami, Unit, Human, Rare, Stinkyscape, \#96**  
\[6/6\]  
**Cry:** discard your hand then fill your hand with random **Stinkyscape** resources   
\~\~\~  
\[12/12\]  
**Cry:** discard your hand then fill your hand with random **Radiant** **Stinkyscape** resources 

**(2) Asteroid Field, Spell, Common, Stinkyscape, \#97**  
**Cast on draw:** deal 2 damage to all heroes and units, add two random **Stinkyscape** resources to your hand  
\~\~\~  
**Cast on draw:** deal 2 damage to all enemy heroes and units, add two random **Radiant** **Stinkyscape** resources to your hand

**(1) Hull Booster, Unit, Common, Stinkyscape, \#98**  
1/1  
**Starship Piece**  
**Equip**  
**Armor 2**  
\~\~\~  
2/2  
**Starship Piece**  
**Equip**  
**Armor 4**

**(X) No-No Words, Spell, Common, Stinkyscape, \#99**  
Destroy a unit with less than \[4X\] attack  
\~\~\~  
Destroy a unit with less than \[4X\] attack, **Cleave**  
   
**(1) AP Rounds, Unit, Common, Stinkyscape, \#100**  
3/1  
**Starship Piece**  
**Equip**  
**Pierce**  
\~\~\~  
6/2  
**Starship Piece**  
**Equip**  
**Pierce**

**(2) Overdrive Module, Unit, Rare, Stinkyscape, \#101**  
5/5  
**Starship Piece**  
**Equip**  
**Brittle 2**  
**Reborn**  
\~\~\~  
10/10  
**Starship Piece**  
**Equip**  
**Brittle 4**  
**Reborn**

**(1) Book of Nono-Words, Spell, Epic, Book, Stinkyscape, \#102**  
Add 4 copies of **No-No Words** to your hand, their prices are set at 1,2,3,4  
\~\~\~  
Add 4 copies of **Radiant** **No-No Words** to your hand, their prices are set at 1,2,3,4

**(2) Vxyl, Unit, Human, Rare, Stinkyscape, \#103**  
**Cloaked**   
\[1/4\]  
**Poisonous**   
**On reveal:** Add a **No-No Words** to your hand  
\~\~\~  
**Cloaked**   
\[2/8\]  
**Poisonous, Divine Shield**   
**On reveal:** Add a **No-No Words** to your hand

**(2) Reflective Plating, Unit, Rare, Stinkyscape, \#104**  
0/6  
**Starship Piece**  
**Equip**  
Any damage this takes is dealt to the enemy hero.  
\~\~\~  
0/12  
**Starship Piece**  
**Equip**  
Any damage this takes is dealt to the enemy hero and the attacker.

**(2) Disintegrex, Unit, Epic, Stinkyscape, \#105**  
6/3  
**Starship Piece**  
**Aura:** Anytime something would be Destroyed, it is Exiled instead.  
\~\~\~  
12/6  
**Starship Piece**  
**Aura:** Anytime an enemy permanent would be Destroyed, it is Exiled instead.

**(1) Warpfield Amplifier, Unit, Rare, Stinkyscape, \#106**  
2/3  
**Starship Piece**  
When launched, set all of your Units to attack position and give them \+2 attack.  
\~\~\~  
4/6  
**Starship Piece**  
When launched, set all of your Units to attack position and give them \+4 attack.

**(2) RR-5000, Unit, Rare, Stinkyscape, \#107**  
4/4  
**Starship Piece**  
**Ranged**  
**Start of Turn**: Gain \+4 attack.  
\~\~\~  
8/8  
**Starship Piece**  
**Ranged**  
**Start of Turn**: Gain \+8 attack.

**(3) Trila 3, Unit, Common, Stinkyscape, \#108**  
7/7  
**Starship Piece**  
**Ranged**  
**Windfury**  
**\~\~\~**  
14/14  
**Starship Piece**  
**Ranged**  
**Rush**  
**Windfury**

**(4) xxFreshJack, Unit, Human, Acclaimed, Mythic, Stinkyscape, \#109**  
**6/11**  
**Cry:** Play a random BUSTED card that exists anywhere in the game’s code. {For AI: A busted card is any card that required more than 100 lines of code to implement. This card is also allowed to pull from pools of cards that are normally restricted to single-player only game modes.}   
\~\~\~  
**Cry**: Discover and play a Radiant BUSTED card that exists anywhere in the game’s code.

**(2) iKwzki, Unit, Human, Acclaimed, Mythic, Stinkyscape, \#110**  
6/6  
**Cry:** Duo with a permanent and make it Radiant next turn. {For AI: Reuse the Allure animation but recolor it to be fitting for making something Radiant}   
\~\~\~  
12/12  
**Cry:** Squad with all your permanents and make them Radiant next turn.

**(4) Shockwave447, Unit, Human, Acclaimed, Mythic, Stinkyscape, \#111**  
4/7  
**Aura:** Has 3 random Combat Keywords.  
**Cry:** Replace both player’s hands with complicated cards.  
\~\~\~  
8/14  
**Aura:** Has 3 random Combat Keywords.  
**Cry**: Replace both player’s hands with complicated cards. Make yours Radiant.

**(1) Job Application, Field Spell, Quest, Legendary, Stinkyscape, \#112**  
\[Lucky 1\]  
Get employed\!  
**Death:** Resummon this at Stage 1\.

* Stage 1: Upload Resume  
  * Quest: Send a card from your hand in as your resume.  
  * Reward: Hear back in 2-6 turns.  
    * Create some evaluation algorithm based on the card uploaded.  
      * Should have a little bit of RNG in it.  
      * {Easter egg: if it’s not super dumb and resource intensive, have a small % chance to send an email to the player thanking them for their application with the analysis results in it}  
      * {Easter egg: an iKwzki upload always hears back in 6}  
* Stage 2: The Interview (Quest chosen at random)  
  * Quest: Prove your resume. Play a card that costs the same as your resume.   
  * Quest: Behavioral round. Play 3 cards of different costs in a single turn.  
  * Quest: Hardball question. Take or deal 12+ damage in a single instance.  
  * Reward: Get your resume back, buffed 5 \[10\] times and it costs (0).  
* Stage 3: The Offer  
  * Quest: Activate to claim next turn or to send in another application (return to Stage 1\) and triple rewards.  
  * Reward: At the start of every turn get a **Paycheck**

**(0) Paycheck**, **Spell, Legendary, Token, Stinkyscape, \#112.1**  
Choose one: Gain 2 mana, draw 1 card, and gain 20 **credits** or go all in on red with a 18/39 chance to gain double the reward  
\~\~\~  
**Lucky 1**  
Gain 2 mana, draw 1 card, and gain 20 **credits** or go all in on red with a 18/39 chance to gain double the reward

**(4) A Very Dandy Lad, Unit, Human, Epic, Stinkyscape, \#113**  
\[9/15\]  
**Tribute 2**  
**Aura:** all **Permanents** that cost (2) or less are **Exiled**  
\~\~\~  
\[18/30\]  
**Tribute 2**  
**Aura:** all enemy **Permanents** that cost (2) or less are **Exiled**

**(4) Arctic Citadel, Unit, Human, Epic, Stinkyscape, \#114**  
\[4/4\]  
**Cry:** Fuse all cards from your **Exile** into this  
\~\~\~  
\[8/8\]  
**Cry:** Fuse all cards from BOTH **Exile** into this

**(4) Scornboi, Unit, Human, Legendary, Acclaimed, Stinkyscape, \#115**  
\[12/9\]  
**Tribute 3**   
**Aura:** all **non-human**, **non-starship piece, non-starship,** are exiled  
\~\~\~  
\[24/18\]  
**Tribute 3**   
**Aura:** all enemy **non-human**, **non-starship piece, non-starship,** are exiled

**(1) Megabase, Field Spell, Quickdraw, Legendary, Stinkyscape, \#116**  
**Can’t be in defense position**  
**Can’t attack, Immune to spells, Immutable, Can’t be tributed**  
**Armor 15**  
\[0/20\]  
**Activate:** choose to give both players **5, 10,** or **15 credits**  
**Death:** Give **100 credits** to both players  
\~\~\~  
**Can’t be in defense position**  
**Can’t attack, Immune to spells, Immutable, Can’t be tributed**  
**Armor 30**  
\[0/40\]  
**Activate:** choose to give players **5, 10,** or **15 credits,** your opponent gets **5** less  
**Death:** Give **200 credits** to both players

**(3) Core System, Spell, Epic, Stinkyscape, \#117**  
Bounce all permanents, lose 2 mana next turn   
\~\~\~  
Bounce all enemy permanents, lose 2 mana next turn 

**(3) Secure System, Spell, Epic, Stinkyscape, \#118**  
Set all units’ attack to 0, lose 1 mana next turn   
\~\~\~  
Set all enemy unit’s attack to 0, lose 1 mana next turn 

**(3) Unsecure System, Spell, Epic, Stinkyscape, \#119**  
Double the attack of all units, all units gain **first strike**  
\~\~\~  
Double the attack of all units, your units gain **first strike**

**(3) Wild System, Spell, Epic, Stinkyscape, \#120**  
All units attack each other until only one survives, gain 1 mana next turn  
*{allies can attack each other, max 20 attacks}*  
\~\~\~  
All units attack each other until only one survives, gain 1 mana next turn  
*{allies can attack each other, max 20 attacks}*

**(1) System Schematic, Spell, Epic, Stinkyscape, \#121**  
Add a **Core System, Secure System, Unsecure System,** or **Wild System** to your hand  
\~\~\~  
Add a **Radiant** **Core System, Secure System, Unsecure System,** or **Wild System** to your hand

**(1) Vamp Rounds, Unit, Common, Stinkyscape, \#122**  
2/1  
**Starship Piece**  
**Equip**  
**Lifesteal**  
\~\~\~  
4/2  
**Starship Piece**  
**Equip**  
**Lifesteal**

**(1) Frost Module, Unit, Common, Stinkyscape, \#122**  
2/1  
**Starship Piece**  
**Equip**  
**Activate:** Freeze an enemy  
\~\~\~  
4/2  
**Starship Piece**  
**Equip**  
**Activate 2:** Freeze an enemy

**(1) Gokugains, Unit, Rare, Stinkyscape, \#123**  
\[2/1\]  
**Cloaked**  
**Cry:** a random unit in your opponents hand is nerfed 3 times secretly *{nerfs avoid cost, opponent does not see the nerfs until they play the card}*  
\~\~\~  
\[4/2\]  
**Cloaked**  
**Cry:** a random unit in your opponents hand is nerfed 30 times secretly *{nerfs avoid cost, opponent does not see the nerfs until they play the card}*

**(2) Jlockeed’s Shipbuilding Grant, Spell, Common, Jlockeed’s, Stinkscape, \#124**  
Add 3 random **Starship Pieces** to your hand  
\~\~\~  
Add 3 random **Radiant** **Starship Pieces** to your hand

**(2) Jlockeed’s Spacerace Propaganda, Spell, Rare, Stinkscape, \#125**  
**Activate:** add a random **Starship Piece** to your hand  
\~\~\~  
**Activate:** add a random **Radiant** **Starship Piece** to your hand

**(2) Jlockeed’s Permanent Procurement Lobbying**, **Epic, Jlockeed, Spell, Stinkyscape, \#126**  
For the rest of the game, at the **start of your turn** add a random **Starship Piece** card to your hand.  
\~\~\~  
For the rest of the game, at the **start of your turn** add a random **Radiant**  **Starship Piece** card to your hand, it costs (1) less.

**(3 embiggen 4\) Jlockeed’s Siege Tank**, **Rare, Jlockeed, Unit, Stinkyscape, \#127**  
\[8/5\]  
**End of turn:** attacks a **random** enemy  
**Embiggen:** gain **Starship Piece**  
\~\~\~  
\[16/10\]  
**End of turn:** attacks a **random** enemy  
**Embiggen:** gain **Starship Piece**

**(2 embiggen 3\) Jlockeed’s Yamato Cannon**, **Rare, Jlockeed, Unit, Stinkyscape, \#128**  
\[4/1\]  
**Ranged, Cleave**  
**Embiggen:** gain **Starship Piece**  
\~\~\~  
\[8/2\]  
**Ranged, Cleave**  
**Embiggen:** gain **Starship Piece**

**(1 embiggen 2\) Jlockeed’s Missile Pod**, **Epic, Jlockeed, Unit, Stinkyscape, \#129**  
\[2/1\]  
**End of turn:** deal floor(attack / 4\) to all enemies  
**Embiggen:** gain **Starship Piece** and **Equip**  
\~\~\~  
\[4/2\]  
**End of turn:** deal floor(attack / 3\) to all enemies  
**Embiggen:** gain **Starship Piece** and **Equip**

**(1) Jlockeed’s Starship Retrofitting Program**, **Epic, Jlockeed, Spell, Stinkyscape, \#130**  
Add a random **Jlockeed** card to your hand, it gains **Starship Piece**  
\~\~\~  
Add a random **Radiant Jlockeed** card to your hand, it gains **Starship Piece,** and costs (1) less

**(1) Spacerace Stimulus, Field Trap, Common, Stinkyscape, \#131**  
6/6  
**Animated**  
**Starship Piece**  
Reveals when either player launches a **Starship**.  
\~\~\~  
12/12  
**Animated**  
**Starship Piece**  
Reveals when either player launches a **Starship** or plays a **Starship Piece**.

**(2) Defensive Crystal**, **Common, Unit, Stinkyscape, \#132**  
\[3/6\]  
**Starship Piece**  
**Death:** Give your hero 1 armor.  
\~\~\~  
\[6/12\]  
**Starship Piece**  
**Death:** Give your hero 2 armor.

**(2) Ultra Capacitor**, **Common, Unit, Stinkyscape, \#133**  
\[2/2\]  
**Starship Piece**  
When launched: gain \+1/+1 per **starship piece** for each other **starship piece** launched  
\~\~\~  
\[4/4\]  
**Starship Piece**  
When launched: gain \+2/+2 per **starship piece** for each **starship piece** launched

**(1) Felinor Containment Chamber**, **Common, Unit, Stinkyscape, \#134**  
\[2/1\]  
**Starship Piece**  
Death: Summon a **Felinor** token with stats equal to this units  
\~\~\~  
\[4/2\]  
**Starship Piece**  
Death: Summon a **Felinor** token with stats equal to twice this units, it gains **Divine Shield**

**(1) Sacked, Trap, Common,** **Stinkyscape, \#135**  
Reveals when your opponent plays a Unit. Stuff it in a 0/4 Sack.  
\~\~\~  
Reveals when your opponent plays a Unit. Stuff it in a 0/4 Sack for yourself.

**(1) Sack, Unit, Token,** **Common,** **Stinkyscape, \#135.1**  
0/4  
**Death:** Summon the Unit stuffed into this.  
\~\~\~  
0/8  
**Brittle 1**  
**Death:** Summon the Unit stuffed into this.

**(2) Magic Module**, **Common, Unit, Stinkyscape, \#136**  
\[4/3\]  
**Starship Piece**  
**Equip**  
**Spell damage \+2**  
When launched: Add a random spell to your hand that deals damage  
\~\~\~  
\[8/6\]  
**Starship Piece**  
**Equip**  
**Spell damage \+4**  
When launched: Add two random spells to your hand that deals damage

**(4) Firststrike Capability**, **Common, Unit, Stinkyscape, \#137**  
\[9/9\]   
**First strike**  
**Cry:** If you are building a ship **Launch** it and activate *{discover}* a **War doctrine**  
\~\~\~  
\[18/18\]   
**First strike**  
**Cry:** If you are building a ship **Launch** it and activate *{discover}* a **Radiant** **War doctrine**

**(4) Land Doctrine**, **Legendary, War Doctrine, Spell, Stinkyscape, \#137.1**  
For the rest of the game your units have **trample**, your **Starship** attacks a random enemy unit  
\~\~\~  
For the rest of the game your units have **trample**, your **Starship** attacks all enemy units

**(4) Sea Doctrine**, **Legendary, War Doctrine, Spell, Stinkyscape, \#137.2**  
For the rest of the game your units have **deft**, your heal your hero for twice the amount of health as your **Starship**  
\~\~\~  
For the rest of the game your units have **deft**, your heal your hero for five times the amount of health as your **Starship**

**(4) Air Doctrine**, **Legendary, War Doctrine, Spell, Stinkyscape, \#137.3**  
For the rest of the game your units have **rush**, you deal damage split randomly among all enemies equal to your **Starship’s** attack  
\~\~\~  
For the rest of the game your units have **rush**, you deal damage split randomly among all enemies equal to twice your **Starship’s** attack

**(4) Space Doctrine**, **Legendary, War Doctrine, Spell, Stinkyscape, \#137.4**  
For the rest of the game your units have **flying**, fill your hand with the **Starship Pieces** that are part of the launched **Starship**   
\~\~\~  
For the rest of the game your units have **flying**, fill your hand with the **Starship Pieces** that are part of the launched **Starship**, they cost (1)  
   
**(1) Pocket Peptides**, **Common, Trap, Stinkyscape, \#138**  
Reveals when one of your Units is about to die by combat. Cast **CN Peptides** on it.  
\~\~\~  
Reveals when one of your Units is about to die by combat. Cast Radiant **CN Peptides** on all your Units.

**(2) Wormhole, Rare, Trap, Stinkyscape, \#139**  
Reveals when your opponent casts a beneficial Spell on themselves or a permanent they control. Cast it beneficially for yourself instead.  
\~\~\~  
Reveals when your opponent casts a beneficial Spell on themselves or a permanent they control. Cast its Radiant version beneficially for yourself instead.

**(2) Rush Token Launcher, Unit, Rare, Weapon, Stinkyscape, \#140**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target an enemy, summon a **Rush** token to attack it *{obey’s taunt}*  
*{deals half damage to heroes}*  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target an enemy, summon a **Radiant** **Rush** token to attack it *{obey’s taunt}*  
*{deals half damage to heroes}*

**(2) Fruit Launcher, Unit, Rare, Weapon, Stinkyscape, \#141**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target a permanent, cast a **random** fruit  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target a permanent, cast a **random Radiant** fruit

**(3) Jade Launcher, Unit, Epic, CN, Weapon, Stinkyscape, \#142**  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target an enemy, shoot a **Auspicious Rock** at it *{deal 2 damage, you gain the reward}*   
\~\~\~  
\[0/12\]  
**Starship Piece**  
**Equip**  
**Activate 2:** Target an enemy, shoot a **Auspicious Rock** at it *{deal 2 damage, you gain the reward}* 

**(3) Book Launcher, Unit, Epic, Weapon, Stinkyscape, \#143**  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target an enemy, cast a random harmful book on it  
**Activate 1:** Target an ally, cast a random helpful book on it  
\~\~\~  
\[0/12\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target an enemy, cast a random **Radiant** harmful book on it  
**Activate 1:** Target an ally, cast a random **Radiant** helpful book on it  
   
**(1) Prepare For Liftoff**, **Common, Trap, Stinkyscape, \#144**  
**Tradable**  
**Flashback**  
Reveals when one of your **Starship Pieces** dies. Your Starship launch costs (0).  
\~\~\~  
**Tradable**  
**Flashback**  
Reveals when one of your **Starship Pieces** dies. Resummon it. Your Starship launch costs (0).

**(4) Fortify Flesh, Spell, Rare, Stinkyscape, \#145**  
**Gain 3\~5 Armor**  
**\~\~\~**  
**Gain 3\~7 Armor**  
**Lucky 1**

**(0) Flash Freeze, Spell, Rare, Stinkyscape, \#146**  
**Cast on draw: Freeze** all permanents   
\~\~\~  
**Cast on draw: Freeze** all enemy permanents 

**(0) IN33DH3, Unit, Legendary, Catalyst, Human, Stinkyscape, \#147**  
\[1/2\]  
**Cry:** For the rest of your turn an **AI** will help you make the **Pareto Optimal** decisions  
**Death:** Shuffle **IN33DH3 Prime** into your deck  
\~\~\~  
\[2/4\]  
**Cry:** For the rest of your turn an **AI** will help you make the **Pareto Optimal** decisions  
**Death:** Shuffle a **Radiant** **IN33DH3 Prime** into your deck

**(4) IN33DH3 Prime, Unit, Legendary, Prime, Human, Token, Stinkyscape, \#147.1**  
\[9/5\]  
**Cry:** Summon a **Wardrum Blob**  
\~\~\~  
\[18/10\]  
**Cry:** Summon a **Radiant** **Wardrum Blob**

**(2) Wardrum Blob, Field Spell, Legendary, Token, Stinkyscape, \#147.2**  
**Aura:** An **AI** will always help you make the **Pareto Optimal** decisions  
**Start of Turn:** Do something helpful *{should read board state and be a quasi zeph and do actions for no additional cost}*  
\~\~\~  
**Aura:** An **AI** will always help you make the **Pareto Optimal** decisions  
**Start of Turn:** Do something REALLY helpful *{should read board state and be a quasi zeph and do actions for no additional cost}*

**(1) Alteran Reach, Field Spell, Mythic, Wincon, Quickdraw, Stinkyscape, \#148**  
**Indestructable**  
**Activate Infinite (cost 1 mana):** Traverse the endless expanse to get closer to the **edge of the universe**  
*{Open up a Pseudo Starscape Map wherein the player can traverse, there should be many nodes and to reach the edge of the universe it should take at least 20 travels, there should be rewards along the way, think Reach to the Summit}*   
\~\~\~  
**Indestructable**  
Your first Traversal per turn is Free  
**Activate Infinite (cost 1 mana):** Traverse the endless expanse to get closer to the **edge of the universe**

**(0) Edge of the Universe, Spell, Mythic, Token, Wincon, Stinkyscape, \#148.1**  
**Escape the** **Universe** and **Win the Game**   
{Your opponent does not lose any elo since you fled}  
*\~\~\~*  
**Escape the** **Universe** and **Win the Game**  
{You may choose if your opponent loses elo since you fled}

**(3) Wildar, Spell, Legendary, Stinkyscape, \#149**  
Add a **Call to Chaos** that costs (0) to your opponents hand then add a **Radiant Call to Chaos** to your hand  
\~\~\~  
Add a **Radiant Call to Chaos** to your hand

**(4) Call to Chaos (Stinky Edition) Spell, Call to Chaos, Stinkyscape, Legendary, \#150**  
???  
*(One of the following random effects)*

- Add Radiant **ETHICS** and **Vanguard Coalition** to your hand  
- Add a **Very Dandy Lad** and an **Arctic Citadel** to your hand, they cost (0)  
- Summon 3 random Quests or Wincons.  
- Add an Acclaimed **Stinkyscape** Unit to your hand. Refresh mana so you can play it.  
- Place all enemies in attack position and Freeze them.  
- Fill your board with random Human **Stinkyscape** Units.  
- **Allure** all enemy Units.  
- For the rest of the game, all Acclaimed cards have “**Death**: Cast a random Call to Chaos”.  
- Summon a **Stinkyscape Golem**  
- Cast a random **Call to Chaos**

\~\~\~  
\!\!\!  
*(three effects)*

**(4) Stinkyscape Golem Unit, Token, Stinkyscape, Legendary, \#150.1**  
10/10  
**Starpiece, Flying, Ranged, Rush**  
\~\~\~   
20/20  
**Starpiece, Flying, Ranged, Rush**

**(4) Call to Chaos (Scape Edition) Spell, Call to Chaos, Stinkyscape, Legendary, \#151**  
???  
*(One of the following random effects)*

- Trigger any **Anomaly**  
- Summon **3 Drone Corvettes**  
- Summon the **Drone Battleship**  
- Cast two of the following: **Core System, Secure System, Unsecure System, Wild System**  
- Add **4** random **Starship Pieces** to your hand, they cost **0**  
- Fill your board with **Drone Interceptors**, they have **stack**   
- **Launch** a ship with **4 Random Starpieces** fused into it   
- Add 2 **Radiant Weapons** to your hand they cost (0)  
- For the rest of the game the **Marketplace** offers you a **Call to Chaos** at a very cheap rate *(always 20 **Credits)***  
- Cast a random **Call to Chaos**

\~\~\~  
\!\!\!  
*(three effects)*

**(4) Sanctum, Field Spell, Stinkscape, Legendary, \#152**  
**Indestructable**  
**Aura:** all cards in your **Deck** and **GY Flashback**  
\~\~\~  
**Indestructable**  
**Cry:** gain (4) mana  
**Aura:** all cards in your **Deck** and **GY Flashback**

**(0) Stimmycheck, Spell, Stinkscape, Legendary, \#153**  
Gain 1 mana, cards in your hand and deck have a 15% chance for **Inflation** to hit which causes them to cost (1) more. At the end of your turn, add a **Stimmycheck** to your hand with a \+10% chance at **Inflation**  
\~\~\~  
**Lucky 1**  
Gain 1 mana, cards in your hand and deck have a 15% chance for **Inflation** to hit which causes them to cost (1) more. At the end of your turn, add a **Radiant Stimmycheck** to your hand with a \+10% chance at **Inflation**

**(0 embiggen 1\) Dice of Doom, Spell, Stinkyscape, Quickdraw, Mythic, \#154**  
Roll a 100 sided die, if it lands on 67 add every **Terminus** card to your hand. They cost (0). **End of your turn** add this back to your hand.  
**Embiggen** permanently gain **Lucky \+1**  
\~\~\~  
Roll a 80 sided die, if it lands on 67 add every **Terminus** card to your hand. They cost (0). **End of your turn** add this back to your hand.  
**Embiggen** permanently gain **Lucky \+1**

**(2) Comet Crash**, **Field Spell, Rare, Stinkyscape, \#155**  
**Crater**  
**Brittle 1**  
**Death:** Add 3 Stinkyscape resources to your hand.  
\~\~\~  
**Crater**  
**Brittle 1**  
**Death:** Add 3 Radiant Stinkyscape resources to your hand.

**(1) Catalyst Comet, Epic, Spell, Stinkyscape, \#156**  
Add a **Catalyst** card to your hand. Shuffle this back into your deck.  
\~\~\~  
Add a **Radiant Catalyst** card to your hand. Shuffle back into your deck.

**(1) The Gauntlet, Field Spell, Epic, Stinkyscape, \#157**  
**Brittle 3**  
The middle 3 unit lanes are trapped in a Gauntlet. No other unit can target them and they can only target each other.  
\~\~\~  
**Brittle 3**  
The middle 3 unit lanes are trapped in a Gauntlet. No other unit can target them and they can only target each other.   
**Activate:** Choose to either to add **Brittle** **\+2** to this or **Tribute** this

**(3) Stratos Swarm, Spell, Rare, Stinkyscape, \#158**  
Fill your board with **Startos**  
\~\~\~  
Fill your board with **Radiant Startos**

**(3) ETHICS, Spell, Rare, Stinkyscape, \#159**  
Add **iKwzki, Shockwave447, xxFreshJack, Tnack Trav, Kwzka,** and **Tylannt** to your hand  
\~\~\~  
Add Radiant **iKwzki, Shockwave447, xxFreshJack, Tnack Trav, Kwzka** and **Tylannt** to your hand

**(3) Golden Accord, Spell, Rare, Stinkyscape, \#160**  
Add a **Paragon mk1, Paragon mk2, Paragon X,** and **Falcon** to your hand  
\~\~\~  
Add a **Radiant** **Paragon mk1, Paragon mk2, Paragon X,** and **Falcon** to your hand

**(3) Pegasus, Spell, Rare, Stinkyscape, \#161**  
Add **A Very Dandy Lad, Tmack Trav, Gokugains** and **Arctic Citadel** to your hand  
\~\~\~  
Add **Radiant A Very Dandy Lad, Tmack Trav, Gokugains** and **Arctic Citadel** 

**(3) Star-jumper core, Spell, Rare, Stinkyscape, \#162**  
Add a **Badger, Honey Badger, Ozark, Orca,** and **Galaxy** to your hand  
\~\~\~  
Add a **Badger, Honey Badger, Ozark, Orca,** and **Galaxy** to your hand

**(1) Swear Loyalities, Spell, Rare, Stinkyscape, \#163**  
Add either a **Vanguard Coalition, ETHICS, Golden, Pegasus,** or **Star-jumper Core** to your hand. **Exile** the other options you didn’t choose. *{wherever they are}*  
\~\~\~  
Add either a **Radiant** **Vanguard Coalition, ETHICS, Golden, Pegasus,** or **Star-jumper Core** to your hand. **Exile** the other options you didn’t choose. *{wherever they are}*

**(2) The Syndicate, Spell, Epic, Stinkyscape, \#164**  
**Tradable**  
**Aura:** Your cards have **Tradable,** whenever you a **Trade** a card it has a 30% chance to be exiled and a 30% chance to become **Radiant**  
\~\~\~  
**Tradable**  
**Aura:** Your cards have **Tradable,** whenever you a **Trade** a card it has a 30% chance to be exiled and a 70% chance to become **Radiant**

**(1) The Trade Union, Field Spell, Rare, Stinkyscape, \#165**  
**Tradable**  
**Aura:** Your cards have **Tradable**  
\~\~\~  
**Tradable**  
**Aura:** Your cards have **Tradable,** you can **Trade** up to two cards a turn

**(1) Scanning Probe Mk1, Spell, Common, Stinkyscape, \#166**  
5% chance to add a **Radiant** **Anomaly Found\!** to your hand  
20% chance to add a **Anomaly Found\!** to your hand  
Else: future **Scanning Probes** have \+1% & \+4% chance to find an **Anomaly**  
**\~\~\~**  
**Lucky 1**  
5% chance to add a **Radiant** **Anomaly Found\!** to your hand  
20% chance to add a **Anomaly Found\!** to your hand  
Else: future **Scanning Probes** have \+1% & \+4% chance to find an **Anomaly**

**(2) Scanning Probe Mk2, Spell, Rare, Stinkyscape, \#167**  
**Lucky 1**  
5% chance to add a **Radiant** **Anomaly Found\!** to your hand  
20% chance to add a **Anomaly Found\!** to your hand  
Else: future **Scanning Probes** have \+1% & \+4% chance to find an **Anomaly**  
**\~\~\~**  
**Luck 3**  
5% chance to add a **Radiant** **Anomaly Found\!** to your hand  
20% chance to add a **Anomaly Found\!** to your hand  
Else: future **Scanning Probes** have \+1% & \+4% chance to find an **Anomaly**

**(2) Drone Hanger, Field Spell, Rare, Stinkyscape, \#168**  
**Start of your turn:** Summon a **Drone Interceptor**  
\~\~\~  
**Start of your turn:** Summon a **Radiant** **Drone Interceptor**

**(1) Tiny Asteroid, Unit, Common, Stinkyscape, \#169**  
0/4  
**Can’t Attack**  
**Can’t be in Defense Positio**n  
**Death**: Gain 1 Stinkyscape Resource  
**Activate**: Take 2 damage. Tribute this.  
\~\~\~  
0/8  
**Can’t Attack**  
**Can’t be in Defense Positio**n  
**Death**: Gain 1 Radiant Stinkyscape Resource  
**Activate**: Tribute this.

**(1) Medium Asteroid, Unit, Common, Stinkyscape, \#170**  
0/6  
**Can’t Attack**  
**Can’t be in Defense Positio**n  
**Death**: Gain 2 Stinkyscape Resource  
**Activate**: Take 3 damage. Tribute this.  
\~\~\~  
0/12  
**Can’t Attack**  
**Can’t be in Defense Positio**n  
**Death**: Gain 2 Radiant Stinkyscape Resources.  
**Activate**: Tribute this.

**(3) Large Asteroid, Unit, Rare, Stinkyscape, \#171**  
0/8  
**Can’t Attack**  
**Can’t be in Defense Positio**n  
**Death**: Gain 3 Stinkyscape Resources  
**Activate**: Take 4 damage. Tribute this.  
\~\~\~  
0/16  
**Can’t Attack**  
**Can’t be in Defense Positio**n  
**Death**: Gain 3 Radiant Stinkyscape Resources.  
**Activate**: Tribute this.

**(4) Huge Asteroid, Unit, Epic, Stinkyscape, \#172**  
0/10  
**Can’t Attack**  
**Can’t be in Defense Positio**n  
**Death**: Gain 5 Stinkyscape Resources  
**Activate**: Take 5 damage. Tribute this.  
\~\~\~  
0/20  
**Can’t Attack**  
**Can’t be in Defense Positio**n  
**Death**: Gain 5 Radiant Stinkyscape Resources.  
**Activate**: Tribute this.

**(3) Abandoned Spice Tower, Unit, Rare, Stinkyscape, \#173**  
0/8  
**Can’t Attack**  
**Can’t be in Defense Position**  
**Death:** Add a three **Spice** to your hand  
**Activate:** Take 4 damage, Tribute this  
\~\~\~  
0/16  
**Can’t Attack**  
**Can’t be in Defense Position**  
**Death:** Add a three **Radiant** **Spice** to your hand  
**Activate:** Take 4 damage, Tribute this


```
